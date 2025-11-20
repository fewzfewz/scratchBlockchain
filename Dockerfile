# Multi-stage Dockerfile for modular blockchain node

# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY common ./common
COPY network ./network
COPY consensus ./consensus
COPY storage ./storage
COPY execution ./execution
COPY zk ./zk
COPY rollup ./rollup
COPY interop ./interop
COPY governance ./governance
COPY node ./node

# Build release binary
RUN cargo build --release --bin node

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/node /usr/local/bin/node

# Create data directory
RUN mkdir -p /data

# Expose ports
EXPOSE 30333 9933 9944

# Set working directory
WORKDIR /data

# Run node
ENTRYPOINT ["node"]
CMD ["start"]
