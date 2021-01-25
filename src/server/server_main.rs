use flume::{unbounded};
use futures::TryStreamExt;
use log::*;
use simple_logger::SimpleLogger;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Duration;

use syncplay::server::*;
use syncplay::server::actor_proto::{LogInAction, HubMessage};
use syncplay::server::actor_proto::LogInAction::Connected;
use syncplay::server::util::into_framed_split;
use syncplay::server::net_proto::{Input, Output};
use syncplay::server::net_proto::InputAction::Connect;
use syncplay::server::peer::{PeerProxy, Peer};
use syncplay::server::hub::Hub;
use tokio::runtime::Handle;
use anyhow::Context;
use syncplay::ignore;

const TIMEOUT: u64 = 20000;

#[tokio::main]
async fn main() -> Res {
    SimpleLogger::new().with_level(LevelFilter::Info).init().unwrap();

    let (hub_t, rx) = flume::unbounded();
    let listener = TcpListener::bind("127.0.0.1:5135").await?;
    // Spawn the hub
    let handle = Handle::current();
    std::thread::spawn(move || {
        Hub::new(rx, handle).run();
    });
    loop {
        let (socket, addr) = listener.accept().await?;
        info!("Connection accepted from : {}", addr);
        let hub_t = hub_t.clone();
        tokio::spawn(async move {
            match handle_first_connection(socket, hub_t).await {
                Ok(_) => ignore(),
                Err(e) => error!("Error handling first connection: {}, from {}", e, addr)
            }
        });
    }
}


async fn handle_first_connection(stream: TcpStream, hub_t: HubTransmitter) -> Res<Peer> {
    let (mut rs, ws) = into_framed_split::<Input, Output>(stream);

    if rs.try_next().await? == Some(Input { from: None, action: Connect }) {
        let (peer_t, peer_r) = unbounded();

        let peer_id = PeerId::new_v4();
        let peer = Peer {
            id: peer_id,
            pseudo: "".to_string(), // Not handled yet
            proxy_tx: peer_t,
        };

        let peer_proxy_handle = {
            let hub_t = hub_t.clone();
            tokio::spawn(async move {
                let error = PeerProxy::new(hub_t.clone(), peer_r, ws).run().await.unwrap_err();
                info!("Peer: {} reader stopped, {}", peer_id, error);
            })
        };

        hub_t.send_async(HubMessage::LogInAction(Connected(peer.clone()))).await?;
        info!("User logged, id created: {}", peer_id);
        let mut event_loop = PeerEventReader {
            net_reader: rs,
            hub_tx: hub_t.clone(),
        };
        tokio::spawn(async move {
            trace!("Starting user: {} event loop", peer_id);
            {
                let e = event_loop.run(Duration::from_millis(TIMEOUT), peer_id).await.unwrap_err();
                info!("Connection to {} closed, error: {}", peer_id, e);
            }
            event_loop.hub_tx.send_async(HubMessage::LogInAction(LogInAction::Disconnect(peer_id))).await.context("Couldn't send to the hub").unwrap();
            peer_proxy_handle.abort();
        });
        trace!("user : {}, event loop started", peer_id);
        Ok(peer)
    } else {
        todo!("return error protocol")
    }
}