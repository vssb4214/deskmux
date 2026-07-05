use std::sync::Arc;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::api::events::ApplySource;
use crate::api::{apply_preset_to_arc, AppState, ApplyPresetStateError};

const MENU_SHOW: &str = "show";
const MENU_QUIT: &str = "quit";
const MENU_APPLY_PREFIX: &str = "apply:";

pub fn init(app: &AppHandle, state: Arc<AppState>) -> tauri::Result<()> {
    let icon = app.default_window_icon().cloned().ok_or_else(|| {
        tauri::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing default window icon",
        ))
    })?;

    let show = MenuItem::with_id(app, MENU_SHOW, "Show DeskMux", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;

    let mut preset_items = Vec::new();
    if let Some(config) = &state.config {
        let mut preset_names: Vec<_> = config.presets.keys().collect();
        preset_names.sort();
        for name in preset_names {
            let label = &config.presets[name].label;
            let id = format!("{MENU_APPLY_PREFIX}{name}");
            let text = format!("Apply: {label}");
            preset_items.push(MenuItem::with_id(app, id, text, true, None::<&str>)?);
        }
    }

    let mut items: Vec<&dyn tauri::menu::IsMenuItem<_>> = vec![&show];
    if !preset_items.is_empty() {
        items.push(&separator);
        for item in &preset_items {
            items.push(item);
        }
        items.push(&separator);
    }
    items.push(&quit);

    let menu = Menu::with_items(app, &items)?;

    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .tooltip("DeskMux")
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                MENU_SHOW => show_main_window(app),
                MENU_QUIT => app.exit(0),
                id if id.starts_with(MENU_APPLY_PREFIX) => {
                    let preset = id.trim_start_matches(MENU_APPLY_PREFIX);
                    let state = state.clone();
                    let preset = preset.to_string();
                    tauri::async_runtime::spawn(async move {
                        match apply_preset_to_arc(state, &preset, false, false, ApplySource::Tray)
                            .await
                        {
                            Ok(_) => {
                                eprintln!("deskmux: tray preset '{preset}' applied with errors");
                            }
                            Err(ApplyPresetStateError::ConfigNotLoaded) => {
                                eprintln!(
                                    "deskmux: tray preset '{preset}' failed: config not loaded"
                                );
                            }
                            Err(ApplyPresetStateError::PresetNotFound { preset_name }) => {
                                eprintln!(
                                    "deskmux: tray preset '{preset_name}' failed: preset not found"
                                );
                            }
                        }
                    });
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
