use flume::{Receiver,  Sender, bounded};
use crate::server::{Res, OK};
use anyhow::*;
use crate::client::player::PlayerState::{Playing, Paused};
use crate::{ignore, Ignore};
use log::*;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::thread;

#[derive(Debug)]
pub enum PlayerMessage {
    Play,
    Pause,
    Stop,
}

#[derive(Debug, Eq, PartialEq)]
enum PlayerState {
    Paused,
    Playing,
    Stopping,
}

pub struct PlayerManager {
    sender_player: Option<Sender<PlayerMessage>>,
    player_handle: Option<JoinHandle<()>>,
    //TODO media etc..
}

impl PlayerManager {
    pub fn new() -> Self {
        Self {
            sender_player: None,
            player_handle: None
        }
    }
    
    pub fn start(&mut self) {
        let (tx, rx) = bounded(8);
        self.sender_player = Some(tx);
        self.player_handle = Some(thread::spawn(|| {
            Player {
                proxy_receiver: rx,
                state: PlayerState::Paused,
            }.run();
        }
        ));
    }

    fn sender(&self) -> &Sender<PlayerMessage> {
        self.sender_player.as_ref().expect("Player not started")
    }

    pub fn play(&self) -> Res {
        self.sender().send(PlayerMessage::Play).context("Couldn't send the play message to the player")
    }

    pub fn stop(&self) -> Res {
        self.sender().send(PlayerMessage::Pause)
            .context("Couldn't send the stop message to the player")
    }

    pub fn pause(&self) -> Res {
        self.sender().send(PlayerMessage::Stop)
            .context("Couldn't send pause message to the player")
    }
}

pub struct Player {
    proxy_receiver: Receiver<PlayerMessage>,
    state: PlayerState,
}

impl Player {
    pub fn run(&mut self) {
        let start = Instant::now();
        let mut needle = Duration::from_millis(0);
        let duration = Duration::from_secs(30);
        loop {
            // TODO handle error
            self.proxy_receiver.recv_timeout(Duration::from_millis(10)).map(|message| self.handle(&message).unwrap()).ignore();

            match self.state {
                PlayerState::Playing  => {
                    if needle <= duration {
                        needle = Instant::now().duration_since(start);
                    }else {
                        self.pause().unwrap();
                        info!("Video finished, paused");
                    }
                    debug!("needle video {:?}", &needle)
                }
                PlayerState::Paused => ignore(), // paused,
                PlayerState::Stopping => break
            }
        }
    }

    pub fn handle(&mut self, message: &PlayerMessage) -> Res {
        match message {
            PlayerMessage::Play => {
                self.play();
                OK
            }
            PlayerMessage::Pause => {
                self.pause()
            }
            PlayerMessage::Stop => {
                self.stop();
                OK
            }
        }
    }

    fn play(&mut self) {
        self.state = Playing;
    }

    fn pause(&mut self) -> Res {
        ensure!(self.state == Playing, "the player is not playing");
        self.state = Paused;
        OK
    }

    fn stop(&mut self) {
        self.state = PlayerState::Stopping;
    }
}

