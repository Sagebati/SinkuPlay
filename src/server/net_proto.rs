use serde::{Deserialize, Serialize};
use crate::server::{PeerId, SessionId};
use crate::server::session::{Session};
use crate::server::peer::Peer;

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Input {
    pub from: Option<PeerId>,
    pub action: InputAction,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum InputAction {
    HubAction(HubAction),
    SessionAction(PlayerAction),
    Connect,
    Alive,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum PlayerAction {
    Play,
    Pause,
    Stop,
}

#[derive(Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum HubAction {
    CreateSession(String),
    Share(String),
    Join(SessionId, String),
    SessionStart,
    Ready,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PeerDTO {
    id: PeerId,
    pseudo: String,
}

impl From<&Peer> for PeerDTO{
    fn from(p: &Peer) -> Self {
        PeerDTO {
            id: p.id,
            pseudo: p.pseudo.to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SessionDTO {
    id: SessionId,
    name: String,
    public: bool,
    participants: Option<Vec<PeerDTO>>,
    owner: PeerId
}

impl From<&Session> for SessionDTO {
    fn from(s: &Session) -> Self {
        SessionDTO {
            id: s.id(),
            name: s.name().to_string(),
            public: s.password() == "",
            participants: None,
            owner: s.owner()
        }
    }
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct HubState {
    pub me : PeerDTO,
    pub sessions: Vec<SessionDTO>,
    pub connected: Vec<PeerDTO>,
    pub my_session: Option<SessionDTO>,
}


#[derive(Clone,Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OutputError {
    PasswordDoesntMatch,
    NotConnected,
    InvalidProtocol,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Output {
    Connected(PeerId),
    Joined,
    Timestamp(u64),
    World(HubState),
    Error(OutputError),
    PlayerAction(PlayerAction)
}






