// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![recursion_limit = "512"]

mod commands;
mod tray;

use commands::{config, keys, backup, system, settings};
use tauri::Manager;

#[cfg(target_os = "macos")]
use objc2_app_kit::NSApplication;
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;

fn main() {
    eprintln!("[MAIN] Starting application...");

    eprintln!("[MAIN] Building Tauri app...");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            // Removed startup test - load_config() manages its own stack internally
            eprintln!("[SETUP] Application initialized, ready to handle commands");

            // Initialize system tray
            tray::create_tray(app.handle())?;
            eprintln!("[SETUP] System tray initialized");

            // Intercept window close event to minimize to tray instead of exit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();

                        #[cfg(target_os = "macos")]
                        {
                            let mtm = unsafe { MainThreadMarker::new_unchecked() };
                            let app = NSApplication::sharedApplication(mtm);
                            app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Accessory);
                        }

                        eprintln!("[WINDOW] Close button clicked, minimized to tray and hid Dock icon");
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            config::test_simple,
            config::load_ssh_config,
            config::save_ssh_config,
            config::serialize_ssh_config,
            config::parse_ssh_config,
            config::add_host,
            config::update_host,
            config::delete_host,
            config::search_hosts,
            keys::scan_ssh_keys,
            keys::get_key_fingerprint,
            keys::generate_ssh_key,
            keys::delete_ssh_key,
            keys::read_public_key,
            backup::list_backups,
            backup::restore_backup,
            backup::delete_backup,
            system::open_config_dir,
            settings::load_settings,
            settings::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
