# Deployment Guide

This guide covers various deployment scenarios for RPC Shield.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Bare Metal Deployment](#bare-metal-deployment)
- [Cloud Deployments](#cloud-deployments)
- [Production Best Practices](#production-best-practices)

## Prerequisites

### Hardware Requirements

**Minimum:**
- 2 CPU cores
- 4GB RAM
- 20GB SSD storage
- 100 Mbps network

**Recommended (Production):**
- 4+ CPU cores
- 8GB+ RAM
- 100GB SSD storage
- 1 Gbps network

### Software Requirements

- Docker 20.10+ and Docker Compose 2.0+
- Kubernetes 1.24+ (for K8s deployment)
- PostgreSQL 14+ (optional, for SaaS features)
- Redis 6+ (optional, for distributed rate limiting)

## Docker Deployment

### 1. Basic Docker Deployment

```bash
# Clone repository
git clone https://github.com/yourusername/rpc-shield.git
cd rpc-shield

# Copy and edit configuration
cp config.yaml config.prod.yaml
vim config.prod.yaml

# Build image
docker build -f docker/Dockerfile -t rpc-shield:latest .

# Run container
docker run -d \
  --name rpc-shield \
  -p 8545:8545 \
  -p 8555:8555 \
  -p 9090:9090 \
  -v $(pwd)/config.prod.yaml:/app/config.yaml:ro \
  --restart unless-stopped \
  rpc-shield:latest
```

### 2. Docker Compose Deployment

**docker-compose.prod.yml:**

```yaml
version: '3.8'

services:
  rpc-shield:
    image: rpc-shield:latest
    build:
      context: .
      dockerfile: docker/Dockerfile
    ports:
      - "8545:8545"
      - "8555:8555"
      - "9090:9090"
    environment:
      - RUST_LOG=info,rpc_shield=debug
      - DATABASE_URL=${DATABASE_URL}
      - REDIS_URL=${REDIS_URL}
    volumes:
      - ./config.prod.yaml:/app/config.yaml:ro
      - ./logs:/app/logs
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    restart: unless-stopped
    networks:
      - rpc-shield-net
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8545/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: ${POSTGRES_DB:-rpc_shield}
      POSTGRES_USER: ${POSTGRES_USER:-rpc_shield}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
    volumes:
      - postgres_data:/var/lib/postgresql/data
    networks:
      - rpc-shield-net
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${POSTGRES_USER:-rpc_shield}"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis_data:/data
    networks:
      - rpc-shield-net
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "redis-cli", "--raw", "incr", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9091:9090"
    volumes:
      - ./docker/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus_data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--storage.tsdb.retention.time=30d'
    networks:
      - rpc-shield-net
    restart: unless-stopped

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=${GRAFANA_PASSWORD:-admin}
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_SERVER_ROOT_URL=https://monitoring.yourdomain.com
    volumes:
      - grafana_data:/var/lib/grafana
      - ./docker/grafana/dashboards:/etc/grafana/provisioning/dashboards:ro
      - ./docker/grafana/datasources:/etc/grafana/provisioning/datasources:ro
    depends_on:
      - prometheus
    networks:
      - rpc-shield-net
    restart: unless-stopped

volumes:
  postgres_data:
  redis_data:
  prometheus_data:
  grafana_data:

networks:
  rpc-shield-net:
    driver: bridge
```

**Environment File (.env):**

```bash
# Database
POSTGRES_DB=rpc_shield
POSTGRES_USER=rpc_shield
POSTGRES_PASSWORD=your_secure_password_here

# Redis
REDIS_PASSWORD=your_redis_password_here

# Grafana
GRAFANA_PASSWORD=your_grafana_password_here

# RPC Shield
DATABASE_URL=postgres://rpc_shield:your_secure_password_here@postgres:5432/rpc_shield
REDIS_URL=redis://:your_redis_password_here@redis:6379
```

**Deploy:**

```bash
# Create .env file
cp .env.example .env
vim .env  # Edit with your passwords

# Deploy
docker-compose -f docker-compose.prod.yml up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f rpc-shield
```

## Kubernetes Deployment

### 1. Namespace

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: rpc-shield
```

### 2. ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rpc-shield-config
  namespace: rpc-shield
data:
  config.yaml: |
    server:
      host: "0.0.0.0"
      port: 8545
      mode: saas
    
    rpc_backend:
      url: "http://ethereum-node:8545"
      timeout_seconds: 30
    
    # ... rest of your config
```

### 3. Secrets

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: rpc-shield-secrets
  namespace: rpc-shield
type: Opaque
stringData:
  database-url: "postgres://user:pass@postgres:5432/rpc_shield"
  redis-url: "redis://redis:6379"
```

### 4. Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rpc-shield
  namespace: rpc-shield
  labels:
    app: rpc-shield
spec:
  replicas: 3
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
          name: proxy
        - containerPort: 8555
          name: admin
        - containerPort: 9090
          name: metrics
        env:
        - name: RUST_LOG
          value: "info,rpc_shield=debug"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: rpc-shield-secrets
              key: database-url
        - name: REDIS_URL
          valueFrom:
            secretKeyRef:
              name: rpc-shield-secrets
              key: redis-url
        volumeMounts:
        - name: config
          mountPath: /app/config.yaml
          subPath: config.yaml
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8545
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8545
          initialDelaySeconds: 10
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: rpc-shield-config
```

### 5. Service

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: rpc-shield
  namespace: rpc-shield
spec:
  type: LoadBalancer
  selector:
    app: rpc-shield
  ports:
  - name: proxy
    port: 8545
    targetPort: 8545
  - name: admin
    port: 8555
    targetPort: 8555
  - name: metrics
    port: 9090
    targetPort: 9090
```

### 6. HorizontalPodAutoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rpc-shield-hpa
  namespace: rpc-shield
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: rpc-shield
  minReplicas: 3
  maxReplicas: 10
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
```

**Deploy to Kubernetes:**

```bash
# Apply all manifests
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/service.yaml
kubectl apply -f k8s/hpa.yaml

# Check status
kubectl get pods -n rpc-shield
kubectl get svc -n rpc-shield

# View logs
kubectl logs -f deployment/rpc-shield -n rpc-shield
```

## Bare Metal Deployment

### 1. System Preparation

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y build-essential pkg-config libssl-dev curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 2. Build Application

```bash
# Clone repository
git clone https://github.com/yourusername/rpc-shield.git
cd rpc-shield

# Build release
cargo build --release --features saas

# Install binary
sudo cp target/release/rpc-shield /usr/local/bin/
sudo chmod +x /usr/local/bin/rpc-shield
```

### 3. Create User

```bash
# Create system user
sudo useradd -r -s /bin/false rpcshield

# Create directories
sudo mkdir -p /etc/rpc-shield
sudo mkdir -p /var/log/rpc-shield
sudo mkdir -p /var/lib/rpc-shield

# Set permissions
sudo chown -R rpcshield:rpcshield /var/log/rpc-shield
sudo chown -R rpcshield:rpcshield /var/lib/rpc-shield
```

### 4. Configuration

```bash
# Copy config
sudo cp config.yaml /etc/rpc-shield/config.yaml
sudo chown rpcshield:rpcshield /etc/rpc-shield/config.yaml
sudo chmod 640 /etc/rpc-shield/config.yaml

# Edit config
sudo vim /etc/rpc-shield/config.yaml
```

### 5. Systemd Service

Create `/etc/systemd/system/rpc-shield.service`:

```ini
[Unit]
Description=RPC Shield - Web3 RPC Rate Limiter
After=network.target postgresql.service redis.service
Wants=postgresql.service redis.service

[Service]
Type=simple
User=rpcshield
Group=rpcshield
ExecStart=/usr/local/bin/rpc-shield --config /etc/rpc-shield/config.yaml
Restart=always
RestartSec=10

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/rpc-shield /var/lib/rpc-shield

# Logging
StandardOutput=append:/var/log/rpc-shield/rpc-shield.log
StandardError=append:/var/log/rpc-shield/rpc-shield-error.log

# Environment
Environment="RUST_LOG=info,rpc_shield=debug"
Environment="DATABASE_URL=postgres://user:pass@localhost/rpc_shield"
Environment="REDIS_URL=redis://localhost:6379"

[Install]
WantedBy=multi-user.target
```

**Enable and Start:**

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service
sudo systemctl enable rpc-shield

# Start service
sudo systemctl start rpc-shield

# Check status
sudo systemctl status rpc-shield

# View logs
sudo journalctl -u rpc-shield -f
```

### 6. Nginx Reverse Proxy (Optional)

```nginx
# /etc/nginx/sites-available/rpc-shield
upstream rpc_shield {
    least_conn;
    server 127.0.0.1:8545 max_fails=3 fail_timeout=30s;
}

server {
    listen 443 ssl http2;
    server_name rpc.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/rpc.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/rpc.yourdomain.com/privkey.pem;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000" always;
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;

    location / {
        proxy_pass http://rpc_shield;
        proxy_http_version 1.1;
        
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
        
        # Buffering
        proxy_buffering off;
    }
}

server {
    listen 80;
    server_name rpc.yourdomain.com;
    return 301 https://$server_name$request_uri;
}
```

Enable the site:

```bash
sudo ln -s /etc/nginx/sites-available/rpc-shield /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

## Cloud Deployments

### AWS ECS Deployment

**Task Definition (task-definition.json):**

```json
{
  "family": "rpc-shield",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "1024",
  "memory": "2048",
  "containerDefinitions": [
    {
      "name": "rpc-shield",
      "image": "your-account.dkr.ecr.region.amazonaws.com/rpc-shield:latest",
      "portMappings": [
        {
          "containerPort": 8545,
          "protocol": "tcp"
        },
        {
          "containerPort": 8555,
          "protocol": "tcp"
        },
        {
          "containerPort": 9090,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "RUST_LOG",
          "value": "info,rpc_shield=debug"
        }
      ],
      "secrets": [
        {
          "name": "DATABASE_URL",
          "valueFrom": "arn:aws:secretsmanager:region:account:secret:rpc-shield/database-url"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/rpc-shield",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:8545/health || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 60
      }
    }
  ]
}
```

**Deploy:**

```bash
# Create task definition
aws ecs register-task-definition --cli-input-json file://task-definition.json

# Create service
aws ecs create-service \
  --cluster your-cluster \
  --service-name rpc-shield \
  --task-definition rpc-shield \
  --desired-count 3 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[subnet-xxx],securityGroups=[sg-xxx],assignPublicIp=ENABLED}" \
  --load-balancers "targetGroupArn=arn:aws:elasticloadbalancing:region:account:targetgroup/rpc-shield/xxx,containerName=rpc-shield,containerPort=8545"
```

### GCP Cloud Run Deployment

```bash
# Build and push image
gcloud builds submit --tag gcr.io/PROJECT_ID/rpc-shield

# Deploy
gcloud run deploy rpc-shield \
  --image gcr.io/PROJECT_ID/rpc-shield \
  --platform managed \
  --region us-central1 \
  --port 8545 \
  --memory 2Gi \
  --cpu 2 \
  --min-instances 2 \
  --max-instances 10 \
  --set-env-vars RUST_LOG=info,rpc_shield=debug \
  --set-secrets DATABASE_URL=rpc-shield-db-url:latest
```

## Production Best Practices

### 1. Security

- Always use TLS/SSL in production
- Rotate API keys regularly
- Use secrets management (Vault, AWS Secrets Manager)
- Enable firewall rules
- Regular security audits

### 2. High Availability

- Run multiple instances (minimum 3)
- Use load balancer
- Configure auto-scaling
- Set up health checks
- Plan for failover

### 3. Monitoring

- Enable Prometheus metrics
- Set up Grafana dashboards
- Configure alerting (PagerDuty, Slack)
- Monitor logs (ELK Stack, Loki)
- Track error rates

### 4. Backup & Recovery

- Regular database backups
- Config version control
- Disaster recovery plan
- Document procedures
- Test recovery regularly

### 5. Performance Optimization

- Tune rate limits based on load
- Enable Redis for distributed limiting
- Use connection pooling
- Monitor resource usage
- Regular performance testing

### 6. Maintenance

- Plan maintenance windows
- Blue-green deployments
- Rolling updates
- Keep dependencies updated
- Regular security patches

## Troubleshooting

### Common Issues

1. **Service won't start:**
   - Check config file syntax
   - Verify ports are available
   - Check file permissions
   - Review logs

2. **High memory usage:**
   - Adjust cleanup intervals
   - Check for memory leaks
   - Monitor stats retention

3. **Database connection issues:**
   - Verify credentials
   - Check network connectivity
   - Ensure database is running
   - Review connection pool settings

4. **Rate limiting not working:**
   - Verify config is loaded
   - Check limits are reasonable
   - Test with curl
   - Review debug logs

### Getting Help

- Check logs: `docker-compose logs -f rpc-shield`
- Review metrics: `http://localhost:9090/metrics`
- Admin API: `http://localhost:8555/api/admin/stats`
- GitHub Issues: https://github.com/yourusername/rpc-shield/issues

---

For more information, see:
- [Configuration Guide](CONFIGURATION.md)
- [API Documentation](API.md)
- [Monitoring Guide](MONITORING.md)