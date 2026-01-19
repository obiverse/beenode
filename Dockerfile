# Beenode Dockerfile
# Multi-stage build for minimal image size
#
# Build from parent directory (beeeeb.platform):
#   docker build -f beenode/Dockerfile -t beenode .
# Or via docker-compose in beenode/:
#   docker compose build

FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy docker-specific workspace (excludes poffice/beeoffice/etc.)
COPY beenode/docker-workspace.toml /app/Cargo.toml
COPY Cargo.lock* /app/

# Copy beebank crates (beenode depends on these)
COPY beebank/crates /app/beebank/crates

# Copy beenode source
COPY beenode /app/beenode

WORKDIR /app/beenode

# Build with bitcoind-rpc feature for regtest
RUN cargo build --release --features bitcoind-rpc

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary (workspace builds to /app/target, not /app/beenode/target)
COPY --from=builder /app/target/release/beenode /usr/local/bin/

# Create data directory
RUN mkdir -p /data

# Environment defaults
ENV BEENODE_DATA_DIR=/data
ENV BEENODE_PORT=8080

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["beenode", "serve"]
