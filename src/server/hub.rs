use std::collections::HashMap;

use flume::Receiver;
use log::*;
use tap::prelude::Pipe;
use tokio::time::Duration;

use crate::server::{PeerId, REFRESH_TICK, SessionId, Res, OK};
use crate::server::actor_proto::{HubMessage, LogInAction};
use crate::server::net_proto::{HubAction, Input, InputAction, Output, PlayerAction, HubState, SessionDTO, PeerDTO, OutputError};
use crate::server::peer::Peer;
use crate::server::session::Session;
use tokio::runtime::Handle;
use crate::ignore;
use anyhow::Context;

#[derive(Debug, Clone)]
pub enum PeerStatus {
    InSession(SessionId),
    Idle,
}

pub struct Hub {
    sessions: HashMap<SessionId, Session>,
    connected: HashMap<PeerId, (Peer, PeerStatus)>,
    r_messages: Receiver<HubMessage>,
    handle_runtime: Handle,
}

impl Hub {
    pub fn new(rx: Receiver<HubMessage>, handle_runtime: Handle) -> Self {
        Hub {
            sessions: HashMap::new(),
            connected: HashMap::new(),
            r_messages: rx,
            handle_runtime,
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.r_messages.recv() {
                Ok(message) => {
                    debug!("Hub received {:?}", message);
                    match self.handle(message.clone()) {
                        Ok(_) => ignore(),
                        Err(e) => error!("Error handling message {:?}, error: {}", message, e)
                    }
                }
                Err(e) => {
                    error!("Error occurred in hub: {:?}", e);
                }
            }
        }
    }

    pub fn start_session(&mut self, user_id: PeerId) -> Res {
        let (_, status) = self.connected.get(&user_id).context("The user is not connected and attempts to start a session")?;
        if let PeerStatus::InSession(session_id) = status {
            let session = self.sessions.get_mut(session_id).unwrap();
            session.start(Duration::from_millis(REFRESH_TICK), &self.handle_runtime);
            OK
        } else {
            panic!("The user is not in a session");
        }
    }

    pub fn create_session(&mut self, user_id: PeerId, name: String, password: String) -> SessionId {
        let new_session = Session::new(user_id, name, password);
        let session_id = new_session.id();
        self.sessions.insert(session_id, new_session);
        session_id
    }

    pub fn join_session(&mut self, peer_id: PeerId, session_to_join: SessionId, password: &str) -> Res {
        let (peer, status) = self.connected.get(&peer_id).unwrap();

       let option_peer = match status {
            PeerStatus::InSession(session_id) => {
                let session = self.sessions.get_mut(session_id).expect("The session were the user is, doesn't exist.");
                let peer = session.rm_peer(peer_id);
                if session.is_empty() {
                    self.sessions.remove(session_id);
                }
                Some(peer)
            }
            PeerStatus::Idle => None
        };

        let session = self.sessions.get_mut(&session_to_join).expect("The destination session doesn't exist");
        if session.password() == password {
            session.add_peer(option_peer.unwrap_or(peer.clone()));
        } else {
            todo!("PASSWORD CHECK WRONG STATE CORRUPTED")
        }
        self.connected.entry(peer_id)
            .and_modify(|(_, status)| *status = PeerStatus::InSession(session_to_join));

        OK
    }

    pub fn connect(&mut self, peer: Peer) -> &Peer {
        let id = peer.id;
        self.connected.insert(id, (peer, PeerStatus::Idle));
        &self.connected.get(&id).unwrap().0
    }

    pub fn disconnect(&mut self, peer_id: PeerId) {
        info!("User: {} disconnected", peer_id);
        self.connected.remove(&peer_id)
            .expect("The user is not connected and tries to disconnect")
            .pipe(|(_, status)| if let PeerStatus::InSession(session_id) = status {
                self.sessions.get_mut(&session_id).unwrap().rm_peer(peer_id);
            });
        self.connected.remove(&peer_id);
    }

    pub fn send_specific(&mut self, _peer_id: PeerId) {
        unimplemented!()
    }

    pub fn handle_login_action(&mut self, action: LogInAction) -> Res {
        match action {
            LogInAction::Connected(peer) => {
                let id = peer.id;
                self.connect(peer).send(Output::Connected(id))?;
                OK
            }
            LogInAction::Disconnect(user_id) => {
                self.disconnect(user_id);
                OK
            }
        }
    }

    pub fn handle_hub_action(&mut self, from: PeerId, action: HubAction) -> Res {
        match action {
            HubAction::CreateSession(password) => {
                let created_session = self.create_session(from, "Test".to_string(), password.clone());
                self.join_session(from, created_session, &password)?;
                info!("Created new Session from the initiative of {}", from);
                OK
            }
            HubAction::Join(session_id, session_password) => {
                self.join_session(from, session_id, &session_password)?;
                info!("User: {}, joined {}", from, session_id);
                OK
            }
            HubAction::SessionStart => {
                self.start_session(from)
            }
            _ => {
                warn!("Action not handled");
                OK
            }
        }
    }

    pub fn handle_net_input(&mut self, input: Input) -> Res {
        match input.action {
            InputAction::SessionAction(session_action) => {
                self.handle_session_action(input.from.unwrap(), session_action)
            }
            InputAction::HubAction(hub_action) => {
                self.handle_hub_action(input.from.unwrap(), hub_action)
            }
            _ => { todo!("AZD") }
        }
    }

    pub fn handle_session_action(&mut self, from: PeerId, input: PlayerAction) -> Res {
        let session = self.get_mut_session(from)
            .context("The peer didn't join any session")?;
        session.handle_action(input)?;
        OK
    }

    pub fn handle(&mut self, message: HubMessage) -> Res {
        match message {
            HubMessage::NetInput(input) => {
                self.handle_net_input(input)
            }
            HubMessage::LogInAction(action) => {
                self.handle_login_action(action)
            }
        }
    }

    pub fn get_peer(&self, peer_id: PeerId) -> &Peer {
        self.connected.get(&peer_id).map(|s| &s.0).unwrap()
    }

    pub fn get_session(&self, peer_id: PeerId) -> Option<&Session> {
        match self.connected.get(&peer_id).unwrap().1 {
            PeerStatus::InSession(ses) => self.sessions.get(&ses),
            PeerStatus::Idle => None
        }
    }

    pub fn get_mut_session(&mut self, peer_id: PeerId) -> Option<&mut Session> {
        match self.connected.get(&peer_id).expect("The user is not connected").1 {
            PeerStatus::InSession(ses) => self.sessions.get_mut(&ses),
            PeerStatus::Idle => None
        }
    }


    pub fn send_new_state_to_peers(&self) -> Res {
        for (_, (peer, _)) in self.connected.iter() {
            let hub_state = self.create_hub_state(peer);
            peer.send(Output::World(hub_state))?;
        }
        OK
    }

    pub fn create_hub_state(&self, peer: &Peer) -> HubState {
        let peer_id = peer.id;
        HubState {
            me: peer.into(),
            sessions: self.sessions.iter().map(|(_, s)| s).map(SessionDTO::from).collect(),
            connected: self.connected.iter().map(|(_, (peer, _))| PeerDTO::from(peer)).collect(),
            my_session: self.get_session(peer_id).map(SessionDTO::from),
        }
    }

    pub fn handle_error(&self, peer_id: PeerId, output_error: OutputError) -> Res {
        self.get_peer(peer_id).send(Output::Error(output_error))?;
        Err(anyhow::anyhow!("{:?}", output_error))
    }
}


