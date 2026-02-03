# RpcShield Usage Examples

Русская версия: `doc/EXAMPLES.ru.md`

## Basic Scenarios

### 1. Run with the default configuration

```bash
# Start proxy
cargo run --release -- --config config.yaml

# In another terminal - test request
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_blockNumber",
    "params": [],
    "id": 1
  }'
```

**Expected response:**
```json
{
  "jsonrpc": "2.0",
  "result": "0x10e5c8a",
  "id": 1
}
```

### 2. Using MetaMask

**MetaMask setup:**

1. Open Settings → Networks → Add Network
2. Fill in:
   - Network Name: `My Private RPC`
   - RPC URL: `http://localhost:8545`
   - Chain ID: `1` (for mainnet)
   - Currency Symbol: `ETH`

3. Save and switch to this network

MetaMask will now go through your proxy with rate limiting.

### 3. Web3.js integration

```javascript
const Web3 = require('web3');

// Connect via proxy
const web3 = new Web3('http://localhost:8545');

// All requests are rate limited
async function getBlockNumber() {
  try {
    const blockNumber = await web3.eth.getBlockNumber();
    console.log('Current block:', blockNumber);
  } catch (error) {
    if (error.message.includes('Rate limit')) {
      console.log('Rate limit exceeded, retry later');
    }
  }
}

getBlockNumber();
```

### 4. Ethers.js integration

```javascript
const { ethers } = require('ethers');

// Provider via proxy
const provider = new ethers.JsonRpcProvider('http://localhost:8545');

async function getBalance(address) {
  try {
    const balance = await provider.getBalance(address);
    console.log('Balance:', ethers.formatEther(balance), 'ETH');
  } catch (error) {
    if (error.code === -32005) {
      console.log('Rate limited!');
    }
  }
}

getBalance('0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb');
```

## Advanced Scenarios

### 5. API keys for different applications

**config.yaml:**
```yaml
api_keys:
  frontend_app_key:
    tier: pro
    enabled: true
    limits:
      eth_call: { requests: 500, period: "1m" }
      eth_blockNumber: { requests: 1000, period: "1m" }
  
  bot_trader_key:
    tier: enterprise
    enabled: true
    limits:
      eth_call: { requests: 2000, period: "1m" }
      eth_sendRawTransaction: { requests: 100, period: "1m" }
  
  analytics_readonly_key:
    tier: free
    enabled: true
    limits:
      eth_getLogs: { requests: 50, period: "1m" }
      eth_getBlockByNumber: { requests: 100, period: "1m" }
```

**Usage in code:**

```javascript
// Frontend app
const provider = new ethers.JsonRpcProvider(
  'http://localhost:8545',
  undefined,
  {
    headers: {
      'Authorization': 'Bearer frontend_app_key'
    }
  }
);

// Trading bot
const botProvider = new ethers.JsonRpcProvider(
  'http://localhost:8545',
  undefined,
  {
    headers: {
      'X-API-Key': 'bot_trader_key'
    }
  }
);
```

### 6. Rate limiting for specific methods

**Protect against heavy calls:**

```yaml
rate_limits:
  method_limits:
    # Very heavy queries
    eth_getLogs:
      requests: 5
      period: "1m"
    
    # Heavy computations
    eth_call:
      requests: 20
      period: "1m"
    
    # Normal methods
    eth_blockNumber:
      requests: 100
      period: "1m"
    eth_chainId:
      requests: 200
      period: "1m"
```

### 7. Blocking malicious IPs

**blocklist config:**
```yaml
blocklist:
  ips:
    - "123.45.67.89"  # Suspicious bot
    - "98.76.54.32"   # DDoS attacker
  enable_auto_ban: false
  auto_ban_threshold: 1000
```

### 8. Monitoring & metrics

**Prometheus config:**
```yaml
scrape_configs:
  - job_name: 'rpc-shield'
    static_configs:
      - targets: ['localhost:9090']
```

**Metrics to watch:**
- `rpc_shield_requests_total`
- `rpc_shield_requests_rate_limited_total`
- `rpc_shield_request_duration_seconds`
- `rpc_shield_requests_upstream_fail_total`

### 9. Failover setup with multiple nodes

**Backend load balancing (example):**

```yaml
rpc_backend:
  url: "http://rpc-node-1:8546"
  timeout_seconds: 30
```

Use external load balancing (HAProxy/Nginx) to split traffic across multiple RPC nodes.

### 10. Development vs Production configs

**dev-config.yaml:**
```yaml
server:
  host: "127.0.0.1"
  port: 8545

rate_limits:
  default_ip_limit:
    requests: 1000
    period: "1m"  # More permissive for dev

monitoring:
  log_level: "debug"  # Verbose logs
```

**prod-config.yaml:**
```yaml
server:
  host: "0.0.0.0"
  port: 8545

rate_limits:
  default_ip_limit:
    requests: 100
    period: "1m"  # Strict limits

api_key_tiers:
  free:
    eth_call: { requests: 20, period: "1m" }
  pro:
    eth_call: { requests: 200, period: "1m" }

blocklist:
  enable_auto_ban: true  # Enable in prod

monitoring:
  log_level: "info"  # Less logs
  prometheus_port: 9090
```

## Docker Integration

### 11. Docker Compose setup

**docker-compose.yml:**

```yaml
version: '3.8'

services:
  geth:
    image: ethereum/client-go:latest
    ports:
      - "8546:8546"
    command: |
      --http --http.addr=0.0.0.0 --http.port=8546
      --http.api=eth,net,web3
      --syncmode=snap

  rpc-shield:
    build: .
    ports:
      - "8545:8545"
      - "9090:9090"
    volumes:
      - ./config.yaml:/app/config.yaml
    command: ["/app/rpc-shield", "--config", "/app/config.yaml"]
```

### 12. Kubernetes Deployment

**Deployment example:**

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
```

## Testing & Debugging

### 13. Load Testing

**Using vegeta:**

```bash
echo "POST http://localhost:8545" | vegeta attack -duration=30s -rate=100 | vegeta report
```

**Using k6:**

```javascript
import http from 'k6/http';
import { sleep } from 'k6';

export const options = {
  vus: 50,
  duration: '30s'
};

export default function () {
  http.post('http://localhost:8545', JSON.stringify({
    jsonrpc: '2.0',
    method: 'eth_blockNumber',
    params: [],
    id: 1
  }), {
    headers: { 'Content-Type': 'application/json' }
  });
  sleep(0.1);
}
```

### 14. Debug mode

```bash
RUST_LOG=debug cargo run -- --config config.yaml
```

### 15. Testing rate limits programmatically

```javascript
async function spamRequests() {
  for (let i = 0; i < 100; i++) {
    try {
      await provider.send('eth_blockNumber', []);
    } catch (err) {
      if (err.code === -32005) {
        console.log('Rate limited at request', i);
        break;
      }
    }
  }
}
```

## Integrations with popular tools

### 16. Hardhat integration

```javascript
module.exports = {
  networks: {
    rpcshield: {
      url: 'http://localhost:8545'
    }
  }
};
```

### 17. Foundry integration

```toml
[rpc_endpoints]
rpcshield = "http://localhost:8545"
```

### 18. Alchemy/Infura replacement

Replace your Alchemy/Infura endpoint URL with `http://localhost:8545` in your app config.
