#!/bin/bash
set -e

echo "=========================================="
echo "Modular Blockchain - Local Testnet Setup"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check Docker
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found. Please install Docker first."
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "❌ Docker Compose not found. Please install Docker Compose first."
    exit 1
fi

echo -e "${GREEN}✓${NC} Docker and Docker Compose found"
echo ""

# Step 1: Generate genesis and validator keys
echo -e "${BLUE}Step 1: Generating genesis and validator keys...${NC}"

cd deployment/local

# Create configs directory
mkdir -p configs

# Write validator keys
echo "Writing validator keys..."
echo "a71ac7c754d1e66871151f3a5f2529777fb828fa576918cade292acf38293f3f" > configs/validator1_key.json
echo "6114eb98616e2f4faaa91e35e1fc5f176275dfd29ff1085a4600860ecee49bc4" > configs/validator2_key.json
echo "a298b2f7f33061444d38d828f4562962d3281387ab08acf54a05f6229b98bb58" > configs/validator3_key.json

# Create genesis config
cat > configs/genesis.toml << 'EOF'
[chain]
chain_id = "modular-testnet-1"
timestamp = 1700000000
initial_height = 0

[consensus]
block_time_ms = 3000
max_validators = 100
min_stake = "100000"

[governance]
proposal_deposit = "1000"
voting_period_blocks = 1000
quorum_threshold = "0.334"

[[validators]]
address = "0x1111111111111111111111111111111111111111"
stake = "1000000"
commission_rate = "0.10"
public_key = "401c76b85552dfd28fd120e236b252b4eb7f45ef6b72d3103ea0082fbb476642"

[[validators]]
address = "0x2222222222222222222222222222222222222222"
stake = "800000"
commission_rate = "0.10"
public_key = "9f140c78dec55de6a777baa88ba33e65d0d7e46243557169f8b827f725392acc"

[[validators]]
address = "0x3333333333333333333333333333333333333333"
stake = "600000"
commission_rate = "0.10"
public_key = "ec77526bdb058b2684570107a3581ccfe8cec4e03dfd5ac409213f181e66b93a"

[[accounts]]
address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
balance = "10000000000"

[[accounts]]
address = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
balance = "5000000000"
EOF

# Generate genesis using genesis-builder
echo "Generating genesis file..."
cd ../../tools/genesis-builder
cargo run --release -- \
    --config ../../deployment/local/configs/genesis.toml \
    --output ../../deployment/local/configs/genesis.json

cd ../../deployment/local

echo -e "${GREEN}✓${NC} Genesis file created"
echo ""

# Step 2: Create validator configurations
echo -e "${BLUE}Step 2: Creating validator configurations...${NC}"

# Validator 1 config
cat > configs/validator1.toml << 'EOF'
[network]
chain_id = "modular-testnet-1"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = []

[consensus]
block_time_ms = 3000
max_validators = 100

[validator]
enabled = true
commission_rate = "0.10"

[storage]
data_dir = "/data"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
EOF

# Validator 2 config (connects to validator1)
cat > configs/validator2.toml << 'EOF'
[network]
chain_id = "modular-testnet-1"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = ["validator1:26656"]

[consensus]
block_time_ms = 3000
max_validators = 100

[validator]
enabled = true
commission_rate = "0.10"

[storage]
data_dir = "/data"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
EOF

# Validator 3 config
cat > configs/validator3.toml << 'EOF'
[network]
chain_id = "modular-testnet-1"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = ["validator1:26656"]

[consensus]
block_time_ms = 3000
max_validators = 100

[validator]
enabled = true
commission_rate = "0.10"

[storage]
data_dir = "/data"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
EOF

# RPC node configs
cat > configs/rpc1.toml << 'EOF'
[network]
chain_id = "modular-testnet-1"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = ["validator1:26656", "validator2:26656"]

[consensus]
block_time_ms = 3000

[validator]
enabled = false

