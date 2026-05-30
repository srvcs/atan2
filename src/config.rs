use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub log_level: String,
    pub environment: String,
    /// Base URL of the srvcs-isnumber dependency.
    pub isnumber_url: String,
}

impl Config {
    pub fn from_vars(
        bind: Option<String>,
        log: Option<String>,
        env: Option<String>,
        isnumber_url: Option<String>,
    ) -> Self {
        let bind_addr = bind
            .unwrap_or_else(|| "0.0.0.0:8080".to_string())
            .parse()
            .expect("SRVCS_BIND_ADDR must be host:port");
        Config {
            bind_addr,
            log_level: log.unwrap_or_else(|| "info,tower_http=info".to_string()),
            environment: env.unwrap_or_else(|| "development".to_string()),
            isnumber_url: isnumber_url.unwrap_or_else(|| "http://127.0.0.1:8081".to_string()),
        }
    }

    pub fn from_env() -> Self {
        Self::from_vars(
            std::env::var("SRVCS_BIND_ADDR").ok(),
            std::env::var("RUST_LOG").ok(),
            std::env::var("SRVCS_ENV").ok(),
            std::env::var("SRVCS_ISNUMBER_URL").ok(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = Config::from_vars(None, None, None, None);
        assert_eq!(c.bind_addr.port(), 8080);
        assert_eq!(c.environment, "development");
        assert_eq!(c.isnumber_url, "http://127.0.0.1:8081");
    }

    #[test]
    fn parses_explicit_bind_addr() {
        let c = Config::from_vars(Some("127.0.0.1:9000".into()), None, None, None);
        assert_eq!(c.bind_addr.port(), 9000);
    }
}
