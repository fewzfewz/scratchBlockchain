# Monitoring Infrastructure

Complete monitoring stack for Modular Blockchain testnet using Prometheus, Grafana, and Alertmanager.

## Components

### 1. Prometheus Metrics Exporter
- **Location**: `monitoring/src/`
- **Features**:
  - 18 blockchain-specific metrics
  - Block, transaction, validator, network, consensus, storage metrics
  - HTTP endpoint at `:9090/metrics`
  - Health check at `:9090/health`

### 2. Prometheus Server
- **Configuration**: `prometheus/prometheus.yml`
- **Alert Rules**: `prometheus/alerts.yml`
- **Scrape Targets**: Validators, RPC nodes, relayers, services

### 3. Grafana Dashboards
- **Network Overview**: Block height, tx rate, validators, peers
- **Validator Performance**: Uptime, missed blocks, stake
- **Consensus Metrics**: Round time, finality time
- **Storage Metrics**: State size, DB performance

### 4. Alertmanager
- **Critical Alerts**: Validator down, consensus stalled
- **Warning Alerts**: High missed blocks, slow consensus, low peers
- **Info Alerts**: Low transaction rate

## Quick Start

### 1. Start Monitoring Stack

```bash
cd monitoring
docker-compose up -d
```

This starts:
- Prometheus on `:9090`
- Grafana on `:3000`
- Alertmanager on `:9093`

### 2. Access Dashboards

- **Grafana**: http://localhost:3000
  - Username: `admin`
  - Password: `admin`

- **Prometheus**: http://localhost:9090

- **Alertmanager**: http://localhost:9093

### 3. Add to Node

Add to your node's `Cargo.toml`:
```toml
[dependencies]
monitoring = { path = "../monitoring" }
```

Use in your node:
```rust
use monitoring::{BlockchainMetrics, MetricsServer};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    // Create metrics
    let metrics = Arc::new(RwLock::new(BlockchainMetrics::new().unwrap()));
    
    // Start metrics server
    let server = MetricsServer::new(metrics.clone());
    tokio::spawn(async move {
        server.serve("0.0.0.0:9090").await.unwrap();
    });
    
    // Update metrics
    {
        let m = metrics.write().await;
        m.block_height.set(100);
        m.tx_total.inc();
        m.validator_count.set(3);
    }
}
```

## Metrics Reference

### Block Metrics
- `chain_block_height` - Current block height
- `chain_block_time_seconds` - Block production time
- `chain_blocks_produced_total` - Total blocks produced

### Transaction Metrics
- `chain_transactions_total` - Total transactions processed
- `chain_transactions_pending` - Pending transactions
- `chain_tx_processing_seconds` - Transaction processing time

### Validator Metrics
- `chain_validator_count` - Active validators
- `chain_validator_stake_total` - Total staked amount
- `chain_missed_blocks_total` - Missed blocks

### Network Metrics
- `chain_peer_count` - Connected peers
- `chain_network_bytes_sent_total` - Bytes sent
- `chain_network_bytes_received_total` - Bytes received

### Consensus Metrics
- `chain_consensus_rounds_total` - Consensus rounds
- `chain_consensus_time_seconds` - Consensus round time
- `chain_finality_time_seconds` - Block finality time

### Storage Metrics
- `chain_state_size_bytes` - State size
- `chain_db_read_seconds` - DB read time
- `chain_db_write_seconds` - DB write time

## Alert Rules

### Critical
- **ValidatorDown**: Validator offline for 2+ minutes
- **ConsensusStalled**: No blocks for 2+ minutes

### Warning
- **HighMissedBlocks**: Missing >10% of blocks
- **SlowConsensus**: Consensus >10 seconds
- **LowPeerCount**: <3 peers connected
- **HighDiskUsage**: State >80GB
- **SlowDatabaseWrites**: Writes >100ms

### Info
- **LowTransactionRate**: <1 tx/s for 10 minutes

## Grafana Dashboards

### Network Overview
- Block height over time
- Transaction rate
- Active validators
- Total stake
- Peer count
- Pending transactions
- Consensus time
- Block time

### Validator Performance
- Uptime percentage
- Missed blocks
- Stake amount
- Commission rate
- Rewards earned

### System Health
- CPU usage
- Memory usage
- Disk usage
- Network I/O

## Production Deployment

### 1. Configure Targets

Edit `prometheus/prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'validators'
    static_configs:
      - targets:
          - 'validator1.modular.io:9090'
          - 'validator2.modular.io:9090'
          - 'validator3.modular.io:9090'
```

### 2. Set Up Alerts

Configure Slack/Email in `alertmanager/config.yml`:
```yaml
receivers:
  - name: 'team'
    slack_configs:
      - api_url: 'YOUR_WEBHOOK_URL'
        channel: '#alerts'
```

### 3. Secure Grafana

Change default password:
```bash
docker exec -it grafana grafana-cli admin reset-admin-password newpassword
```

### 4. Enable HTTPS

Use reverse proxy (nginx/Caddy) with SSL certificates.

## Troubleshooting

### Metrics Not Showing

1. Check metrics endpoint:
   ```bash
   curl http://localhost:9090/metrics
   ```

2. Verify Prometheus scraping:
   ```bash
   curl http://localhost:9090/api/v1/targets
   ```

### Alerts Not Firing

1. Check alert rules:
   ```bash
   curl http://localhost:9090/api/v1/rules
   ```

2. Verify Alertmanager:
   ```bash
   curl http://localhost:9093/api/v1/alerts
   ```

## License

MIT
