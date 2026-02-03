# Configuration Guide

This document describes the **current** RpcShield configuration. It intentionally avoids future or unimplemented features.

## Table of Contents

- [Configuration File](#configuration-file)
- [Server Configuration](#server-configuration)
- [RPC Backend](#rpc-backend)
- [Rate Limiting](#rate-limiting)
- [API Keys](#api-keys)
- [API Key Tiers](#api-key-tiers)
- [Blocklist](#blocklist)
- [Monitoring](#monitoring)
- [Examples](#examples)

## Configuration File

RpcShield uses YAML configuration files. By default, it looks for `config.yaml` in the current directory.

```bash
rpc-shield --config /path/to/config.yaml
```

## Server Configuration

```yaml
server:
  host: "0.0.0.0"  # Bind address
  port: 8545       # Proxy port
```

## RPC Backend

```yaml
rpc_backend:
  url: "http://localhost:8546"  # Backend RPC endpoint
  timeout_seconds: 30            # Request timeout
```

## Rate Limiting

### Default IP limit

```yaml
rate_limits:
  default_ip_limit:
    requests: 100
    period: "1m"
```

### Method-specific limits

```yaml
rate_limits:
  method_limits:
    eth_getLogs:
      requests: 10
      period: "1m"
    eth_call:
      requests: 20
      period: "1m"
```

### Period format

The `period` field supports a number + unit:

- `<number>s` (seconds)
- `<number>m` (minutes)
- `<number>h` (hours)

Examples: `"30s"`, `"5m"`, `"2h"`.

### Limit precedence

Limits are applied in this order (highest to lowest):

1. `api_keys.<key>.limits.<method>`
2. `api_key_tiers.<tier>.<method>`
3. `rate_limits.method_limits.<method>`
4. `rate_limits.default_ip_limit`

## API Keys

```yaml
api_keys:
  my_key_123:
    tier: pro
    enabled: true
    limits:
      eth_call:
        requests: 500
        period: "1m"
```

Fields:
- `tier`: one of `free`, `pro`, `enterprise`.
- `enabled`: `false` disables the key (requests will return `401`).
- `limits`: per-method limits for this key.

## API Key Tiers

`api_key_tiers` defines **default** per-method limits for each tier label. Tiers are only labels for defaults and do **not** imply special permissions.

```yaml
api_key_tiers:
  free:
    eth_call: { requests: 20, period: "1m" }
  pro:
    eth_call: { requests: 200, period: "1m" }
  enterprise:
    eth_call: { requests: 1000, period: "1m" }
```

## Blocklist

```yaml
blocklist:
  ips:
    - "192.168.1.100"
    - "10.0.0.50"
  enable_auto_ban: false
  auto_ban_threshold: 1000
```

**Important:** only static `ips` are enforced. `enable_auto_ban` and `auto_ban_threshold` are present in config but not implemented.

## Monitoring

```yaml
monitoring:
  prometheus_port: 9090
  log_level: "info"  # trace | debug | info | warn | error
```

Metrics are exposed at `http://<server.host>:<prometheus_port>/metrics`:

- `rpc_shield_requests_total`
- `rpc_shield_requests_allowed_total`
- `rpc_shield_requests_rate_limited_total`
- `rpc_shield_requests_blocked_total`
- `rpc_shield_requests_auth_failed_total`
- `rpc_shield_requests_upstream_fail_total`
- `rpc_shield_requests_internal_fail_total`
- `rpc_shield_request_duration_seconds`

Logging is controlled via `RUST_LOG`. The `monitoring.log_level` field is currently not read by the binary.

Example:

```bash
RUST_LOG=info,rpc_shield=debug rpc-shield --config config.yaml
```

## Examples

### Example 1: Minimal config

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
  method_limits: {}

api_keys: {}
api_key_tiers: {}

blocklist:
  ips: []
  enable_auto_ban: false
  auto_ban_threshold: 1000

monitoring:
  prometheus_port: 9090
  log_level: "info"
```

### Example 2: API keys with tier defaults

```yaml
api_keys:
  team_key:
    tier: pro
    enabled: true
    limits:
      eth_call: { requests: 500, period: "1m" }

api_key_tiers:
  free:
    eth_call: { requests: 20, period: "1m" }
  pro:
    eth_call: { requests: 200, period: "1m" }
  enterprise:
    eth_call: { requests: 1000, period: "1m" }
```
