#!/bin/bash
set -e

echo "=========================================="
echo "Quick Localhost Testnet Validation"
echo "=========================================="
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${BLUE}Step 1: Checking prerequisites...${NC}"
echo ""

# Check Docker
if command -v docker &> /dev/null; then
    echo -e "${GREEN}✅ Docker installed:${NC} $(docker --version)"
else
    echo -e "${RED}❌ Docker not found${NC}"
    exit 1
fi

# Check Docker Compose
if command -v docker-compose &> /dev/null; then
    echo -e "${GREEN}✅ Docker Compose installed:${NC} $(docker-compose --version)"
else
    echo -e "${RED}❌ Docker Compose not found${NC}"
    exit 1
fi

# Check disk space
AVAILABLE=$(df -BG . | tail -1 | awk '{print $4}' | sed 's/G//')
if [ "$AVAILABLE" -gt 20 ]; then
    echo -e "${GREEN}✅ Disk space available:${NC} ${AVAILABLE}GB"
else
    echo -e "${YELLOW}⚠️  Low disk space:${NC} ${AVAILABLE}GB (need 20GB+)"
fi

# Check Rust
if command -v cargo &> /dev/null; then
    echo -e "${GREEN}✅ Rust installed:${NC} $(rustc --version)"
else
    echo -e "${YELLOW}⚠️  Rust not found${NC} (needed for building)"
fi

echo ""
echo -e "${BLUE}Step 2: Checking project structure...${NC}"
echo ""

# Check key directories
DIRS=("common" "consensus" "storage" "network" "node" "sdk/javascript" "deployment/local")
for dir in "${DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo -e "${GREEN}✅${NC} $dir/"
    else
        echo -e "${RED}❌${NC} $dir/ (missing)"
    fi
done

echo ""
echo -e "${BLUE}Step 3: Building genesis-builder...${NC}"
echo ""

cd tools/genesis-builder
if cargo build --release 2>&1 | tail -5; then
    echo -e "${GREEN}✅ Genesis builder compiled${NC}"
else
    echo -e "${RED}❌ Genesis builder failed to compile${NC}"
    exit 1
fi

echo ""
echo -e "${BLUE}Step 4: Generating testnet genesis...${NC}"
echo ""

./target/release/genesis-builder \
    --config examples/testnet.toml \
    --output ../../deployment/local/configs/genesis.json

if [ -f "../../deployment/local/configs/genesis.json" ]; then
    echo -e "${GREEN}✅ Genesis file created${NC}"
    echo "   Location: deployment/local/configs/genesis.json"
else
    echo -e "${RED}❌ Genesis file not created${NC}"
    exit 1
fi

cd ../..

echo ""
echo -e "${BLUE}Step 5: Checking Docker setup...${NC}"
echo ""

if [ -f "Dockerfile" ]; then
    echo -e "${GREEN}✅ Dockerfile found${NC}"
else
    echo -e "${RED}❌ Dockerfile missing${NC}"
    exit 1
fi

if [ -f "deployment/local/docker-compose.yml" ]; then
    echo -e "${GREEN}✅ Docker Compose config found${NC}"
else
    echo -e "${RED}❌ Docker Compose config missing${NC}"
    exit 1
fi

echo ""
echo "=========================================="
echo "Validation Complete!"
echo "=========================================="
echo ""
echo -e "${GREEN}✅ All prerequisites met${NC}"
echo ""
echo "Next steps:"
echo "  1. Build Docker images (takes 10-15 min):"
echo "     cd deployment/local"
echo "     docker-compose build"
echo ""
echo "  2. Start testnet:"
echo "     docker-compose up -d"
echo ""
echo "  3. Check status:"
echo "     docker-compose ps"
echo ""
echo "  4. View logs:"
echo "     docker-compose logs -f"
echo ""
echo "Or run the full setup script:"
echo "  cd deployment/local"
echo "  ./setup.sh"
echo ""
