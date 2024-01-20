use std::ops::Range;

use super::*;

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

    pub fn available_world_ids() -> Range<u32> {
        0..13
    }
}

impl TryFrom<(u32, u32)> for World {
    type Error = ();

    fn try_from((world, track): (u32, u32)) -> Result<Self, Self::Error> {
        use World::*;

        let track = track.try_into().map_err(|_| ())?;
        let world = match world {
            0 => Mountain(track),
            1 => Water(track),
            2 => Forest(track),
            3 => Town(track),
            4 => Lava(track),
            5 => Dolly(track),
            6 => Mechanic(track),
            7 => Interface(track),
            8 => Watch(track),
            9 => Vid(track),
            10 => Forests(track),
            11 => Waters(track),
            12 => Mounts(track),
            _ => Err(())?,
        };

        Ok(world)
    }
}
