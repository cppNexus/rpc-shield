# Примеры использования RpcShield



## Базовые сценарии

### 1. Запуск с дефолтной конфигурацией

```bash
# Запуск прокси
cargo run --release -- --config config.yaml

# В другом терминале - тестовый запрос
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_blockNumber",
    "params": [],
    "id": 1
  }'
```

**Ожидаемый ответ:**
```json
{
  "jsonrpc": "2.0",
  "result": "0x10e5c8a",
  "id": 1
}
```

### 2. Использование с MetaMask

**Настройка MetaMask:**

1. Открыть Settings → Networks → Add Network
2. Заполнить:
   - Network Name: `My Private RPC`
   - RPC URL: `http://localhost:8545`
   - Chain ID: `1` (для mainnet)
   - Currency Symbol: `ETH`

3. Сохранить и переключиться на эту сеть

MetaMask теперь будет проходить через ваш прокси с rate limiting!

### 3. Интеграция с Web3.js

```javascript
const Web3 = require('web3');

// Подключение через прокси
const web3 = new Web3('http://localhost:8545');

// Все запросы теперь проходят rate limiting
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

### 4. Использование с Ethers.js

```javascript
const { ethers } = require('ethers');

// Провайдер через прокси
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

## Продвинутые сценарии

### 5. API-ключи для различных приложений

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

**Использование в коде:**

```javascript
// Frontend приложение
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

### 6. Rate Limiting для специфичных методов

**Защита от тяжёлых запросов:**

```yaml
rate_limits:
  method_limits:
    # Очень тяжёлые запросы
    eth_getLogs:
      requests: 5
      period: "1m"
    
    # Тяжёлые вычисления
    eth_call:
      requests: 20
      period: "1m"
    
    # Лёгкие запросы
    eth_blockNumber:
      requests: 100
      period: "1m"
    
    # Защита от spam транзакций
    eth_sendRawTransaction:
      requests: 10
      period: "1m"
```

**Тест защиты:**

```bash
# Этот скрипт быстро достигнет лимита eth_getLogs
for i in {1..10}; do
  curl -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{
      "jsonrpc": "2.0",
      "method": "eth_getLogs",
      "params": [{"fromBlock": "latest"}],
      "id": '$i'
    }'
done
```

### 7. Блокировка вредоносных IP

```yaml
blocklist:
  ips:
    - "192.168.1.100"    # Замеченный в DDoS
    - "10.0.0.50"        # Подозрительный bot
    - "172.16.0.0/12"    # Целая подсеть
  enable_auto_ban: true
  auto_ban_threshold: 1000  # ban после 1000 отказов в минуту
```

**Логика auto-ban (в разработке):**
- Если IP превышает лимиты 1000 раз за период
- Автоматическое добавление в blocklist
- Уведомление админа
- Возможность unban через Admin API

### 8. Мониторинг и метрики

**Prometheus scraping:**

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'polymorph-proxy'
    static_configs:
      - targets: ['localhost:9090']
```

**Grafana dashboard queries:**

```promql
# RPS по методам
rate(rpc_requests_total[1m])

# Rate limit hit rate
rate(rate_limit_exceeded_total[1m])

# Latency percentiles
histogram_quantile(0.99, 
  rate(rpc_request_duration_seconds_bucket[5m])
)
```

### 9. Failover setup с несколькими нодами

```yaml
# config.yaml для primary прокси
rpc_backend:
  url: "http://geth-primary:8546"
  timeout_seconds: 30

# config-fallback.yaml для резервного
rpc_backend:
  url: "http://geth-secondary:8546"
  timeout_seconds: 30
```

**HAProxy перед проксями:**

```
frontend rpc_frontend
  bind *:8545
  default_backend rpc_proxies

backend rpc_proxies
  balance roundrobin
  option httpchk GET /health
  server proxy1 localhost:8545 check
  server proxy2 localhost:8546 check
```

### 10. Development vs Production конфиги

**dev-config.yaml:**
```yaml
server:
  host: "127.0.0.1"
  port: 8545
  mode: self-hosted

rate_limits:
  default_ip_limit:
    requests: 1000
    period: "1m"  # Более щедрые лимиты для dev

monitoring:
  log_level: "debug"  # Подробные логи
```

