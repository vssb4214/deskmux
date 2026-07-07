use std::path::{Path, PathBuf};

/// Fixed DeskMux config filename — startup load and save both use this path.
pub const CONFIG_FILENAME: &str = "deskmux.config.json";

pub fn default_config_path() -> &'static Path {
    Path::new(CONFIG_FILENAME)
}

/// `deskmux.config.json.bak` — sibling backup path for atomic save.
pub fn backup_path_for(config_path: &Path) -> PathBuf {
    sibling_with_suffix(config_path, ".bak")
}

/// `deskmux.config.json.tmp` — sibling temp path for atomic save.
pub fn temp_path_for(config_path: &Path) -> PathBuf {
    sibling_with_suffix(config_path, ".tmp")
}

fn sibling_with_suffix(config_path: &Path, suffix: &str) -> PathBuf {
    let mut path = config_path.as_os_str().to_os_string();
    path.push(suffix);
    PathBuf::from(path)
}
