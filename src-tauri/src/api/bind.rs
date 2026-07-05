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
        }
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
