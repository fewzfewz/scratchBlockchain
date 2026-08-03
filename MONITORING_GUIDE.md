# Monitoring Guide: Prometheus & Grafana (Nebula Testnet)

A detailed, step-by-step guide for using the Prometheus + Grafana monitoring stack in this project. It covers what the stack does, how data flows, how to query metrics, how to build dashboards, and how to troubleshoot.

---

## 1. What the Stack Does in This Project

Your blockchain nodes continuously record numbers about themselves (blocks produced, transactions, mempool size, peers, finalized height, TPS, …). Two tools turn those numbers into useful graphs:

| Tool | Job |
|------|-----|
| **Prometheus** | The **collector + database**. Every 15 seconds it asks ("scrapes") each node for its metrics and stores the values as a time series (a history). It also evaluates alert rules. |
| **Grafana** | The **dashboard UI**. It reads the history Prometheus stored and draws it as graphs, tables, and status panels. |

```
Node1 ──/metrics──┐
Node2 ──/metrics──┤   Prometheus (scrapes every 15s,  stores history,  raises alerts)
Node3 ──/metrics──┼──► ┌───────────┐
Node4 ──/metrics──┤   │  :9095     │────► Grafana (graphs on :3000)
Node5 ──/metrics──┘   └───────────┘
```

**Concrete value for you**: you can see the block height climb in real time, spot a validator that stopped producing, watch TPS when you send transactions, and get alerted if a node goes down.

Node metrics are scraped from each container's `/metrics` endpoint (port 26657 in Docker). JSON-RPC for applications is on port **8545**; WebSocket at `ws://localhost:8545/ws`.

---

## 2. Starting and Accessing

### 2.1 Start the monitoring stack

It is already included in the main testnet compose file. Starting the testnet starts Prometheus and Grafana too:

```bash
cd deployment/local
docker-compose up -d
```

If you ever run the standalone stack (includes Alertmanager), use:

```bash
cd monitoring
docker-compose up -d
```

### 2.2 Access URLs

| Service | URL (host) | Container address | Notes |
|---------|-----------|-------------------|-------|
| Grafana | http://localhost:3000 | `grafana:3000` | login `admin` / `admin` |
| Prometheus | http://localhost:9095 | `prometheus:9090` | direct API + expression browser |
| Alertmanager | http://localhost:9093 | `alertmanager:9093` | only if you start `monitoring/docker-compose.yml` |

### 2.3 Verify everything is running

```bash
# Containers
docker ps --format "{{.Names}}: {{.Status}}" | grep -iE "prom|graf"

# The 5 nodes are being scraped successfully
curl -s http://localhost:9095/api/v1/targets | \
  python3 -c "import sys,json; [print(t['labels']['job'], t['health']) for t in json.load(sys.stdin)['data']['activeTargets']]"
```

Expected: every job prints `up`.

---

## 3. How the Data Gets There (the /metrics endpoint)

Each node serves its metrics as plain text at **`http://<node>:26657/metrics`**. Try it:

```bash
curl -s http://localhost:26657/metrics
```

You'll see the actual metrics this node exports (defined in `node/src/metrics.rs`):

| Metric | Meaning |
|--------|---------|
| `blockchain_blocks_total` | Total blocks produced by this node |
| `blockchain_transactions_total` | Total transactions processed |
| `blockchain_mempool_size` | Pending transactions waiting in the mempool |
| `blockchain_peer_count` | Connected P2P peers |
| `blockchain_finalized_height` | Highest finalized block height |
| `blockchain_tps` | Transactions per second (computed) |
| `blockchain_consensus_round` | Current consensus round |
| `blockchain_network_bytes_rx_total` / `blockchain_network_bytes_tx_total` | Bytes received / sent on the P2P network |
| `blockchain_block_latency_ms` | Block propagation latency |
| `blockchain_mev_protected_txs_total` / `blockchain_aa_operations_total` | MEV-protected / account-abstraction op counts |

**Important**: a few Grafana panels reference metrics named `chain_*` (e.g. `chain_block_height`, `chain_missed_blocks_total`). Those come from a separate `monitoring` crate that is **not** wired into the running nodes, so those panels may show "No data." Panels using the `blockchain_*` names above will show live data.

---

## 4. Using Prometheus (the collector + database)

### 4.1 The main UI

Open **http://localhost:9095**. The key tabs:

- **Graph** — the expression browser. Type a query, click *Execute* to see a table, or switch to the *Graph* tab for a line chart over time.
- **Status → Targets** — shows every scrape target and whether it's `UP` or `DOWN`. **Check this first if anything looks missing.**
- **Status → Rules** — shows the alert rules and their current state.
- **Status → Command-Line Flags** — shows retention, scrape interval, etc.

