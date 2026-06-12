use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime, AppHandle,
};

#[cfg(target_os = "macos")]
use objc2_app_kit::NSApplication;
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let hide_i = MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_i, &hide_i, &quit_i])?;

    // Load and decode icon from embedded bytes - use icon.png for consistency with Dock icon
    let icon_bytes = include_bytes!("../icons/icon.png");
    let img = image::load_from_memory(icon_bytes)
        .map_err(|e| tauri::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let raw_pixels = rgba.into_raw();
    let icon = tauri::image::Image::new(&raw_pixels, width, height);

    let _tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => {
                eprintln!("[TRAY] Quit menu item clicked");
                app.exit(0);
            }
            "show" => {
                eprintln!("[TRAY] Show menu item clicked");

                #[cfg(target_os = "macos")]
                {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };
                    let app = NSApplication::sharedApplication(mtm);
                    app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Regular);
                }

                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "hide" => {
                eprintln!("[TRAY] Hide menu item clicked");

                #[cfg(target_os = "macos")]
                {
                    let mtm = unsafe { MainThreadMarker::new_unchecked() };
                    let app = NSApplication::sharedApplication(mtm);
                    app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Accessory);
                }

                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                eprintln!("[TRAY] Tray icon clicked");
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        #[cfg(target_os = "macos")]
                        {
                            let mtm = unsafe { MainThreadMarker::new_unchecked() };
                            let ns_app = NSApplication::sharedApplication(mtm);
                            ns_app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Accessory);
                        }

                        let _ = window.hide();
                    } else {
                        #[cfg(target_os = "macos")]
                        {
                            let mtm = unsafe { MainThreadMarker::new_unchecked() };
                            let ns_app = NSApplication::sharedApplication(mtm);
                            ns_app.setActivationPolicy(objc2_app_kit::NSApplicationActivationPolicy::Regular);
                        }

                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
