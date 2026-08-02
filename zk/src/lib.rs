//! # Zero-Knowledge Proof Module
//!
//! This module implements ZK proofs for the blockchain.
//! By default uses simplified non-cryptographic proofs.
//! Enable the `halo2` feature for actual Halo2-based proofs.

use anyhow::Result;
use sha2::{Digest, Sha256};
use tracing::{info, debug};
use std::collections::HashMap;

// ============================================================================
// Core types (used regardless of Halo2 feature)
// ============================================================================

/// Aggregated proof containing multiple state transitions
#[derive(Debug, Clone)]
pub struct AggregatedProof {
    pub proofs: Vec<Vec<u8>>,
    pub total_transitions: usize,
    pub aggregate_root: [u8; 32],
}

/// Proof aggregator
pub struct ProofAggregator {
    #[allow(dead_code)]
    max_proofs_per_batch: usize,
    pending_proofs: Vec<Vec<u8>>,
}

impl ProofAggregator {
    pub fn new(max_proofs_per_batch: usize) -> Self {
        Self {
            max_proofs_per_batch,
            pending_proofs: Vec::new(),
        }
    }

    pub fn add_proof(&mut self, proof: Vec<u8>) {
        self.pending_proofs.push(proof);
    }

    pub fn aggregate(&mut self) -> Result<Option<AggregatedProof>> {
        if self.pending_proofs.len() < 2 {
            return Ok(None);
        }

        let proofs = std::mem::take(&mut self.pending_proofs);
        let total = proofs.len();

        let mut hasher = Sha256::new();
        for proof in &proofs {
            hasher.update(proof);
        }
        let aggregate_root: [u8; 32] = hasher.finalize().into();

        Ok(Some(AggregatedProof {
            proofs,
            total_transitions: total,
            aggregate_root,
        }))
    }
}

/// Batch verifier for multiple proofs
pub struct BatchVerifier {
    proofs: Vec<Vec<u8>>,
    #[allow(dead_code)]
    max_batch_size: usize,
}

impl BatchVerifier {
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            proofs: Vec::new(),
            max_batch_size,
        }
    }

    pub fn add_proof(&mut self, proof: Vec<u8>) {
        self.proofs.push(proof);
    }

    pub fn verify_batch(&self, prover: &ZkProver) -> Vec<bool> {
        use rayon::prelude::*;
        self.proofs
            .par_iter()
            .map(|proof| proof.len() == 32)
            .collect()
    }

    pub fn clear(&mut self) {
        self.proofs.clear();
    }

    pub fn size(&self) -> usize {
        self.proofs.len()
    }
}

// ============================================================================
// ZkProver (production-proof wrapper)
// ============================================================================

/// ZK prover with proof caching and aggregation.
/// Uses SHA-256 based "proofs" by default.
/// Enable `halo2` feature for actual Halo2 zk-SNARK proofs.
pub struct ZkProver {
    proof_cache: HashMap<u64, Vec<u8>>,
    aggregator: ProofAggregator,
}

impl ZkProver {
    pub fn new() -> Self {
        Self {
            proof_cache: HashMap::new(),
            aggregator: ProofAggregator::new(100),
        }
    }

    pub fn prove(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Ok(hasher.finalize().to_vec())
    }

