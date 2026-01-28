use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::identity::ClientIdentity;
use crate::rate_limiter::RateLimiter;

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
}

pub async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    let client_ip = addr.ip();
    let identity = ClientIdentity::from_request(&headers, client_ip);

    // Проверка rate limit
    match state
        .rate_limiter
        .check_rate_limit(&identity, &req.method)
        .await
    {
        Ok(allowed) if allowed => {
            // Rate limit пройден, проксируем запрос
            match forward_request(&state, &req).await {
                Ok(response) => Json(response).into_response(),
                Err(e) => {
                    tracing::error!("Failed to forward request: {}", e);
                    (
                        StatusCode::BAD_GATEWAY,
                        Json(JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            result: None,
                            error: Some(JsonRpcError {
                                code: -32603,
                                message: "Internal error".to_string(),
                                data: None,
                            }),
                            id: req.id,
                        }),
                    )
                        .into_response()
                }
            }
        }
        Ok(_) => {
            // Rate limit превышен
            tracing::warn!(
                "Rate limit exceeded for {} on method {}",
                identity.to_string(),
                req.method
            );
            (
                StatusCode::TOO_MANY_REQUESTS,
                Json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32005,
                        message: "Rate limit exceeded".to_string(),
                        data: None,
                    }),
                    id: req.id,
                }),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Rate limiter error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "Internal error".to_string(),
                        data: None,
                    }),
                    id: req.id,
                }),
            )
                .into_response()
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

    let rpc_response: JsonRpcResponse = response.json().await?;
    Ok(rpc_response)
}

pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "polymorph-proxy"
    }))
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