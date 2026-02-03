# RpcShield

[![Rust 1.75+](https://img.shields.io/badge/Rust-1.75%2B-informational)](Cargo.toml)
[![Status: As-Is](https://img.shields.io/badge/Status-As--Is-lightgrey)](README.md)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://github.com/cppNexus/rpc-shield/actions/workflows/ci.yml/badge.svg)](https://github.com/cppNexus/rpc-shield/actions/workflows/ci.yml)
[![Release](https://github.com/cppNexus/rpc-shield/actions/workflows/release.yml/badge.svg)](https://github.com/cppNexus/rpc-shield/actions/workflows/release.yml)

**Rate Limiter & JSON-RPC Proxy for Web3 RPC Endpoints**
---
<p align="center">
  <img src="https://github.com/cppNexus/rpc-shield/raw/main/images/rpcshield-logo.jpg" alt="rpc-shield Logo" width="300"/>
</p>

## Overview

RpcShield is a reverse proxy in front of a Web3 JSON-RPC node (Geth, Erigon, Nethermind, etc.) that provides:

- **Rate limiting** per IP or per API key
- **Per-method limits** for heavy RPC calls
- **Static IP blocklist**
- **Prometheus metrics** on a separate `/metrics` port
- **Simple YAML configuration**

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

# Run
./target/release/rpc-shield --config config.yaml
```

The proxy will be available at `http://localhost:8545`.

### Docker Compose (proxy + geth + prometheus)

```bash
docker compose up -d
```

The default compose file uses:
- `8545` for the proxy
- `8546` for the local geth node
- `9090` for Prometheus

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

blocklist:
  ips: []
  enable_auto_ban: false
  auto_ban_threshold: 1000

monitoring:
  prometheus_port: 9090
  log_level: "info"
```

### Per-method limits

You can set custom limits for specific RPC methods:

| Method | Suggested limit | Reason |
|-------|------------------|--------|
| `eth_getLogs` | 10/min | Heavy DB scans |
| `eth_call` | 20/min | CPU-intensive |
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

`tier` is only a label for default limits. It does not imply special permissions.

Priority order:
1. `api_keys.<key>.limits` (per-key override)
2. `api_key_tiers.<tier>` (tier defaults)
3. `rate_limits.method_limits`
4. `rate_limits.default_ip_limit`

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

**Note:** auto-ban is not implemented; only static `ips` are enforced.

## Rate Limit Headers

When a request is rate-limited, the proxy returns `429 Too Many Requests` and adds:

```
Retry-After: <seconds>
```

`Retry-After` is rounded up to seconds and is always ≥ 1.

## Monitoring

### Health Check

```bash
curl http://localhost:8545/health
```

### Prometheus

Metrics are exposed on `monitoring.prometheus_port` (default `9090`):

- `rpc_shield_requests_total`
- `rpc_shield_requests_allowed_total`
- `rpc_shield_requests_rate_limited_total`
- `rpc_shield_requests_blocked_total`
- `rpc_shield_requests_auth_failed_total`
- `rpc_shield_requests_upstream_fail_total`
- `rpc_shield_requests_internal_fail_total`
- `rpc_shield_request_duration_seconds`

Logging is controlled via `RUST_LOG`. The `monitoring.log_level` field is currently not read by the binary.

## Scope

RpcShield is intentionally small and focused. The following are **not** implemented:

- WebSocket proxying
- Admin API
- Multi-tenancy

## Development

### Run tests

```bash
cargo test
```

### Run in dev mode

```bash
RUST_LOG=debug cargo run -- --config config.yaml
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).

Additional notice: [NOTICE](NOTICE.md).
