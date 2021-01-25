
use flume::Sender;
use futures::TryStreamExt;
use log::*;
use tokio::net::tcp::OwnedReadHalf;
use tokio::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use util::NetReader;

use crate::server::actor_proto::{HubMessage, SessionMessage};
use crate::server::net_proto::{Input, InputAction, Output};

pub mod net_proto;
pub mod util;
pub mod peer;
pub mod session;
pub mod actor_proto;
pub mod hub;


const REFRESH_TICK: u64 = 40;

pub type PeerMessage = Output;

pub type PeerTransmitter = Sender<PeerMessage>;
pub type SessionTransmitter = Sender<SessionMessage>;
pub type HubTransmitter = Sender<HubMessage>;

pub type Res<T=()> = anyhow::Result<T>;
pub const OK: Res = Ok(());

pub type SessionId = Uuid;
pub type PeerId = Uuid;

pub struct PeerEventReader {
    pub net_reader: NetReader<Input, OwnedReadHalf>,
    pub hub_tx: HubTransmitter,
}

impl PeerEventReader {
    pub async fn run(&mut self, duration: Duration, peer_id: PeerId) -> Res {
        loop {
            if let Some(input) = timeout(duration, self.net_reader.try_next()).await?? {
                // We ignore alive packets they serve only to reset timeout
                debug!("Event reader of peer_id: {} received: {:?}", peer_id, input);
                if input.action != InputAction::Alive {
                    assert_eq!(input.from.unwrap(), peer_id);
                    self.hub_tx.send_async(HubMessage::NetInput(input)).await?
                } else {
                    trace!("Alive received from : {}", peer_id)
                }
            }
        }
    }
}


