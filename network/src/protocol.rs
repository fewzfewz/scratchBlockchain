//! # Block Exchange Protocol Implementation
//! 
//! This module implements the wire protocol for block synchronization between nodes.
//! It uses length-delimited framing to handle persistent connections correctly.
//! 
//! ## Protocol Design
//! - Protocol ID: `/blockchain/sync/1.0.0`
//! - Message format: `[varint length][JSON message]`
//! - Requests: BlockRequest (start_height, limit)
//! - Responses: BlockResponse (Vec<Block>)
//! 
//! ## Why Length-Delimited?
//! Without length prefixes, `read_to_end()` waits for EOF (connection close).
//! With length prefixes, we can read exactly N bytes per message and keep the
//! connection open for multiple exchanges.

use async_trait::async_trait;
use common::types::Block;
use futures::prelude::*;
use libp2p::request_response::Codec;
use serde::{Deserialize, Serialize};
use std::io;
use unsigned_varint::codec::UviBytes;  // For length-delimited framing

/// Protocol identifier for block synchronization
/// 
/// The version (1.0.0) allows future protocol upgrades without breaking compatibility.
/// Different versions can coexist on the network.
#[derive(Debug, Clone)]
pub struct BlockExchangeProtocol();

/// Codec for serializing/deserializing block exchange messages
/// 
/// This codec handles the wire format for block requests and responses.
/// It uses:
/// 1. Length-delimited framing (UviBytes) to handle persistent connections
/// 2. JSON serialization for human-readable debugging
/// 3. Async I/O for non-blocking operations
#[derive(Clone, Default)]
pub struct BlockExchangeCodec {
    /// Length-delimited codec that prefixes each message with its byte length
    /// 
    /// This allows reading exact message sizes without blocking on EOF.
    /// Format: [varint length][message bytes]
    length_codec: UviBytes<Vec<u8>>,
}

/// Request to download blocks from a peer
/// 
/// Peers can request a range of blocks starting from `start_height`.
/// The `limit` prevents DoS attacks by bounding response size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRequest {
    /// Starting block height (inclusive)
    pub start_height: u64,
    
    /// Maximum number of blocks to return (prevents response flooding)
    pub limit: u32,
}

/// Response containing requested blocks
/// 
/// May return fewer blocks than requested if the peer doesn't have them.
/// Empty Vec indicates no blocks available at or after start_height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockResponse {
    /// Blocks in ascending height order (oldest first)
    pub blocks: Vec<Block>,
}

impl AsRef<str> for BlockExchangeProtocol {
    fn as_ref(&self) -> &str {
        "/blockchain/sync/1.0.0"
    }
}

#[async_trait]
impl Codec for BlockExchangeCodec {
    type Protocol = BlockExchangeProtocol;
    type Request = BlockRequest;
    type Response = BlockResponse;

    /// Reads a request from the wire
    /// 
    /// This implementation correctly handles persistent connections by:
    /// 1. First reading the length prefix (varint)
    /// 2. Then reading exactly that many bytes
    /// 3. Deserializing JSON from those bytes
    /// 
    /// # Returns
    /// - `Ok(Request)` - Successfully parsed request
    /// - `Err(io::Error)` - I/O error or malformed data
    async fn read_request<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Step 1: Read length-prefixed message from the stream
        // The length_codec handles varint decoding and buffer management
        let raw_bytes = self.length_codec.read(io).await?;
        