**prod-config.yaml:**
```yaml
server:
  host: "0.0.0.0"
  port: 8545
  mode: saas

rate_limits:
  default_ip_limit:
    requests: 100
    period: "1m"  # Строгие лимиты

blocklist:
  enable_auto_ban: true  # Включить в prod

monitoring:
  log_level: "info"  # Меньше логов
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
      --mainnet
    volumes:
      - geth-data:/root/.ethereum

  polymorph-proxy:
    build: .
    ports:
      - "8545:8545"
      - "9090:9090"
    depends_on:
      - geth
    volumes:
      - ./config.yaml:/app/config.yaml
    environment:
      - RUST_LOG=info
    command: ["--config", "/app/config.yaml"]

volumes:
  geth-data:
```

**Dockerfile:**

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/polymorph-proxy /usr/local/bin/
EXPOSE 8545 9090
CMD ["polymorph-proxy"]
```

**Запуск:**
```bash
docker-compose up -d
```

### 12. Kubernetes Deployment

**k8s-deployment.yaml:**

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: polymorph-proxy
spec:
  replicas: 3
  selector:
    matchLabels:
      app: polymorph-proxy
  template:
    metadata:
      labels:
        app: polymorph-proxy
    spec:
      containers:
      - name: proxy
        image: your-registry/polymorph-proxy:latest
        ports:
        - containerPort: 8545
        - containerPort: 9090
        env:
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: config
          mountPath: /app/config.yaml
          subPath: config.yaml
      volumes:
      - name: config
        configMap:
          name: proxy-config

---
apiVersion: v1
kind: Service
metadata:
  name: polymorph-proxy
spec:
  type: LoadBalancer
  selector:
    app: polymorph-proxy
  ports:
  - name: rpc
    port: 8545
    targetPort: 8545
  - name: metrics
    port: 9090
    targetPort: 9090
```

## Тестирование и Debugging

### 13. Load Testing

**Apache Bench:**
```bash
# 1000 запросов, 10 параллельных соединений
ab -n 1000 -c 10 -p request.json -T application/json \
   http://localhost:8545/
```

**request.json:**
```json
{
  "jsonrpc": "2.0",
  "method": "eth_blockNumber",
  "params": [],
  "id": 1
}
```

**k6 script:**
```javascript
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 50,
  duration: '30s',
};

export default function () {
  const payload = JSON.stringify({
    jsonrpc: '2.0',
    method: 'eth_blockNumber',
    params: [],
    id: 1,
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
  };

  const res = http.post('http://localhost:8545', payload, params);
  
  check(res, {
    'status is 200': (r) => r.status === 200,
    'not rate limited': (r) => !r.body.includes('Rate limit'),
  });
}
```

### 14. Debug mode

```bash
# Запуск с debug логами
RUST_LOG=debug cargo run -- --config config.yaml

# Отслеживание конкретного модуля
RUST_LOG=polymorph_proxy::rate_limiter=trace cargo run
```

### 15. Testing rate limits programmatically

```python
import requests
import time

PROXY_URL = "http://localhost:8545"

def test_rate_limit():
    """Проверяет, что rate limit работает"""
    payload = {
        "jsonrpc": "2.0",
        "method": "eth_blockNumber",
        "params": [],
        "id": 1
    }
    
    success_count = 0
    rate_limited_count = 0
    
    # Отправляем 150 запросов (лимит 100/мин)
    for i in range(150):
        response = requests.post(PROXY_URL, json=payload)
        
        if response.status_code == 200:
            data = response.json()
            if 'result' in data:
                success_count += 1
        elif response.status_code == 429:
            rate_limited_count += 1
    
    print(f"Success: {success_count}")
    print(f"Rate Limited: {rate_limited_count}")
    
    assert rate_limited_count > 0, "Rate limiting не работает!"
    print("✅ Rate limiting работает корректно")

if __name__ == "__main__":
    test_rate_limit()
```

## Интеграция с популярными инструментами

### 16. Hardhat интеграция

**hardhat.config.js:**
```javascript
module.exports = {
  networks: {
    custom: {
      url: "http://localhost:8545",
      accounts: [PRIVATE_KEY],
      // Все Hardhat запросы проходят через прокси!
    }
  }
};
```

### 17. Foundry интеграция

```bash
# .env файл
RPC_URL=http://localhost:8545

# Использование
forge script Script --rpc-url $RPC_URL --broadcast
```

### 18. Alchemy/Infura замена

**Было:**
```javascript
const provider = new ethers.JsonRpcProvider(
  'https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY'
);
```

**Стало:**
```javascript
const provider = new ethers.JsonRpcProvider(
  'http://your-proxy.com:8545',
  undefined,
  {
    headers: { 'Authorization': 'Bearer YOUR_PROXY_KEY' }
  }
);
```

Теперь у вас полный контроль над rate limiting и мониторингом!