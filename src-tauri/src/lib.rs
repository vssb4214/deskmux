pub mod api;
mod config;
pub mod executor;
pub mod orchestrator;

pub use api::{PeerClient, PeerClientError};
pub use config::Config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            let config_result = config::load_config(std::path::Path::new("deskmux.config.json"));
            match &config_result {
                Ok(cfg) => println!("deskmux: loaded config for device '{}'", cfg.device_name),
                Err(err) => {
                    // Log load/validation failures; do not hide them behind HTTP 503 alone.
                    eprintln!("deskmux: failed to load deskmux.config.json\n{err}");
                    // TODO(ui): surface config errors in the dashboard (banner or settings).
                }
            }
            // Start the API even when config failed: /health stays up with configLoaded=false;
            // /status and /apply-preset return 503 until a valid config is loaded.
            api::spawn_server(config_result.ok());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
