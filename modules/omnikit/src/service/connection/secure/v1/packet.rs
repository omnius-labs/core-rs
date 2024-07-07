use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tokio_util::bytes::Bytes;

use crate::connection::framed::{FramedRecv, FramedSend};

#[async_trait]
pub trait FramedRecvExt: FramedRecv {
    async fn recv_message<T: DeserializeOwned>(&mut self) -> anyhow::Result<T>;
}

#[async_trait]
impl<T: FramedRecv> FramedRecvExt for T
where
    T: ?Sized + Send + Unpin,
{
    async fn recv_message<TItem: DeserializeOwned>(&mut self) -> anyhow::Result<TItem> {
        let b = self.recv().await?;
        let item = Packet::deserialize(b)?;
        Ok(item)
    }
}

#[async_trait]
pub trait FramedSendExt: FramedSend {
    async fn send_message<T: Serialize + Send>(&mut self, item: T) -> anyhow::Result<()>;
}

#[async_trait]
impl<T: FramedSend> FramedSendExt for T
where
    T: ?Sized + Send + Unpin,
{
    async fn send_message<TItem: Serialize + Send>(&mut self, item: TItem) -> anyhow::Result<()> {
        let b = Packet::serialize(item)?;
        self.send(b).await?;
        Ok(())
    }
}

enum PacketType {
    #[allow(unused)]
    Unknown = 0,
    V1 = 1,
}

impl From<u8> for PacketType {
    fn from(value: u8) -> Self {
        match value {
            1 => PacketType::V1,
            _ => PacketType::Unknown,
        }
    }
}

pub struct Packet;

impl Packet {
    pub fn serialize<T: Serialize>(item: T) -> anyhow::Result<Bytes> {
        let buf = serde_json::to_vec(&item)?;
        Ok(Bytes::from(buf))
    }

    pub fn deserialize<T: DeserializeOwned>(buf: Bytes) -> anyhow::Result<T> {
        let item = serde_json::from_slice(&buf)?;
        Ok(item)
    }
}
