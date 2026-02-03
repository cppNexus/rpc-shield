# RpcShield

[![Rust 1.75+](https://img.shields.io/badge/Rust-1.75%2B-informational)](Cargo.toml)
[![Status: As-Is](https://img.shields.io/badge/Status-As--Is-lightgrey)](README.md)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://github.com/cppNexus/rpc-shield/actions/workflows/ci.yml/badge.svg)](https://github.com/cppNexus/rpc-shield/actions/workflows/ci.yml)
[![Release](https://github.com/cppNexus/rpc-shield/actions/workflows/release.yml/badge.svg)](https://github.com/cppNexus/rpc-shield/actions/workflows/release.yml)

**Rate Limiter & JSON-RPC Proxy для Web3 RPC Endpoints**
---
<p align="center">
  <img src="https://github.com/cppNexus/rpc-shield/raw/main/images/rpcshield-logo.jpg" alt="rpc-shield Logo" width="300"/>
</p>

## Описание

RpcShield — это reverse proxy перед Web3 JSON-RPC нодой (Geth, Erigon, Nethermind и др.), который предоставляет:

- **Rate limiting** по IP-адресам и API-ключам
- **Лимиты по методам** для тяжёлых RPC вызовов
- **Статический IP blocklist**
- **Prometheus метрики** на отдельном порту `/metrics`
- **Простую YAML-конфигурацию**

English documentation: `README.md`

## Быстрый старт

### Требования

- Rust 1.75+
- Работающая RPC нода (например, Geth на порту 8546)

### Установка и запуск

```bash
# Клонирование репозитория
git clone https://github.com/cppNexus/rpc-shield.git
cd rpc-shield

# Сборка проекта
cargo build --release

# Запуск
./target/release/rpc-shield --config config.yaml
```

Прокси будет доступен на `http://localhost:8545`.

### Docker Compose (proxy + geth + prometheus)

```bash
docker compose up -d
```

Дефолтный compose файл использует:
- `8545` для прокси
- `8546` для локальной geth ноды
- `9090` для Prometheus

## Конфигурация

Основная конфигурация находится в `config.yaml`:

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

### Лимиты по методам

Можно задать индивидуальные лимиты для каждого RPC метода:

| Метод | Рекомендованный лимит | Причина |
|-------|----------------------|---------|
| `eth_getLogs` | 10/мин | Тяжёлые запросы к БД |
| `eth_call` | 20/мин | Вычислительные операции |
| `eth_blockNumber` | 60/мин | Лёгкие запросы |
| `eth_sendRawTransaction` | 5/мин | Защита от спама |

## API-ключи

### Создание ключей

Добавьте ключи в `config.yaml`:

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

Можно указывать лимиты для любых методов, а не только `eth_call`.

### Использование

```bash
# С Bearer токеном
curl -X POST http://localhost:8545 \
  -H "Authorization: Bearer your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "eth_blockNumber",
    "params": [],
    "id": 1
  }'

# С X-API-Key заголовком
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

### Про tiers (free/pro/enterprise)

`tier` — это только ярлык для дефолтных лимитов. Он не связан с особыми правами.

Приоритет такой:
1. `api_keys.<key>.limits` (пер‑key override)
2. `api_key_tiers.<tier>` (дефолт по tier)
3. `rate_limits.method_limits`
4. `rate_limits.default_ip_limit`

Пример `api_key_tiers`:

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

Добавьте IP-адреса в блоклист, чтобы немедленно отклонять запросы:

```yaml
blocklist:
  ips:
    - "192.168.1.100"
    - "10.0.0.50"
  enable_auto_ban: false
  auto_ban_threshold: 1000
```

**Примечание:** auto-ban пока не реализован, используется только статический список `ips`.

## Rate Limit Headers

Когда лимит превышен, прокси отвечает `429 Too Many Requests` и добавляет заголовок:

```
Retry-After: <seconds>
```

`Retry-After` округляется вверх до секунд и всегда ≥ 1.

## Мониторинг

### Health Check

```bash
curl http://localhost:8545/health
```

### Prometheus

Метрики доступны на `monitoring.prometheus_port` (по умолчанию `9090`):

- `rpc_shield_requests_total`
- `rpc_shield_requests_allowed_total`
- `rpc_shield_requests_rate_limited_total`
- `rpc_shield_requests_blocked_total`
- `rpc_shield_requests_auth_failed_total`
- `rpc_shield_requests_upstream_fail_total`
- `rpc_shield_requests_internal_fail_total`
- `rpc_shield_request_duration_seconds`

Уровень логирования задаётся через `RUST_LOG`. Поле `monitoring.log_level` сейчас не используется приложением.

## Область применения

RpcShield специально держится небольшим и сфокусированным. Следующего **нет**:

- WebSocket proxying
- Admin API
- Multi-tenancy

## Разработка

### Запуск тестов

```bash
cargo test
```

### Запуск в dev режиме

```bash
RUST_LOG=debug cargo run -- --config config.yaml
```

## Лицензия

Apache License 2.0 — см. [LICENSE](LICENSE).

Дополнительно см. файл [NOTICE](NOTICE.md).
