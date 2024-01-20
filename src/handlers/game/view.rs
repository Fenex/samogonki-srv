use super::*;

pub(super) async fn handler(
    reg: Data<Registry>,
    app: AppTpl,
    path: Path<u32>,
) -> ::aw::Result<impl Responder> {
    let game_id = path.into_inner();

    let (game, owner) = match entity::prelude::Game::find_by_id(game_id)
        .find_also_related(entity::user::Entity)
        .one(&reg.db)
        .await
        .unwrap()
    {
        Some((game, Some(owner))) => (game, owner),
        _ => {
            return Ok(
                HttpResponse::NotFound().body(GameView::not_found(app, game_id).render().unwrap())
            );
        }
    };

    let players = game.find_linked(GameToUsers).all(&reg.db).await.unwrap();

    let is_available_join = !players
        .iter()
        .any(|u| matches!(app.me.as_ref().map(|u| u.id), Some(user_id) if user_id == u.id));

    let turns = game
        .find_related(entity::turn::Entity)
        .all(&reg.db)
        .await
        .unwrap();
    let last_step = turns.iter().map(|t| t.step_number).max().unwrap();

    let mut steps = vec![];

    for i in 1..=last_step {
        steps.push(
            turns
                .iter()
                .filter(|t| t.step_number == i)
                .cloned()
                .collect::<Vec<_>>(),
        );
    }

    Ok(GameView {
        app,
        game_id,
        data: Some(GameViewData { game, owner }),
        players: players.as_ref(),
        is_available_join,
        steps,
    }
    .to_response()
    .map_into_boxed_body())
}

#[derive(Template)]
#[template(path = "games/view.html")]
struct GameView<'a> {
    app: AppTpl,
    game_id: u32,
    data: Option<GameViewData>,
    players: &'a [entity::user::Model],
    is_available_join: bool,
    steps: Vec<Vec<entity::turn::Model>>,
}

impl GameView<'_> {
    fn not_found(app: AppTpl, game_id: u32) -> Self {
        Self {
            app,
            game_id,
            data: None,
            players: &[],
            is_available_join: false,
            steps: Default::default(),
        }
    }
}

struct GameViewData {
    game: entity::game::Model,
    owner: entity::user::Model,
}
