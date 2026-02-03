use axum::response::Response;
use prometheus::{Encoder, Histogram, HistogramOpts, IntCounter, Registry, TextEncoder};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum Outcome {
    Allowed,
    RateLimited,
    Blocked,
    AuthFailed,
    UpstreamFail,
    InternalFail,
}

pub struct Metrics {
    registry: Registry,
    requests_total: IntCounter,
    allowed_total: IntCounter,
    rate_limited_total: IntCounter,
    blocked_total: IntCounter,
    auth_failed_total: IntCounter,
    upstream_fail_total: IntCounter,
    internal_fail_total: IntCounter,
    request_duration_seconds: Histogram,
}

impl Metrics {
    fn new() -> Self {
        let registry = Registry::new();

        let requests_total =
            IntCounter::new("rpc_shield_requests_total", "Total RPC requests").unwrap();
        let allowed_total =
            IntCounter::new("rpc_shield_requests_allowed_total", "Allowed RPC requests").unwrap();
        let rate_limited_total = IntCounter::new(
            "rpc_shield_requests_rate_limited_total",
            "Requests rejected by rate limiter",
        )
        .unwrap();
        let blocked_total = IntCounter::new(
            "rpc_shield_requests_blocked_total",
            "Requests blocked by IP blocklist",
        )
        .unwrap();
        let auth_failed_total = IntCounter::new(
            "rpc_shield_requests_auth_failed_total",
            "Requests rejected due to invalid API key or auth scheme",
        )
        .unwrap();
        let upstream_fail_total = IntCounter::new(
            "rpc_shield_requests_upstream_fail_total",
            "Requests failed due to upstream errors",
        )
        .unwrap();
        let internal_fail_total = IntCounter::new(
            "rpc_shield_requests_internal_fail_total",
            "Requests failed due to internal errors",
        )
        .unwrap();

        let request_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "rpc_shield_request_duration_seconds",
            "Proxy request duration in seconds",
        ))
        .unwrap();

        registry.register(Box::new(requests_total.clone())).unwrap();
        registry.register(Box::new(allowed_total.clone())).unwrap();
        registry.register(Box::new(rate_limited_total.clone())).unwrap();
        registry.register(Box::new(blocked_total.clone())).unwrap();
        registry.register(Box::new(auth_failed_total.clone())).unwrap();
        registry.register(Box::new(upstream_fail_total.clone())).unwrap();
        registry.register(Box::new(internal_fail_total.clone())).unwrap();
        registry
            .register(Box::new(request_duration_seconds.clone()))
            .unwrap();

        Self {
            registry,
            requests_total,
            allowed_total,
            rate_limited_total,
            blocked_total,
            auth_failed_total,
            upstream_fail_total,
            internal_fail_total,
            request_duration_seconds,
        }
    }

    fn observe(&self, outcome: Outcome, duration: Duration) {
        self.requests_total.inc();
        self.request_duration_seconds
            .observe(duration.as_secs_f64());

        match outcome {
            Outcome::Allowed => self.allowed_total.inc(),
            Outcome::RateLimited => self.rate_limited_total.inc(),
            Outcome::Blocked => self.blocked_total.inc(),
            Outcome::AuthFailed => self.auth_failed_total.inc(),
            Outcome::UpstreamFail => self.upstream_fail_total.inc(),
            Outcome::InternalFail => self.internal_fail_total.inc(),
        }
    }
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

fn metrics() -> &'static Metrics {
    METRICS.get_or_init(Metrics::new)
}

pub fn record(outcome: Outcome, duration: Duration) {
    metrics().observe(outcome, duration);
}

pub async fn metrics_handler() -> Response {
    let encoder = TextEncoder::new();
    let metric_families = metrics().registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(axum::body::Body::from(buffer))
        .unwrap()
}