[storage]
data_dir = "/data"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
EOF

cat > configs/rpc2.toml << 'EOF'
[network]
chain_id = "modular-testnet-1"
p2p_port = 26656
rpc_port = 26657
bootstrap_nodes = ["validator1:26656", "validator2:26656"]

[consensus]
block_time_ms = 3000

[validator]
enabled = false

[storage]
data_dir = "/data"

[api]
enabled = true
address = "0.0.0.0:8545"
cors_origins = ["*"]

[metrics]
enabled = true
address = "0.0.0.0:9090"
EOF

echo -e "${GREEN}✓${NC} Configurations created"
echo ""

# Step 3: Create Nginx config
echo -e "${BLUE}Step 3: Creating Nginx reverse proxy config...${NC}"

cat > configs/nginx.conf << 'EOF'
events {
    worker_connections 1024;
}

http {
    upstream rpc_backend {
        server rpc1:8545;
        server rpc2:8545 backup;
    }

    server {
        listen 80;
        server_name localhost;

        # RPC endpoint
        location /rpc {
            proxy_pass http://rpc_backend;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
        }

        # Faucet
        location /faucet {
            proxy_pass http://faucet:3000;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
        }

        # Grafana
        location /grafana/ {
            proxy_pass http://grafana:3000/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
        }

        # Prometheus
        location /prometheus/ {
            proxy_pass http://prometheus:9090/;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
        }
    }
}
EOF

echo -e "${GREEN}✓${NC} Nginx config created"
echo ""

# Step 4: Create environment file
echo -e "${BLUE}Step 4: Creating environment file...${NC}"

cat > .env << 'EOF'
# Faucet configuration
FAUCET_PRIVATE_KEY=0x0000000000000000000000000000000000000000000000000000000000000001
EOF

echo -e "${GREEN}✓${NC} Environment file created"
echo ""

# Step 5: Build and start services
echo -e "${BLUE}Step 5: Building Docker images...${NC}"
echo -e "${YELLOW}This may take 10-15 minutes on first run...${NC}"
echo ""

docker-compose build

echo ""
echo -e "${GREEN}✓${NC} Docker images built"
echo ""

echo -e "${BLUE}Step 6: Starting testnet...${NC}"
docker-compose up -d

echo ""
echo -e "${GREEN}✓${NC} Testnet started!"
echo ""

# Wait for services to be ready
echo "Waiting for services to be ready..."
sleep 10

# Check status
echo ""
echo "=========================================="
echo "Testnet Status"
echo "=========================================="
echo ""

docker-compose ps

echo ""
echo "=========================================="
echo "Access Points"
echo "=========================================="
echo ""
echo -e "${GREEN}RPC Endpoint:${NC}      http://localhost/rpc"
echo -e "${GREEN}Faucet:${NC}            http://localhost/faucet"
echo -e "${GREEN}Grafana:${NC}           http://localhost/grafana (admin/admin)"
echo -e "${GREEN}Prometheus:${NC}        http://localhost/prometheus"
echo ""
echo -e "${GREEN}Validator 1 RPC:${NC}   http://localhost:26657"
echo -e "${GREEN}Validator 2 RPC:${NC}   http://localhost:26659"
echo -e "${GREEN}Validator 3 RPC:${NC}   http://localhost:26661"
echo ""
echo -e "${GREEN}Validator 1 API:${NC}   http://localhost:8545"
echo -e "${GREEN}Validator 2 API:${NC}   http://localhost:8546"
echo -e "${GREEN}Validator 3 API:${NC}   http://localhost:8547"
echo ""
echo "=========================================="
echo "Useful Commands"
echo "=========================================="
echo ""
echo "View logs:           docker-compose logs -f"
echo "Stop testnet:        docker-compose down"
echo "Restart testnet:     docker-compose restart"
echo "Clean data:          docker-compose down -v"
echo ""
echo -e "${GREEN}Testnet is running!${NC}"
echo ""
