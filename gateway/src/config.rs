use std::env;

/// Application configuration loaded from environment variables.
///
/// All fields have sensible defaults except `database_url`, which is required
/// and will cause a panic at startup if not set.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Bind address for the HTTP server.
    pub host: String,
    /// Port number for the HTTP server.
    pub port: u16,
    /// PostgreSQL connection string. Optional -- runs in standalone mode if missing.
    pub database_url: Option<String>,
    /// Redis connection string.
    pub redis_url: String,
    /// Default cost profile for routing decisions.
    pub default_cost_profile: String,
    /// Time-to-live for cached responses, in seconds.
    pub cache_ttl_seconds: u64,
    /// Maximum number of entries in the response cache.
    pub max_cache_entries: usize,
    /// Interval (seconds) between pricing data syncs from providers.
    pub pricing_sync_interval_seconds: u64,
    /// URL of the admin dashboard frontend (used for CORS).
    pub dashboard_url: String,
}

impl AppConfig {
    /// Load configuration from environment variables.
    ///
    /// # Panics
    ///
    /// Panics if `DATABASE_URL` is not set.
    pub fn from_env() -> Self {
        let database_url = env::var("DATABASE_URL").ok();

        Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database_url,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            default_cost_profile: env::var("DEFAULT_COST_PROFILE")
                .unwrap_or_else(|_| "balanced".into()),
            cache_ttl_seconds: env::var("CACHE_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            max_cache_entries: env::var("MAX_CACHE_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10_000),
            pricing_sync_interval_seconds: env::var("PRICING_SYNC_INTERVAL_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            dashboard_url: env::var("DASHBOARD_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
        }
    }

    /// Returns the `host:port` bind address for Actix-Web.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_address() {
        let config = AppConfig {
            host: "127.0.0.1".into(),
            port: 9090,
            database_url: Some("postgres://localhost/test".into()),
            redis_url: "redis://127.0.0.1:6379".into(),
            default_cost_profile: "balanced".into(),
            cache_ttl_seconds: 3600,
            max_cache_entries: 10_000,
            pricing_sync_interval_seconds: 3600,
            dashboard_url: "http://localhost:3000".into(),
        };
        assert_eq!(config.bind_address(), "127.0.0.1:9090");
    }

    #[test]
    fn test_defaults() {
        // Clear DATABASE_URL to check optional behavior
        env::remove_var("DATABASE_URL");
        env::remove_var("HOST");
        env::remove_var("PORT");
        env::remove_var("REDIS_URL");
        env::remove_var("DEFAULT_COST_PROFILE");
        env::remove_var("CACHE_TTL_SECONDS");
        env::remove_var("MAX_CACHE_ENTRIES");
        env::remove_var("PRICING_SYNC_INTERVAL_SECONDS");
        env::remove_var("DASHBOARD_URL");

        let config = AppConfig::from_env();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.database_url, None);
        assert_eq!(config.redis_url, "redis://127.0.0.1:6379");
        assert_eq!(config.default_cost_profile, "balanced");
        assert_eq!(config.cache_ttl_seconds, 3600);
        assert_eq!(config.max_cache_entries, 10_000);
        assert_eq!(config.pricing_sync_interval_seconds, 3600);
        assert_eq!(config.dashboard_url, "http://localhost:3000");
    }
}
