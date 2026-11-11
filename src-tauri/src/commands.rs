//! Tauri commands exposing the read-only keyring API to the frontend.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, State};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

use crate::clipboard;

use crate::keyring::{
    CollectionInfo, ItemInfo, ItemUpdate, Keyring, KeyringError, Result, SecretValue,
};

/// Lazily connected Secret Service client, rebuilt if the connection is lost
/// (for example after oo7-daemon restarts).
#[derive(Default)]
pub struct KeyringState {
    keyring: Mutex<Option<Arc<Keyring>>>,
}

impl KeyringState {
    async fn get(&self) -> Result<Arc<Keyring>> {
        let mut guard = self.keyring.lock().await;
        if let Some(keyring) = guard.as_ref() {
            return Ok(Arc::clone(keyring));
        }
        let keyring = Arc::new(Keyring::connect().await?);
        *guard = Some(Arc::clone(&keyring));
        Ok(keyring)
    }

    /// Drops the cached connection, e.g. when oo7-daemon restarts: its
    /// encryption session died with the old daemon.
    pub async fn reset(&self) {
        *self.keyring.lock().await = None;
    }

    /// Runs `op`, reconnecting and retrying once if the connection was lost.
    async fn run<T, F, Fut>(&self, op: F) -> Result<T>
    where
        F: Fn(Arc<Keyring>) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        match op(self.get().await?).await {
            Err(err) if err.is_connection_lost() => {
                log::info!("Secret Service connection lost, reconnecting");
                self.reset().await;
                op(self.get().await?).await
            }
            result => result,
        }
    }

    /// Runs a mutation once. Never retried, so a dropped connection can't
    /// apply a change twice; the connection is just rebuilt for next time.
    async fn mutate<T, Fut>(&self, op: impl FnOnce(Arc<Keyring>) -> Fut) -> Result<T>
    where
        Fut: Future<Output = Result<T>>,
    {
        let result = op(self.get().await?).await;
        if matches!(&result, Err(err) if err.is_connection_lost()) {
            self.reset().await;
        }
        result
    }
}

fn log_err<T>(what: &str, result: Result<T>) -> Result<T> {
    if let Err(err) = &result
        && !matches!(err, KeyringError::Locked | KeyringError::Dismissed)
    {
        log::warn!("{what} failed: {err}");
    }
    result
}

#[tauri::command]
pub async fn list_collections(state: State<'_, KeyringState>) -> Result<Vec<CollectionInfo>> {
    log_err(
        "list_collections",
        state.run(|k| async move { k.collections().await }).await,
    )
}

#[tauri::command]
pub async fn list_items(
    state: State<'_, KeyringState>,
    collection_id: String,
) -> Result<Vec<ItemInfo>> {
    log_err(
        "list_items",
        state
            .run(|k| {
                let id = collection_id.clone();
                async move { k.items(&id).await }
            })
            .await,
    )
}

#[tauri::command]
pub async fn get_secret(
    state: State<'_, KeyringState>,
    collection_id: String,
    item_id: String,
) -> Result<SecretValue> {
    log_err(
        "get_secret",
        state
            .run(|k| {
                let (c, i) = (collection_id.clone(), item_id.clone());
                async move { k.secret(&c, &i).await }
            })
            .await,
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    /// Seconds until the clipboard is cleared.
    pub clear_after_secs: u64,
}

/// Copies a text secret to the clipboard without sending it to the frontend.
#[tauri::command]
pub async fn copy_secret(
    app: AppHandle,
    state: State<'_, KeyringState>,
    collection_id: String,
    item_id: String,
) -> Result<CopyResult> {
    let secret = get_secret(state, collection_id, item_id).await?;
    let SecretValue::Text { value } = secret else {
        return Err(KeyringError::Backend(
            "Only text secrets can be copied".into(),
        ));
    };
    if let Err(err) =
        clipboard::copy_and_clear_later(&app, Zeroizing::new(value.as_str().to_owned()))
    {
        log::warn!("copy_secret failed: {err}");
        return Err(KeyringError::Backend(err.to_string()));
    }
    Ok(CopyResult {
        clear_after_secs: clipboard::CLEAR_AFTER.as_secs(),
    })
}

#[tauri::command]
pub async fn unlock_collection(
    state: State<'_, KeyringState>,
    collection_id: String,
) -> Result<()> {
    log_err(
        "unlock_collection",
        state
            .mutate(|k| async move { k.unlock(&collection_id).await })
            .await,
    )
}

#[tauri::command]
pub async fn lock_collection(state: State<'_, KeyringState>, collection_id: String) -> Result<()> {
    log_err(
        "lock_collection",
        state
            .mutate(|k| async move { k.lock(&collection_id).await })
            .await,
    )
}

/// Returns the new keyring's id.
#[tauri::command]
pub async fn create_collection(state: State<'_, KeyringState>, label: String) -> Result<String> {
    log_err(
        "create_collection",
        state
            .mutate(|k| async move { k.create_collection(&label).await })
            .await,
    )
}

#[tauri::command]
pub async fn delete_collection(
    state: State<'_, KeyringState>,
    collection_id: String,
) -> Result<()> {
    log_err(
        "delete_collection",
        state
            .mutate(|k| async move { k.delete_collection(&collection_id).await })
            .await,
    )
}

#[tauri::command]
pub async fn change_password(state: State<'_, KeyringState>, collection_id: String) -> Result<()> {
    log_err(
        "change_password",
        state
            .mutate(|k| async move { k.change_password(&collection_id).await })
            .await,
    )
}

#[tauri::command]
pub async fn create_item(
    state: State<'_, KeyringState>,
    collection_id: String,
    label: String,
    attributes: BTreeMap<String, String>,
    secret: String,
) -> Result<ItemInfo> {
    let secret = Zeroizing::new(secret);
    log_err(
        "create_item",
        state
            .mutate(|k| async move {
                k.create_item(&collection_id, &label, attributes, secret)
                    .await
            })
            .await,
    )
}

#[tauri::command]
pub async fn update_item(
    state: State<'_, KeyringState>,
    collection_id: String,
    item_id: String,
    label: Option<String>,
    attributes: Option<BTreeMap<String, String>>,
    secret: Option<String>,
) -> Result<ItemInfo> {
    let update = ItemUpdate {
        label,
        attributes,
        secret: secret.map(Zeroizing::new),
    };
    log_err(
        "update_item",
        state
            .mutate(|k| async move { k.update_item(&collection_id, &item_id, update).await })
            .await,
    )
}

#[tauri::command]
pub async fn delete_item(
    state: State<'_, KeyringState>,
    collection_id: String,
    item_id: String,
) -> Result<()> {
    log_err(
        "delete_item",
        state
            .mutate(|k| async move { k.delete_item(&collection_id, &item_id).await })
            .await,
    )
}
