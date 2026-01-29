use crate::config::{LimitRule, RateLimitConfig};
use crate::identity::ClientIdentity;
use anyhow::{anyhow, Result};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

type LimiterMap =
    Arc<RwLock<HashMap<String, GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>>>;

pub struct RateLimiter {
    limiters: LimiterMap,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Проверяет, разрешён ли запрос для данного клиента и метода
    pub async fn check_rate_limit(&self, identity: &ClientIdentity, method: &str) -> Result<bool> {
        let key = self.make_key(identity, method);
        let limit_rule = self.get_limit_rule(identity, method);

        let mut limiters = self.limiters.write().await;

        let limiter = limiters.entry(key.clone()).or_insert_with(|| {
            let quota = Self::parse_limit_rule(&limit_rule).unwrap_or_else(|_| {
                // Fallback: 100 requests per minute
                Quota::per_minute(NonZeroU32::new(100).unwrap())
            });
            GovernorRateLimiter::direct(quota)
        });

        Ok(limiter.check().is_ok())
    }

    fn make_key(&self, identity: &ClientIdentity, method: &str) -> String {
        format!("{identity}:{method}")
    }

    fn get_limit_rule(&self, _identity: &ClientIdentity, method: &str) -> LimitRule {
        // Для API ключей используем кастомные лимиты (будет реализовано позже)
        // Пока используем method-specific или default лимиты

        self.config
            .method_limits
            .get(method)
            .cloned()
            .unwrap_or_else(|| self.config.default_ip_limit.clone())
    }

    fn parse_limit_rule(rule: &LimitRule) -> Result<Quota> {
        let duration = Self::parse_duration(&rule.period)?;
        let requests =
            NonZeroU32::new(rule.requests).ok_or_else(|| anyhow!("Requests must be > 0"))?;

        let quota = match duration {
            d if d == Duration::from_secs(1) => Quota::per_second(requests),
            d if d == Duration::from_secs(60) => Quota::per_minute(requests),
            d if d == Duration::from_secs(3600) => Quota::per_hour(requests),
            _ => Quota::with_period(duration)
                .ok_or_else(|| anyhow!("Invalid quota configuration"))?
                .allow_burst(requests),
        };

        Ok(quota)
    }

    fn parse_duration(period: &str) -> Result<Duration> {
        let period = period.trim();

        if period.ends_with('s') {
            let secs: u64 = period.trim_end_matches('s').parse()?;
            Ok(Duration::from_secs(secs))
        } else if period.ends_with('m') {
            let mins: u64 = period.trim_end_matches('m').parse()?;
            Ok(Duration::from_secs(mins * 60))
        } else if period.ends_with('h') {
            let hours: u64 = period.trim_end_matches('h').parse()?;
            Ok(Duration::from_secs(hours * 3600))
        } else {
            Err(anyhow!("Invalid period format: {period}"))
        }
    }

    /// Очистка старых лимитеров (для экономии памяти)
    #[allow(dead_code)]
    pub async fn cleanup(&self) {
        let mut limiters = self.limiters.write().await;
        // В production: удаляем неиспользуемые лимитеры старше N минут
        limiters.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_parse_duration() {
        assert_eq!(
            RateLimiter::parse_duration("1s").unwrap(),
            Duration::from_secs(1)
        );
        assert_eq!(
            RateLimiter::parse_duration("5m").unwrap(),
            Duration::from_secs(300)
        );
        assert_eq!(
            RateLimiter::parse_duration("2h").unwrap(),
            Duration::from_secs(7200)
        );
    }

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let config = RateLimitConfig {
            default_ip_limit: LimitRule {
                requests: 5,
                period: "1m".to_string(),
            },
            method_limits: HashMap::new(),
        };

        let limiter = RateLimiter::new(config);
        let identity = ClientIdentity::IpAddress("127.0.0.1".parse::<IpAddr>().unwrap());

        // Первые 5 запросов должны пройти
        for _ in 0..5 {
            assert!(limiter
                .check_rate_limit(&identity, "eth_call")
                .await
                .unwrap());
        }

        // 6-й запрос должен быть отклонён
        assert!(!limiter
            .check_rate_limit(&identity, "eth_call")
            .await
            .unwrap());
    }
}
