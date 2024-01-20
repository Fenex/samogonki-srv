use super::*;

use crate::manager::{GameManager, GameManagerError};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("")
            .route(web::get().to(get))
            .route(web::post().to(post)),
    );
}

impl ResponseError for GameManagerError {
    fn status_code(&self) -> StatusCode {
        match self {
            GameManagerError::GameNotFound(_) => StatusCode::NOT_FOUND,
            GameManagerError::IncorrectPlayerId(_) => StatusCode::NOT_ACCEPTABLE,
            GameManagerError::DbErr(_) => StatusCode::SERVICE_UNAVAILABLE,
            GameManagerError::GameNotActive(_) => StatusCode::NOT_ACCEPTABLE,
            GameManagerError::IncorrectStepNumber => StatusCode::NOT_ACCEPTABLE,
            GameManagerError::IncorrectIncomeSteps => StatusCode::NOT_ACCEPTABLE,
            GameManagerError::IncorrectIncomePlayers => StatusCode::NOT_ACCEPTABLE,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let mut res = HttpResponse::new(self.status_code());
        let h_value = header::HeaderValue::from_static("text/html; charset=utf-8");
        res.headers_mut().insert(header::CONTENT_TYPE, h_value);
        res.set_body(BoxBody::new(format!("Error occurred: {:?}", self)))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ParamGameInfo {
    #[serde(alias = "USERID")]
    player_id: u32,
    #[serde(alias = "ID")]
    game_id: u32,
    // #[serde(alias = "API_KEY")]
    // api_key: String,
}

async fn get(
    reg: Data<Registry>,
    Query(ParamGameInfo { player_id, game_id }): Query<ParamGameInfo>,
) -> ::aw::Result<impl Responder, GameManagerError> {
    let mut gm = GameManager::load_game(&reg.db, game_id).await?;
    gm.set_pid(player_id)?;

    let packet = gm.get_info(PacketType::OG_CONTROL_PACKET);

    Ok(KdlabNetObject(packet))
}

async fn post(reg: Data<Registry>, packet: ::aw::Result<Packet>) -> ::aw::Result<impl Responder> {
    let mut p = packet?;

    let mut gm = GameManager::load_game(&reg.db, p.gmid).await?;
    gm.set_pid(p.packet_owner_pid)?;

    match p.t_type {
        PacketType::OG_CONTROL_PACKET => {
            gm.apply_results(&mut p).await?;
        }
        PacketType::OG_SEEDS_PACKET => {
            return match gm.apply_step(&mut p).await {
                Ok(_) => Ok(Either::Left("OK:KDLAB")),
                Err(GameManagerError::IncorrectIncomeSteps) => {
                    Ok(Either::Left("NEXT_MOVE")) // it's wrong
                }
                Err(e) => Err(e)?,
            };
        }
        PacketType::OG_REFRESH_PACKET => {
            let packet = gm.get_refresh_packet(&p).await?;
            return Ok(Either::Right(KdlabNetObject(packet)));
        }
        t => {
            warn!("unimplemented OG packet: {:?}", t);
            warn!("packet IN: {:#?}", &p);
        }
    }

    Ok(Either::Left("OK:KDLAB"))
}
