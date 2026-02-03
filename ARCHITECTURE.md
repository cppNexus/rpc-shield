# RpcShield Architecture

This document describes the **current** behavior of RpcShield. It intentionally avoids planned or hypothetical features.

Русская версия: `doc/ARCHITECTURE.ru.md`

## System Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Client Layer                         │
│  (Wallets, DApps, Bots, Scripts)                        │
└─────────────────┬───────────────────────────────────────┘
                  │ HTTP/JSON-RPC
                  ↓
┌─────────────────────────────────────────────────────────┐
│              rpc-shield (Port 8545)                     │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Identity   │  │     Rate     │  │   Metrics    │   │
│  │   Resolver   │  │   Limiter    │  │  Collector   │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                  │                  │        │
│         └──────────┬───────┴──────────────────┘        │
│                    ↓                                    │
│         ┌────────────────────┐                          │
│         │   Proxy Handler    │                          │
│         └──────────┬─────────┘                          │
└────────────────────┼─────────────────────────────────────┘
                     │
                     ↓
┌─────────────────────────────────────────────────────────┐
│              Backend RPC Node (Port 8546)               │
│           (Geth, Erigon, Nethermind, etc.)             │
└─────────────────────────────────────────────────────────┘
```

## Request Flow

1. Accept HTTP POST JSON-RPC request.
2. Resolve client identity (API key or IP).
3. Check static IP blocklist.
4. Apply rate limit for `identity + method`.
5. Forward request to backend RPC node.
6. Return backend response (or an error response).

## Components

### 1. Identity Resolver

**Inputs:** HTTP headers and client IP address.

**Resolution order:**
1. `Authorization: Bearer <token>`
2. `X-API-Key: <token>`
3. Fallback to client IP address

Only the Bearer and X-API-Key schemes are accepted. Any other auth scheme returns `401`.

### 2. Rate Limiter Engine

**Algorithm:** Token bucket (via the `governor` crate).

**Limiter key:** `identity:method` (one limiter per identity + RPC method).

**Limit precedence (top → bottom):**
1. `api_keys.<key>.limits.<method>`
2. `api_key_tiers.<tier>.<method>`
3. `rate_limits.method_limits.<method>`
4. `rate_limits.default_ip_limit`

### 3. Proxy Handler

**Behavior:**
- If the client IP is in `blocklist.ips`, return `403`.
- If an API key is provided but not found or disabled, return `401`.
- If rate limit is exceeded, return `429` with a `Retry-After` header.
- Otherwise, proxy the JSON-RPC request to the backend.

**Error codes:**
- `-32000` (401) — invalid auth scheme or invalid API key
- `-32001` (403) — IP blocked
- `-32005` (429) — rate limit exceeded
- `-32007` (502) — upstream error
- `-32603` (500) — internal error

### 4. Configuration (YAML)

```yaml
server:
  host: string
  port: u16

rpc_backend:
  url: string
  timeout_seconds: u64

rate_limits:
  default_ip_limit:
    requests: u32
    period: string
  method_limits:
    <method_name>:
      requests: u32
      period: string

api_keys:
  <key_value>:
    tier: "free" | "pro" | "enterprise"
    enabled: bool
    limits: { <method_name>: { requests: u32, period: string } }

api_key_tiers:
  free:
    <method_name>: { requests: u32, period: string }
  pro:
    <method_name>: { requests: u32, period: string }
  enterprise:
    <method_name>: { requests: u32, period: string }

blocklist:
  ips: [string]
  enable_auto_ban: bool
  auto_ban_threshold: u32

monitoring:
  prometheus_port: u16
  log_level: string
```

**Note:** `blocklist.enable_auto_ban` and `blocklist.auto_ban_threshold` are present in config but not implemented; only static `ips` are enforced. The `monitoring.log_level` field is currently ignored; use `RUST_LOG`.

### 5. Metrics (Prometheus)

- `rpc_shield_requests_total`
- `rpc_shield_requests_allowed_total`
- `rpc_shield_requests_rate_limited_total`
- `rpc_shield_requests_blocked_total`
- `rpc_shield_requests_auth_failed_total`
- `rpc_shield_requests_upstream_fail_total`
- `rpc_shield_requests_internal_fail_total`
- `rpc_shield_request_duration_seconds`

## Scope

RpcShield currently supports **HTTP JSON-RPC only**. There is no WebSocket proxy, admin API, or multi-tenancy.
