//! Copies secrets to the clipboard through GTK, the toolkit Tauri's window is
//! built on, so it works natively on Wayland, and clears them again after a
//! timeout if nothing else has been copied in the meantime.
//!
//! GNOME's compositor keeps clipboard contents after the app that copied them
//! exits, so a secret still on the clipboard is also cleared when Moneypenny
//! quits ([`clear_now`]); otherwise quitting within the timeout would leave it
//! there indefinitely.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use zeroize::Zeroizing;

/// How long a copied secret stays on the clipboard.
pub const CLEAR_AFTER: Duration = Duration::from_secs(15);

/// Bumped on every copy so only the most recent copy's timer clears.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The secret most recently copied, until it's been cleared.
static PENDING: Mutex<Option<Arc<Zeroizing<String>>>> = Mutex::new(None);

fn clipboard() -> gtk::Clipboard {
    gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD)
}

fn take_pending() -> Option<Arc<Zeroizing<String>>> {
    PENDING.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// Replaces the clipboard with an empty string, then gives up ownership. The
/// empty string matters: the compositor saves whatever it last read from us.
fn wipe(cb: &gtk::Clipboard) {
    cb.set_text("");
    cb.clear();
}

/// Puts `text` on the clipboard and schedules it to be cleared after
/// [`CLEAR_AFTER`], unless the user has copied something else by then.
pub fn copy_and_clear_later(app: &AppHandle, text: Zeroizing<String>) -> tauri::Result<()> {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let text = Arc::new(text);
    *PENDING.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&text));

    let to_copy = Arc::clone(&text);
    // GTK must be used from the main thread.
    app.run_on_main_thread(move || clipboard().set_text(&to_copy))?;

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(CLEAR_AFTER).await;
        if GENERATION.load(Ordering::SeqCst) != generation {
            return; // A newer copy owns the clipboard and its own timer.
        }
        drop(take_pending());
        let result = app.run_on_main_thread(move || {
            clipboard().request_text(move |cb, current| {
                if current == Some(text.as_str()) {
                    wipe(cb);
                }
            });
        });
        if let Err(err) = result {
            log::warn!("could not clear clipboard: {err}");
        }
    });
    Ok(())
}

/// Clears a copied secret that's still on the clipboard. Called on the main
/// thread as the app exits. Briefly runs the GTK main loop so the compositor
/// can read the empty replacement before the process goes away.
pub fn clear_now() {
    let Some(text) = take_pending() else { return };
    let cb = clipboard();
    if cb.wait_for_text().as_deref() != Some(text.as_str()) {
        return; // Something else was copied since; leave it alone.
    }
    wipe(&cb);
    let context = gtk::glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_millis(300);
    while Instant::now() < deadline {
        if !context.iteration(false) {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
