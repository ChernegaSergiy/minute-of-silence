//! System-tray icon setup and context-menu event handling.

use crate::app::next_skip_date;
use crate::platform::is_dark_mode;
use rust_i18n::t;
use tauri::{
    App, Emitter, Listener, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::AppState;

/// Build and register the system-tray icon for `app`.
pub fn build_tray(app: &App) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let is_skipped = {
        let inner = state.lock();
        let target = next_skip_date(chrono::Local::now(), inner.settings.weekdays_only);
        inner.settings.skip_date == Some(target)
    };
    let initial_label = if is_skipped {
        t!("unskip_next")
    } else {
        t!("skip_next")
    };

    let open_i = MenuItem::with_id(app, "open", t!("open"), true, None::<&str>)?;
    let skip_i = MenuItem::with_id(app, "skip_next", initial_label, true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", t!("quit"), true, None::<&str>)?;

    // Synchronize tray skip menu item text whenever status changes (e.g. from Settings UI)
    let skip_item_sync = skip_i.clone();
    let app_handle = app.handle().clone();
    app.listen("status-updated", move |_| {
        let state = app_handle.state::<AppState>();
        let is_skipped = {
            let inner = state.lock();
            let target = next_skip_date(chrono::Local::now(), inner.settings.weekdays_only);
            inner.settings.skip_date == Some(target)
        };
        let label = if is_skipped {
            t!("unskip_next")
        } else {
            t!("skip_next")
        };
        let _ = skip_item_sync.set_text(label);
    });

    let menu = Menu::with_items(app, &[&open_i, &skip_i, &sep, &quit_i])?;

    let icon = if is_dark_mode() {
        tauri::include_image!("icons/tray-icon-32-light.png")
    } else {
        tauri::include_image!("icons/tray-icon-32-dark.png")
    };

    let skip_item = skip_i.clone();

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .tooltip(t!("tray_tooltip"))
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "skip_next" => {
                let state = app.state::<AppState>();
                let target = {
                    let inner = state.lock();
                    next_skip_date(chrono::Local::now(), inner.settings.weekdays_only)
                };

                let mut was_skipped = false;
                if let Err(e) = state.update_settings(|s| {
                    was_skipped = s.skip_date == Some(target);
                    s.skip_date = if was_skipped { None } else { Some(target) };
                }) {
                    log::warn!("Tray: failed to update skip_date: {e}");
                } else if was_skipped {
                    let _ = skip_item.set_text(t!("skip_next"));
                    log::info!("Tray: ceremony restored (skip removed)");
                } else {
                    let _ = skip_item.set_text(t!("unskip_next"));
                    log::info!("Tray: next ceremony skipped ({target})");
                }

                // Notify the frontend that the status has changed
                let _ = app.emit("status-updated", ());
            }
            "quit" => {
                log::info!("Quit requested via tray");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click toggles the main window.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