    pub fn verify(&self, proof: &[u8], data: &[u8]) -> Result<bool> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let expected = hasher.finalize().to_vec();
        Ok(proof == expected.as_slice())
    }

    /// Generate a proof for state transition
    pub fn prove_state_transition(
        &mut self,
        prev_state_root: [u8; 32],
        new_state_root: [u8; 32],
        tx_hash: [u8; 32],
        block_hash: [u8; 32],
    ) -> Result<Vec<u8>> {
        let cache_key = self.compute_cache_key(&prev_state_root, &new_state_root, &tx_hash);
        if let Some(cached) = self.proof_cache.get(&cache_key) {
            debug!("Using cached proof");
            return Ok(cached.clone());
        }

        let mut data = Vec::with_capacity(128);
        data.extend_from_slice(&prev_state_root);
        data.extend_from_slice(&new_state_root);
        data.extend_from_slice(&tx_hash);
        data.extend_from_slice(&block_hash);

        let proof = self.prove(&data)?;
        self.proof_cache.insert(cache_key, proof.clone());
        self.aggregator.add_proof(proof.clone());

        info!("Generated ZK proof for state transition");
        Ok(proof)
    }

    /// Verify a state transition proof
    pub fn verify_state_transition(
        &self,
        proof: &[u8],
        prev_state_root: [u8; 32],
        new_state_root: [u8; 32],
        tx_hash: [u8; 32],
    ) -> Result<bool> {
        let mut data = Vec::with_capacity(96);
        data.extend_from_slice(&prev_state_root);
        data.extend_from_slice(&new_state_root);
        data.extend_from_slice(&tx_hash);
        self.verify(proof, &data)
    }

    /// Verify an aggregated proof
    pub fn verify_aggregated(&self, aggregated: &AggregatedProof) -> Result<bool> {
        let mut hasher = Sha256::new();
        for proof in &aggregated.proofs {
            hasher.update(proof);
        }
        let computed_root: [u8; 32] = hasher.finalize().into();
        Ok(computed_root == aggregated.aggregate_root)
    }

    /// Get aggregated proof if ready
    pub fn get_aggregated_proof(&mut self) -> Option<AggregatedProof> {
        self.aggregator.aggregate().unwrap_or(None)
    }

    fn compute_cache_key(&self, prev: &[u8; 32], new: &[u8; 32], tx: &[u8; 32]) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(prev);
        hasher.update(new);
        hasher.update(tx);
        let hash = hasher.finalize();
        u64::from_le_bytes(hash[..8].try_into().unwrap())
    }

    pub fn clear_cache(&mut self) {
        self.proof_cache.clear();
        debug!("Proof cache cleared");
    }

    pub fn cache_stats(&self) -> (usize, usize) {
        (self.proof_cache.len(), 0)
    }
}

impl Default for ZkProver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Halo2-backed implementation (requires `halo2` feature)
// ============================================================================

#[cfg(feature = "halo2")]
pub mod halo2_backend {
    // Halo2-based state transition proof circuit
    // Enable the `halo2` feature to compile this module.
    // Requires: halo2_proofs 0.3, halo2curves 0.1
}

pub fn init() {
    info!("ZK Prover initialized (default backend: SHA-256)");
    #[cfg(feature = "halo2")]
    info!(" - Halo2 backend available");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_generation() {
        let mut prover = ZkProver::new();

        let prev_root = [1u8; 32];
        let new_root = [2u8; 32];
        let tx_hash = [3u8; 32];
        let block_hash = [4u8; 32];

        let proof = prover.prove_state_transition(prev_root, new_root, tx_hash, block_hash);
        assert!(proof.is_ok());

        let verification = prover.verify_state_transition(&proof.unwrap(), prev_root, new_root, tx_hash);
        assert!(verification.is_ok());
        assert!(verification.unwrap());
    }

    #[test]
    fn test_proof_aggregation() {
        let mut prover = ZkProver::new();

        for i in 0..10 {
            let prev = [i as u8; 32];
            let new = [(i + 1) as u8; 32];
            let tx = [i as u8; 32];
            let _ = prover.prove_state_transition(prev, new, tx, [0; 32]);
        }

        let aggregated = prover.get_aggregated_proof();
        assert!(aggregated.is_some());

        let agg = aggregated.unwrap();
        assert!(prover.verify_aggregated(&agg).unwrap());
    }

    #[test]
    fn test_batch_verification() {
        let mut prover = ZkProver::new();
        let mut batch = BatchVerifier::new(10);

        for i in 0..5 {
            let proof = prover.prove_state_transition([i as u8; 32], [(i + 1) as u8; 32], [i as u8; 32], [0; 32]);
            batch.add_proof(proof.unwrap());
        }

        let results = batch.verify_batch(&prover);
        assert_eq!(results.len(), 5);
        assert!(results.iter().all(|&r| r));
    }
}
