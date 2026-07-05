use std::fs;
use std::path::Path;

use super::error::LoadError;
use super::model::Config;
use super::validate::validate;

pub fn load_config(path: &Path) -> Result<Config, LoadError> {
    let contents = fs::read_to_string(path)?;
    let config: Config = serde_json::from_str(&contents)?;
    validate(&config)?;
    Ok(config)
}
