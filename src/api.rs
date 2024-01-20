use std::{future::Future, pin::Pin};

use ::aw::{
    error::{ErrorBadRequest, ErrorNotAcceptable},
    FromRequest, HttpRequest,
};
use ::futures::StreamExt;
use ::log::trace;

use crate::data::{KdlabCodec, Packet};

// pub fn config(cfg: &mut web::ServiceConfig) {
//     cfg.service(web::resource("/").route(web::get().to(index)));

//     cfg.service(web::resource("/game-on-line/default.asp")
//         .route(web::get().to(game_info))
//         .route(web::post().to(game_info_post))
//     );

//     // cfg.service(
//     //     web::scope("/api/v1")
//     //         .service(web::scope("browser").configure(browser::config))
//     //         .service(web::scope("user").configure(account::config))
//     //         .service(web::scope("group").configure(group::config)),
//     // );
// }

impl FromRequest for Packet {
    type Error = ::aw::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Packet, Self::Error>>>>;

    fn from_request(_: &HttpRequest, payload: &mut ::aw::dev::Payload) -> Self::Future {
        let payload = payload.take();

        Box::pin(async move {
            let chunks = payload.collect::<Vec<_>>().await;

            let mut bytes = vec![];
            for chunk in chunks {
                match chunk {
                    Ok(chunk) => bytes.extend_from_slice(&chunk),
                    Err(err) => Err(ErrorNotAcceptable(err))?,
                }
            }

            String::from_utf8(bytes)
                .map_err(|_| ErrorBadRequest("parse body failed (utf-8)"))
                .map(|t| {
                    trace!("INPUT REQUEST: `{}`", &t);
                    return Packet::decode(&t);
                })?
                .ok_or(ErrorBadRequest("parse body failed (syntax)"))
        })
    }
}
