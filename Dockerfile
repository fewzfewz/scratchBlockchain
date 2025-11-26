# Multi-stage build for Rust blockchain node
FROM rust:latest as builder

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY storage ./storage
COPY consensus ./consensus
COPY execution ./execution
COPY network ./network
COPY mempool ./mempool
COPY governance ./governance
COPY da ./da
COPY mev ./mev
COPY interop ./interop
COPY zk ./zk
COPY runtime ./runtime
COPY monitoring ./monitoring
COPY node ./node
COPY tools ./tools

# Build release binary
RUN cargo build --release -p node

# Runtime image
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/node /usr/local/bin/modular-node

# Create data directory
RUN mkdir -p /data

WORKDIR /data

# Expose ports
EXPOSE 26656 26657 8545 9090

ENTRYPOINT ["modular-node"]
CMD ["start"]
