mod audio;
mod config;
mod window_pos;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use audio::models::{AppConfig, AudioSnapshot, OsdToastPayload};
use audio::wasapi;

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub ducked_apps: Mutex<HashMap<String, f32>>,
    pub toast_seq: AtomicU64,
}

fn show_osd_toast(app: &AppHandle, payload: OsdToastPayload) {
    let state = app.state::<AppState>();
    let seq = state.toast_seq.fetch_add(1, Ordering::SeqCst) + 1;

    if let Some(toast_win) = app.get_webview_window("toast") {
        window_pos::position_toast(&toast_win);
        let _ = toast_win.emit("osd-toast", &payload);
        let _ = toast_win.show();

        let app_clone = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(2200));
            let st = app_clone.state::<AppState>();
            if st.toast_seq.load(Ordering::SeqCst) == seq {
                if let Some(w) = app_clone.get_webview_window("toast") {
                    let _ = w.hide();
                }
            }
        });
    }
}

fn perform_switch_favorite_device(app: &AppHandle) -> Result<String, String> {
    let (fav_a, fav_b, lang) = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        (
            cfg.favorite_device_a.clone(),
            cfg.favorite_device_b.clone(),
            cfg.language.clone(),
        )
    };

    let ducked = {
        let state = app.state::<AppState>();
        let map = state.ducked_apps.lock().map_err(|e| e.to_string())?.clone();
        map
    };
    let snap = wasapi::get_audio_snapshot(&ducked)?;
    if snap.output_devices.len() < 2 {
        return Err(if lang == "tr" {
            "Geçiş için en az 2 ses çıkış cihazı gerekli".to_string()
        } else {
            "At least 2 audio output devices are required".to_string()
        });
    }

    let current_id = snap.default_output_id.clone().unwrap_or_default();

    // Determine the target device ID
    let target_id = match (fav_a.as_deref(), fav_b.as_deref()) {
        (Some(a), Some(b)) if a != b => {
            let a_exists = snap.output_devices.iter().any(|d| d.id == a);
            let b_exists = snap.output_devices.iter().any(|d| d.id == b);
            if a_exists && b_exists {
                if current_id == a {
                    b.to_string()
                } else {
                    a.to_string()
                }
            } else {
                // Fallback to next available device
                snap.output_devices
                    .iter()
                    .find(|d| d.id != current_id)
                    .map(|d| d.id.clone())
                    .unwrap_or(current_id)
            }
        }
        _ => snap
            .output_devices
            .iter()
            .find(|d| d.id != current_id)
            .map(|d| d.id.clone())
            .unwrap_or(current_id),
    };

    let device_name = wasapi::set_default_output_device(&target_id)?;
    let kind = snap
        .output_devices
        .iter()
        .find(|d| d.id == target_id)
        .map(|d| d.kind.clone())
        .unwrap_or_else(|| "speaker".to_string());

    let title = if lang == "tr" {
        "Çıkış Cihazı Değiştirildi".to_string()
    } else {
        "Output Device Switched".to_string()
    };

    show_osd_toast(
        app,
        OsdToastPayload {
            title,
            subtitle: device_name.clone(),
            icon: if kind == "headphones" {
                "headphones".to_string()
            } else {
                "speaker".to_string()
            },
        },
    );

    let _ = app.emit("audio-changed", ());
    Ok(device_name)
}

fn perform_toggle_mic_mute(app: &AppHandle) -> Result<bool, String> {
    let lang = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().map_err(|e| e.to_string())?;
        cfg.language.clone()
    };

    let (mic_name, muted) = wasapi::toggle_mic_mute()?;
    let title = match (lang.as_str(), muted) {
        ("tr", true) => "Mikrofon Susturuldu",
        ("tr", false) => "Mikrofon Açıldı",
        (_, true) => "Microphone Muted",
        (_, false) => "Microphone Unmuted",
    };

    show_osd_toast(
        app,
        OsdToastPayload {
            title: title.to_string(),
            subtitle: mic_name,
            icon: if muted {
                "mic-off".to_string()
            } else {
                "mic-on".to_string()
            },
        },
    );

    let _ = app.emit("audio-changed", ());
    Ok(muted)
}

