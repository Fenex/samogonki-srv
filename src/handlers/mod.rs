use std::future::Ready;

use ::log::warn;

use ::actix_session::Session;
use ::askama_actix::{Template, TemplateToResponse};
use ::aw::{
    body::BoxBody,
    http::{header, StatusCode},
    web::{self, Data, Either, Form, Path, Query, Redirect},
    FromRequest, HttpMessage, HttpRequest, HttpResponse, Responder, ResponseError,
};
use ::log::error;
use ::sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DbErr, EntityTrait, FromQueryResult, JoinType,
    ModelTrait, QueryFilter, QuerySelect, RelationTrait,
};
use ::serde::{Deserialize, Serialize};

use crate::state::*;
use crate::{
    data::*,
    middleware::{MaybeAuthenticated, MiddlewareError},
};
use entity::*;

// mod archive;
mod auth;
mod game;
mod index;
mod rating;
mod samogonki;
mod users;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/").route(web::get().to(index::get)));
    // cfg.service(web::scope("/archive").configure(archive::config));
    cfg.service(web::scope("/games").configure(game::config));
    cfg.service(web::scope("/game-on-line/default.asp").configure(samogonki::config));
    cfg.service(web::scope("/auth").configure(auth::config));
    cfg.service(web::scope("/users").configure(users::config));
    cfg.service(web::scope("/rating").configure(rating::config));
}

/// общая информация которая будет передана шаблонам для рендеринга
#[derive(Clone)]
pub struct AppTpl {
    /// текущий юзер, чей запрос обрабатывается
    pub me: Option<user::Model>,
}

impl FromRequest for AppTpl {
    type Error = MiddlewareError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut ::aw::dev::Payload) -> Self::Future {
        std::future::ready({
            let user = match req.extensions().get::<MaybeAuthenticated>().cloned() {
                Some(MaybeAuthenticated(user)) => user,
                _ => None,
            };
            Ok(Self { me: user })
        })
    }
}

#[derive(Clone, Template)]
#[template(path = "error.html")]
pub struct ErrorTpl {
    pub app: AppTpl,
    pub status_code: u16,
}
