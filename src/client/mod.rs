pub mod player;

use crate::server::util::{NetWriter, into_framed_split};
use tokio::net::tcp::OwnedWriteHalf;
use crate::server::net_proto::{Input, InputAction, HubAction, PlayerAction, Output, OutputError};
use tokio::net::{TcpStream, ToSocketAddrs};
use tap::prelude::Pipe;
use crate::server::{PeerId, SessionId, Res, OK};
use futures::{SinkExt, TryStreamExt};
use flume::{unbounded, Receiver, Sender};
use tokio::time::{Duration, interval};
use tokio::select;
use log::*;
use crate::client::player::PlayerManager;


const ALIVE_TICK: u64 = 100;

#[derive(Debug)]
pub enum ProxyMessage {
    ServerMessage(Output),
    Alive,
    CreateSession(String),
    JoinSession(SessionId),
    PauseSession,
    StartSession,
    StopSession,
    PlaySession,
}

pub async fn create_client(ip: impl ToSocketAddrs) -> anyhow::Result<Client> {
    let (mut rs, mut ws) =
        TcpStream::connect(ip).await?.pipe(|s| into_framed_split(s));
    let (tx, rx) = unbounded();

    // Handle connection to the sever
    ws.send(Input { from: None, action: InputAction::Connect }).await?;
    let mut proxy_client = if let Output::Connected(user_id) = rs.try_next().await?.unwrap() {
        info!("Connected id: {}", user_id);
        ClientProxy::new(user_id, ws, rx)
    } else {
        todo!("Error protocol received another message")
    };

    // Event loop from the server
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(ALIVE_TICK));
            loop {
                let tick_fut = interval.tick();
                let rec_message_fut = rs.try_next();
                select! {
                    res_message = rec_message_fut => {
                        let message = res_message.unwrap().unwrap();
                        debug!("Received message from the server: {:?}", message);
                        tx.send_async(ProxyMessage::ServerMessage(message)).await.unwrap();
                    }
                    _ = tick_fut => {
                        debug!("Alive tick sent");
                        tx.send_async(ProxyMessage::Alive).await.unwrap();
                    }
                }
            }
        });
    }
    tokio::spawn(async move {
        let e = proxy_client.run().await.unwrap_err();
        error!("Error in the proxy: {}", e)
    });
    Ok(Client(tx))
}

pub struct Client(Sender<ProxyMessage>);

impl Client {
    pub fn join_session(&self) -> Res {
        unimplemented!()
    }

    pub async fn create_session(&self, password: String) {
        self.0.send_async(ProxyMessage::CreateSession(password)).await.unwrap();
    }

    pub async fn start_session(&self) {
        self.0.send_async(ProxyMessage::StartSession).await.unwrap();
    }

    pub async fn pause(&self) {
        self.0.send_async(ProxyMessage::PauseSession).await.unwrap();
    }

    pub async fn play(&self) {
        self.0.send_async(ProxyMessage::PlaySession).await.unwrap();
    }

    pub async fn stop(&self) {
        self.0.send_async(ProxyMessage::StopSession).await.unwrap();
    }
}

pub struct ClientProxy {
    user_id: PeerId,
    writer: NetWriter<Input, OwnedWriteHalf>,
    client_rx: Receiver<ProxyMessage>,
    player_manager: PlayerManager,
}

impl ClientProxy {
    pub async fn run(&mut self) -> Res {
        loop {
            let message = self.client_rx.recv_async().await?;
            debug!("Proxy handling message: {:?}", &message);
            self.handle(message).await?;
        }
    }

    async fn handle(&mut self, message: ProxyMessage) -> Res {
        match message {
            ProxyMessage::ServerMessage(message) => {
                self.handle_server_message(message)
            }
            ProxyMessage::Alive => {
                self.alive().await
            }
            ProxyMessage::CreateSession(password) => {
                self.create_session(password).await
            }
            ProxyMessage::JoinSession(_) => {
                self.join_session().await
            }
            ProxyMessage::StartSession => {
                self.start().await
            }
            ProxyMessage::StopSession => {
                self.stop().await
            }
            ProxyMessage::PlaySession => {
                self.play().await
            }
            ProxyMessage::PauseSession => {
                self.pause().await
            }
        }
    }

    pub fn handle_error(&self, error: OutputError) {
        error!("Error received from the server {:?}", error);
    }

    pub fn new(id: PeerId, writer: NetWriter<Input, OwnedWriteHalf>, rec: Receiver<ProxyMessage>) -> Self {
        Self { writer, user_id: id, client_rx: rec, player_manager: PlayerManager::new() }
    }

    fn handle_server_message(&self, message: Output) -> Res {
        match message {
            Output::Connected(_) => { todo!("Bizarre") }
            Output::Joined => { todo!() }
            Output::Timestamp(timestamp) => {
                trace!("timestamp: received {}", timestamp);
                OK
            }
            Output::World(_) => { todo!() }
            Output::Error(e) => {
                self.handle_error(e);
                OK
            }
            Output::PlayerAction(action) => {
                match action {
                    PlayerAction::Play => self.player_manager.play(),
                    PlayerAction::Pause => self.player_manager.pause(),
                    PlayerAction::Stop => self.player_manager.stop(),
                }
            }
        }
    }


    pub async fn join_session(&mut self) -> Res {
        unimplemented!()
    }

    pub async fn create_session(&mut self, password: String) -> Res<()> {
        self.writer.send(Input { from: Some(self.user_id), action: InputAction::HubAction(HubAction::CreateSession(password)) }).await?;
        Ok(())
    }

    pub async fn start(&mut self) -> Res<()> {
        self.writer.send(Input { from: Some(self.user_id), action: InputAction::HubAction(HubAction::SessionStart) }).await?;
        Ok(())
    }

    pub async fn pause(&mut self) -> Res<()> {
        self.writer.send(Input { from: Some(self.user_id), action: InputAction::SessionAction(PlayerAction::Pause) }).await?;
        Ok(())
    }

    pub async fn play(&mut self) -> Res<()> {
        self.writer.send(Input { from: Some(self.user_id), action: InputAction::SessionAction(PlayerAction::Play) }).await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Res<()> {
        unimplemented!()
    }

    pub async fn alive(&mut self) -> Res<()> {
        self.writer.send(Input { from: Some(self.user_id), action: InputAction::Alive }).await?;
        Ok(())
    }
}
