//! `nebula-relayer`: a standalone Ethereum → Nebula bridge relayer.
//!
//! Each relayer runs independently with its own Ed25519 key (one of the
//! genesis relayer set). It
//!
//! 1. follows `Bridge.sol` on Ethereum for native-ETH `TokensLocked` events,
//!    only once they have `--confirmations` blocks on top;
//! 2. signs an attestation for each lock (amount and recipient come from the
//!    event) and posts it to the Nebula nodes' attestation mailbox;
//! 3. once `threshold` relayers have attested, submits the `bridge_mint`
//!    transaction — relayers take turns per lock, so normally exactly one
//!    pays for it, and consensus rejects any duplicate.
//!
//! Progress (last scanned block, unminted locks) is kept in `--state-file`,
//! so a restarted relayer resumes where it stopped.

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use common::bridge::{self, Attestation, BridgeMint, EthLock};
use common::types::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, info, warn};

#[derive(Parser, Debug)]
#[command(version, about = "Standalone Ethereum → Nebula bridge relayer")]
struct Args {
    /// Ethereum JSON-RPC endpoint.
    #[arg(long, env = "ETH_RPC_URL", default_value = "http://127.0.0.1:9545")]
    eth_rpc: String,
    /// Bridge.sol contract address.
    #[arg(long, env = "ETH_BRIDGE_ADDRESS")]
    bridge: String,
    /// Nebula RPC endpoints (comma-separated); attestations go to all of them.
    #[arg(
        long,
        env = "NEBULA_RPC_URLS",
        default_value = "http://127.0.0.1:8545",
        value_delimiter = ','
    )]
    nebula_rpc: Vec<String>,
    /// Hex Ed25519 seed of this relayer's attestation key.
    #[arg(long, env = "RELAYER_KEY", hide_env_values = true)]
    relayer_key: String,
    /// Hex Ed25519 seed of the Nebula account that pays for mint
    /// transactions (defaults to the relayer key).
    #[arg(long, env = "SUBMITTER_KEY", hide_env_values = true)]
    submitter_key: Option<String>,
    /// Nebula chain id the attestations are bound to.
    #[arg(long, env = "NEBULA_CHAIN_ID", default_value_t = 1)]
    chain_id: u64,
    /// Blocks a lock must be buried under before it is attested.
    #[arg(long, env = "ETH_CONFIRMATIONS", default_value_t = 2)]
    confirmations: u64,
    /// First Ethereum block to scan when there is no saved state.
    #[arg(long, env = "ETH_START_BLOCK", default_value_t = 0)]
    start_block: u64,
    /// Where progress is saved.
    #[arg(long, env = "RELAYER_STATE_FILE", default_value = "relayer-state.json")]
    state_file: PathBuf,
    /// Poll interval.
    #[arg(long, env = "RELAYER_POLL_MS", default_value_t = 2000)]
    poll_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PendingLock {
    recipient: String,
    amount: String,
    /// Polls since the lock reached the attestation threshold.
    #[serde(default)]
    ready_polls: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    next_block: u64,
    /// ETH lock tx hash (0x…) → lock, until it is minted on Nebula.
    pending: BTreeMap<String, PendingLock>,
}

impl State {
    fn load(path: &PathBuf, start_block: u64) -> State {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or(State {
                next_block: start_block,
                pending: BTreeMap::new(),
            })
    }

