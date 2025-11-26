# Local Testnet Deployment

Run a complete Modular Blockchain testnet on your local machine using Docker.

## Quick Start

```bash
cd deployment/local
./setup.sh
```

This will:
1. Generate genesis file
2. Create validator configurations
3. Build Docker images
4. Start 3 validators + 2 RPC nodes + monitoring stack

## Services

### Blockchain Nodes
- **Validator 1** (Bootstrap): `localhost:26657` (RPC), `localhost:8545` (API)
- **Validator 2**: `localhost:26659` (RPC), `localhost:8546` (API)
- **Validator 3**: `localhost:26661` (RPC), `localhost:8547` (API)
- **RPC Node 1**: `localhost:8548` (API)
- **RPC Node 2**: `localhost:8549` (API)

### Services
- **Nginx (Load Balancer)**: `http://localhost`
  - RPC: `http://localhost/rpc`
  - Faucet: `http://localhost/faucet`
  - Grafana: `http://localhost/grafana`
  - Prometheus: `http://localhost/prometheus`

- **Grafana**: `http://localhost:3000` (admin/admin)
- **Prometheus**: `http://localhost:9095`
- **Faucet**: `http://localhost:3001`

## Usage

### Connect with SDK

```typescript
import { ModularClient, HttpProvider } from '@modular-blockchain/sdk';

const provider = new HttpProvider('http://localhost/rpc');
const client = new ModularClient(provider);

const chainId = await client.getChainId();
console.log('Connected to:', chainId);
```

### Get Test Tokens

Visit: `http://localhost/faucet`

Or use curl:
```bash
curl -X POST http://localhost/faucet \
  -H "Content-Type: application/json" \
  -d '{"address": "0x..."}'
```

### View Metrics

- **Grafana**: http://localhost/grafana
  - Username: `admin`
  - Password: `admin`

- **Prometheus**: http://localhost/prometheus

## Management

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f validator1
docker-compose logs -f rpc1
```

### Check Status

```bash
docker-compose ps
```

### Restart Services

```bash
# Restart all
docker-compose restart

# Restart specific service
docker-compose restart validator1
```

### Stop Testnet

```bash
docker-compose down
```

### Clean Data (Fresh Start)

```bash
docker-compose down -v
./setup.sh
```

## Testing

### Check Block Height

```bash
curl http://localhost/rpc/status | jq '.result.sync_info.latest_block_height'
```

### Send Transaction

```bash
curl -X POST http://localhost/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "send_transaction",
    "params": [{
      "to": "0x...",
      "value": "1000000000000000000"
    }],
    "id": 1
  }'
```

### Check Validator Status

```bash
curl http://localhost:26657/status | jq
```

## Troubleshooting

### Services Not Starting

1. Check Docker is running:
   ```bash
   docker ps
   ```

2. Check logs:
   ```bash
   docker-compose logs
   ```

3. Rebuild images:
   ```bash
   docker-compose build --no-cache
   ```

### Port Conflicts

If ports are already in use, edit `docker-compose.yml` to change port mappings.

### Out of Disk Space

Clean up Docker:
```bash
docker system prune -a
```

## Migration to Cloud

When ready to deploy to cloud, see: [Cloud Deployment Guide](../cloud/README.md)

The local setup is designed to be identical to cloud deployment, making migration seamless.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                    Nginx                         │
│         (Reverse Proxy & Load Balancer)         │
└─────────────────┬───────────────────────────────┘
                  │
        ┌─────────┼─────────┐
        │         │         │
   ┌────▼───┐ ┌──▼────┐ ┌──▼────┐
   │Validator│ │Validator│ │Validator│
   │    1    │ │    2    │ │    3    │
   └────┬───┘ └───┬───┘ └───┬───┘
        │         │         │
        └─────────┼─────────┘
                  │
        ┌─────────┼─────────┐
        │         │         │
   ┌────▼───┐ ┌──▼────┐ ┌──▼────────┐
   │  RPC   │ │  RPC  │ │ Monitoring │
   │   1    │ │   2   │ │  Stack     │
   └────────┘ └───────┘ └────────────┘
```

## Performance

### Resource Usage
- **CPU**: ~2-4 cores
- **RAM**: ~4-8 GB
- **Disk**: ~10-20 GB
- **Network**: Minimal (localhost)

### Expected Performance
- **Block Time**: 3 seconds
- **TPS**: 100-500 (depending on hardware)
- **Finality**: ~9 seconds (3 blocks)

## Next Steps

1. ✅ Run local testnet
2. ✅ Test with SDK
3. ✅ Build example dApp
4. ✅ Test governance
5. ✅ Test bridge (with local Ethereum node)
6. 🚀 Deploy to cloud

## Support

- Check logs: `docker-compose logs`
- GitHub Issues: https://github.com/modular-blockchain/node/issues
- Discord: #testnet-support
