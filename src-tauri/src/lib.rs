mod clipboard;
mod commands;
mod desktop;
pub mod keyring;
pub mod watch;

/// App ID: GNOME matches windows to `.desktop` files by it.
pub const APP_ID: &str = "io.github.neuralpunk.Moneypenny";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // GTK uses the program name as the Wayland app ID (X11: WM_CLASS), so
    // GNOME can match the window to its launcher. Tauri's `enableGTKAppId`
    // does this through GApplication instead, but then a second launch
    // re-runs setup in the first instance and crashes it.
    #[cfg(target_os = "linux")]
    gtk::glib::set_prgname(Some(APP_ID));
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("moneypenny_lib=info,warn"),
    )
    .init();
    tauri::Builder::default()
        .manage(commands::KeyringState::default())
        .invoke_handler(tauri::generate_handler![
            desktop::desktop_settings,
            commands::list_collections,
            commands::list_items,
            commands::get_secret,
            commands::copy_secret,
            commands::unlock_collection,
            commands::lock_collection,
            commands::create_collection,
            commands::delete_collection,
            commands::change_password,
            commands::create_item,
            commands::update_item,
            commands::delete_item,
        ])
        .setup(|app| {
            tauri::async_runtime::spawn(desktop::watch(app.handle().clone()));
            tauri::async_runtime::spawn(watch::run(app.handle().clone()));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Moneypenny")
        .run(|_app, event| {
            // Don't leave a copied secret behind on the clipboard.
            if let tauri::RunEvent::Exit = event {
                clipboard::clear_now();
            }
        });
}