    fn save(&self, path: &PathBuf) -> Result<()> {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

struct Relayer {
    args: Args,
    http: reqwest::Client,
    key: ed25519_dalek::SigningKey,
    submitter: ed25519_dalek::SigningKey,
    state: State,
}

fn parse_key(hex_seed: &str) -> Result<ed25519_dalek::SigningKey> {
    let seed: [u8; 32] =
        bridge::decode_fixed(hex_seed).ok_or_else(|| anyhow!("key must be 32 hex bytes"))?;
    Ok(ed25519_dalek::SigningKey::from_bytes(&seed))
}

impl Relayer {
    async fn eth(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let res: serde_json::Value = self
            .http
            .post(&self.args.eth_rpc)
            .json(&serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params }))
            .send()
            .await?
            .json()
            .await?;
        if let Some(err) = res.get("error") {
            bail!("{} failed: {}", method, err);
        }
        Ok(res
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    /// Scan newly confirmed Ethereum blocks for bridge locks.
    async fn scan(&mut self) -> Result<()> {
        let head = self.eth("eth_blockNumber", serde_json::json!([])).await?;
        let head =
            u64::from_str_radix(head.as_str().unwrap_or("0x0").trim_start_matches("0x"), 16)?;
        // A block has `confirmations` confirmations when head - n + 1 >= it.
        let Some(safe) = (head + 1).checked_sub(self.args.confirmations.max(1)) else {
            return Ok(());
        };
        while self.state.next_block <= safe {
            let from = self.state.next_block;
            let to = safe.min(from + 999);
            let logs = self
                .eth(
                    "eth_getLogs",
                    serde_json::json!([{
                        "address": self.args.bridge,
                        "fromBlock": format!("0x{:x}", from),
                        "toBlock": format!("0x{:x}", to),
                        "topics": [bridge::TOKENS_LOCKED_TOPIC],
                    }]),
                )
                .await?;
            for log in logs.as_array().into_iter().flatten() {
                let tx = log
                    .get("transactionHash")
                    .and_then(|h| h.as_str())
                    .unwrap_or_default()
                    .to_lowercase();
                match bridge::parse_tokens_locked_log(log, &self.args.bridge) {
                    Ok(Some(EthLock { amount, recipient })) => {
                        info!(
                            "🔒 ETH lock {} — {} wei to 0x{}",
                            short(&tx),
                            amount,
                            hex::encode(recipient)
                        );
                        self.state.pending.entry(tx).or_insert(PendingLock {
                            recipient: format!("0x{}", hex::encode(recipient)),
                            amount: amount.to_string(),
                            ready_polls: 0,
                        });
                    }
                    Ok(None) => {}
                    Err(e) => warn!("Skipping lock {}: {}", short(&tx), e),
                }
            }
            self.state.next_block = to + 1;
            self.state.save(&self.args.state_file)?;
        }
        Ok(())
    }

    /// Attest every pending lock and submit the ones that are ready.
    async fn process_pending(&mut self) -> Result<()> {
        let me = hex::encode(self.key.verifying_key().to_bytes());
        let locks: Vec<(String, PendingLock)> = self
            .state
            .pending
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (eth_tx, lock) in locks {
            let status = match self
                .nebula_get(&format!("/bridge/attestations/{}", eth_tx))
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    warn!("No Nebula node reachable: {}", e);
                    return Ok(());
                }
            };
            if status["minted"].as_bool() == Some(true) {
                info!("✅ Lock {} is minted on Nebula", short(&eth_tx));
                self.state.pending.remove(&eth_tx);
                continue;
            }
            let eth_hash: [u8; 32] =
                bridge::decode_fixed(&eth_tx).ok_or_else(|| anyhow!("bad tx hash"))?;
            let recipient: [u8; 20] =
                bridge::decode_fixed(&lock.recipient).ok_or_else(|| anyhow!("bad recipient"))?;
            let amount: u128 = lock.amount.parse()?;

            // Post our attestation to every node (idempotent), so the pool is
            // complete wherever the mint gets assembled.
            let attestation =
                bridge::attest(&self.key, self.args.chain_id, &eth_hash, &recipient, amount);
            let body = serde_json::json!({
                "eth_tx_hash": eth_tx,
                "recipient": lock.recipient,
                "amount": lock.amount,
                "attestation": attestation,
            });
            for url in &self.args.nebula_rpc {
                match self.post(url, "/bridge/attestations", &body).await {
                    Ok(r) if r.get("error").is_some() => {
                        warn!("{} refused attestation: {}", url, r["error"])
                    }
                    Ok(_) => {}
                    Err(e) => debug!("{} unreachable: {}", url, e),
                }
            }

            // Attestations for exactly this claim, from any node's pool.
            let threshold = status["threshold"].as_u64().unwrap_or(u64::MAX);
            let mut attestations: BTreeMap<String, Attestation> = BTreeMap::new();
            attestations.insert(me.clone(), attestation);
            for url in &self.args.nebula_rpc {
                let Ok(s) = self
                    .get(url, &format!("/bridge/attestations/{}", eth_tx))
                    .await
                else {
                    continue;
                };
                for entry in s["entries"].as_array().into_iter().flatten() {
                    let same = entry["recipient"]
                        .as_str()
                        .is_some_and(|r| r.eq_ignore_ascii_case(&lock.recipient))
                        && entry["amount"].as_str() == Some(lock.amount.as_str());
                    if !same {
                        continue;
                    }
                    for a in entry["attestations"].as_array().into_iter().flatten() {
                        if let Ok(a) = serde_json::from_value::<Attestation>(a.clone()) {
                            attestations.insert(a.relayer.to_lowercase(), a);
                        }
                    }
                }
            }
            if (attestations.len() as u64) < threshold {
                debug!(
                    "Lock {}: {}/{} attestations",
                    short(&eth_tx),
                    attestations.len(),
                    threshold
                );
                continue;
            }

            // Take turns: relayer i submits after waiting i polls, so the
            // first in line normally mints and the others see it minted.
            let mut relayers: Vec<String> = status["relayers"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|r| {
                    r.as_str()
                        .map(|s| s.trim_start_matches("0x").to_lowercase())
                })
                .collect();
            relayers.sort();
            let n = relayers.len().max(1);
            let pos = relayers.iter().position(|r| *r == me).unwrap_or(0);
            let turn = (pos + n - (eth_hash[0] as usize % n)) % n;
            let ready = self.state.pending.get_mut(&eth_tx).map(|l| {
                l.ready_polls += 1;
                l.ready_polls
            });
            if ready.unwrap_or(0) <= turn as u64 * 2 {
                continue;
            }
            let mint = BridgeMint {
                eth_tx_hash: eth_tx.clone(),
                recipient: lock.recipient.clone(),
                amount: lock.amount.clone(),
                attestations: attestations.into_values().collect(),
            };
            match self.submit_mint(&mint).await {
                Ok(hash) => info!(
                    "🌉 Submitted mint for lock {} (tx {})",
                    short(&eth_tx),
                    short(&hash)
                ),
                Err(e) => warn!("Mint submission for {} failed: {}", short(&eth_tx), e),
            }
        }
        self.state.save(&self.args.state_file)?;
        Ok(())
    }

