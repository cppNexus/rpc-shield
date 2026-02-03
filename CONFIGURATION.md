# Configuration Guide

Complete reference for RPC Shield configuration.

## Table of Contents

- [Configuration File](#configuration-file)
- [Server Configuration](#server-configuration)
- [RPC Backend](#rpc-backend)
- [Rate Limiting](#rate-limiting)
- [Method Groups](#method-groups)
- [API Keys](#api-keys)
- [Blocklist](#blocklist)
- [Billing & Webhooks](#billing--webhooks)
- [Monitoring](#monitoring)
- [Hot Reload](#hot-reload)
- [Admin API](#admin-api)
- [Environment Variables](#environment-variables)
- [Examples](#examples)

## Configuration File

RPC Shield uses YAML configuration files. By default, it looks for `config.yaml` in the current directory.

```bash
# Specify custom config file
rpc-shield --config /path/to/config.yaml
```

## Server Configuration

```yaml
server:
  host: "0.0.0.0"          # Bind address (0.0.0.0 for all interfaces)
  port: 8545               # Port for the main proxy
  mode: self-hosted        # Operation mode: self-hosted | saas
```

### Operation Modes

**self-hosted:**
- In-memory rate limiting
- File-based configuration
- No external dependencies
- Perfect for single-node deployments

**saas:**
- PostgreSQL for persistence
- Redis for distributed rate limiting
- Billing hooks support
- Designed for multi-node deployments

## RPC Backend

Configure the upstream Ethereum node:

```yaml
rpc_backend:
  url: "http://localhost:8546"    # Backend RPC endpoint
  timeout_seconds: 30             # Request timeout
```

### Multiple Backends (Future Feature)

```yaml
rpc_backend:
  nodes:
    - url: "http://node1:8545"
      weight: 2
    - url: "http://node2:8545"
      weight: 1
  strategy: round-robin    # or weighted, least-connections
```

## Rate Limiting

### Basic Rate Limit

```yaml
rate_limits:
  default_ip_limit:
    requests: 100        # Number of requests
    period: "1m"         # Time period: 1s, 1m, 1h
```

### With Burst Support

```yaml
rate_limits:
  default_ip_limit:
    requests: 100
    period: "1m"
    burst:
      size: 20           # Extra tokens for burst traffic
      refill_rate: 5     # Tokens added per refill period
      refill_period: "10s"  # How often to refill
```

### Method-Specific Limits

```yaml
rate_limits:
  method_limits:
    eth_blockNumber:
      requests: 200
      period: "1m"
      burst:
        size: 50
        refill_rate: 10
        refill_period: "10s"
    
    eth_call:
      requests: 50
      period: "1m"
    
    eth_getLogs:
      requests: 20
      period: "1m"
```

### Group Limits

Apply limits to groups of methods:

```yaml
rate_limits:
  group_limits:
    read-only:
      requests: 500
      period: "1m"
      burst:
        size: 100
        refill_rate: 20
        refill_period: "10s"
```

### Limit Priority

Limits are applied in this order (highest to lowest):

1. API key + specific method
2. API key + method group
3. Global method limit
4. Global group limit
5. Default IP limit

## Method Groups

Group related methods together:

```yaml
method_groups:
  read-only:
    description: "Read-only RPC methods"
    category: read-only    # read-only | state-changing | heavy | light | custom
    methods:
      - "eth_blockNumber"
      - "eth_getBalance"
      - "eth_call"
      - "eth_getCode"
      - "eth_getStorageAt"
      - "eth_estimateGas"
  
  state-changing:
    description: "Methods that change blockchain state"
    category: state-changing
    methods:
      - "eth_sendRawTransaction"
      - "eth_sendTransaction"
  
  heavy:
    description: "Resource-intensive operations"
    category: heavy
    methods:
      - "eth_getLogs"
      - "eth_newFilter"
      - "eth_getFilterChanges"
      - "debug_traceTransaction"
      - "debug_traceBlockByNumber"
```

### Method Categories

- **read-only** - Read operations, no state changes
- **state-changing** - Transactions and state modifications
- **heavy** - Resource-intensive operations
- **light** - Fast, cheap operations
- **custom** - User-defined category

## API Keys

Define API keys with custom limits:

```yaml
api_keys:
  # Free tier
  free_tier_key:
    tier: free
    enabled: true
    limits:
      eth_blockNumber:
        requests: 300
        period: "1m"
    group_limits:
      read-only:
        requests: 500
        period: "1m"
    billing_hooks: []
  
  # Pro tier
  pro_tier_key:
    tier: pro
    enabled: true
    limits:
      eth_blockNumber:
        requests: 1000
        period: "1m"
        burst:
          size: 200
          refill_rate: 50
          refill_period: "10s"
      eth_call:
        requests: 500
        period: "1m"
    group_limits:
      read-only:
        requests: 5000
        period: "1m"
        burst:
          size: 1000
          refill_rate: 200
          refill_period: "10s"
    billing_hooks:
      - type: webhook
        url: "https://billing.example.com/webhook"
        events:
          - "rate_limit_exceeded"
          - "quota_warning"
  
  # Enterprise tier
  enterprise_key:
    tier: enterprise
    enabled: true
    limits:
      eth_blockNumber:
        requests: 10000
        period: "1m"
    group_limits:
      read-only:
        requests: 50000
        period: "1m"
    billing_hooks:
      - type: database
        events:
          - "request_completed"
```

### Subscription Tiers

- **free** - Basic access with lower limits
- **pro** - Enhanced limits and burst support
- **enterprise** - Highest limits and priority support
- **custom** - Fully customizable limits

### API Key Usage

Clients can use API keys via:

```bash
# Authorization header (Bearer token)
curl -X POST http://localhost:8545 \
  -H "Authorization: Bearer your_api_key" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# X-API-Key header
curl -X POST http://localhost:8545 \
  -H "X-API-Key: your_api_key" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

## Blocklist

### Static Blocklist

```yaml
blocklist:
  ips:
    - "192.168.1.100"
    - "10.0.0.50"
    - "172.16.0.25"
  enable_auto_ban: false
  auto_ban_threshold: 1000
```

### Auto-Ban

Automatically block IPs that exceed rate limits:

```yaml
blocklist:
  ips: []
  enable_auto_ban: true
  auto_ban_threshold: 100    # Ban after 100 violations in 1 hour
```

When `enable_auto_ban` is true:
- System tracks rate limit violations per IP
- After threshold violations in a rolling 1-hour window, IP is auto-banned
- Bans are stored in-memory (or PostgreSQL in SaaS mode)
- Violation counter resets after 1 hour of no violations

## Billing & Webhooks

### Basic Configuration

```yaml
billing:
  enabled: false                 # Enable billing features
  webhook_timeout_ms: 5000      # Webhook timeout
  hooks: []
```

### Webhook Hooks

```yaml
billing:
  enabled: true
  webhook_timeout_ms: 5000
  hooks:
    - type: webhook
      url: "https://your-system.com/billing/webhook"
      events:
        - "rate_limit_exceeded"
        - "quota_warning"
        - "quota_exceeded"
        - "request_completed"
        - "request_failed"
```

### Queue Hooks (Future Feature)

```yaml
billing:
  enabled: true
  hooks:
    - type: queue
      queue_name: "billing_events"
      events:
        - "rate_limit_exceeded"
        - "request_completed"
```

### Database Hooks

```yaml
billing:
  enabled: true
  hooks:
    - type: database
      events:
        - "request_completed"
        - "rate_limit_exceeded"
```

### Billing Events

- **rate_limit_exceeded** - Client exceeded rate limit
- **quota_warning** - Client approaching quota limit (e.g., 80%)
- **quota_exceeded** - Client exceeded quota
- **request_completed** - Successful request (for metering)
- **request_failed** - Failed request

### Webhook Payload

```json
{
  "event": "rate_limit_exceeded",
  "identity": "ip:192.168.1.1",
  "method": "eth_call",
  "timestamp": "2024-01-29T12:00:00Z",
  "metadata": {
    "limit": "100",
    "method": "eth_call"
  }
}
```

## Monitoring

```yaml
monitoring:
  prometheus_port: 9090        # Port for Prometheus metrics
  log_level: "info"            # trace | debug | info | warn | error
```

### Log Levels

- **trace** - Very detailed, for deep debugging
- **debug** - Detailed information for debugging
- **info** - General informational messages
- **warn** - Warning messages
- **error** - Error messages only

### Prometheus Metrics

Available at `http://localhost:9090/metrics`:

- `polymorph_proxy_requests_total` - Total requests
- `polymorph_proxy_requests_successful` - Successful requests
- `polymorph_proxy_requests_failed` - Failed requests
- `polymorph_proxy_requests_rate_limited` - Rate limited requests
- `polymorph_proxy_requests_blocked` - Blocked requests
- `polymorph_proxy_bytes_sent` - Total bytes sent
- `polymorph_proxy_bytes_received` - Total bytes received
- `polymorph_proxy_uptime_seconds` - Uptime
- `polymorph_proxy_method_requests_total{method}` - Per-method requests
- `polymorph_proxy_method_response_time_ms{method,stat}` - Response times

## Hot Reload

Enable configuration hot reload:

```yaml
hot_reload:
  enabled: true
  watch_file: "config.yaml"    # File to watch
  debounce_ms: 1000            # Debounce delay
```

When enabled:
- Configuration file is monitored for changes
- Changes are automatically reloaded
- No service restart required
- Debounce prevents rapid reloads

### Reload via Signal (Unix only)

```bash
# Send SIGHUP to reload
kill -HUP <pid>

# Or with systemd
systemctl reload rpc-shield
```

## Admin API

```yaml
admin_api:
  enabled: true
  port: 8555                   # Admin API port
  auth_token: null             # Optional authentication token
```

### With Authentication

```yaml
admin_api:
  enabled: true
  port: 8555
  auth_token: "your-secret-token"
```

Then use:

```bash
curl -H "Authorization: Bearer your-secret-token" \
  http://localhost:8555/api/admin/stats
```

## Environment Variables

Override config with environment variables:

```bash
# Database (SaaS mode)
export DATABASE_URL="postgres://user:pass@localhost/rpc_shield"

# Redis (SaaS mode)
export REDIS_URL="redis://localhost:6379"

# Logging
export RUST_LOG="info,rpc_shield=debug"

# Custom config file
export RPC_SHIELD_CONFIG="/path/to/config.yaml"
```

## Examples

### Example 1: Simple Self-Hosted

```yaml
server:
  host: "0.0.0.0"
  port: 8545
  mode: self-hosted

rpc_backend:
  url: "http://localhost:8546"
  timeout_seconds: 30

rate_limits:
  default_ip_limit:
    requests: 100
    period: "1m"
  method_limits: {}
  group_limits: {}

method_groups: {}
api_keys: {}

blocklist:
  ips: []
  enable_auto_ban: false
  auto_ban_threshold: 1000

billing:
  enabled: false
  hooks: []
  webhook_timeout_ms: 5000

monitoring:
  prometheus_port: 9090
  log_level: "info"

hot_reload:
  enabled: false
  watch_file: "config.yaml"
  debounce_ms: 1000

admin_api:
  enabled: false
  port: 8555
  auth_token: null
```

### Example 2: Production with API Keys

```yaml
server:
  host: "0.0.0.0"
  port: 8545
  mode: self-hosted

rpc_backend:
  url: "http://ethereum-node:8545"
  timeout_seconds: 30

rate_limits:
  default_ip_limit:
    requests: 50
    period: "1m"
    burst:
      size: 10
      refill_rate: 2
      refill_period: "10s"
  
  method_limits:
    eth_call:
      requests: 30
      period: "1m"
    eth_getLogs:
      requests: 10
      period: "1m"
  
  group_limits:
    read-only:
      requests: 200
      period: "1m"

method_groups:
  read-only:
    description: "Read-only methods"
    category: read-only
    methods:
      - "eth_blockNumber"
      - "eth_getBalance"
      - "eth_call"

api_keys:
  premium_user_abc123:
    tier: pro
    enabled: true
    limits:
      eth_call:
        requests: 500
        period: "1m"
        burst:
          size: 100
          refill_rate: 20
          refill_period: "10s"
    group_limits:
      read-only:
        requests: 2000
        period: "1m"

blocklist:
  ips:
    - "192.168.1.100"
  enable_auto_ban: true
  auto_ban_threshold: 50

billing:
  enabled: false
  hooks: []
  webhook_timeout_ms: 5000

monitoring:
  prometheus_port: 9090
  log_level: "info"

hot_reload:
  enabled: true
  watch_file: "config.yaml"
  debounce_ms: 1000

admin_api:
  enabled: true
  port: 8555
  auth_token: "your-secure-token"
```

### Example 3: Full SaaS Setup

```yaml
server:
  host: "0.0.0.0"
  port: 8545
  mode: saas

rpc_backend:
  url: "http://ethereum-node:8545"
  timeout_seconds: 30

rate_limits:
  default_ip_limit:
    requests: 100
    period: "1m"
  method_limits: {}
  group_limits: {}

method_groups:
  read-only:
    description: "Read-only methods"
    category: read-only
    methods:
      - "eth_blockNumber"
      - "eth_getBalance"
      - "eth_call"

api_keys:
  # Keys managed via Admin API in SaaS mode
  {}

blocklist:
  ips: []
  enable_auto_ban: true
  auto_ban_threshold: 100

billing:
  enabled: true
  webhook_timeout_ms: 5000
  hooks:
    - type: webhook
      url: "https://billing.example.com/webhook"
      events:
        - "rate_limit_exceeded"
        - "quota_exceeded"
        - "request_completed"

redis:
  url: "redis://redis:6379"
  enabled: true

database:
  url: "postgres://rpc_shield:password@postgres:5432/rpc_shield"
  max_connections: 20

monitoring:
  prometheus_port: 9090
  log_level: "info"

hot_reload:
  enabled: true
  watch_file: "config.yaml"
  debounce_ms: 1000

admin_api:
  enabled: true
  port: 8555
  auth_token: "your-secure-admin-token"
```

## Configuration Validation

Validate your configuration:

```bash
# Test configuration
rpc-shield --config config.yaml --validate

# Start with debug logging
RUST_LOG=debug rpc-shield --config config.yaml
```

## Best Practices

1. **Start Conservative**
   - Begin with stricter limits
   - Gradually increase based on load

2. **Use Burst for Peaks**
   - Allow burst for legitimate traffic spikes
   - Set reasonable refill rates

3. **Group Similar Methods**
   - Create meaningful method groups
   - Apply group limits for efficiency

4. **Monitor and Adjust**
   - Watch Prometheus metrics
   - Adjust limits based on patterns

5. **Security**
   - Use auth tokens for Admin API
   - Rotate API keys regularly
   - Enable auto-ban for DDoS protection

6. **High Availability**
   - Use SaaS mode for distributed setup
   - Enable hot reload for zero-downtime updates
   - Configure proper health checks

---

For more information:
- [Deployment Guide](DEPLOYMENT.md)
- [API Documentation](API.md)
- [Monitoring Guide](MONITORING.md)