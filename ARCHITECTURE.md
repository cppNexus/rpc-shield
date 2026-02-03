# RpcShield Architecture

This document provides a full overview of the community edition architecture.

Русская версия: `doc/ARCHITECTURE.ru.md`

## System Overview

rpc-shield is built on a modular architecture with a clear separation of responsibilities.

```
┌─────────────────────────────────────────────────────────┐
│                    Client Layer                         │
│  (Wallets, DApps, Bots, Scripts)                       │
└─────────────────┬───────────────────────────────────────┘
                  │ HTTP/JSON-RPC
                  ↓
┌─────────────────────────────────────────────────────────┐
│              rpc-shield (Port 8545)                     │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Identity   │  │     Rate     │  │    Stats     │   │
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

## Components

### 1. Identity Resolver

**Responsibility:** resolve a client identity per request.

**Logic:**
1. Check `Authorization: Bearer <token>` header
2. Check `X-API-Key` header
3. Fallback to client IP address

**Identity types:**
```rust
enum ClientIdentity {
    ApiKey(String),      // Authenticated user
    IpAddress(IpAddr),   // Anonymous client by IP
    Anonymous,           // Undefined
}
```

**Priority order:**
1. API key (highest priority – per‑key limits)
2. IP address (default limits)

### 2. Rate Limiter Engine

**Algorithm:** Token Bucket (via `governor` crate)

**Structure:**
```rust
HashMap<String, RateLimiter>
Key = "identity:method"
// Examples:
// "apikey:abc123:eth_call"
// "ip:192.168.1.1:eth_getLogs"
```

**Decision flow:**
1. Extract identity + method
2. Load the matching limit from config
3. Check quota in the matching bucket
4. Allow or reject the request

**Limit precedence (top → bottom):**
1. API key + specific method
2. Method‑specific limit (`config.method_limits`)
3. Default IP limit

**Quota examples:**
```yaml
# 100 requests per minute
requests: 100
period: "1m"

# 5 requests per second
requests: 5
period: "1s"

# 1000 requests per hour
requests: 1000
period: "1h"
```

### 3. Proxy Handler

**Responsibility:** route and proxy JSON‑RPC requests.

**Request flow:**
```
1. Accept HTTP POST request
   ↓
2. Parse JSON-RPC body
   ↓
3. Extract IP and headers
   ↓
4. Resolve ClientIdentity
   ↓
5. Check rate limit
   ↓
6a. Limit exceeded → 429 Too Many Requests
6b. Limit OK → forward to RPC node
   ↓
7. Receive backend response
   ↓
8. Return response to client
```

**Error codes:**
- `-32005`: Rate limit exceeded (custom)
- `-32603`: Internal error (JSON‑RPC standard)
- HTTP 429: Too Many Requests

### 4. Config Loader

**Format:** YAML

**Structure:**
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
    limits: {...}

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

### 5. Metrics (Prometheus)

**Metrics collected:**
```
- rpc_shield_requests_total (counter)
- rpc_shield_requests_allowed_total (counter)
- rpc_shield_requests_rate_limited_total (counter)
- rpc_shield_requests_blocked_total (counter)
- rpc_shield_requests_auth_failed_total (counter)
- rpc_shield_requests_upstream_fail_total (counter)
- rpc_shield_requests_internal_fail_total (counter)
- rpc_shield_request_duration_seconds (histogram)
```

## Operating Mode

### Self‑Hosted

**Characteristics:**
- YAML configuration
- In‑memory limiters
- Stats in stdout/logs
- No database required

**Use cases:**
- Private RPC nodes
- Internal enterprise networks
- Development and testing

## Data Flows

### Successful request

```
Client → Proxy Handler
         ↓
      Identity Resolver (client lookup)
         ↓
      Rate Limiter (quota check)
         ↓ PASS
      HTTP Client → Backend RPC
         ↓
      Response
         ↓
      Client
```

### Rejected request (Rate Limited)

```
Client → Proxy Handler
         ↓
      Identity Resolver
         ↓
      Rate Limiter (quota exceeded)
         ↓ FAIL
      Stats Collector (increment counter)
         ↓
      429 Response → Client
```

## Scaling

### Vertical

**Single instance:**
- 10K‑50K RPS on modern CPU
- In‑memory limiters (very fast)
- Low latency (~1‑2ms proxy overhead)

**Optimizations:**
- Async I/O (Tokio)
- Zero‑copy where possible
- Connection pooling to RPC node

## Security

### Rate limiting as first line of defense

**Protects against:**
- DDoS attacks
- Method spam (eth_getLogs)
- Resource‑heavy requests
- Bot traffic

### Additional measures

**IP blocklist:**
- Static list of blocked IPs
- Auto‑ban thresholds (planned)

**Method filtering (future):**
- Blacklist dangerous methods
- Whitelist allowed methods

**Request validation:**
- JSON‑RPC format
- Payload size
- Signature verification (optional)

## Extensions

### WebSocket Support (planned)

```rust
// New handler for ws://
async fn ws_proxy_handler() {
    // Upgrade connection
    // Forward eth_subscribe events
    // Maintain persistent connection
}
```

### Admin API (planned)

**Endpoints:**
```
POST   /admin/api-keys              - Create key
GET    /admin/api-keys/:id          - Get key
PUT    /admin/api-keys/:id          - Update limits
DELETE /admin/api-keys/:id          - Delete key
GET    /admin/stats/:id             - Key statistics
POST   /admin/blocklist/add         - Add IP to blocklist
```

**Authentication:**
- JWT tokens
- Admin API key
- Role‑based access control

### Machine Learning Integration (future)

**Bot detection:**
- Pattern analysis
- Anomaly detection
- Automated ban recommendations

**Traffic prediction:**
- Usage forecasting for auto‑scaling
- Predictive rate limiting
- Cost optimization

## Monitoring & Observability

### Logging

**Levels:**
- ERROR: critical failures
- WARN: rate limits, suspicious activity
- INFO: startup, config changes
- DEBUG: per‑request logs (dev only)

**Log structure:**
```json
{
  "timestamp": "2025-01-28T10:30:00Z",
  "level": "WARN",
  "identity": "ip:1.2.3.4",
  "method": "eth_getLogs",
  "message": "Rate limit exceeded",
  "limit": "10/min",
  "current": 15
}
```

### Prometheus metrics

**Dashboard metrics:**
- Requests per second by method
- Rate limit hit rate
- Backend latency
- Active connections
- Error rates

### Alerts

**Critical:**
- Backend RPC unavailable
- Errors > 5%
- Latency > 1s

**Warnings:**
- Unusual traffic spikes
- New attack patterns
- Approaching resource limits

## Performance

### Benchmarks (expected)

```
Throughput:  20,000 RPS (single instance)
Latency:     p50: 2ms, p99: 10ms (proxy overhead)
Memory:      ~100MB base + 1KB per active limiter
CPU:         ~30% at 10K RPS (4 cores)
```

### Profiling

**Hotspots:**
1. Rate limiter lookup – O(1) HashMap
2. JSON parsing – serde
3. HTTP forwarding – connection pooling

**Optimizations:**
- Batch processing for stats
- LRU cache for config
- Pre‑compiled regex for validation
