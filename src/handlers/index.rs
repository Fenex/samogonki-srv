use super::*;

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    app: AppTpl,
}

pub async fn get(app: AppTpl) -> impl Responder {
    Index { app }
}