fn perform_app_shortcut(app: &AppHandle, app_key: &str, mode: &str, duck_percent: u8) {
    let lang = {
        let state = app.state::<AppState>();
        state
            .config
            .lock()
            .map(|c| c.language.clone())
            .unwrap_or_else(|_| "tr".to_string())
    };

    if mode == "duck" {
        let state = app.state::<AppState>();
        let res = {
            if let Ok(mut ducked) = state.ducked_apps.lock() {
                wasapi::toggle_app_duck(app_key, duck_percent as f32 / 100.0, &mut ducked)
            } else {
                return;
            }
        };
        if let Ok(Some((display_name, is_ducked, new_vol))) = res {
            let pct = (new_vol * 100.0).round() as u32;
            let title = match (lang.as_str(), is_ducked) {
                ("tr", true) => format!("{display_name} Kısıldı (%{pct})"),
                ("tr", false) => format!("{display_name} Geri Yükseltildi (%{pct})"),
                (_, true) => format!("{display_name} Ducked ({pct}%)"),
                (_, false) => format!("{display_name} Restored ({pct}%)"),
            };
            show_osd_toast(
                app,
                OsdToastPayload {
                    title,
                    subtitle: app_key.to_string(),
                    icon: if is_ducked {
                        "duck".to_string()
                    } else {
                        "volume".to_string()
                    },
                },
            );
            let _ = app.emit("audio-changed", ());
        }
    } else if let Ok(Some((display_name, muted))) = wasapi::toggle_app_mute(app_key) {
        let title = match (lang.as_str(), muted) {
            ("tr", true) => format!("{display_name} Susturuldu"),
            ("tr", false) => format!("{display_name} Sesi Açıldı"),
            (_, true) => format!("{display_name} Muted"),
            (_, false) => format!("{display_name} Unmuted"),
        };
        show_osd_toast(
            app,
            OsdToastPayload {
                title,
                subtitle: app_key.to_string(),
                icon: if muted {
                    "mute".to_string()
                } else {
                    "volume".to_string()
                },
            },
        );
        let _ = app.emit("audio-changed", ());
    }
}

fn sync_global_shortcuts(app: &AppHandle) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let cfg = {
        let state = app.state::<AppState>();
        let c = state.config.lock().map_err(|e| e.to_string())?.clone();
        c
    };

    // 1. Output device switch shortcut
    if !cfg.switch_shortcut.trim().is_empty() {
        if let Ok(sc) = Shortcut::from_str(cfg.switch_shortcut.trim()) {
            let _ = gs.on_shortcut(sc, |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let _ = perform_switch_favorite_device(app);
                }
            });
        }
    }

    // 2. Mic mute shortcut
    if !cfg.mic_mute_shortcut.trim().is_empty() {
        if let Ok(sc) = Shortcut::from_str(cfg.mic_mute_shortcut.trim()) {
            let _ = gs.on_shortcut(sc, |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    let _ = perform_toggle_mic_mute(app);
                }
            });
        }
    }

    // 3. Per-app mute / duck shortcuts
    for rule in cfg.app_shortcuts {
        if rule.shortcut.trim().is_empty() || rule.app_key.trim().is_empty() {
            continue;
        }
        if let Ok(sc) = Shortcut::from_str(rule.shortcut.trim()) {
            let app_key = rule.app_key.clone();
            let mode = rule.mode.clone();
            let duck_percent = rule.duck_percent;
            let _ = gs.on_shortcut(sc, move |app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    perform_app_shortcut(app, &app_key, &mode, duck_percent);
                }
            });
        }
    }

    Ok(())
}

fn apply_pin_state(app: &AppHandle, pinned: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_skip_taskbar(!pinned);
        let _ = win.set_always_on_top(!pinned);
    }
}

