# Deployment Guide

This guide covers **current** deployment options for RpcShield (HTTP JSON-RPC proxy + rate limiting). It intentionally avoids unimplemented features.

## Prerequisites

- A running RPC node (Geth, Erigon, Nethermind, etc.)
- One of:
  - Rust toolchain (for bare metal builds)
  - Docker / Docker Compose (for containerized deployment)

## Docker Deployment

### 1. Docker Compose (recommended)

The repository includes `docker-compose.yml` with:
- `rpc-shield` (proxy)
- `geth` (local node for testing)
- `prometheus` (metrics scraping)

```bash
docker compose up -d
```

Ports (default):
- `8545` — proxy
- `8546` — geth
- `9090` — Prometheus

### 2. Docker Run

```bash
# Build the image
docker build -t rpc-shield:latest .

# Run the container
docker run -d \
  --name rpc-shield \
  -p 8545:8545 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/app/config.yaml:ro \
  rpc-shield:latest --config /app/config.yaml
```

## Kubernetes Deployment (minimal example)

**ConfigMap:**

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rpc-shield-config
data:
  config.yaml: |
    server:
      host: "0.0.0.0"
      port: 8545
    rpc_backend:
      url: "http://ethereum-node:8545"
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

**Deployment + Service:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rpc-shield
spec:
  replicas: 2
  selector:
    matchLabels:
      app: rpc-shield
  template:
    metadata:
      labels:
        app: rpc-shield
    spec:
      containers:
        - name: rpc-shield
          image: rpc-shield:latest
          ports:
            - containerPort: 8545
            - containerPort: 9090
          volumeMounts:
            - name: config
              mountPath: /app/config.yaml
              subPath: config.yaml
      volumes:
        - name: config
          configMap:
            name: rpc-shield-config
---
apiVersion: v1
kind: Service
metadata:
  name: rpc-shield
spec:
  selector:
    app: rpc-shield
  ports:
    - name: proxy
      port: 8545
      targetPort: 8545
    - name: metrics
      port: 9090
      targetPort: 9090
```

## Bare Metal Deployment

### Build

```bash
cargo build --release
```

### Run

```bash
./target/release/rpc-shield --config /etc/rpc-shield/config.yaml
```

### systemd (optional)

Create `/etc/systemd/system/rpc-shield.service`:

```ini
[Unit]
Description=RPC Shield - Web3 RPC Rate Limiter
After=network.target

[Service]
Type=simple
User=rpcshield
Group=rpcshield
ExecStart=/usr/local/bin/rpc-shield --config /etc/rpc-shield/config.yaml
Restart=always
RestartSec=10

# Logging
Environment="RUST_LOG=info,rpc_shield=debug"

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable rpc-shield
sudo systemctl start rpc-shield
```

## Production Notes

- Use TLS via a reverse proxy (nginx, HAProxy, etc.).
- Set sane limits for heavy methods like `eth_getLogs`.
- Monitor `/metrics` and `/health` for visibility.
- Rotate API keys if you use them for external clients.

## Troubleshooting

1. **Service won't start**
   - Validate YAML syntax
   - Check port availability
   - Run with `RUST_LOG=debug`

2. **Rate limiting not working**
   - Confirm you are hitting the proxy, not the backend
   - Check limit precedence in `CONFIGURATION.md`

3. **Metrics not available**
   - Ensure `monitoring.prometheus_port` is reachable
   - Check for firewall or container port mapping issues

For configuration details, see `CONFIGURATION.md`.
