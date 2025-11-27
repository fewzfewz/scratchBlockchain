# Docker Compose Files Analysis

## Overview
Your project has **3 docker-compose files** serving different purposes:

1. `deployment/local/docker-compose.yml` - **MAIN TESTNET** (Currently Running)
2. `docker-compose.yml` - **ALTERNATIVE SETUP** (Root directory)
3. `monitoring/docker-compose.yml` - **STANDALONE MONITORING**

---

## 1. deployment/local/docker-compose.yml ✅ **ACTIVE**

**Purpose**: Complete local testnet with all services  
**Location**: `/home/fewzan/.gemini/antigravity/scratch/deployment/local/`  
**Status**: **Currently Running** (This is what you're using)

### Services (9 total):

#### Validators (3)
- **validator1** - Bootstrap validator
  - Ports: 26656 (P2P), 26657 (RPC), 8545 (API), 9090 (Metrics)
  - Volumes: Mounts node key from `configs/validator1_key.json`
  - Command: `start --config /config.toml`
  
- **validator2** - Second validator
  - Ports: 26658 (P2P), 26659 (RPC), 8546 (API), 9091 (Metrics)
  - Volumes: Mounts node key from `configs/validator2_key.json`
  
- **validator3** - Third validator
  - Ports: 26660 (P2P), 26661 (RPC), 8547 (API), 9096 (Metrics)
  - Volumes: Mounts node key from `configs/validator3_key.json`

#### RPC Nodes (2)
- **rpc1** - Public RPC node
  - Ports: 8548 (API), 26662 (RPC), 9093 (Metrics)
  - No validator key (read-only)
  
- **rpc2** - Backup RPC node
  - Ports: 8549 (API), 26663 (RPC), 9094 (Metrics)
  - No validator key (read-only)

#### Monitoring (2)
- **prometheus** - Metrics collection
  - Port: 9095
  - Config: `configs/prometheus.yml`
  
- **grafana** - Metrics visualization
  - Port: 3000
  - Password: admin/admin

#### Services (2)
- **faucet** - Test token distribution
  - Port: 3001
  - Drip amount: 1000 tokens
  - Cooldown: 24 hours
  
- **nginx** - Reverse proxy
  - Ports: 80, 443
  - Routes: /rpc, /faucet, /grafana, /prometheus

### Key Features:
✅ Persistent node keys (fixes peer ID issue!)
✅ Shared genesis file
✅ Custom network (testnet)
✅ Health checks
✅ Auto-restart

### How to Use:
```bash
# Start
cd deployment/local
docker-compose up -d

# View logs
docker-compose logs -f validator1

# Stop
docker-compose down

# Clean data
docker-compose down -v
```

---

## 2. docker-compose.yml ⚠️ **ALTERNATIVE**

**Purpose**: Simpler 3-node setup with different ports  
**Location**: `/home/fewzan/.gemini/antigravity/scratch/` (root)  
**Status**: **Not Currently Used**

### Services (5 total):

#### Nodes (3)
- **node1** - Validator
  - Ports: 9933 (RPC), 30333 (P2P)
  - IP: 172.25.0.10
  
- **node2** - Validator
  - Ports: 9934 (RPC), 30334 (P2P)
  - IP: 172.25.0.11
  
- **node3** - Validator
  - Ports: 9935 (RPC), 30335 (P2P)
  - IP: 172.25.0.12

#### Monitoring (2)
- **prometheus** - Port 9090
- **grafana** - Port 3000 (password: blockchain2024)

### Differences from deployment/local:
- ❌ No node key persistence
- ❌ No faucet service
- ❌ No nginx proxy
- ❌ Different port scheme (9933 vs 26657)
- ✅ Fixed IP addresses
- ✅ Simpler configuration

### How to Use:
```bash
# Start (from root directory)
docker-compose up -d

# Stop
docker-compose down
```

**Note**: This appears to be an older/alternative setup. The `deployment/local` version is more complete.

---

## 3. monitoring/docker-compose.yml 📊 **STANDALONE**

**Purpose**: Monitoring stack only (no blockchain nodes)  
**Location**: `/home/fewzan/.gemini/antigravity/scratch/monitoring/`  
**Status**: **Standalone** (can run independently)

### Services (3 total):
- **prometheus** - Port 9090
- **grafana** - Port 3000
- **alertmanager** - Port 9093 (for alerts)

### Use Case:
Run monitoring separately to connect to external blockchain nodes.

### How to Use:
```bash
cd monitoring
docker-compose up -d
```

---

## Comparison Table

| Feature | deployment/local | root docker-compose | monitoring only |
|---------|-----------------|---------------------|-----------------|
| Validators | 3 | 3 | 0 |
| RPC Nodes | 2 | 0 | 0 |
| Faucet | ✅ | ❌ | ❌ |
| Nginx | ✅ | ❌ | ❌ |
| Prometheus | ✅ | ✅ | ✅ |
| Grafana | ✅ | ✅ | ✅ |
| Alertmanager | ❌ | ❌ | ✅ |
| Node Keys | ✅ Persistent | ❌ Regenerate | N/A |
| Network | testnet | blockchain-net | default |
| Port Scheme | 26657 | 9933 | 9090 |

---

## Which One to Use?

### ✅ **Use `deployment/local/docker-compose.yml`** if you want:
- Complete testnet with all features
- Persistent node keys (fixes peer ID issue!)
- Faucet service
- Nginx reverse proxy
- Production-like setup

### ⚠️ **Use `docker-compose.yml`** (root) if you want:
- Simpler setup
- Different port scheme
- Fixed IP addresses
- Quick testing

### 📊 **Use `monitoring/docker-compose.yml`** if you want:
- Only monitoring stack
- Connect to external nodes
- Alerting with Alertmanager

---

## Current Setup Analysis

**You are currently running**: `deployment/local/docker-compose.yml`

**Evidence**:
```bash
$ docker ps
validator1, validator2, validator3  # From deployment/local
faucet, prometheus, grafana, nginx  # From deployment/local
```

**Key Insight**: This setup has **persistent node keys**!
```yaml
volumes:
  - ./configs/validator1_key.json:/data/node_key.json:ro
```

This means peer IDs should NOT change on restart if the key files exist.

---

## Critical Discovery: Node Keys! 🔑

Looking at the docker-compose file, I see it's trying to mount node key files:
- `./configs/validator1_key.json`
- `./configs/validator2_key.json`
- `./configs/validator3_key.json`

**Let me check if these files exist...**
