# Multi-stage production Dockerfile for blockchain node
# Stage 1: Build
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY common/Cargo.toml common/
COPY consensus/Cargo.toml consensus/
COPY execution/Cargo.toml execution/
COPY storage/Cargo.toml storage/
COPY mempool/Cargo.toml mempool/
COPY network/Cargo.toml network/
COPY node/Cargo.toml node/
COPY da/Cargo.toml da/
COPY zk/Cargo.toml zk/
COPY rollup/Cargo.toml rollup/
COPY interop/Cargo.toml interop/
COPY governance/Cargo.toml governance/
COPY mev/Cargo.toml mev/
COPY benchmarks/Cargo.toml benchmarks/
COPY integration-tests/Cargo.toml integration-tests/

# Create dummy source files to cache dependencies
RUN mkdir -p common/src consensus/src execution/src storage/src mempool/src \
    network/src node/src da/src zk/src rollup/src interop/src governance/src \
    mev/src benchmarks/src integration-tests/tests && \
    echo "fn main() {}" > node/src/main.rs && \
    echo "pub fn dummy() {}" > common/src/lib.rs && \
    echo "pub fn dummy() {}" > consensus/src/lib.rs && \
    echo "pub fn dummy() {}" > execution/src/lib.rs && \
    echo "pub fn dummy() {}" > storage/src/lib.rs && \
    echo "pub fn dummy() {}" > mempool/src/lib.rs && \
    echo "pub fn dummy() {}" > network/src/lib.rs && \
    echo "pub fn dummy() {}" > node/src/lib.rs && \
    echo "pub fn dummy() {}" > da/src/lib.rs && \
    echo "pub fn dummy() {}" > zk/src/lib.rs && \
    echo "pub fn dummy() {}" > rollup/src/lib.rs && \
    echo "pub fn dummy() {}" > interop/src/lib.rs && \
    echo "pub fn dummy() {}" > governance/src/lib.rs && \
    echo "pub fn dummy() {}" > mev/src/lib.rs && \
    echo "pub fn dummy() {}" > benchmarks/src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release --bin node

# Remove dummy files
RUN rm -rf common/src consensus/src execution/src storage/src mempool/src \
    network/src node/src da/src zk/src rollup/src interop/src governance/src \
    mev/src benchmarks/src

# Copy actual source code
COPY common common/
COPY consensus consensus/
COPY execution execution/
COPY storage storage/
COPY mempool mempool/
COPY network network/
COPY node node/
COPY da da/
COPY zk zk/
COPY rollup rollup/
COPY interop interop/
COPY governance governance/
COPY mev mev/
COPY benchmarks benchmarks/
COPY integration-tests integration-tests/

# Build actual binary
RUN cargo build --release --bin node

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash blockchain && \
    mkdir -p /data /config && \
    chown -R blockchain:blockchain /data /config

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/node /app/node

# Set ownership
RUN chown -R blockchain:blockchain /app

# Switch to non-root user
USER blockchain

# Expose ports
EXPOSE 9933 30333

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:9933/health || exit 1

# Volume for persistent data
VOLUME ["/data"]

# Default command
ENTRYPOINT ["/app/node"]
CMD ["start"]
