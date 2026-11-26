# Multi-stage build for standalone Nebula Ethereum -> Nebula bridge relayer
FROM rust:1.90-slim AS builder

ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /build

# Copy workspace manifests required by Cargo (Cargo.lock is gitignored)
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

# Copy sources needed by nebula-relayer (plus stub/workspace dirs)
COPY common/src common/src/
COPY relayer/src relayer/src/
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
COPY benchmarks/src benchmarks/src/
COPY benchmarks/benches benchmarks/benches/
COPY tools/genesis-builder/src tools/genesis-builder/src/
COPY interop/fuzz/fuzz_targets interop/fuzz/fuzz_targets/
COPY consensus/fuzz/fuzz_targets consensus/fuzz/fuzz_targets/

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release -p nebula-relayer

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/sh appuser

COPY --from=builder /build/target/release/nebula-relayer /usr/local/bin/nebula-relayer

RUN mkdir -p /data && chown -R appuser:appuser /data
WORKDIR /data
USER appuser

ENTRYPOINT ["nebula-relayer"]
