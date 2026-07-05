mod error;
mod loader;
mod model;
mod validate;

pub use error::{ConfigError, LoadError};
pub use loader::load_config;
pub use model::{Config, Input, Monitor, Peer};
