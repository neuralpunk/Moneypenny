//! Live updates: watches the Secret Service for changes made by any app
//! (items or keyrings created, changed, deleted, locked or unlocked) and for
//! the daemon restarting, so the UI can refresh itself.

use std::num::NonZeroUsize;
use std::time::Duration;

use futures_util::StreamExt;
use oo7::zbus::{self, MatchRule, MessageStream, message::Type};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::KeyringState;

/// Event emitted to the frontend after a burst of changes.
pub const CHANGED_EVENT: &str = "keyring-changed";

/// Signals arriving within this window are reported as one change.
const DEBOUNCE: Duration = Duration::from_millis(150);
const RETRY_AFTER: Duration = Duration::from_secs(3);

const SECRETS_NAME: &str = "org.freedesktop.secrets";
const SECRETS_PATH: &str = "/org/freedesktop/secrets";
const NAME_OWNER_CHANGED: &str = "NameOwnerChanged";

/// What a burst of signals contained.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Change {
    /// oo7-daemon started, stopped or was replaced.
    pub daemon_changed: bool,
}

fn member(msg: &zbus::Message) -> Option<String> {
    msg.header().member().map(|m| m.as_str().to_owned())
}

/// Calls `on_change` once per burst of Secret Service changes, until the
/// connection closes. Prompt completions are ignored: whatever they change
/// is announced by its own signal.
pub async fn watch_changes(
    cnx: &zbus::Connection,
    mut on_change: impl FnMut(Change),
) -> zbus::Result<()> {
    let queue = NonZeroUsize::new(256);
    let secrets = MatchRule::builder()
        .msg_type(Type::Signal)
        .path_namespace(SECRETS_PATH)
        .build()?;
    let owner = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.DBus")
        .member(NAME_OWNER_CHANGED)
        .arg(0, SECRETS_NAME)
        .build()?;
    let secrets = MessageStream::for_match_rule(secrets, cnx, queue).await?;
    let owner = MessageStream::for_match_rule(owner, cnx, queue).await?;
    let mut messages = futures_util::stream::select(secrets, owner);

    while let Some(msg) = messages.next().await {
        let mut change = Change::default();
        match member(&msg?).as_deref() {
            None | Some("Completed") => continue,
            Some(NAME_OWNER_CHANGED) => change.daemon_changed = true,
            Some(_) => {}
        }
        // Coalesce the burst (e.g. ItemCreated + PropertiesChanged + ...).
        loop {
            match tokio::time::timeout(DEBOUNCE, messages.next()).await {
                Ok(Some(Ok(next))) => {
                    change.daemon_changed |= member(&next).as_deref() == Some(NAME_OWNER_CHANGED);
                }
                Ok(Some(Err(_))) => {}
                Ok(None) => {
                    on_change(change);
                    return Ok(());
                }
                Err(_) => break,
            }
        }
        on_change(change);
    }
    Ok(())
}

/// Runs for the app's lifetime, reconnecting if the session bus connection
/// fails, and emits [`CHANGED_EVENT`] on every change.
pub async fn run(app: AppHandle) {
    loop {
        let result = async {
            let cnx = zbus::Connection::session().await?;
            watch_changes(&cnx, |change| {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if change.daemon_changed {
                        // The old daemon's encryption session died with it;
                        // drop it now so the next save doesn't fail on it.
                        app.state::<KeyringState>().reset().await;
                    }
                    if let Err(err) = app.emit(CHANGED_EVENT, ()) {
                        log::warn!("could not emit keyring change: {err}");
                    }
                });
            })
            .await
        }
        .await;
        if let Err(err) = result {
            log::warn!("watching the Secret Service failed, retrying: {err}");
        }
        tokio::time::sleep(RETRY_AFTER).await;
    }
}