        // Step 2: Deserialize JSON into BlockRequest
        // Using from_slice instead of from_str for efficiency (no UTF-8 validation needed)
        serde_json::from_slice(&raw_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Reads a response from the wire
    /// 
    /// Same as `read_request` but deserializes into BlockResponse.
    async fn read_response<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        // Read length-prefixed message
        let raw_bytes = self.length_codec.read(io).await?;
        
        // Deserialize as BlockResponse
        serde_json::from_slice(&raw_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Writes a request to the wire
    /// 
    /// Serializes the request to JSON, then writes with length prefix.
    /// The length prefix allows the receiver to know exactly how many bytes to read.
    async fn write_request<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Step 1: Serialize request to JSON bytes
        let json_bytes = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        // Step 2: Write length prefix + JSON bytes
        // UviBytes encodes the length as varint before the payload
        self.length_codec.write(json_bytes, io).await
    }

    /// Writes a response to the wire
    /// 
    /// Same as `write_request` but for responses.
    async fn write_response<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        // Serialize response to JSON bytes
        let json_bytes = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        
        // Write length prefix + JSON bytes
        self.length_codec.write(json_bytes, io).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::io::Cursor;
    
    /// Helper to create a test block (simplified for unit tests)
    fn create_test_block(height: u64) -> Block {
        use common::types::{Block, Header};
        Block {
            header: Header {
                parent_hash: [0u8; 32],
                slot: height,
                state_root: [0u8; 32],
                extrinsics_root: [0u8; 32],
                epoch: 0,
                validator_set_id: 0,
                signature: vec![],
                gas_used: 0,
                base_fee: 0,
            },
            extrinsics: vec![],
        }
    }
    
    #[tokio::test]
    async fn test_request_response_roundtrip() {
        // Create codec
        let mut codec = BlockExchangeCodec::default();
        let protocol = BlockExchangeProtocol();
        
        // Create a request
        let original_request = BlockRequest {
            start_height: 100,
            limit: 50,
        };
        
        // Write to a buffer
        let mut write_buffer = Vec::new();
        let write_cursor = Cursor::new(&mut write_buffer);
        codec.write_request(&protocol, write_cursor, original_request.clone())
            .await
            .unwrap();
        
        // Read back from buffer
        let read_cursor = Cursor::new(&write_buffer);
        let decoded_request = codec.read_request(&protocol, read_cursor)
            .await
            .unwrap();
        
        // Verify roundtrip
        assert_eq!(original_request, decoded_request);
    }
    
    #[tokio::test]
    async fn test_multiple_messages_on_same_connection() {
        let mut codec = BlockExchangeCodec::default();
        let protocol = BlockExchangeProtocol();
        
        // Write two requests to the same buffer
        let mut write_buffer = Vec::new();
        let write_cursor = Cursor::new(&mut write_buffer);
        
        let req1 = BlockRequest { start_height: 1, limit: 10 };
        let req2 = BlockRequest { start_height: 11, limit: 10 };
        
        codec.write_request(&protocol, write_cursor, req1.clone()).await.unwrap();
        codec.write_request(&protocol, write_cursor, req2.clone()).await.unwrap();
        
        // Read both back (this would hang with read_to_end!)
        let read_cursor = Cursor::new(&write_buffer);
        let decoded1 = codec.read_request(&protocol, read_cursor).await.unwrap();
        let decoded2 = codec.read_request(&protocol, read_cursor).await.unwrap();
        
        assert_eq!(req1, decoded1);
        assert_eq!(req2, decoded2);
    }
    
    #[tokio::test]
    async fn test_response_roundtrip() {
        let mut codec = BlockExchangeCodec::default();
        let protocol = BlockExchangeProtocol();
        
        let original_response = BlockResponse {
            blocks: vec![
                create_test_block(100),
                create_test_block(101),
            ],
        };
        
        // Write response
        let mut write_buffer = Vec::new();
        let write_cursor = Cursor::new(&mut write_buffer);
        codec.write_response(&protocol, write_cursor, original_response.clone())
            .await
            .unwrap();
        
        // Read response
        let read_cursor = Cursor::new(&write_buffer);
        let decoded_response = codec.read_response(&protocol, read_cursor)
            .await
            .unwrap();
        
        assert_eq!(original_response.blocks.len(), decoded_response.blocks.len());
    }
    
    #[tokio::test]
    async fn test_invalid_json_returns_error() {
        let mut codec = BlockExchangeCodec::default();
        let protocol = BlockExchangeProtocol();
        
        // Write invalid JSON (just random bytes)
        let mut write_buffer = Vec::new();
        // Write length prefix manually since UviBytes would add correct length
        let invalid_data = b"not valid json";
        let mut varint_buf = unsigned_varint::encode::usize_buffer();
        let encoded_len = unsigned_varint::encode::usize(invalid_data.len(), &mut varint_buf);
        write_buffer.extend_from_slice(encoded_len);
        write_buffer.extend_from_slice(invalid_data);
        
        let read_cursor = Cursor::new(&write_buffer);
        let result = codec.read_request(&protocol, read_cursor).await;
        
        assert!(result.is_err(), "Invalid JSON should cause error");
        assert!(result.unwrap_err().kind() == io::ErrorKind::InvalidData);
    }
}