# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Copy source code
COPY src ./src

# Build release
RUN cargo build --release --features saas

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/rpc-shield /usr/local/bin/rpc-shield

# Copy default config
COPY config.yaml /app/config.yaml

# Create logs directory
RUN mkdir -p /app/logs

# Expose ports
EXPOSE 8545 8555 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8545/health || exit 1

# Run as non-root user
RUN useradd -m -u 1000 rpcshield
USER rpcshield

ENTRYPOINT ["rpc-shield"]
CMD ["--config", "/app/config.yaml"]