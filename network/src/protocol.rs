use async_trait::async_trait;
use common::types::{Block, Hash};
use futures::prelude::*;
use libp2p::request_response::Codec;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone)]
pub struct BlockExchangeProtocol();

#[derive(Clone, Default)]
pub struct BlockExchangeCodec();

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
        _: &BlockExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        io.read_to_end(&mut vec).await?;
        serde_json::from_slice(&vec).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn read_response<T>(
        &mut self,
        _: &BlockExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut vec = Vec::new();
        io.read_to_end(&mut vec).await?;
        serde_json::from_slice(&vec).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    async fn write_request<T>(
        &mut self,
        _: &BlockExchangeProtocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await
    }

    async fn write_response<T>(
        &mut self,
        _: &BlockExchangeProtocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        io.write_all(&data).await
    }
}
