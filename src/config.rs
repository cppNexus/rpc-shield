use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub rpc_backend: RpcBackendConfig,
    pub rate_limits: RateLimitConfig,
    pub api_keys: HashMap<String, ApiKeyConfig>,
    #[serde(default)]
    pub api_key_tiers: HashMap<SubscriptionTier, HashMap<String, LimitRule>>,
    pub blocklist: BlocklistConfig,
    #[cfg(feature = "saas")]
    pub database: Option<DatabaseConfig>,
    pub monitoring: MonitoringConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcBackendConfig {
    pub url: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub default_ip_limit: LimitRule,
    pub method_limits: HashMap<String, LimitRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitRule {
    pub requests: u32,
    pub period: String, // "1s", "1m", "1h"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub tier: SubscriptionTier,
    pub limits: HashMap<String, LimitRule>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionTier {
    Free,
    Pro,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistConfig {
    pub ips: Vec<String>,
    pub enable_auto_ban: bool,
    pub auto_ban_threshold: u32,
}

#[cfg(feature = "saas")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub prometheus_port: u16,
    pub log_level: String,
}

impl Config {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let config = Config {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8545,
            },
            rpc_backend: RpcBackendConfig {
                url: "http://localhost:8546".to_string(),
                timeout_seconds: 30,
            },
            rate_limits: RateLimitConfig {
                default_ip_limit: LimitRule {
                    requests: 100,
                    period: "1m".to_string(),
                },
                method_limits: HashMap::new(),
            },
            api_keys: HashMap::new(),
            api_key_tiers: HashMap::new(),
            blocklist: BlocklistConfig {
                ips: vec![],
                enable_auto_ban: false,
                auto_ban_threshold: 1000,
            },
            #[cfg(feature = "saas")]
            database: None,
            monitoring: MonitoringConfig {
                prometheus_port: 9090,
                log_level: "info".to_string(),
            },
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("server:"));
    }
}
