mod config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            match config::load_config(std::path::Path::new("deskmux.config.json")) {
                Ok(cfg) => println!("deskmux: loaded config for device '{}'", cfg.device_name),
                Err(err) => eprintln!("deskmux: failed to load deskmux.config.json\n{err}"),
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
