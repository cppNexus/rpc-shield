# Contributing to RPC Shield

Thank you for your interest in contributing to RPC Shield! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Coding Standards](#coding-standards)
- [Documentation](#documentation)

## Code of Conduct

This project adheres to a Code of Conduct. By participating, you are expected to uphold this code. Please be respectful and constructive in all interactions.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/rpc-shield.git`
3. Add upstream remote: `git remote add upstream https://github.com/original/rpc-shield.git`
4. Create a branch: `git checkout -b feature/your-feature-name`

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Docker and Docker Compose

### Local Development

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/yourusername/rpc-shield.git
cd rpc-shield

# Install dependencies
cargo build

# Run tests
cargo test

# Start development environment
docker compose up -d

# Run in development mode
cargo run -- --config config.yaml
```

### With Auto-Reload

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload
cargo watch -x run
```

## Making Changes

### Branch Naming

- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Test additions or modifications

Example: `feature/add-metrics-docs`

### Commit Messages

Follow conventional commits format:

```
type(scope): subject

body (optional)

footer (optional)
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(rate-limiter): add precedence tests

Adds tests that verify per-key and tier limits override
global method and default IP limits.

Closes #123
```

```
fix(proxy): handle connection timeouts gracefully

Previously, connection timeouts would cause panics.
Now they are properly handled and logged.

Fixes #456
```

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with logging
RUST_LOG=debug cargo test

# Run integration tests
cargo test --test integration_tests

# Run ignored tests (like load tests)
cargo test -- --ignored
```

### Writing Tests

- Unit tests: Place in the same file as the code being tested
- Integration tests: Place in `tests/` directory
- Follow existing test patterns
- Ensure tests are deterministic
- Mock external dependencies

Example:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(config);
        let identity = ClientIdentity::IpAddress("127.0.0.1".parse().unwrap());
        let decision = limiter
            .check_rate_limit_with_rule(&identity, "eth_call", None)
            .await
            .unwrap();
        assert!(decision.allowed);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = some_async_function().await;
        assert_eq!(result, expected);
    }
}
```

## Pull Request Process

1. **Update your branch**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Run tests and checks**
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt -- --check
   ```

3. **Push your changes**
   ```bash
   git push origin feature/your-feature-name
   ```

4. **Create Pull Request**
   - Provide clear description of changes
   - Link related issues
   - Add screenshots for UI changes
   - Ensure CI passes

5. **Code Review**
   - Address review comments
   - Keep discussion focused and professional
   - Update PR as needed

6. **Merge**
   - Squash commits if requested
   - Maintainer will merge when approved

## Coding Standards

### Rust Style Guide

Follow the official Rust style guide and these project conventions:

```rust
// Use explicit types when helpful for clarity
let rate_limit: RateLimit = RateLimit::new(100, Duration::from_secs(60));

// Prefer descriptive names over abbreviations
fn calculate_rate_limit_decision() { }  // Good
fn calc_rl_dec() { }  // Bad

// Use Result for error handling
fn process_request() -> Result<Response, Error> {
    // ...
}

// Document public APIs
/// Checks if the client has exceeded their rate limit.
///
/// # Arguments
/// * `identity` - Client identifier (IP or API key)
/// * `method` - RPC method being called
///
/// # Returns
/// * `Ok(RateLimitDecision)` - Decision on whether to allow the request
/// * `Err(Error)` - If rate limit check fails
pub async fn check_rate_limit(
    &self,
    identity: &ClientIdentity,
    method: &str,
) -> Result<RateLimitDecision, Error> {
    // ...
}
```

### Code Organization

- Keep modules focused and cohesive
- Separate concerns (e.g., config, rate limiting, stats)
- Use meaningful file and module names
- Limit file size (< 500 lines preferred)

### Error Handling

```rust
// Use anyhow for application errors
use anyhow::{Result, Context};

fn load_config() -> Result<Config> {
    let content = fs::read_to_string("config.yaml")
        .context("Failed to read config file")?;
    
    let config: Config = serde_yaml::from_str(&content)
        .context("Failed to parse config")?;
    
    Ok(config)
}

// Use thiserror for library errors
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RateLimitError {
    #[error("rate limit exceeded")]
    Exceeded,
    
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}
```

### Logging

```rust
use tracing::{debug, info, warn, error};

// Use appropriate log levels
debug!("Detailed debug information");
info!("Normal operational messages");
warn!("Warning conditions");
error!("Error conditions");

// Include context
info!(
    client = %identity,
    method = %method,
    "Rate limit checked"
);
```

## Documentation

### Code Documentation

- Document all public APIs
- Include examples in doc comments
- Explain non-obvious logic
- Keep docs up-to-date with code changes

```rust
/// Rate limiter with token bucket.
///
/// # Examples
///
/// ```
/// use rpc_shield::identity::ClientIdentity;
/// use rpc_shield::RateLimiter;
///
/// let limiter = RateLimiter::new(config);
/// let identity = ClientIdentity::IpAddress("127.0.0.1".parse()?);
/// let decision = limiter
///     .check_rate_limit_with_rule(&identity, "eth_call", None)
///     .await?;
/// ```
pub struct RateLimiter {
    // ...
}
```

### External Documentation

Update relevant documentation files:
- `README.md` - For user-facing changes
- `CONFIGURATION.md` - For config changes
- `DEPLOYMENT.md` - For deployment changes
- `ARCHITECTURE.md` - For architectural changes
- `EXAMPLES.md` - For usage examples

## Areas for Contribution

We welcome contributions in these areas:

- **Rate limiting**: Correctness and edge cases
- **Performance**: Optimizations, benchmarking
- **Documentation**: Guides, examples, tutorials
- **Testing**: More test coverage, load tests
- **Bug fixes**: Check the issue tracker
- **Integrations**: Monitoring tooling and dashboards

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions or ideas
- Email: cppnexus@proton.me

## Recognition

Contributors will be recognized in:
- CONTRIBUTORS.md file
- Release notes
- Project README

Thank you for contributing to RPC Shield!
