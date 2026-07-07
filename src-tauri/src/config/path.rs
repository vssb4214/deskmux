use std::path::{Path, PathBuf};

/// Fixed DeskMux config filename — startup load and save both use this path.
pub const CONFIG_FILENAME: &str = "deskmux.config.json";

/// Resolved config path relative to the process working directory.
///
/// Load and save both use this helper so they always target the same file.
/// Under `npm run tauri dev`, the working directory is typically `src-tauri/`.
pub fn default_config_path() -> PathBuf {
    std::env::current_dir()
        .map(|dir| dir.join(CONFIG_FILENAME))
        .unwrap_or_else(|_| PathBuf::from(CONFIG_FILENAME))
}

/// Human-readable load error that includes the config path.
pub fn format_config_load_error(path: &Path, err: &super::LoadError) -> String {
    format!("failed to load {}: {err}", path.display())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_path_uses_config_filename_in_working_directory() {
        let path = default_config_path();
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(CONFIG_FILENAME)
        );
        if let Ok(cwd) = std::env::current_dir() {
            assert_eq!(path.parent().map(|p| p.to_path_buf()), Some(cwd));
        }
    }

    #[test]
    fn format_config_load_error_includes_path() {
        let path = PathBuf::from("/tmp/deskmux.config.json");
        let err = super::super::LoadError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "missing",
        ));
        let message = format_config_load_error(&path, &err);
        assert!(message.contains("/tmp/deskmux.config.json"));
        assert!(message.contains("failed to load"));
    }
}
