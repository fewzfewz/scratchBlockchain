#!/bin/bash
# Production Deployment Script for Blockchain Network

set -e

echo "🚀 Blockchain Production Deployment"
echo "===================================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check Docker
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker is not installed${NC}"
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}❌ Docker Compose is not installed${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Docker and Docker Compose found${NC}"

# Build images
echo -e "\n${YELLOW}📦 Building Docker images...${NC}"
docker-compose build

# Start services
echo -e "\n${YELLOW}🚀 Starting services...${NC}"
docker-compose up -d

# Wait for services to be healthy
echo -e "\n${YELLOW}⏳ Waiting for services to be healthy...${NC}"
sleep 10

# Check node health
echo -e "\n${YELLOW}🏥 Checking node health...${NC}"
for port in 9933 9934 9935; do
    if curl -sf http://localhost:$port/health > /dev/null; then
        echo -e "${GREEN}✅ Node on port $port is healthy${NC}"
    else
        echo -e "${RED}❌ Node on port $port is not responding${NC}"
    fi
done

# Check Prometheus
echo -e "\n${YELLOW}📊 Checking Prometheus...${NC}"
if curl -sf http://localhost:9090/-/healthy > /dev/null; then
    echo -e "${GREEN}✅ Prometheus is healthy${NC}"
else
    echo -e "${RED}❌ Prometheus is not responding${NC}"
fi

# Check Grafana
echo -e "\n${YELLOW}📈 Checking Grafana...${NC}"
if curl -sf http://localhost:3000/api/health > /dev/null; then
    echo -e "${GREEN}✅ Grafana is healthy${NC}"
else
    echo -e "${RED}❌ Grafana is not responding${NC}"
fi

echo -e "\n${GREEN}✨ Deployment complete!${NC}"
echo -e "\n📍 Access points:"
echo -e "   Node 1 RPC: http://localhost:9933"
echo -e "   Node 2 RPC: http://localhost:9934"
echo -e "   Node 3 RPC: http://localhost:9935"
echo -e "   Prometheus: http://localhost:9090"
echo -e "   Grafana:    http://localhost:3000 (admin/nebula)"
echo -e "\n💡 View logs: docker-compose logs -f"
echo -e "💡 Stop network: docker-compose down"
echo -e "💡 Stop and remove data: docker-compose down -v"
