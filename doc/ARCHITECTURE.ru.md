# Архитектура rpc-shield

Этот документ описывает **текущее** поведение RpcShield. Здесь нет планов и будущих фич.

## Обзор системы

```
┌─────────────────────────────────────────────────────────┐
│                    Client Layer                         │
│  (Wallets, DApps, Bots, Scripts)                        │
└─────────────────┬───────────────────────────────────────┘
                  │ HTTP/JSON-RPC
                  ↓
┌─────────────────────────────────────────────────────────┐
│              rpc-shield (Port 8545)                     │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Identity   │  │     Rate     │  │   Metrics    │   │
│  │   Resolver   │  │   Limiter    │  │  Collector   │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                  │                  │        │
│         └──────────┬───────┴──────────────────┘        │
│                    ↓                                    │
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

## Поток запроса

1. Принимаем HTTP POST JSON-RPC.
2. Определяем identity (API ключ или IP).
3. Проверяем статический IP blocklist.
4. Применяем rate limit для `identity + method`.
5. Проксируем запрос в backend RPC ноду.
6. Возвращаем ответ (или ошибку).

## Компоненты

### 1. Identity Resolver

**Вход:** HTTP заголовки и IP клиента.

**Порядок:**
1. `Authorization: Bearer <token>`
2. `X-API-Key: <token>`
3. Fallback на IP клиента

Поддерживаются только Bearer и X-API-Key. Любая другая схема даёт `401`.

### 2. Rate Limiter Engine

**Алгоритм:** Token bucket (crate `governor`).

**Ключ лимитера:** `identity:method` (отдельный лимитер на identity + RPC метод).

**Приоритет лимитов (сверху вниз):**
1. `api_keys.<key>.limits.<method>`
2. `api_key_tiers.<tier>.<method>`
3. `rate_limits.method_limits.<method>`
4. `rate_limits.default_ip_limit`

### 3. Proxy Handler

**Поведение:**
- Если IP в `blocklist.ips`, возвращаем `403`.
- Если передан API ключ и он не найден/выключен, возвращаем `401`.
- При превышении лимита возвращаем `429` с заголовком `Retry-After`.
- Иначе проксируем JSON-RPC запрос.

**Коды ошибок:**
- `-32000` (401) — неверная схема авторизации или неверный API ключ
- `-32001` (403) — IP заблокирован
- `-32005` (429) — превышен rate limit
- `-32007` (502) — ошибка upstream
- `-32603` (500) — внутренняя ошибка

### 4. Конфигурация (YAML)

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
    limits: { <method_name>: { requests: u32, period: string } }

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

**Примечание:** `blocklist.enable_auto_ban` и `blocklist.auto_ban_threshold` присутствуют в конфиге, но не реализованы; используется только статический список `ips`. Поле `monitoring.log_level` сейчас игнорируется; используйте `RUST_LOG`.

### 5. Метрики (Prometheus)

- `rpc_shield_requests_total`
- `rpc_shield_requests_allowed_total`
- `rpc_shield_requests_rate_limited_total`
- `rpc_shield_requests_blocked_total`
- `rpc_shield_requests_auth_failed_total`
- `rpc_shield_requests_upstream_fail_total`
- `rpc_shield_requests_internal_fail_total`
- `rpc_shield_request_duration_seconds`

## Область применения

RpcShield сейчас поддерживает только **HTTP JSON-RPC**. WebSocket proxy, Admin API и multi-tenancy отсутствуют.
