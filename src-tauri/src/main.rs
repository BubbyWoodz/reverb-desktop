#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Reverb desktop — a native Tauri shell around the Reverb web client.
//!
//! Architecture: the webview loads the user's Reverb server URL (a full
//! Reverb web client). This binary adds the native layer on top:
//! media keys, system tray, notifications, single-instance, autostart.
//! Playback control goes through `src/bridge.js`, injected into every page.

use std::sync::{Arc, Mutex};

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;

/// Managed state: the local setup-page URL, captured before any navigation.
struct SetupUrl(Mutex<Option<String>>);

fn main_window(app: &tauri::AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("main")
}

/// Evaluate a `window.__reverb` bridge call in the main webview.
fn eval_bridge(app: &tauri::AppHandle, call: &str) {
    if let Some(window) = main_window(app) {
        let js = format!(
            "if(window.__reverb&&window.__reverb.ready){{try{{window.__reverb.{call}}}catch(e){{}}}}"
        );
        let _ = window.eval(js.as_str());
    }
}

fn handle_media_key(app: &tauri::AppHandle, code: &Code) {
    match code {
        Code::MediaPlayPause => eval_bridge(app, "togglePlay()"),
        Code::MediaTrackNext => eval_bridge(app, "next()"),
        Code::MediaTrackPrevious => eval_bridge(app, "prev()"),
        Code::MediaStop => eval_bridge(app, "pause()"),
        _ => {}
    }
}

/// Called from the bridge when the playing track changes.
#[tauri::command]
fn notify_track_change(app: tauri::AppHandle, title: String, artist: String, album: String) {
    let body = if artist.is_empty() {
        if album.is_empty() {
            "Now playing".to_string()
        } else {
            album
        }
    } else if album.is_empty() {
        artist
    } else {
        format!("{artist} — {album}")
    };
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}

/// Show the window and navigate back to the local setup page.
fn goto_setup(app: &tauri::AppHandle) {
    let setup_url = app
        .state::<SetupUrl>()
        .0
        .lock()
        .ok()
        .and_then(|guard| guard.clone());
    if let (Some(window), Some(url)) = (main_window(app), setup_url) {
        let _ = window.show();
        let _ = window.set_focus();
        if let Ok(url) = url.parse() {
            let _ = window.navigate(url);
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(SetupUrl(Mutex::new(None)))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = main_window(app) {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        handle_media_key(app, &shortcut.key);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![notify_track_change])
        .setup(|app| {
            // ---- main window ----
            let window_icon =
                tauri::image::Image::from_bytes(include_bytes!("../icons/128x128.png"))
                    .expect("failed to load window icon");

            let window = WebviewWindowBuilder::new(
                app,
                "main",
                WebviewUrl::App("index.html".into()),
            )
            .title("Reverb")
            .icon(window_icon)
            .inner_size(1280.0, 800.0)
            .min_inner_size(960.0, 640.0)
            .decorations(true)
            .initialization_script(include_str!("../../src/bridge.js"))
            .on_window_event(|window, event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    // Hide to tray instead of quitting — Spotify-style.
                    let _ = window.hide();
                    api.prevent_close();
                }
            })
            .build()?;

            // Remember the local setup URL before any navigation happens.
            if let Ok(mut guard) = app.state::<SetupUrl>().0.lock() {
                *guard = window.url().ok().map(|u| u.to_string());
            }

            // ---- global media keys ----
            // Note: macOS may refuse media-key capture without accessibility
            // permission; failures are logged and the app keeps working.
            for key in [
                "MediaPlayPause",
                "MediaTrackNext",
                "MediaTrackPrevious",
                "MediaStop",
            ] {
                if let Err(e) = app.global_shortcut().register(key) {
                    eprintln!("[reverb] global shortcut '{key}' not registered: {e}");
                }
            }

            // ---- system tray ----
            let tray_icon =
                tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                    .expect("failed to load tray icon");

            let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);

            let show_item =
                MenuItem::with_id(app, "tray-show", "Show Reverb", true, None::<&str>)?;
            let playpause_item = MenuItem::with_id(
                app,
                "tray-playpause",
                "Play / Pause",
                true,
                None::<&str>,
            )?;
            let next_item =
                MenuItem::with_id(app, "tray-next", "Next track", true, None::<&str>)?;
            let prev_item =
                MenuItem::with_id(app, "tray-prev", "Previous track", true, None::<&str>)?;
            let autostart_item = Arc::new(CheckMenuItem::with_id(
                app,
                "tray-autostart",
                "Start on login",
                true,
                autostart_enabled,
                None::<&str>,
            )?);
            let server_item = MenuItem::with_id(
                app,
                "tray-server",
                "Change server…",
                true,
                None::<&str>,
            )?;
            let quit_item =
                MenuItem::with_id(app, "tray-quit", "Quit Reverb", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &show_item,
                    &PredefinedMenuItem::separator(app)?,
                    &playpause_item,
                    &next_item,
                    &prev_item,
                    &PredefinedMenuItem::separator(app)?,
                    autostart_item.as_ref(),
                    &server_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit_item,
                ],
            )?;

            let autostart_for_menu = Arc::clone(&autostart_item);
            let _tray = TrayIconBuilder::with_id(app, "main-tray")
                .icon(tray_icon)
                .tooltip("Reverb")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "tray-show" => {
                        if let Some(w) = main_window(app) {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "tray-playpause" => eval_bridge(app, "togglePlay()"),
                    "tray-next" => eval_bridge(app, "next()"),
                    "tray-prev" => eval_bridge(app, "prev()"),
                    "tray-autostart" => {
                        let enabled = app.autolaunch().is_enabled().unwrap_or(false);
                        let res = if enabled {
                            app.autolaunch().disable()
                        } else {
                            app.autolaunch().enable()
                        };
                        if res.is_ok() {
                            let _ = autostart_for_menu.set_checked(!enabled);
                        }
                    }
                    "tray-server" => goto_setup(app),
                    "tray-quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = main_window(app) {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Let the bridge (running in the webview) ask us to show setup too.
            let app_handle = app.handle().clone();
            app.listen("reverb:goto-setup", move |_event| {
                goto_setup(&app_handle);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Reverb");
}