### 4.2 Useful queries to try

| What you want | Query |
|---------------|-------|
| Current block height of a node | `blockchain_blocks_total{job="validator1"}` |
| All nodes' block heights | `blockchain_blocks_total` |
| Block production rate per second | `rate(blockchain_blocks_total[1m])` |
| Finalized height | `blockchain_finalized_height` |
| Pending transactions | `blockchain_mempool_size` |
| Connected peers | `blockchain_peer_count` |
| Transaction throughput | `rate(blockchain_transactions_total[5m])` |
| Network bytes in/out rate | `rate(blockchain_network_bytes_rx_total[1m])`, `rate(blockchain_network_bytes_tx_total[1m])` |
| Is the network advancing right now? | `increase(blockchain_blocks_total[1m]) > 0` |

Notes on the PromQL language:
- `rate(x[1m])` / `increase(x[1m])` only work on **counters** (values that only go up): `blockchain_blocks_total`, `blockchain_transactions_total`, byte counters.
- `{job="validator1"}` is a label filter — each job is one node (see `prometheus.yml`).
- Use the **Graph tab** and set a time range (e.g. last 15 minutes) to see trends, not just the current number.

### 4.3 Configuration you can tweak

Prometheus config lives in `deployment/local/configs/prometheus.yml` (the standalone one is in `monitoring/prometheus/prometheus.yml`):

```yaml
global:
  scrape_interval: 15s        # how often Prometheus asks nodes for metrics

scrape_configs:
  - job_name: 'validator1'    # one job per node
    metrics_path: '/metrics'  # the endpoint served at :26657
    static_configs:
      - targets: ['validator1:26657']
```

To add or remove a node: edit the file, then reload Prometheus without restarting:

```bash
docker exec prometheus kill -HUP 1
```

### 4.4 Useful API calls (for scripts)

```bash
# All targets + health
curl -s http://localhost:9095/api/v1/targets

# Instant query
curl -s 'http://localhost:9095/api/v1/query?query=blockchain_blocks_total'

# Range query (last 30 min, step 60s) — good for building your own graphs
curl -s 'http://localhost:9095/api/v1/query_range?query=blockchain_blocks_total&start=now-30m&end=now&step=60s'

# Alert rules
curl -s http://localhost:9095/api/v1/rules
```

---

## 5. Using Grafana (the dashboards)

### 5.1 First login

1. Open **http://localhost:3000**
2. Username `admin`, password `admin`
3. You'll be asked to set a new password — you can skip this.

### 5.2 Dashboards

Click the **Dashboards** icon in the left sidebar (four squares). The project pre-loads dashboards from `monitoring/grafana/dashboards/` (auto-provisioned at startup):

| Dashboard | Shows |
|-----------|-------|
| **Network Overview** | Block height, transaction rate, validators, total stake, peers, pending txs, consensus time, block time |
| **Blockchain Overview** | Current block height, connected peers, TPS, mempool size |
| **Validator Performance** | Validator status, active validators, missed-block rate, peers, total staked |

Each dashboard is made of **panels**. On each panel you can:
- Change the **time range** (top-right corner): Last 5 min, 1 hour, 6 hours, today, etc.
- Set the **refresh interval** (next to the time picker), e.g. `5s` to watch the height tick up live.
- Hover the graph to see exact values; drag across a region to zoom.

### 5.3 Building your own dashboard (5 minutes)

1. Left sidebar → **Dashboards** → **New** → **New Dashboard** → **Add visualization**.
2. In the query editor, the **Prometheus** data source is already connected (`http://prometheus:9090`, see `monitoring/grafana/datasources/prometheus.yml`).
3. Enter a query, e.g. `blockchain_blocks_total`.
4. Pick a visualization on the right (Time series, Stat, Table, Gauge…).
5. Give the panel a title and click **Save dashboard** (top-right).

Tip: use a `Stat` panel with query `blockchain_finalized_height` and a `Time series` panel with `rate(blockchain_transactions_total[5m])` for an instant "is it alive" board.

### 5.4 Panels that show "No data"

If a panel shows **No data**, the metric name in its query doesn't exist on the nodes. Compare with the real metric list in section 3. The `chain_*` metrics (e.g. `chain_block_height`, `chain_missed_blocks_total`) are not emitted by the running nodes — only `blockchain_*` metrics are. Edit the panel's query (click the panel → **Edit**) and replace `chain_*` with the equivalent `blockchain_*` name, or leave it if you don't need that panel.

