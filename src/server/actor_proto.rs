use crate::server::{PeerId};
use crate::server::net_proto::Input;
use crate::server::peer::Peer;

#[derive(Clone, Debug)]
pub enum LogInAction {
    Connected(Peer),
    Disconnect(PeerId)
}


#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum HubMessage {
    LogInAction(LogInAction),
    NetInput(Input),
}

#[derive(Clone, Debug)]
pub enum SessionMessage {
    Play,
    Pause,
    Stop
}
