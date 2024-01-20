pub use ::aw::{
    test,
    web::{self, Data, Json, Query},
    Responder,
};

pub use ::serde::{Deserialize, Serialize};
pub use ::tokio::sync::Mutex;

pub use crate::main::{data::Player, data::PlayerTurnInfo};

pub mod tapi;

#[allow(unused)]
pub const TEST_URL_GET_GAME: &str = "/test/get-game";