    async fn submit_mint(&self, mint: &BridgeMint) -> Result<String> {
        let sender = common::crypto::derive_address_from_public_key(
            &self.submitter.verifying_key().to_bytes(),
        );
        let balance = self
            .nebula_get(&format!("/balance/{}", hex::encode(sender)))
            .await?;
        let nonce = balance["nonce"].as_u64().unwrap_or(0);
        let mut tx = Transaction::simple_transfer(sender, [0; 20], 0, nonce, self.args.chain_id);
        tx.to = None;
        tx.payload = mint.to_payload();
        tx.gas_limit = 100_000;
        tx.max_priority_fee_per_gas = 1_000_000_000;
        tx.sign_with(&self.submitter);
        let mut last_err = anyhow!("no Nebula node reachable");
        for url in &self.args.nebula_rpc {
            match self
                .post(url, "/submit_tx", &serde_json::to_value(&tx)?)
                .await
            {
                Ok(serde_json::Value::String(hash)) => return Ok(hash),
                Ok(r) => {
                    let status = r["status"].as_str().unwrap_or_default();
                    if status.starts_with("error") {
                        last_err = anyhow!("{}", status);
                        continue;
                    }
                    return Ok(r["hash"].as_str().unwrap_or_default().to_string());
                }
                Err(e) => last_err = e,
            }
        }
        Err(last_err)
    }

    async fn get(&self, base: &str, path: &str) -> Result<serde_json::Value> {
        Ok(self
            .http
            .get(format!("{}{}", base, path))
            .send()
            .await?
            .json()
            .await?)
    }

    async fn post(
        &self,
        base: &str,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let text = self
            .http
            .post(format!("{}{}", base, path))
            .json(body)
            .send()
            .await?
            .text()
            .await?;
        serde_json::from_str(&text).or_else(|_| Ok(serde_json::json!({ "status": text })))
    }

    /// First reachable node.
    async fn nebula_get(&self, path: &str) -> Result<serde_json::Value> {
        let mut last = anyhow!("no Nebula RPC configured");
        for url in &self.args.nebula_rpc {
            match self.get(url, path).await {
                Ok(v) => return Ok(v),
                Err(e) => last = e,
            }
        }
        Err(last)
    }
}

fn short(h: &str) -> &str {
    &h[..h.len().min(12)]
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let args = Args::parse();
    let key = parse_key(&args.relayer_key).context("RELAYER_KEY")?;
    let submitter = match &args.submitter_key {
        Some(k) => parse_key(k).context("SUBMITTER_KEY")?,
        None => key.clone(),
    };
    let state = State::load(&args.state_file, args.start_block);
    info!(
        "Relayer {} — Ethereum {} bridge {} from block {}, {} Nebula node(s), {} confirmations",
        hex::encode(key.verifying_key().to_bytes()),
        args.eth_rpc,
        args.bridge,
        state.next_block,
        args.nebula_rpc.len(),
        args.confirmations
    );
    let poll = Duration::from_millis(args.poll_ms);
    let mut relayer = Relayer {
        http: reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?,
        args,
        key,
        submitter,
        state,
    };
    loop {
        if let Err(e) = relayer.scan().await {
            warn!("Ethereum scan failed: {}", e);
        }
        if let Err(e) = relayer.process_pending().await {
            warn!("Processing pending locks failed: {}", e);
        }
        tokio::time::sleep(poll).await;
    }
}
