# Multi-stage build for optimized Rust blockchain node
FROM rust:1.90-slim as builder

# Set non-interactive mode for apt
ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /build

# Copy only necessary files for building
COPY Cargo.toml Cargo.lock ./
COPY common/Cargo.toml common/Cargo.lock common/
COPY storage/Cargo.toml storage/Cargo.lock storage/
COPY consensus/Cargo.toml consensus/Cargo.lock consensus/
COPY execution/Cargo.toml execution/Cargo.lock execution/
COPY network/Cargo.toml network/Cargo.lock network/
COPY mempool/Cargo.toml mempool/Cargo.lock mempool/
COPY governance/Cargo.toml governance/Cargo.lock governance/
COPY da/Cargo.toml da/Cargo.lock da/
COPY mev/Cargo.toml mev/Cargo.lock mev/
COPY interop/Cargo.toml interop/Cargo.lock interop/
COPY zk/Cargo.toml zk/Cargo.lock zk/
COPY runtime/Cargo.toml runtime/Cargo.lock runtime/
COPY monitoring/Cargo.toml monitoring/Cargo.lock monitoring/
COPY node/Cargo.toml node/Cargo.lock node/
COPY tools/genesis-builder/Cargo.toml tools/genesis-builder/Cargo.lock tools/genesis-builder/
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

# Copy binary from builder
COPY --from=builder /build/target/release/node /usr/local/bin/modular-node

# Create data directory with proper permissions
RUN mkdir -p /data && chown -R appuser:appuser /data

WORKDIR /data

# Expose ports
EXPOSE 26656 26657 8545 9090

# Switch to non-root user
USER appuser

ENTRYPOINT ["modular-node"]
CMD ["start"]
