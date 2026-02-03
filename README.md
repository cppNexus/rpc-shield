# RpcShield

[![Rust 1.75+](https://img.shields.io/badge/Rust-1.75%2B-informational)](Cargo.toml)
[![Status: As-Is](https://img.shields.io/badge/Status-As--Is-lightgrey)](README.md)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://github.com/cppNexus/rpc-shield/actions/workflows/ci.yml/badge.svg)](https://github.com/cppNexus/rpc-shield/actions/workflows/ci.yml)
[![Release](https://github.com/cppNexus/rpc-shield/actions/workflows/release.yml/badge.svg)](https://github.com/cppNexus/rpc-shield/actions/workflows/release.yml)

**Rate Limiter & DDoS Filter for Web3 RPC Endpoints**
---
<p align="center">
  <img src="https://github.com/cppNexus/rpc-shield/raw/main/images/rpcshield-logo.jpg" alt="rpc-shield Logo" width="300"/>
</p>

## Overview

RpcShield is a high‑performance reverse proxy for Web3 RPC nodes (Geth, Erigon, etc.) that provides:

- **Rate limiting** per IP and per API key
- **DDoS protection** and malicious request filtering
- **Monitoring and usage stats**
- **Flexible per‑method limits**

Русская документация: `doc/README.ru.md`

## Quickstart

### Requirements

- Rust 1.75+
- A running RPC node (e.g. Geth on port 8546)

### Install & Run

```bash
# Clone the repository
git clone https://github.com/cppNexus/rpc-shield.git
cd rpc-shield

# Build
cargo build --release

# Run (self-hosted)
./target/release/rpc-shield --config config.yaml
```

The proxy will be available at `http://localhost:8545`.

### Docker Compose (proxy + geth + prometheus)

```bash
docker compose up -d
```

Default ports are defined in `.env.example` (copy to `.env` if needed):
- `RPC_SHIELD_PORT=8545`
- `GETH_PORT=8546`
- `PROMETHEUS_PORT=9090`

## Configuration

Main configuration is in `config.yaml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8545

rpc_backend:
  url: "http://localhost:8546"
  timeout_seconds: 30

rate_limits:
  default_ip_limit:
    requests: 100
    period: "1m"
  
  method_limits:
    eth_call:
      requests: 20
      period: "1m"
    eth_getLogs:
      requests: 10
      period: "1m"
```

### Per‑method limits

You can set custom limits for specific RPC methods:

| Method | Suggested limit | Reason |
|-------|------------------|--------|
| `eth_getLogs` | 10/min | Heavy DB scans |
| `eth_call` | 20/min | CPU‑intensive |
| `eth_blockNumber` | 60/min | Light |
| `eth_sendRawTransaction` | 5/min | Spam protection |

## API Keys

### Create keys

Add keys in `config.yaml`:

```yaml
api_keys:
  your_api_key_here:
    tier: pro
    enabled: true
    limits:
      eth_call:
        requests: 500
        period: "1m"

api_key_tiers:
  free:
    eth_call:
      requests: 20
      period: "1m"
  pro:
    eth_call:
      requests: 200
      period: "1m"
  enterprise:
    eth_call:
      requests: 1000
      period: "1m"
```

You can define limits for any methods, not just `eth_call`.

### Usage

```bash
# Bearer token
curl -X POST http://localhost:8545 \
  -H "Authorization: Bearer your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_blockNumber",
    "params": [],
    "id": 1
  }'

# X-API-Key header
curl -X POST http://localhost:8545 \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_call",
    "params": [...],
    "id": 1
  }'
```

### Tiers (free/pro/enterprise)

In community edition, `tier` provides **default per‑method limits**. Priority order:
1) `api_keys.<key>.limits` (per‑key override)
2) `api_key_tiers.<tier>` (tier defaults)
3) `rate_limits.method_limits`
4) `rate_limits.default_ip_limit`

Example `api_key_tiers`:

```yaml
api_key_tiers:
  free:
    eth_call: { requests: 20, period: "1m" }
  pro:
    eth_call: { requests: 200, period: "1m" }
  enterprise:
    eth_call: { requests: 1000, period: "1m" }
```

## Blocklist (IP)

Add IPs to block immediately:

```yaml
blocklist:
  ips:
    - "192.168.1.100"
    - "10.0.0.50"
  enable_auto_ban: false
  auto_ban_threshold: 1000
```

**Note:** auto‑ban is not implemented yet; only static `ips` are enforced.

## Rate Limit Headers

When a request is rate‑limited, the proxy returns `429 Too Many Requests` and adds:

```
Retry-After: <seconds>
```

`Retry-After` is rounded up to seconds and is always ≥ 1.

## Modes

### Self‑Hosted

```bash
./rpc-shield --config config.yaml
```

- YAML configuration
- Ideal for private nodes

## Architecture

```
[Client/DApp/Bot]
       ↓
[rpc-shield

:8545]
   ├── Rate Limiter
   ├── Identity Resolver
   └── Proxy Handler
       ↓
[RPC Node (Geth):8546]
```

### Core components

- **Proxy Layer** – Axum HTTP server
- **Rate Limiter** – token bucket (governor)
- **Identity Resolver** – client detection via IP/API key
- **Config Loader** – YAML config
- **Metrics** – Prometheus `/metrics` endpoint

## Monitoring

### Health Check

```bash
curl http://localhost:8545/health
```

### Prometheus

Metrics are exposed on port 9090 by default:

```
# HELP rpc_shield_requests_total Total RPC requests
# TYPE rpc_shield_requests_total counter
rpc_shield_requests_total 1234

# HELP rpc_shield_requests_allowed_total Allowed RPC requests
# TYPE rpc_shield_requests_allowed_total counter
rpc_shield_requests_allowed_total 1200

# HELP rpc_shield_requests_rate_limited_total Requests rejected by rate limiter
# TYPE rpc_shield_requests_rate_limited_total counter
rpc_shield_requests_rate_limited_total 20

# HELP rpc_shield_requests_blocked_total Requests blocked by IP blocklist
# TYPE rpc_shield_requests_blocked_total counter
rpc_shield_requests_blocked_total 3

# HELP rpc_shield_requests_auth_failed_total Requests rejected due to invalid API key or auth scheme
# TYPE rpc_shield_requests_auth_failed_total counter
rpc_shield_requests_auth_failed_total 11

# HELP rpc_shield_requests_upstream_fail_total Requests failed due to upstream errors
# TYPE rpc_shield_requests_upstream_fail_total counter
rpc_shield_requests_upstream_fail_total 2

# HELP rpc_shield_requests_internal_fail_total Requests failed due to internal errors
# TYPE rpc_shield_requests_internal_fail_total counter
rpc_shield_requests_internal_fail_total 0

# HELP rpc_shield_request_duration_seconds Proxy request duration in seconds
# TYPE rpc_shield_request_duration_seconds histogram
```

## Development

### Run tests

```bash
cargo test
```

### Run in dev mode

```bash
RUST_LOG=debug cargo run -- --config config.yaml
```

### Feature flags

```bash
# Self-hosted mode (default)
cargo build --features self-hosted
```

## Roadmap

### MVP (v0.1)
- [x] HTTP proxy with JSON‑RPC
- [x] Rate limiting per IP and method
- [x] API keys
- [x] YAML configuration
- [x] Basic logging

### v0.2 (in progress)
- [x] IP blocklist
- [x] Prometheus metrics
- [ ] WebSocket passthrough
- [ ] Redis integration

### v0.3 (planned)
- [ ] Admin REST API
- [ ] PostgreSQL for billing
- [ ] Web Dashboard (Tauri)
- [ ] Auto‑ban thresholds

### v1.0 (future)
- [ ] Stripe/Crypto payments
- [ ] ML‑based bot detection
- [ ] Geo‑blocking
- [ ] Email notifications

## Contributing

Pull requests are welcome. Key areas:

- Performance optimizations
- New rate limiters
- Monitoring integrations
- Documentation

## License

Apache License 2.0 — see [LICENSE](LICENSE).

Additional notice: [NOTICE](NOTICE.md).

## Links

- Documentation: https://docs.rpc-shield.io (soon)
- Discord: https://discord.gg/... (soon)
- Examples: ./examples (soon)
