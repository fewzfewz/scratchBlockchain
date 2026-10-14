//! The block header's `state_root`.
//!
//! It commits to all consensus state after the *parent* block (the
//! CometBFT "app hash" convention): the proposer knows it without executing
//! the new block, and every validator checks it against its own state before
//! voting. A node whose state diverged rejects the next proposal instead of
//! silently building on different balances.
//!
//! `state_root = SHA-256("NEBULA_STATE_V1" || flat || trie)`, where `flat` is
//! the LtHash checksum of the flat state column (accounts, EVM, WASM) and
//! `trie` is the governance / validator / staking Patricia trie root.

use sha2::{Digest, Sha256};

pub const STATE_DOMAIN: &[u8] = b"NEBULA_STATE_V1";

pub fn compute(flat_commitment: &[u8; 32], trie_root: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(STATE_DOMAIN);
    h.update(flat_commitment);
    h.update(trie_root);
    h.finalize().into()
}
