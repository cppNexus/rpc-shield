# Changelog

## [Unreleased]

### Added
- Docker Compose quickstart (proxy + geth + Prometheus).
- `.env.example` for port configuration.
- English documentation parity (README/ARCHITECTURE/EXAMPLES) with RU docs.

### Changed
- Prometheus exposed only via the Prometheus container (no direct metrics port).

## [0.1.0] - 2026-02-03

### Added
- JSON-RPC proxy with per-IP and per-key rate limiting.
- Strict auth parsing (Bearer / X-API-Key only).
- API key tiers with defaults and per-key overrides.
- Static IP blocklist.
- Prometheus metrics endpoint.
- Retry-After header on 429 responses.
- Community documentation (EN/RU).

### Fixed
- Prevent raw API keys from appearing in logs (fingerprints only).
- Align documentation with actual behavior (no SaaS features in community).
