pub mod api;
mod bootstrap;
mod commands;
mod config;
pub mod executor;
mod hotkeys;
pub mod orchestrator;
#[cfg(not(any(target_os = "android", target_os = "ios")))]
mod tray;

pub use api::{PeerClient, PeerClientError};
pub use bootstrap::BootstrapState;
pub use config::Config;

use std::sync::Arc;

use commands::api_base_url_from_config;
use tauri::Manager;

use api::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }

    builder
        .invoke_handler(tauri::generate_handler![commands::get_api_base_url])
        .setup(|app| {
            let config_result = config::load_config(std::path::Path::new("deskmux.config.json"));
            match &config_result {
                Ok(cfg) => println!("deskmux: loaded config for device '{}'", cfg.device_name),
                Err(err) => {
                    eprintln!("deskmux: failed to load deskmux.config.json\n{err}");
                }
            }

            #[cfg(target_os = "windows")]
            {
                let displays = executor::list_native_display_ids();
                if displays.is_empty() {
                    println!("deskmux: no native DDC displays detected");
                } else {
                    println!(
                        "deskmux: detected native DDC displays (copy into monitors[].nativeDdc.displayId): {}",
                        displays.join(", ")
                    );
                }
            }
            let app_state = Arc::new(AppState::from_load_result(config_result));
            let api_base_url = api_base_url_from_config(app_state.config.as_ref());
            app.manage(BootstrapState { api_base_url });

            #[cfg(not(any(target_os = "android", target_os = "ios")))]
            {
                tray::init(app.handle(), app_state.clone())?;
                if let Err(err) = hotkeys::register(app.handle(), app_state.clone()) {
                    eprintln!("deskmux: global hotkey setup failed: {err}");
                }
            }

            api::spawn_server(app_state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
