use ::sea_orm::{
    entity::prelude::*, sea_query::IntoCondition, ActiveValue, FromJsonQueryResult, LinkDef,
};
use ::serde::{Deserialize, Serialize};

pub mod prelude;

pub mod game;
// pub mod player;
pub mod turn;
pub mod user;

fn now() -> ::chrono::NaiveDateTime {
    ::chrono::Utc::now().naive_utc()
}

/// получает всех зарегистрированных игроков для игры
pub struct GameToUsers;
impl Linked for GameToUsers {
    type FromEntity = game::Entity;
    type ToEntity = user::Entity;

    fn link(&self) -> Vec<LinkDef> {
        vec![
            game::Relation::Turn.def().on_condition(|_l, r| {
                Expr::col((r, turn::Column::StepNumber))
                    .eq(1)
                    .into_condition()
            }),
            turn::Relation::User.def(),
        ]
    }
}

/// получает все запрашиваемые ходы
struct GameToPlayers(u32);
impl Linked for GameToPlayers {
    type FromEntity = game::Entity;
    type ToEntity = turn::Entity;

    fn link(&self) -> Vec<LinkDef> {
        let step_number = self.0;

        vec![game::Relation::Turn.def().on_condition(move |_l, r| {
            Expr::col((r, turn::Column::StepNumber))
                .eq(step_number)
                .into_condition()
        })]
    }
}
