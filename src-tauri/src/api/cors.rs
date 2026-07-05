use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

/// CORS for the Tauri dev server and other local dashboard origins calling the API
/// on a different loopback port. Does not allow arbitrary LAN or public origins.
pub fn dashboard_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _parts| {
            is_local_dev_origin(origin)
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
}

fn is_local_dev_origin(origin: &HeaderValue) -> bool {
    origin.to_str().ok().is_some_and(is_allowed_local_origin)
}

/// Returns true for `http://127.0.0.1:<port>`, `http://localhost:<port>`, and the
/// same hosts without an explicit port (default HTTP port).
pub fn is_allowed_local_origin(origin: &str) -> bool {
    let Some(rest) = origin.strip_prefix("http://") else {
        return false;
    };

    if let Some(port) = rest.strip_prefix("127.0.0.1:") {
        return port.parse::<u16>().is_ok();
    }
    if rest == "127.0.0.1" {
        return true;
    }
    if let Some(port) = rest.strip_prefix("localhost:") {
        return port.parse::<u16>().is_ok();
    }
    rest == "localhost"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_loopback_and_localhost_with_port() {
        assert!(is_allowed_local_origin("http://127.0.0.1:1430"));
        assert!(is_allowed_local_origin("http://127.0.0.1:3737"));
        assert!(is_allowed_local_origin("http://localhost:1430"));
    }

    #[test]
    fn allows_loopback_and_localhost_without_port() {
        assert!(is_allowed_local_origin("http://127.0.0.1"));
        assert!(is_allowed_local_origin("http://localhost"));
    }

    #[test]
    fn rejects_non_local_origins() {
        assert!(!is_allowed_local_origin("http://evil.example"));
        assert!(!is_allowed_local_origin("https://127.0.0.1:1430"));
        assert!(!is_allowed_local_origin("http://192.168.1.10:1430"));
        assert!(!is_allowed_local_origin("http://0.0.0.0:1430"));
    }
}
