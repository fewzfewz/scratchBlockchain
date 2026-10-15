//! Incremental commitment to the flat application state.
//!
//! An LtHash ("lattice hash", as used by Solana's accounts hash): each
//! `(key, value)` pair is expanded to 1024 16-bit lanes, and the state's
//! commitment is the lane-wise sum (mod 2^16) over all pairs. Writing a key
//! subtracts the old pair and adds the new one, so a block costs O(changed
//! keys) instead of re-hashing the whole state, and the result does not
//! depend on write order. Finding two states with the same sum is a lattice
//! (short integer solution) problem at this size, unlike a plain 256-bit sum.
//!
//! It proves nothing about individual keys — there are no inclusion proofs —
//! but every node computing the same commitment means every node holds the
//! same state.

use sha2::{Digest, Sha256};

pub const LANES: usize = 1024;
const DOMAIN: &[u8] = b"NEBULA_LTHASH_V1";

#[derive(Clone, PartialEq, Eq)]
pub struct LtHash(pub Box<[u16; LANES]>);

impl std::fmt::Debug for LtHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LtHash({})", hex_prefix(&self.checksum()))
    }
}

fn hex_prefix(bytes: &[u8]) -> String {
    bytes.iter().take(8).map(|b| format!("{:02x}", b)).collect()
}

impl Default for LtHash {
    fn default() -> Self {
        Self::identity()
    }
}

impl LtHash {
    /// Commitment to the empty state.
    pub fn identity() -> Self {
        Self(Box::new([0u16; LANES]))
    }

    /// Expand one key/value pair to lanes.
    fn element(key: &[u8], value: &[u8]) -> Box<[u16; LANES]> {
        let mut seed = Sha256::new();
        seed.update(DOMAIN);
        seed.update((key.len() as u64).to_le_bytes());
        seed.update(key);
        seed.update(value);
        let seed: [u8; 32] = seed.finalize().into();

        let mut lanes = Box::new([0u16; LANES]);
        // Each SHA-256 output fills 16 lanes; 64 blocks fill all 1024.
        for block in 0..(LANES / 16) {
            let mut h = Sha256::new();
            h.update(seed);
            h.update((block as u32).to_le_bytes());
            let out: [u8; 32] = h.finalize().into();
            for i in 0..16 {
                lanes[block * 16 + i] = u16::from_le_bytes([out[2 * i], out[2 * i + 1]]);
            }
        }
        lanes
    }

    pub fn add(&mut self, key: &[u8], value: &[u8]) {
        let e = Self::element(key, value);
        for (lane, x) in self.0.iter_mut().zip(e.iter()) {
            *lane = lane.wrapping_add(*x);
        }
    }

    pub fn remove(&mut self, key: &[u8], value: &[u8]) {
        let e = Self::element(key, value);
        for (lane, x) in self.0.iter_mut().zip(e.iter()) {
            *lane = lane.wrapping_sub(*x);
        }
    }

    /// 32-byte digest of the lanes.
    pub fn checksum(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.to_bytes());
        h.finalize().into()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.iter().flat_map(|l| l.to_le_bytes()).collect()
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != LANES * 2 {
            return None;
        }
        let mut lanes = Box::new([0u16; LANES]);
        for (i, lane) in lanes.iter_mut().enumerate() {
            *lane = u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]);
        }
        Some(Self(lanes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_independent_and_reversible() {
        let mut a = LtHash::identity();
        a.add(b"k1", b"v1");
        a.add(b"k2", b"v2");
        let mut b = LtHash::identity();
        b.add(b"k2", b"v2");
        b.add(b"k1", b"v1");
        assert_eq!(a, b);

        // Updating k1 then reverting returns to the same commitment.
        let before = a.checksum();
        a.remove(b"k1", b"v1");
        a.add(b"k1", b"changed");
        assert_ne!(a.checksum(), before);
        a.remove(b"k1", b"changed");
        a.add(b"k1", b"v1");
        assert_eq!(a.checksum(), before);

        a.remove(b"k1", b"v1");
        a.remove(b"k2", b"v2");
        assert_eq!(a, LtHash::identity());
    }

    #[test]
    fn key_value_boundary_is_unambiguous() {
        let mut a = LtHash::identity();
        a.add(b"ab", b"c");
        let mut b = LtHash::identity();
        b.add(b"a", b"bc");
        assert_ne!(a, b);
    }

    #[test]
    fn bytes_roundtrip() {
        let mut a = LtHash::identity();
        a.add(b"k", b"v");
        assert_eq!(LtHash::from_bytes(&a.to_bytes()).unwrap(), a);
    }
}