fn toggle_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(false);
        if visible {
            let _ = win.hide();
        } else {
            let pinned = app
                .state::<AppState>()
                .config
                .lock()
                .map(|c| c.pinned)
                .unwrap_or(false);
            if !pinned {
                window_pos::position_above_taskbar(&win);
            }
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

// ---------------- Tauri Commands ----------------

#[tauri::command]
fn get_audio_snapshot(state: State<'_, AppState>) -> Result<AudioSnapshot, String> {
    let ducked = state.ducked_apps.lock().map_err(|e| e.to_string())?.clone();
    wasapi::get_audio_snapshot(&ducked)
}

#[tauri::command]
fn set_output_volume(device_id: Option<String>, volume: f32) -> Result<(), String> {
    wasapi::set_output_volume(device_id.as_deref(), volume)
}

#[tauri::command]
fn set_output_mute(device_id: Option<String>, muted: bool) -> Result<(), String> {
    wasapi::set_output_mute(device_id.as_deref(), muted)
}

#[tauri::command]
fn set_default_device(app: AppHandle, device_id: String) -> Result<String, String> {
    let name = wasapi::set_default_output_device(&device_id)?;
    let _ = app.emit("audio-changed", ());
    Ok(name)
}

#[tauri::command]
fn switch_favorite_device(app: AppHandle) -> Result<String, String> {
    perform_switch_favorite_device(&app)
}

#[tauri::command]
fn set_mic_volume(volume: f32) -> Result<(), String> {
    wasapi::set_mic_volume(volume)
}

#[tauri::command]
fn set_mic_mute(muted: bool) -> Result<(), String> {
    wasapi::set_mic_mute(muted)
}

#[tauri::command]
fn toggle_mic_mute(app: AppHandle) -> Result<bool, String> {
    perform_toggle_mic_mute(&app)
}

#[tauri::command]
fn set_app_volume(
    state: State<'_, AppState>,
    app_key: String,
    volume: f32,
) -> Result<(), String> {
    // If user manually drags volume slider, clear duck state for this app
    if let Ok(mut ducked) = state.ducked_apps.lock() {
        ducked.remove(&app_key.to_lowercase());
    }
    wasapi::set_app_volume(&app_key, volume)
}

#[tauri::command]
fn set_app_mute(app_key: String, muted: bool) -> Result<(), String> {
    wasapi::set_app_mute(&app_key, muted)
}

#[tauri::command]
fn toggle_app_duck(
    app: AppHandle,
    state: State<'_, AppState>,
    app_key: String,
    duck_percent: Option<u8>,
) -> Result<bool, String> {
    let pct = duck_percent.unwrap_or(20);
    let mut ducked = state.ducked_apps.lock().map_err(|e| e.to_string())?;
    let res = wasapi::toggle_app_duck(&app_key, pct as f32 / 100.0, &mut ducked)?;
    let is_ducked = res.map(|(_, d, _)| d).unwrap_or(false);
    let _ = app.emit("audio-changed", ());
    Ok(is_ducked)
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let cfg = state.config.lock().map_err(|e| e.to_string())?;
    Ok(cfg.clone())
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<'_, AppState>, new_config: AppConfig) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().map_err(|e| e.to_string())?;
        *cfg = new_config.clone();
    }
    config::save_config_to_disk(&app, &new_config)?;
    apply_pin_state(&app, new_config.pinned);
    sync_global_shortcuts(&app)?;
    Ok(())
}

#[tauri::command]
fn hide_main_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let loaded_cfg = config::load_config(app.handle());
            let initial_pinned = loaded_cfg.pinned;

            app.manage(AppState {
                config: Mutex::new(loaded_cfg),
                ducked_apps: Mutex::new(HashMap::new()),
                toast_seq: AtomicU64::new(0),
            });

            // Position main window above taskbar on startup
            if let Some(win) = app.get_webview_window("main") {
                apply_pin_state(app.handle(), initial_pinned);
                window_pos::position_above_taskbar(&win);
            }

            // Build system tray
            let open_item = MenuItem::with_id(app, "open", "StreamAudio", true, None::<&str>)?;
            let switch_item = MenuItem::with_id(
                app,
                "switch_device",
                "Switch Output Device",
                true,
                None::<&str>,
            )?;
            let mic_item =
                MenuItem::with_id(app, "toggle_mic", "Toggle Mic Mute", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let tray_menu = Menu::with_items(
                app,
                &[&open_item, &switch_item, &mic_item, &sep, &quit_item],
            )?;

            let mut tray_builder = TrayIconBuilder::with_id("streamaudio-tray")
                .tooltip("StreamAudio")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => toggle_main_window(app),
                    "switch_device" => {
                        let _ = perform_switch_favorite_device(app);
                    }
                    "toggle_mic" => {
                        let _ = perform_toggle_mic_mute(app);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main_window(tray.app_handle());
                    }
                });

            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }
            tray_builder.build(app)?;

            // Register global shortcuts
            let _ = sync_global_shortcuts(app.handle());

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                match event {
                    WindowEvent::Focused(false) => {
                        let pinned = window
                            .app_handle()
                            .state::<AppState>()
                            .config
                            .lock()
                            .map(|c| c.pinned)
                            .unwrap_or(false);
                        if !pinned {
                            let _ = window.hide();
                        }
                    }
                    WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    _ => {}
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_audio_snapshot,
            set_output_volume,
            set_output_mute,
            set_default_device,
            switch_favorite_device,
            set_mic_volume,
            set_mic_mute,
            toggle_mic_mute,
            set_app_volume,
            set_app_mute,
            toggle_app_duck,
            get_config,
            save_config,
            hide_main_window,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running StreamAudio");
}
