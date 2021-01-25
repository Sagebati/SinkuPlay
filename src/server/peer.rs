use log::*;
use tokio::net::tcp::{OwnedWriteHalf};
use uuid::Uuid;

use crate::server::util::NetWriter;
use crate::server::net_proto::{Output};
use crate::server::{HubTransmitter, PeerMessage, PeerTransmitter, Res, OK};
use futures::{SinkExt};
use flume::Receiver;


#[derive(Debug, Clone)]
pub struct Peer {
    pub id: Uuid,
    pub pseudo: String,
    pub proxy_tx: PeerTransmitter,
}

impl Peer {
    pub fn send(&self, message: PeerMessage) -> Res {
        self.proxy_tx.send(message)?;
        OK
    }

    pub async fn send_async(&self, message: PeerMessage) -> Res {
        self.proxy_tx.send_async(message).await?;
        OK
    }
}

#[derive(Debug)]
pub struct PeerProxy {
    hub_tx: HubTransmitter,
    receiver: Receiver<PeerMessage>,
    net_writer: NetWriter<Output, OwnedWriteHalf>,
}

impl PeerProxy {
    pub fn new(hub_t: HubTransmitter, receiver: Receiver<PeerMessage>, net_writer: NetWriter<Output, OwnedWriteHalf>) -> Self {
        Self {
            receiver,
            hub_tx: hub_t,
            net_writer,
        }
    }

    pub async fn run(&mut self) -> Res {
        loop {
            let m = self.receiver.recv_async().await?;
            debug!("Received {:?}", &m);
            self.handler(m).await?;
        }
    }

    pub async fn handler(&mut self, message: PeerMessage) -> Res {
        Ok(self.net_writer.send(message).await?)
    }
}

