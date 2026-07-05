pub mod api;
mod bootstrap;
mod commands;
mod config;
pub mod executor;
pub mod orchestrator;

pub use api::{PeerClient, PeerClientError};
pub use bootstrap::BootstrapState;
pub use config::Config;

use commands::api_base_url_from_config;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![commands::get_api_base_url])
        .setup(|app| {
            let config_result = config::load_config(std::path::Path::new("deskmux.config.json"));
            match &config_result {
                Ok(cfg) => println!("deskmux: loaded config for device '{}'", cfg.device_name),
                Err(err) => {
                    // Log load/validation failures; do not hide them behind HTTP 503 alone.
                    eprintln!("deskmux: failed to load deskmux.config.json\n{err}");
                    // TODO(ui): surface config errors in the dashboard (banner or settings).
                }
            }
            let config = config_result.ok();
            let api_base_url = api_base_url_from_config(config.as_ref());
            app.manage(BootstrapState { api_base_url });
            // Start the API even when config failed: /health stays up with configLoaded=false;
            // /status and /apply-preset return 503 until a valid config is loaded.
            api::spawn_server(config);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
