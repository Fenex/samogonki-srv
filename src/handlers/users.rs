use super::*;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("")
            .route("", web::get().to(list))
            .route("register", web::get().to(test_register))
            .route("{user_id}", web::get().to(view)),
    );
}

#[derive(Template)]
#[template(path = "users/list.html")]
struct ListView {
    app: AppTpl,
    users: Vec<user::Model>,
}

async fn list(reg: Data<Registry>, app: AppTpl) -> ::aw::Result<impl Responder> {
    let users = user::Entity::find().all(&reg.db).await.unwrap();

    Ok(ListView { app, users }.to_response())
}

#[derive(Template)]
#[template(path = "users/view.html")]
struct UserView {
    app: AppTpl,
    user: user::Model,
}

async fn view(
    reg: Data<Registry>,
    app: AppTpl,
    req: HttpRequest,
    path: web::Path<u32>,
) -> impl Responder {
    let user_id = path.into_inner();
    let user = user::Entity::find_by_id(user_id)
        .one(&reg.db)
        .await
        .unwrap();

    match user {
        Some(user) => UserView { app, user }.to_response(),
        None => ::aw::web::Redirect::to("/")
            .using_status_code(StatusCode::SEE_OTHER)
            .respond_to(&req)
            .map_into_boxed_body(),
    }
}

#[cfg(debug_assertions)]
async fn test_register(
    reg: Data<Registry>,
    app: AppTpl,
    session: Session,
    req: HttpRequest,
) -> impl Responder {
    use entity::user::UserBlocked;

    match app.me {
        Some(me) => HttpResponse::Ok().body(format!(
            "You already loged in as `{}` (ID: {})",
            me.login(),
            me.id
        )),
        None => {
            let user = entity::user::ActiveModel {
                steam_id: ActiveValue::Set(::rand::random()),
                login: ActiveValue::Set(None),
                is_blocked: ActiveValue::Set(UserBlocked::Nope),
                ..Default::default()
            }
            .insert(&reg.db)
            .await
            .unwrap();

            session.insert("user_id", user.id).unwrap();

            return Redirect::to(format!("/users/{}", user.id))
                .see_other()
                .respond_to(&req)
                .map_into_boxed_body();
        }
    }
}

#[cfg(not(debug_assertions))]

async fn test_register() -> impl Responder {
    "disabled in release"
}
