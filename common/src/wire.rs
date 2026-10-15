//! Peer-to-peer wire encoding: MessagePack with named fields.
//!
//! Gossip and block sync use this instead of JSON. Byte arrays and hashes
//! cost one or two bytes per byte instead of up to four in JSON, and
//! encoding/decoding skips text formatting. Field names are kept, so
//! `#[serde(default, skip_serializing_if)]` fields stay compatible as types
//! evolve. The RPC API is unaffected and stays JSON.
//!
//! The block size limit (`types::MAX_BLOCK_BYTES`) is measured in this
//! encoding, since it bounds what fits in one gossip message.

use serde::{de::DeserializeOwned, Serialize};

pub fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, String> {
    rmp_serde::to_vec_named(value).map_err(|e| e.to_string())
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    rmp_serde::from_slice(bytes).map_err(|e| e.to_string())
}

/// Size of `value` on the wire; `usize::MAX` if it cannot be encoded.
pub fn encoded_len<T: Serialize + ?Sized>(value: &T) -> usize {
    encode(value).map(|b| b.len()).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Block, Header, Transaction};

    fn sample_block() -> Block {
        let mut txs = Vec::new();
        for i in 0..50u64 {
            let mut tx = Transaction::simple_transfer([1u8; 20], [2u8; 20], 1_000_000 + i, i, 1);
            tx.chain_id = Some(1337);
            tx.public_key = [9u8; 32];
            tx.signature = vec![0xab; 64];
            txs.push(tx);
        }
        Block::new(Header::new([3u8; 32], 7), txs)
    }

    #[test]
    fn block_round_trips() {
        let block = sample_block();
        let bytes = encode(&block).unwrap();
        let back: Block = decode(&bytes).unwrap();
        assert_eq!(back, block);
        assert_eq!(back.hash(), block.hash());
    }

    #[test]
    fn smaller_than_json() {
        let block = sample_block();
        let json = serde_json::to_vec(&block).unwrap().len();
        let wire = encoded_len(&block);
        assert!(wire * 3 < json * 2, "wire {} vs json {}", wire, json);
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode::<Block>(&[0xc1, 0x00, 0xff]).is_err());
    }
}
