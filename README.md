# RpcShield

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

**Rate Limiter & DDoS Filter для Web3 RPC Endpoints**
---
<p align="center">
  <img src="https://github.com/cppNexus/rpc-shield/raw/main/images/rpcshield-logo.jpg" alt="rpc-shield Logo" width="300"/>
</p>
## Описание

RpcShield — это высокопроизводительный reverse proxy для Web3 RPC нод (Geth, Erigon и др.), обеспечивающий:

- **Rate Limiting** по IP-адресам и API-ключам
- **Защита от DDoS** и вредоносных запросов
- **Мониторинг и статистика** использования
- **Гибкая конфигурация** лимитов для разных методов
- **SaaS-ready архитектура** для монетизации

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

# Запуск (self-hosted режим)
./target/release/rpc-shield --config config.yaml
```

Прокси будет доступен на `http://localhost:8545`

## Конфигурация

Основная конфигурация находится в `config.yaml`:

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
  
  method_limits:
    eth_call:
      requests: 20
      period: "1m"
    eth_getLogs:
      requests: 10
      period: "1m"
```

### Лимиты по методам

Вы можете настроить индивидуальные лимиты для каждого RPC метода:

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
```

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

## Тарифы (SaaS режим)

| Тариф | Запросов/мес | Цена | Особенности |
|-------|--------------|------|-------------|
| **Free** | 1M | $0 | Базовые лимиты |
| **Pro** | 10M | $99 | Повышенные лимиты |
| **Enterprise** | Custom | Custom | SLA, поддержка, кастомные лимиты |

## Режимы работы

### Self-Hosted

```bash
./rpc-shield --config config.yaml --mode self-hosted
```

- Конфигурация из YAML
- Локальное хранение статистики
- Идеально для частных нод

### SaaS (в разработке)

```bash
./rpc-shield --config config.yaml --mode saas
```

- PostgreSQL для пользователей и биллинга
- Redis для распределённых лимитов
- Admin API для управления

## Архитектура

```
[Client/DApp/Bot]
       ↓
[rpc-shield

:8545]
   ├── Rate Limiter
   ├── Identity Resolver
   ├── Stats Collector
   └── Proxy Handler
       ↓
[RPC Node (Geth):8546]
```

### Основные компоненты

- **Proxy Layer** - HTTP сервер на Axum
- **Rate Limiter** - Token bucket алгоритм (governor)
- **Identity Resolver** - Определение клиента по IP/API-ключу
- **Config Loader** - Загрузка правил из YAML
- **Stats Collector** - Агрегация метрик (готовится)

## Мониторинг

### Health Check

```bash
curl http://localhost:8545/health
```

### Prometheus (в разработке)

Метрики будут доступны на порту 9090:

```
# HELP rpc_requests_total Total RPC requests
# TYPE rpc_requests_total counter
rpc_requests_total{method="eth_call",status="ok"} 1234

# HELP rate_limit_exceeded_total Rate limit violations
# TYPE rate_limit_exceeded_total counter
rate_limit_exceeded_total{identity="ip:1.2.3.4"} 42
```

## Разработка

### Запуск тестов

```bash
cargo test
```

### Запуск в dev режиме

```bash
RUST_LOG=debug cargo run -- --config config.yaml
```

### Feature flags

```bash
# Self-hosted режим (по умолчанию)
cargo build --features self-hosted

# SaaS режим
cargo build --features saas
```

## Roadmap

### MVP (v0.1)
- [x] HTTP Proxy с JSON-RPC
- [x] Rate Limiting по IP и методам
- [x] API-ключи
- [x] YAML конфигурация
- [x] Базовое логирование

### v0.2 (в разработке)
- [ ] IP Blocklist
- [ ] Prometheus метрики
- [ ] WebSocket passthrough
- [ ] Redis интеграция

### v0.3 (планируется)
- [ ] Admin REST API
- [ ] PostgreSQL для биллинга
- [ ] Web Dashboard (Tauri)
- [ ] Auto-ban по threshold

### v1.0 (будущее)
- [ ] Stripe/Crypto платежи
- [ ] ML-based bot detection
- [ ] Geo-blocking
- [ ] Email уведомления

## Вклад

Мы приветствуем pull requests! Основные области:

- Оптимизация производительности
- Новые типы rate limiters
- Интеграции с мониторингом
- Документация

## Лицензия

Apache License 2.0 — см. [LICENSE](LICENSE).

Дополнительно см. файл [NOTICE](NOTICE.md).

## 🔗 Ссылки

- [Документация](https://docs.rpc-shield.io) (скоро)
- [Discord сообщество](https://discord.gg/...) (скоро)
- [Примеры использования](./examples) (скоро)