### 5.5 Admin essentials

- **Data sources**: Left sidebar → **Connections → Data sources**. You should see *Prometheus* (default). Add more sources here if you ever connect other databases.
- **Reset admin password** (if forgotten):
  ```bash
  docker exec grafana grafana-cli admin reset-admin-password newpassword
  ```
- **Settings**: Environment variables like `GF_SECURITY_ADMIN_PASSWORD` in `docker-compose.yml` control admin user/password and sign-up (`GF_USERS_ALLOW_SIGN_UP=false`).

---

## 6. Alerts

### 6.1 Alert rules in Prometheus

Rules are defined in `deployment/local/configs/alerts.yml` (standalone: `monitoring/prometheus/alerts.yml`):

```yaml
groups:
  - name: example
    rules:
    - alert: InstanceDown
      expr: up == 0
      for: 1m
      labels: { severity: page }
      annotations:
        summary: "Instance {{ $labels.instance }} down"
```

Current rules check `up == 0` — i.e. if a node stops answering `/metrics` for 1 minute, the `InstanceDown` alert fires. View their state:

```bash
curl -s http://localhost:9095/api/v1/rules
```

Or in the UI: **Status → Rules**.

### 6.2 Sending alerts out (Alertmanager)

Alertmanager is **not** part of the `deployment/local` compose file — start it separately if you want notifications:

```bash
cd monitoring && docker-compose up -d alertmanager
```

Configuration is in `monitoring/alertmanager/config.yml` (with a Slack template in `monitoring/alertmanager/templates/slack.tmpl`). Point Prometheus at it by adding to the Prometheus service command:

```
--alertmanager.url=http://alertmanager:9093
```

Then configure a receiver (e.g. Slack webhook or email) in `alertmanager/config.yml`. Prometheus sends matched alerts there, and Alertmanager delivers them.

---

## 7. How to Actually Use This to Monitor the Network

Here's a practical "health check" routine:

1. **Is every node being scraped?** → Prometheus **Status → Targets**: all `UP`.
2. **Is the network producing blocks?** → Grafana *Network Overview* → *Block Height* panel is climbing, or query `increase(blockchain_blocks_total[1m]) > 0`.
3. **Is the chain finalizing?** → `blockchain_finalized_height` should grow with block height.
4. **Are validators connected to each other?** → `blockchain_peer_count` per validator should be > 0.
5. **Any congestion?** → `blockchain_mempool_size` spikes = backlog; watch `blockchain_tps` and block latency `blockchain_block_latency_ms`.
6. **Alerts** → any `InstanceDown` in **Status → Rules** means a node is unreachable.

---

## 8. Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Prometheus **Status → Targets** shows `DOWN` for a node | Node not running or not listening on 26657 | `docker ps`; check the node logs: `docker logs <node> \| tail` |
| Grafana panel shows **No data** | Query references a metric the node doesn't export (e.g. `chain_*`) | Edit panel, use real `blockchain_*` metrics from section 3 |
| Prometheus UI empty at `:9095` | Prometheus container not running | `docker ps \| grep prometheus`; `docker-compose up -d prometheus` |
| Grafana won't accept login | You set a new password before and forgot it | `docker exec grafana grafana-cli admin reset-admin-password admin` |
| Height graph looks flat | The node actually stopped producing, OR the time range is too small / refresh too slow | Check `increase(blockchain_blocks_total[1m]) > 0`; widen time range |
| You edited `prometheus.yml` and nothing changed | Prometheus needs a reload | `docker exec prometheus kill -HUP 1` |
| Alerts not firing | Rule expression wrong, or Alertmanager not configured | `curl http://localhost:9095/api/v1/rules`; check `for:` duration vs the actual outage |

---

## 9. Files That Matter

| File | Purpose |
|------|---------|
| `node/src/metrics.rs` | The metrics the nodes export (`blockchain_*`) |
| `deployment/local/configs/prometheus.yml` | Which nodes Prometheus scrapes, how often |
| `deployment/local/configs/alerts.yml` | Alert rules (currently `InstanceDown`) |
| `monitoring/grafana/dashboards/*.json` | Pre-built dashboards auto-loaded into Grafana |
| `monitoring/grafana/datasources/prometheus.yml` | Grafana → Prometheus connection |
| `monitoring/alertmanager/config.yml` | Where alerts get delivered (Slack/email) |
| `deployment/local/docker-compose.yml` | Prometheus (`9095`) and Grafana (`3000`) services |
