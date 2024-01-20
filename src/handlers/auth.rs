use super::*;

use ::steam_connect::{Error as SteamError, Redirect, Verify};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("callback", web::get().to(callback))
            .route("login", web::get().to(login))
            .route("logout", web::post().to(logout)),
    );
}

#[derive(Debug, ::thiserror::Error)]
struct SteamAuthError(SteamError);

impl std::fmt::Display for SteamAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self.0))
    }
}

#[derive(Debug, ::thiserror::Error)]
enum ActionLoginError {
    #[error("`host` key in headers is not found or incorrect")]
    InvalidHostValueInHeader,
    #[error("Steam Auth Error: {0}")]
    SteamReponseError(#[from] SteamAuthError),
    #[error("Steam Auth was disabled on this server")]
    SteamAuthDisabled,
    #[error("Db Connection error: {0}")]
    DbErr(DbErr),
}

impl ResponseError for ActionLoginError {
    fn status_code(&self) -> StatusCode {
        match self {
            ActionLoginError::InvalidHostValueInHeader => StatusCode::NOT_ACCEPTABLE,
            ActionLoginError::SteamReponseError(_) => StatusCode::GATEWAY_TIMEOUT,
            ActionLoginError::SteamAuthDisabled => StatusCode::NOT_IMPLEMENTED,
            ActionLoginError::DbErr(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        ::log::error!("{:?}", &self);
        let res = HttpResponse::new(self.status_code());
        let body = BoxBody::new(format!(
            "error occured (HTTP ERROR #{})",
            self.status_code()
        ));
        res.set_body(body).map_into_boxed_body()
    }
}

async fn login(req: HttpRequest) -> ::aw::Result<HttpResponse> {
    let host = req
        .headers()
        .get("host")
        .map(|v| std::str::from_utf8(v.as_bytes()).ok())
        .flatten()
        .ok_or(ActionLoginError::InvalidHostValueInHeader)?;

    let redirect_url = if matches!(host.split(':').next(), Some("localhost" | "127.0.0.1")) {
        format!("http://{host}/auth/callback")
    } else {
        format!("https://{host}/auth/callback")
    };

    Ok(Redirect::new(&redirect_url).unwrap().redirect())
}

async fn callback(
    reg: Data<Registry>,
    req: HttpRequest,
    session: Session,
) -> ::aw::Result<HttpResponse> {
    let api_key = reg.steam_key.ok_or(ActionLoginError::SteamAuthDisabled)?;

    let verify = Verify::verify_request(req.query_string())
        .await
        .map_err(SteamAuthError)
        .map_err(ActionLoginError::from)?;

    let player_info = verify
        .get_summaries(api_key)
        .await
        .map_err(SteamAuthError)
        .map_err(ActionLoginError::from)?;

    let mut user = user::Entity::find()
        .filter(user::Column::SteamId.eq(verify.claim_id() as i64))
        .one(&reg.db)
        .await
        .map_err(ActionLoginError::DbErr)?;

    if user.is_none() {
        let u = user::ActiveModel {
            steam_id: ActiveValue::Set(verify.claim_id() as i64),
            login: ActiveValue::Set(Some(player_info.personaname)),
            ..Default::default()
        };
        user = Some(u.insert(&reg.db).await.map_err(ActionLoginError::DbErr)?);
    }

    let user = user.unwrap();

    if let Ok(Some(_)) = session.get::<u32>("user_id") {
    } else {
        session.insert("user_id", user.id).unwrap();
    }

    Ok(::aw::web::Redirect::to("/")
        .using_status_code(StatusCode::FOUND)
        .respond_to(&req)
        .map_into_boxed_body())
}

async fn logout(req: HttpRequest, session: Session) -> ::aw::Result<HttpResponse> {
    session.remove("user_id");

    Ok(::aw::web::Redirect::to("/")
        .using_status_code(StatusCode::FOUND)
        .respond_to(&req)
        .map_into_boxed_body())
}
