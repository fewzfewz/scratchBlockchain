#!/bin/bash

# Genesis Setup Script
# Usage: ./setup-genesis.sh [mainnet|testnet|devnet]

set -e

NETWORK=${1:-devnet}
CONFIG_DIR="config"
OUTPUT_DIR="genesis"

# Create directories
mkdir -p $CONFIG_DIR
mkdir -p $OUTPUT_DIR

# Build genesis builder
echo "Building genesis-builder..."
cd tools/genesis-builder
cargo build --release
cd ../..

# Select configuration based on network
case $NETWORK in
    mainnet)
        CONFIG_FILE="$CONFIG_DIR/mainnet.toml"
        OUTPUT_FILE="$OUTPUT_DIR/genesis-mainnet.json"
        ;;
    testnet)
        CONFIG_FILE="$CONFIG_DIR/testnet.toml"
        OUTPUT_FILE="$OUTPUT_DIR/genesis-testnet.json"
        ;;
    devnet)
        CONFIG_FILE="$CONFIG_DIR/devnet.toml"
        OUTPUT_FILE="$OUTPUT_DIR/genesis-devnet.json"
        ;;
    *)
        echo "Usage: $0 [mainnet|testnet|devnet]"
        exit 1
        ;;
esac

# Validate configuration
echo "Validating $NETWORK configuration..."
./target/release/genesis-builder validate --config $CONFIG_FILE

# Generate genesis file
echo "Generating genesis for $NETWORK..."
./target/release/genesis-builder generate \
    --config $CONFIG_FILE \
    --output $OUTPUT_FILE

echo "✅ Genesis file created: $OUTPUT_FILE"
echo ""
echo "To use this genesis file:"
echo "  cp $OUTPUT_FILE ../genesis.json"
echo "  cargo run --bin node -- start --config config.toml --genesis genesis.json"