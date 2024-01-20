use super::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("").route(web::get().to(get)),
    );
}

#[derive(Template)]
#[template(path = "archive.html")]
struct Archive {}

async fn get() -> impl Responder {
    Archive {}
}
