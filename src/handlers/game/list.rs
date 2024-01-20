use super::*;

#[derive(Template)]
#[template(path = "games/list.html")]
struct GamelistView {
    app: AppTpl,
    games: Vec<GameDataView>,
}

#[derive(FromQueryResult)]
struct GameDataView {
    id: u32,
    owner_id: u32,
    world_id: u32,
    track_id: u32,
    rnd: i32,
    game_type: GameType,
    laps: u32,
    seeds: u32,
    duration: u32,
    is_express: bool,
    /// количество слотов под игроков
    players_cnt: u32,
    /// количество зарегистрированных игроков
    players_registered: u32,
    /// логин владельца игры
    login: Option<String>,
    /// steam_id владельца игры
    steam_id: i32,
    created_at: ::chrono::NaiveDateTime,
    updated_at: ::chrono::NaiveDateTime,
}

pub(super) async fn handler(reg: Data<Registry>, app: AppTpl) -> ::aw::Result<impl Responder> {
    let games = entity::game::Entity::find()
        .column_as(entity::game::Column::Id.count(), "players_registered")
        .columns([entity::user::Column::Login, entity::user::Column::SteamId])
        .join(
            JoinType::Join,
            entity::game::Relation::Turn
                .def()
                .on_condition(|_left, right| {
                    Expr::col((right, entity::turn::Column::StepNumber))
                        .eq(1)
                        .into_condition()
                }),
        )
        .join(JoinType::Join, entity::game::Relation::User.def())
        .group_by(entity::game::Column::Id)
        .into_model::<GameDataView>()
        .all(&reg.db)
        .await
        .map_err(GameManagerError::DbErr)?;

    Ok(GamelistView { app, games })
}
