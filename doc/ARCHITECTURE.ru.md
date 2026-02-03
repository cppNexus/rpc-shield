# Архитектура rpc-shield



## Обзор системы

rpc-shield

 построен на модульной архитектуре с четким разделением ответственности между компонентами.

```
┌─────────────────────────────────────────────────────────┐
│                    Client Layer                         │
│  (Wallets, DApps, Bots, Scripts)                       │
└─────────────────┬───────────────────────────────────────┘
                  │ HTTP/JSON-RPC
                  ↓
┌─────────────────────────────────────────────────────────┐
│              rpc-shield

 (Port 8545)                 │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   Identity   │  │     Rate     │  │    Stats     │ │
│  │   Resolver   │  │   Limiter    │  │  Collector   │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │
│         │                  │                  │         │
│         └──────────┬───────┴──────────────────┘         │
│                    ↓                                     │
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

## Компоненты

### 1. Identity Resolver

**Ответственность:** Определение клиента по запросу

**Логика:**
1. Проверка `Authorization: Bearer <token>` заголовка
2. Проверка `X-API-Key` заголовка  
3. Fallback на IP-адрес клиента

**Типы идентификации:**
```rust
enum ClientIdentity {
    ApiKey(String),      // Авторизованный пользователь
    IpAddress(IpAddr),   // Анонимный клиент по IP
    Anonymous,           // Неопределённый
}
```

**Приоритеты:**
1. API ключ (высший приоритет - пользовательские лимиты)
2. IP адрес (базовые лимиты)

### 2. Rate Limiter Engine

**Алгоритм:** Token Bucket (через `governor` crate)

**Структура:**
```rust
HashMap<String, RateLimiter>
Key = "identity:method"
// Примеры:
// "apikey:abc123:eth_call"
// "ip:192.168.1.1:eth_getLogs"
```

**Процесс проверки:**
1. Извлечь identity + method
2. Найти соответствующий лимит из конфига
3. Проверить quota в соответствующем bucket
4. Разрешить/отклонить запрос

**Уровни лимитов (приоритет сверху вниз):**
1. API ключ + конкретный метод
2. Конкретный метод (из config.method_limits)
3. Default IP лимит

**Примеры квот:**
```yaml
# 100 запросов в минуту
requests: 100
period: "1m"

# 5 запросов в секунду
requests: 5
period: "1s"

# 1000 запросов в час
requests: 1000
period: "1h"
```

### 3. Proxy Handler

**Ответственность:** Маршрутизация запросов

**Процесс обработки:**
```
1. Принять HTTP POST запрос
   ↓
2. Распарсить JSON-RPC тело
   ↓
3. Извлечь IP и заголовки
   ↓
4. Определить ClientIdentity
   ↓
5. Проверить rate limit
   ↓
6a. Лимит превышен → 429 Too Many Requests
6b. Лимит OK → forward к RPC ноде
   ↓
7. Получить ответ от ноды
   ↓
8. Вернуть ответ клиенту
```

**Коды ошибок:**
- `-32005`: Rate limit exceeded (кастомный)
- `-32603`: Internal error (стандартный JSON-RPC)
- HTTP 429: Too Many Requests

### 4. Config Loader

**Формат:** YAML

**Структура:**
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

**Метрики для сбора:**
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

## Режимы работы

### Self-Hosted Mode

**Характеристики:**
- Конфигурация из YAML файла
- Лимитеры в памяти процесса
- Статистика в stdout/logs
- Нет необходимости в БД

**Use case:**
- Частные RPC ноды
- Внутренние корпоративные сети
- Разработка и тестирование

## Потоки данных

### Успешный запрос

```
Client → Proxy Handler
         ↓
      Identity Resolver (определяет клиента)
         ↓
      Rate Limiter (проверяет квоту)
         ↓ PASS
      HTTP Client → Backend RPC
         ↓
      Response
         ↓
      Client
```

### Отклонённый запрос (Rate Limited)

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

## Масштабирование

### Вертикальное

**Single Instance:**
- 10K-50K RPS на современном CPU
- In-memory rate limiters (очень быстро)
- Низкая latency (~1-2ms overhead)

**Оптимизации:**
- Async I/O (Tokio)
- Zero-copy где возможно
- Connection pooling к RPC ноде


## Безопасность

### Rate Limiting как первая линия защиты

**Защита от:**
- DDoS атак
- Spam методов (eth_getLogs)
- Ресурсоёмких запросов
- Bot-трафика

### Дополнительные меры

**IP Blocklist:**
- Статический список заблокированных IP
- Auto-ban по threshold (в разработке)

**Method Filtering (будущее):**
- Blacklist опасных методов
- Whitelist только разрешённых методов

**Request Validation:**
- JSON-RPC формат
- Размер payload
- Signature verification (опционально)

## Расширения

### WebSocket Support (планируется)

```rust
// Новый handler для ws://
async fn ws_proxy_handler() {
    // Upgrade connection
    // Forward eth_subscribe events
    // Maintain persistent connection
}
```

### Admin API (планируется)

**Endpoints:**
```
POST   /admin/api-keys              - Создать ключ
GET    /admin/api-keys/:id          - Получить ключ
PUT    /admin/api-keys/:id          - Обновить лимиты
DELETE /admin/api-keys/:id          - Удалить ключ
GET    /admin/stats/:id             - Статистика по ключу
POST   /admin/blocklist/add         - Добавить IP в блоклист
```

**Аутентификация:**
- JWT tokens
- Admin API key
- Role-based access control

### Machine Learning Integration (будущее)

**Bot Detection:**
- Анализ паттернов запросов
- Anomaly detection
- Automated ban recommendations

**Traffic Prediction:**
- Forecast usage для auto-scaling
- Predictive rate limiting
- Cost optimization

## Мониторинг и Observability

### Логирование

**Уровни:**
- ERROR: Критические ошибки
- WARN: Rate limits, suspicious activity
- INFO: Startup, config changes
- DEBUG: Каждый запрос (dev only)

**Структура логов:**
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

### Метрики (Prometheus)

**Dashboard показатели:**
- Requests per second по методам
- Rate limit hit rate
- Backend latency
- Active connections
- Error rates

### Алерты

**Критические:**
- Backend RPC недоступен
- Ошибки > 5%
- Latency > 1s

**Предупреждения:**
- Необычный рост трафика
- Новые паттерны атак
- Приближение к лимитам ресурсов

## Производительность

### Benchmarks (ожидаемые)

```
Throughput:  20,000 RPS (single instance)
Latency:     p50: 2ms, p99: 10ms (proxy overhead)
Memory:      ~100MB base + 1KB per active limiter
CPU:         ~30% на 10K RPS (4 cores)
```

### Профилирование

**Hotspots:**
1. Rate limiter lookup - O(1) HashMap
2. JSON parsing - используем serde
3. HTTP forwarding - connection pooling

**Оптимизации:**
- Batch processing для stats
- LRU cache для config
- Pre-compiled regex для validation
