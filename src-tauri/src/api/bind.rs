use std::net::{Ipv4Addr, SocketAddr};

use crate::config::Config;

/// Loopback-only by default (see docs/CONFIG.md peer port vs bind address).
pub const DEFAULT_BIND_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// Default port for the local HTTP API (see docs/CONFIG.md).
pub const DEFAULT_PORT: u16 = 3737;

/// Resolves the socket address for the local HTTP API.
///
/// Security: `POST /apply-preset` only accepts a preset *name* (not arbitrary shell
/// commands), but remote preset triggering is still a real attack surface — a peer
/// can fire any preset defined in config. Keep the API on loopback unless
/// `apiLanAccess` is explicitly enabled in config.
pub fn resolve_bind_addr(config: Option<&Config>) -> SocketAddr {
    let port = config.map(|c| c.api_port).unwrap_or(DEFAULT_PORT);
    let host = if config.is_some_and(|c| c.api_lan_access) {
        Ipv4Addr::UNSPECIFIED
    } else {
        DEFAULT_BIND_HOST
    };
    SocketAddr::from((host, port))
}

/// Base URL the dashboard should use to reach this machine's local HTTP API.
///
/// Always loopback (`127.0.0.1`), even when `apiLanAccess` binds the server on
/// `0.0.0.0`. Port follows loaded config or [`DEFAULT_PORT`] when config is missing.
pub fn dashboard_api_base_url(config: Option<&Config>) -> String {
    let port = config.map(|c| c.api_port).unwrap_or(DEFAULT_PORT);
    format!("http://{DEFAULT_BIND_HOST}:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::collections::HashMap;
    use std::net::IpAddr;

    fn config_with_lan(lan: bool) -> Config {
        Config {
            device_name: "device-a".to_string(),
            api_port: 3737,
            api_lan_access: lan,
            peers: vec![],
            devices: vec![],
            monitors: vec![],
            presets: HashMap::new(),
            hotkeys: HashMap::new(),
        }
    }

    fn config_with_port(port: u16) -> Config {
        Config {
            api_port: port,
            ..config_with_lan(false)
        }
    }

    #[test]
    fn dashboard_url_defaults_when_config_missing() {
        assert_eq!(dashboard_api_base_url(None), "http://127.0.0.1:3737");
    }

    #[test]
    fn dashboard_url_uses_configured_port() {
        assert_eq!(
            dashboard_api_base_url(Some(&config_with_port(4000))),
            "http://127.0.0.1:4000"
        );
    }

    #[test]
    fn dashboard_url_stays_loopback_when_lan_access_enabled() {
        assert_eq!(
            dashboard_api_base_url(Some(&config_with_lan(true))),
            "http://127.0.0.1:3737"
        );
    }
    #[test]
    fn defaults_to_loopback_when_config_missing() {
        let addr = resolve_bind_addr(None);
        assert_eq!(addr.ip(), IpAddr::from(DEFAULT_BIND_HOST));
        assert_eq!(addr.port(), DEFAULT_PORT);
    }

    #[test]
    fn defaults_to_loopback_when_lan_access_disabled() {
        let addr = resolve_bind_addr(Some(&config_with_lan(false)));
        assert_eq!(addr.ip(), IpAddr::from(DEFAULT_BIND_HOST));
        assert_eq!(addr.port(), 3737);
    }

    #[test]
    fn binds_all_interfaces_only_when_lan_access_enabled() {
        let addr = resolve_bind_addr(Some(&config_with_lan(true)));
        assert_eq!(addr.ip(), IpAddr::from(Ipv4Addr::UNSPECIFIED));
        assert_eq!(addr.port(), 3737);
    }
}
