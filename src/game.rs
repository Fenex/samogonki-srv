use ::serde::{Deserialize, Serialize};
use crate::data::{Player, PlayerTurnInfo};
use entity::game::GameType;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransferWatch {
    pub received: Vec<u32>,
    pub sent: Vec<u32>,
}

impl TransferWatch {
    pub fn is_completed(&self) -> bool {
        self.received.len() == self.sent.len() && !self.received.is_empty()
    }

    /// этот ход в процессе отправки данных клиентам
    pub fn is_active_sending<'a>(&self, total_players: usize) -> bool {
        self.received.len() == total_players && self.received.len() != self.sent.len()
    }
}

impl TransferWatch {
    pub fn new() -> Self {
        Self {
            received: Vec::with_capacity(5),
            sent: Vec::with_capacity(5),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum World {
    Mountain(u8),
    Water(u8),
    Forest(u8),
    Town(u8),
    Lava(u8),
    Dolly(u8),
    Mechanic(u8),
    Interface(u8),
    Watch(u8),
    Vid(u8),
    Forests(u8),
    Waters(u8),
    Mounts(u8),
}

impl World {
    pub fn world_id(&self) -> u8 {
        self.id().0
    }

    pub fn track_id(self) -> u8 {
        self.id().1
    }

    pub fn id(self) -> (u8, u8) {
        use World::*;

        match self {
            Mountain(track) => (0, track),
            Water(track) => (1, track),
            Forest(track) => (2, track),
            Town(track) => (3, track),
            Lava(track) => (4, track),
            Dolly(track) => (5, track),
            Mechanic(track) => (6, track),
            Interface(track) => (7, track),
            Watch(track) => (8, track),
            Vid(track) => (9, track),
            Forests(track) => (10, track),
            Waters(track) => (11, track),
            Mounts(track) => (12, track),
        }
    }
}
