# Changelog

## [Unreleased]

### Changed
- Documentation updated to match current behavior (no admin or multi-tenancy features).
- Deployment and configuration examples simplified to current scope.

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
