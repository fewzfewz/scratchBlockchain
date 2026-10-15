//! Inspect a node's data directory: stored vs. recomputed flat-state
//! commitment, trie root, and the resulting state root.
//!
//! Usage: cargo run -p node --example state_inspect -- <data_dir> [--keys]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sha2::{Digest, Sha256};
use storage::trie::PatriciaTrie;
use storage::{ChainStore, ColumnFamily, StagedStore, STATE_ROOT_KEY};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).expect("data dir");
    let base = ChainStore::open(dir).unwrap().inner().clone();
    let staged = StagedStore::new(base.clone());
    let chain = ChainStore::new(base.clone());

    let stored = staged.committed_lthash().unwrap().checksum();
    let recomputed = staged.recompute_lthash().unwrap().checksum();
    let trie = PatriciaTrie::new(base.clone()).unwrap();
    let trie_root = trie.root_hash();
    println!("tip height        {:?}", chain.get_latest_height().unwrap());
    println!("stored lthash     {}", hex::encode(stored));
    println!("recomputed lthash {}", hex::encode(recomputed));
    println!("match             {}", stored == recomputed);
    println!("trie root         {}", hex::encode(trie_root));
    println!(
        "state root stored {}",
        hex::encode(
            base.get(ColumnFamily::Meta, STATE_ROOT_KEY)
                .unwrap()
                .unwrap_or_default()
        )
    );
    println!(
        "state root (stored lthash)     {}",
        hex::encode(node::state_root::compute(&stored, &trie_root))
    );
    println!(
        "state root (recomputed lthash) {}",
        hex::encode(node::state_root::compute(&recomputed, &trie_root))
    );
    if let Some(v) = trie.get(b"validators").unwrap() {
        let vals: Vec<serde_json::Value> = serde_json::from_slice(&v).unwrap();
        for val in vals {
            let pk = val["public_key"].as_str().unwrap_or("");
            let addr = node::accounts::address_of_public_key(&hex::decode(pk).unwrap_or_default());
            let (bal, _) = node::accounts::load(&chain, &addr);
            println!(
                "VAL {} produced {} missed {} balance {}",
                &pk[..12.min(pk.len())],
                val["blocks_produced"],
                val["blocks_missed"],
                bal
            );
        }
    }
    if args.iter().any(|a| a == "--keys") {
        for (k, v) in base.iter(ColumnFamily::State).unwrap() {
            let h: [u8; 32] = Sha256::digest(&v).into();
            let printable = if k.iter().all(|c| c.is_ascii_graphic()) {
                String::from_utf8_lossy(&k).to_string()
            } else {
                hex::encode(&k)
            };
            println!("KEY {} {}", printable, hex::encode(&h[..8]));
        }
    }
}
