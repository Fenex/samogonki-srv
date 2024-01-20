use std::{future::Ready, ops::Deref, rc::Rc};

use ::actix_session::Session;
use ::actix_web_lab::middleware::Next;
use ::aw::{
    body::{BoxBody, MessageBody},
    dev::{ServiceRequest, ServiceResponse},
    http,
    web::Data,
    FromRequest, HttpMessage, HttpRequest, HttpResponse,
};
use ::sea_orm::EntityTrait;

use crate::state::Registry;
use entity::user::Model as UserModel;
use entity::{prelude::User, user::UserBlocked};

#[derive(Debug, thiserror::Error)]
pub enum MiddlewareError {
    #[error("Authentication failure")]
    AuthenticationError,
}

impl ::aw::error::ResponseError for MiddlewareError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            MiddlewareError::AuthenticationError => http::StatusCode::UNAUTHORIZED,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Authenticated(pub Rc<UserModel>);

impl FromRequest for Authenticated {
    type Error = MiddlewareError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut ::aw::dev::Payload) -> Self::Future {
        std::future::ready({
            req.extensions()
                .get::<Self>()
                .cloned()
                .ok_or(MiddlewareError::AuthenticationError)
        })
    }
}

impl Deref for Authenticated {
    type Target = UserModel;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct MaybeAuthenticated(pub Option<UserModel>);

impl FromRequest for MaybeAuthenticated {
    type Error = MiddlewareError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut ::aw::dev::Payload) -> Self::Future {
        std::future::ready(Ok(Self(req.extensions().get::<UserModel>().cloned())))
    }
}

impl Deref for MaybeAuthenticated {
    type Target = Option<UserModel>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn auth(
    session: Session,
    reg: Data<Registry>,
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, ::aw::Error> {
    if reg.steam_key.is_some() {
        if let Ok(Some(user_id)) = session.get::<u32>("user_id") {
            if let Ok(Some(user)) = User::find_by_id(user_id).one(&reg.db).await {
                if user.is_blocked == UserBlocked::Nope {
                    req.extensions_mut()
                        .insert(Authenticated(Rc::new(user.clone())));
                    req.extensions_mut().insert(MaybeAuthenticated(Some(user)));
                }
            }
        }
    }

    // pre-processing
    next.call(req).await
    // post-processing
}
