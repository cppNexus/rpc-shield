use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Instant;
use std::sync::Arc;

use crate::config::ApiKeyConfig;
use crate::identity::ClientIdentity;
use crate::identity::AuthError;
use crate::metrics;
use crate::metrics::Outcome;
use crate::rate_limiter::RateLimiter;
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub struct ProxyState {
    pub rate_limiter: Arc<RateLimiter>,
    pub rpc_backend_url: String,
    pub http_client: reqwest::Client,
    pub api_keys: HashMap<String, ApiKeyConfig>,
    pub api_key_tiers: HashMap<crate::config::SubscriptionTier, HashMap<String, crate::config::LimitRule>>,
    pub blocklist: HashSet<IpAddr>,
}

pub async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    let started = Instant::now();
    let client_ip = addr.ip();
    if state.blocklist.contains(&client_ip) {
        let response = error_response(
            StatusCode::FORBIDDEN,
            -32001,
            "IP blocked",
            req.id.clone(),
        );
        metrics::record(Outcome::Blocked, started.elapsed());
        return response;
    }

    let identity = match ClientIdentity::from_request(&headers, client_ip) {
        Ok(identity) => identity,
        Err(AuthError::InvalidScheme) => {
            let response = error_response(
                StatusCode::UNAUTHORIZED,
                -32000,
                "Invalid authorization scheme",
                req.id.clone(),
            );
            metrics::record(Outcome::AuthFailed, started.elapsed());
            return response;
        }
    };

    if let Some(api_key) = identity.api_key_raw() {
        match state.api_keys.get(api_key) {
            Some(cfg) if cfg.enabled => {}
            _ => {
                let response = error_response(
                    StatusCode::UNAUTHORIZED,
                    -32000,
                    "Invalid API key",
                    req.id.clone(),
                );
                metrics::record(Outcome::AuthFailed, started.elapsed());
                return response;
            }
        }
    }

    // Проверка rate limit
    let custom_rule = identity.api_key_raw().and_then(|key| {
        state.api_keys.get(key).and_then(|cfg| {
            if let Some(rule) = cfg.limits.get(&req.method) {
                Some(rule.clone())
            } else {
                state
                    .api_key_tiers
                    .get(&cfg.tier)
                    .and_then(|limits| limits.get(&req.method))
                    .cloned()
            }
        })
    });
    match state
        .rate_limiter
        .check_rate_limit_with_rule(&identity, &req.method, custom_rule)
        .await
    {
        Ok(decision) if decision.allowed => {
            // Rate limit пройден, проксируем запрос
            match forward_request(&state, &req).await {
                Ok(response) => {
                    metrics::record(Outcome::Allowed, started.elapsed());
                    Json(response).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to forward request: {}", e);
                    metrics::record(Outcome::UpstreamFail, started.elapsed());
                    error_response(
                        StatusCode::BAD_GATEWAY,
                        -32007,
                        "Upstream error",
                        req.id,
                    )
                }
            }
        }
        Ok(decision) => {
            // Rate limit превышен
            tracing::warn!(
                "Rate limit exceeded for {} on method {}",
                identity.to_string(),
                req.method
            );
            let mut response = error_response(
                StatusCode::TOO_MANY_REQUESTS,
                -32005,
                "Rate limit exceeded",
                req.id,
            );
            if let Some(wait) = decision.retry_after {
                let seconds = wait.as_secs_f64().ceil().max(1.0) as u64;
                response.headers_mut().insert(
                    "Retry-After",
                    seconds.to_string().parse().unwrap(),
                );
            }
            metrics::record(Outcome::RateLimited, started.elapsed());
            response
        }
        Err(e) => {
            tracing::error!("Rate limiter error: {}", e);
            let response =
                error_response(StatusCode::INTERNAL_SERVER_ERROR, -32603, "Internal error", req.id);
            metrics::record(Outcome::InternalFail, started.elapsed());
            response
        }
    }
}

async fn forward_request(
    state: &ProxyState,
    req: &JsonRpcRequest,
) -> anyhow::Result<JsonRpcResponse> {
    let response = state
        .http_client
        .post(&state.rpc_backend_url)
        .json(req)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Upstream responded with status {}",
            response.status()
        ));
    }

    let rpc_response: JsonRpcResponse = response.json().await?;
    Ok(rpc_response)
}

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "rpc-shield"
    }))
}

fn error_response(status: StatusCode, code: i32, message: &str, id: Option<Value>) -> Response {
    (
        status,
        Json(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
            id,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_parsing() {
        let json = r#"{
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        }"#;

        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "eth_blockNumber");
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_jsonrpc_error_response() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32005,
                message: "Rate limit exceeded".to_string(),
                data: None,
            }),
            id: Some(Value::Number(1.into())),
        };

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("Rate limit exceeded"));
    }
}
