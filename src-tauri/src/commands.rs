use tauri::State;

use crate::api::bind::dashboard_api_base_url;
use crate::BootstrapState;

#[tauri::command]
pub fn get_api_base_url(state: State<BootstrapState>) -> String {
    state.api_base_url.clone()
}

pub fn api_base_url_from_config(config: Option<&crate::config::Config>) -> String {
    dashboard_api_base_url(config)
}
