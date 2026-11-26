# Multi-stage build for optimized Rust blockchain node
FROM rust:1.90-slim AS builder

# Set non-interactive mode for apt
ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /build

# Copy workspace manifests (Cargo.lock is gitignored, so Cargo generates it at build time)
COPY Cargo.toml ./
COPY common/Cargo.toml common/
COPY storage/Cargo.toml storage/
COPY consensus/Cargo.toml consensus/
COPY execution/Cargo.toml execution/
COPY network/Cargo.toml network/
COPY mempool/Cargo.toml mempool/
COPY governance/Cargo.toml governance/
COPY da/Cargo.toml da/
COPY mev/Cargo.toml mev/
COPY interop/Cargo.toml interop/
COPY zk/Cargo.toml zk/
COPY runtime/Cargo.toml runtime/
COPY monitoring/Cargo.toml monitoring/
COPY node/Cargo.toml node/
COPY relayer/Cargo.toml relayer/
COPY benchmarks/Cargo.toml benchmarks/
COPY tools/genesis-builder/Cargo.toml tools/genesis-builder/
COPY interop/fuzz/Cargo.toml interop/fuzz/
COPY consensus/fuzz/Cargo.toml consensus/fuzz/

# Copy source files for all workspace members
COPY common/src common/src/
COPY storage/src storage/src/
COPY consensus/src consensus/src/
COPY execution/src execution/src/
COPY network/src network/src/
COPY mempool/src mempool/src/
COPY governance/src governance/src/
COPY da/src da/src/
COPY mev/src mev/src/
COPY interop/src interop/src/
COPY zk/src zk/src/
COPY runtime/src runtime/src/
COPY monitoring/src monitoring/src/
COPY node/src node/src/
COPY relayer/src relayer/src/
COPY benchmarks/src benchmarks/src/
COPY benchmarks/benches benchmarks/benches/
COPY tools/genesis-builder/src tools/genesis-builder/src/
COPY interop/fuzz/fuzz_targets interop/fuzz/fuzz_targets/
COPY consensus/fuzz/fuzz_targets consensus/fuzz/fuzz_targets/

# Install build dependencies (openssl-sys, bindgen/libclang for zstd-sys)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl-dev \
    pkg-config \
    clang \
    && rm -rf /var/lib/apt/lists/*

# Build only the node binary
RUN cargo build --release -p node

# Runtime image - use minimal base
FROM rust:1.90-slim

# Install only runtime dependencies needed by the binary
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create app user
RUN useradd -m -s /bin/sh appuser

# Copy binary and Render configs (both solo demo and multi-validator Blueprint)
COPY --from=builder /build/target/release/node /usr/local/bin/modular-node
COPY config/render.toml /etc/nebula/config.toml
COPY config/render-genesis.json /etc/nebula/genesis.json
COPY deployment/render/ /etc/nebula/render/
COPY deployment/local/configs/genesis.json /etc/nebula/multi-genesis.json
COPY deployment/local/configs/validator1_key.json /data/validator_key.json

# Create data directory with proper permissions
RUN mkdir -p /data && chown -R appuser:appuser /data /etc/nebula

WORKDIR /data

# Expose ports
EXPOSE 26656 26657 8545 9090

# Switch to non-root user
USER appuser

ENTRYPOINT ["modular-node"]
CMD ["start", "--config", "/etc/nebula/config.toml", "--genesis", "/etc/nebula/genesis.json"]
