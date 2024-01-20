use super::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("").route("", web::get().to(index)));
}

#[derive(Template)]
#[template(path = "rating.html")]
struct RatingView {
    app: AppTpl,
}

pub async fn index(app: AppTpl) -> impl Responder {
    RatingView { app }
}
