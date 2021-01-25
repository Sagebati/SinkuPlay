use std::collections::HashMap;

use flume::{Receiver, Sender, unbounded};
use log::*;
use tap::pipe::Pipe;
use tokio::runtime::Handle;
use tokio::select;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use uuid::Uuid;

use crate::server::{PeerId, SessionId, Res, OK};
use crate::server::actor_proto::SessionMessage;
use crate::server::net_proto::{PlayerAction};
use crate::server::Output;
use crate::server::peer::Peer;
use crate::server::session::State::Started;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStatus {
    Playing,
    Paused,
    Interrupted,
}

pub struct SessionProxy {
    participants: HashMap<PeerId, Peer>,
    receiver: Receiver<SessionMessage>,
    status: SessionStatus,
}

impl SessionProxy {
    pub fn run(&mut self, tick: Duration, id: SessionId) -> Res {
        info!("Session: {}, started", id);
        let mut interval = tokio::time::interval(tick);
        loop {
            let message_fut = self.receiver.recv()?;
            let message_to_send = match message_fut {
                SessionMessage::Play => {

                }
                SessionMessage::Pause => {}
                SessionMessage::Stop => {}
            }
            match self.status {
                SessionStatus::Paused => message_fut.await?.pipe(|m| self.handle(id, m)),
                SessionStatus::Playing => {
                    select! {
                        message = message_fut => {
                            self.handle(id, message?);
                        },
                        _ = tick => {
                            for (key, peer) in &self.participants {
                                trace!("Send refresh tick to {}", key);
                                peer.proxy_tx.send_async(Output::Timestamp(64)).await?;
                            }
                        }
                    }
                }
                SessionStatus::Interrupted => {
                    info!("Session {} interrupted", id);
                    break Ok(());
                }
            }
        }
    }

    pub fn handle(&mut self, id: SessionId, message: SessionMessage) {
        info!("Session {}, transited to {:?}", id, message);
        match message {
            SessionMessage::Play => {
                self.status = SessionStatus::Playing;
            }
            SessionMessage::Pause => {
                self.status = SessionStatus::Paused;
            }
            SessionMessage::Stop => {
                self.status = SessionStatus::Interrupted;
            }
        }
    }
}

#[derive(Debug)]
enum State {
    Started(Sender<SessionMessage>, JoinHandle<()>, HashMap<PeerId, Peer>),
    Waiting(HashMap<PeerId, Peer>),
}

#[derive(Debug)]
pub struct Session {
    id: SessionId,
    password: String,
    media: String,
    name: String,
    owner: PeerId,
    state: State,
}

impl Session {
    pub fn new(owner: PeerId, name: String, mdp: String) -> Self {
        let proxy = Session {
            id: Uuid::new_v4(),
            password: mdp.into(),
            media: String::new(),
            name,
            owner,
            state: State::Waiting(HashMap::new()),
        };
        proxy
    }

    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn media(&self) -> &str {
        &self.media
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn owner(&self) -> PeerId { self.owner }

    pub fn set_media(&mut self, media: String) {
        self.media = media;
    }

    pub fn set_password(&mut self, pwd: String) {
        self.password = pwd;
    }

    pub fn add_peer(&mut self, peer: Peer) {
        match self.state {
            State::Started(_, _, _) => {
                panic!("Invalid state");
            }
            State::Waiting(ref mut participants) => {
                participants.insert(peer.id, peer);
            }
        }
    }
    pub fn rm_peer(&mut self, peer_id: PeerId) -> Peer {
        match self.state {
            State::Started(_, _, _) => panic!("Invalid state, no modifications"),
            State::Waiting(ref mut participants) => participants.remove(&peer_id).unwrap()
        }
    }

    pub fn contains_peer(&self, _peer_id: PeerId) -> bool {
        todo!("")
    }

    pub fn is_empty(&self) -> bool {
        todo!("")
    }

    pub fn start(&mut self, refresh_tick: Duration, handle_runtime: &Handle) {
        self.state = match &self.state {
            State::Started(_, _, _) => panic!("Invalid state, session already started"),
            State::Waiting(participants) => {
                let (tx, rx) = unbounded();
                let handle = {
                    let participants = participants.clone();
                    let id = self.id;
                    handle_runtime.spawn(async move {
                        let mut session = SessionProxy {
                            participants,
                            receiver: rx,
                            status: SessionStatus::Paused,
                        };
                        let e = session.run(refresh_tick, id).await.unwrap_err();
                        warn!("Session {} encountered an error: {}", id, e);
                    })
                };
                Started(tx, handle, participants.clone())
            }
        };
    }

    pub fn handle_action(&mut self, action: PlayerAction) -> Res {
        if let Started(sender, _, _) = &mut self.state {
            match action {
                PlayerAction::Play => {
                    sender.send(SessionMessage::Play)?;
                    OK
                }
                PlayerAction::Pause => {
                    sender.send(SessionMessage::Pause)?;
                    OK
                }
                PlayerAction::Stop => {
                    sender.send(SessionMessage::Stop)?;
                    OK
                }
            }
        }else {
            Err(anyhow::anyhow!("Session is not started, cannot perform operations"))
        }
    }

    pub fn participants(&self) -> impl Iterator<Item=&Peer> {
        match &self.state {
            Started(_, _, participants) => {
                participants
            }
            State::Waiting(participants) => {
                participants
            }
        }.iter().map(|(_k, v)| v)
    }

    pub fn stop(&mut self) {
        todo!()
    }
}


