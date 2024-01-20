use std::borrow::Cow;

use super::*;

use crate::{
    manager::{GameManager, GameManagerError},
    middleware::Authenticated,
};

use entity::game::GameType;
use migration::{Expr, IntoCondition};

mod list;
mod new;
mod view;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("new")
            .route(web::get().to(new::get))
            .route(web::post().to(new::post)),
    );

    cfg.service(
        web::resource("{game_id}")
            .route(web::get().to(view::handler))
            .route(web::post().to(HttpResponse::NotImplemented)),
    );

    cfg.service(web::resource("{game_id}/join").route(web::post().to(join)));

    cfg.service(web::resource("").route(web::get().to(list::handler)));
}

async fn join(
    reg: Data<Registry>,
    app: AppTpl,
    path: Path<u32>,
    req: HttpRequest,
) -> ::aw::Result<impl Responder> {
    let game_id = path.into_inner();

    if app.me.is_none() {
        return Ok(Redirect::to(format!("/games/{}", game_id))
            .see_other()
            .respond_to(&req)
            .map_into_boxed_body());
    }

    let manager = GameManager::load_game(&reg.db, game_id).await?;

    let is_already_joined = manager
        .turns
        .iter()
        .find(|(_, u)| u.id == app.me.as_ref().unwrap().id)
        .is_some();
    let game_is_full = manager.turns.len() >= manager.game.players_cnt as usize;
    if is_already_joined || game_is_full {
        return Ok(Redirect::to(format!("/games/{}", game_id))
            .see_other()
            .respond_to(&req)
            .map_into_boxed_body());
    }

    let _turn = {
        let mut p = <entity::turn::ActiveModel as ::sea_orm::ActiveModelTrait>::default();
        p.user_id = ActiveValue::Set(app.me.as_ref().unwrap().id);
        p.game_id = ActiveValue::Set(manager.game.id);
        p.player_number = ActiveValue::Set(manager.turns.len() as u32);
        p.step_number = ActiveValue::Set(1);
        p
    }
    .insert(&reg.db)
    .await
    .map_err(GameManagerError::DbErr)?;

    return Ok(Redirect::to(format!("/games/{}", game_id))
        .see_other()
        .respond_to(&req)
        .map_into_boxed_body());
}
