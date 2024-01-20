use super::*;

#[derive(Template)]
#[template(path = "./games/new.html")]
struct GameNew {
    app: AppTpl,
    error: Vec<Cow<'static, str>>,
}

pub(super) async fn get(app: AppTpl) -> impl Responder {
    GameNew { app, error: vec![] }
}

#[derive(Debug, Deserialize)]
pub(super) struct FormGameNew {
    track_id: u32,
    game_type: GameType,
    laps: u32,
    seeds: u32,
    duration: u32,
    // is_express: bool,
    players_cnt: u32,
}

pub(super) async fn post(
    reg: Data<Registry>,
    req: HttpRequest,
    form: Form<FormGameNew>,
    Authenticated(user): Authenticated,
    app: AppTpl,
) -> impl Responder {
    use ActiveValue::*;

    let mut error = vec![];

    if form.laps > 50 {
        error.push(Cow::Borrowed("`laps` must be less than 50"));
    }

    if form.seeds > 1000 {
        error.push(Cow::Borrowed("`seeds` must be less than 1000"));
    }

    if form.duration < 10 || form.duration > 34000 {
        error.push(Cow::Borrowed("`duration` must be between 10 and 34000"));
    }

    if form.players_cnt < 2 || form.players_cnt > 5 {
        error.push(Cow::Borrowed("`players` must be between 2 and 5"));
    }

    if !error.is_empty() {
        return GameNew { app, error }.respond_to(&req);
    }

    let game = {
        let mut g = <entity::game::ActiveModel as ::sea_orm::ActiveModelTrait>::default();

        g.is_express = Set(true);
        g.owner_id = Set(user.id);
        g.world_id = Set(0); // TODO: set correct world
        g.track_id = Set(0); // TODO: set correct track
        g.game_type = Set(form.game_type);
        g.laps = Set(form.laps);
        g.seeds = Set(form.seeds);
        g.duration = Set(form.duration);
        g.is_express = Set(true);
        g.players_cnt = Set(form.players_cnt);
        g
    };

    let game = match game.insert(&reg.db).await {
        Ok(g) => g,
        Err(e) => {
            error.push(Cow::Owned(e.to_string()));
            return GameNew { app, error }.respond_to(&req);
        }
    };

    let turn = {
        let mut t = <entity::turn::ActiveModel as ::sea_orm::ActiveModelTrait>::default();
        t.game_id = Set(game.id);
        t.user_id = Set(user.id);
        t.player_number = Set(0);
        t.step_number = Set(1);
        t
    };

    let _turn = match turn.insert(&reg.db).await {
        Ok(t) => t,
        Err(e) => {
            error.push(Cow::Owned(e.to_string()));
            return GameNew { app, error }.respond_to(&req);
        }
    };

    Redirect::to(format!("/games/{}", game.id))
        .using_status_code(StatusCode::FOUND)
        .respond_to(&req)
        .map_into_boxed_body()
}
