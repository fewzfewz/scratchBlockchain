use async_trait::async_trait;
use common::types::Block;
use futures::prelude::*;
use libp2p::request_response::Codec;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone)]
pub struct BlockExchangeProtocol();

#[derive(Clone, Default)]
pub struct BlockExchangeCodec;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRequest {
    pub start_height: u64,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockResponse {
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

    async fn read_request<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let raw_bytes = read_length_prefixed(io).await?;
        serde_json::from_slice(&raw_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let raw_bytes = read_length_prefixed(io).await?;
        serde_json::from_slice(&raw_bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let json_bytes =
            serde_json::to_vec(&req).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_length_prefixed(io, &json_bytes).await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &BlockExchangeProtocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let json_bytes =
            serde_json::to_vec(&res).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_length_prefixed(io, &json_bytes).await
    }
}

async fn read_length_prefixed<T>(io: &mut T) -> io::Result<Vec<u8>>
where
    T: AsyncRead + Unpin,
{
    let len = read_varint(io).await?;
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_length_prefixed<T>(io: &mut T, data: &[u8]) -> io::Result<()>
where
    T: AsyncWrite + Unpin,
{
    write_varint(io, data.len()).await?;
    io.write_all(data).await?;
    Ok(())
}

async fn read_varint<T>(io: &mut T) -> io::Result<usize>
where
    T: AsyncRead + Unpin,
{
    use unsigned_varint::decode;

    let mut buf = [0u8; 10];
    for i in 0..buf.len() {
        io.read_exact(&mut buf[i..i + 1]).await?;
        if buf[i] & 0x80 == 0 {
            let (value, _) = decode::usize(&buf[..=i])
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            return Ok(value);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "varint too long",
    ))
}

async fn write_varint<T>(io: &mut T, value: usize) -> io::Result<()>
where
    T: AsyncWrite + Unpin,
{
    use unsigned_varint::encode;

    let mut buf = encode::usize_buffer();
    let encoded = encode::usize(value, &mut buf);
    io.write_all(encoded).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::io::Cursor;

    fn create_test_block(height: u64) -> Block {
        use common::types::Header;
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
        let mut codec = BlockExchangeCodec::default();
        let protocol = BlockExchangeProtocol();

        let original_request = BlockRequest {
            start_height: 100,
            limit: 50,
        };

        let mut write_buffer = Vec::new();
        let write_cursor = Cursor::new(&mut write_buffer);
        codec
            .write_request(&protocol, write_cursor, original_request.clone())
            .await
            .unwrap();

        let read_cursor = Cursor::new(&write_buffer);
        let decoded_request = codec.read_request(&protocol, read_cursor).await.unwrap();

        assert_eq!(original_request, decoded_request);
    }

    #[tokio::test]
    async fn test_multiple_messages_on_same_connection() {
        let mut codec = BlockExchangeCodec::default();
        let protocol = BlockExchangeProtocol();

        let mut write_buffer = Vec::new();
        let write_cursor = Cursor::new(&mut write_buffer);

        let req1 = BlockRequest {
            start_height: 1,
            limit: 10,
        };
        let req2 = BlockRequest {
            start_height: 11,
            limit: 10,
        };

        codec
            .write_request(&protocol, write_cursor, req1.clone())
            .await
            .unwrap();
        codec
            .write_request(&protocol, write_cursor, req2.clone())
            .await
            .unwrap();

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
            blocks: vec![create_test_block(100), create_test_block(101)],
        };

        let mut write_buffer = Vec::new();
        let write_cursor = Cursor::new(&mut write_buffer);
        codec
            .write_response(&protocol, write_cursor, original_response.clone())
            .await
            .unwrap();

        let read_cursor = Cursor::new(&write_buffer);
        let decoded_response = codec.read_response(&protocol, read_cursor).await.unwrap();

        assert_eq!(
            original_response.blocks.len(),
            decoded_response.blocks.len()
        );
    }

    #[tokio::test]
    async fn test_invalid_json_returns_error() {
        let mut codec = BlockExchangeCodec::default();
        let protocol = BlockExchangeProtocol();

        let mut write_buffer = Vec::new();
        let invalid_data = b"not valid json";
        write_varint(&mut write_buffer, invalid_data.len())
            .await
            .unwrap();
        use futures::io::AsyncWriteExt;
        write_buffer.write_all(invalid_data).await.unwrap();

        let read_cursor = Cursor::new(&write_buffer);
        let result = codec.read_request(&protocol, read_cursor).await;

        assert!(result.is_err(), "Invalid JSON should cause error");
        assert!(result.unwrap_err().kind() == io::ErrorKind::InvalidData);
    }
}
