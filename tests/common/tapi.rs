use ::sea_orm::{EntityTrait, ModelTrait};

use super::*;

use crate::state::Registry;

#[derive(Debug, Serialize, Deserialize)]
pub struct GetGame {
    id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RetGame {
    pub game: entity::game::Model,
    pub turns: Vec<(entity::turn::Model, entity::user::Model)>,
}

pub(crate) async fn get_game(
    Query(GetGame { id }): Query<GetGame>,
    reg: Data<Registry>,
) -> impl Responder {
    let game = entity::game::Entity::find_by_id(id)
        .one(&reg.db)
        .await
        .unwrap()
        .unwrap();
    let turns = game
        .find_related(entity::turn::Entity)
        .find_also_related(entity::user::Entity)
        .all(&reg.db)
        .await
        .unwrap()
        .into_iter()
        .map(|(t, u)| (t, u.unwrap()))
        .collect::<Vec<_>>();

    Json(RetGame { game, turns })
}
