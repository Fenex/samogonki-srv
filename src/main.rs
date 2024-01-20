extern crate actix_web as aw;

use ::actix_session::{config::BrowserSession, storage::CookieSessionStore, SessionMiddleware};
use ::actix_web_lab::middleware::from_fn;
use ::aw::{cookie::SameSite, web::Data, HttpServer};

mod session;
use session::key;

use ::dotenvy::dotenv;
use ::log::{error, trace};
use ::sea_orm::{Database, DbErr};
use migration::{Migrator, MigratorTrait};

pub mod api;
pub mod data;
pub mod handlers;
pub mod manager;
pub mod middleware;
pub mod state;
use state::*;

macro_rules! app {
    () => {
        ::actix_web::App::new()
            .wrap(::actix_web::middleware::Logger::default())
            .service(
                actix_files::Files::new("/static", "./static/")
                    .show_files_listing()
                    .use_last_modified(true),
            )
            .configure(handlers::config)
    };
}

#[derive(Debug, ::thiserror::Error)]
enum StartUpError {
    #[allow(dead_code)]
    #[error("DATABASE_URL is not set")]
    DatabaseUrlIsNotSet,
    #[error("Database IO error: {0}")]
    DbErr(#[from] DbErr),
    #[error("general IO error: {0}")]
    IO(#[from] std::io::Error),
}

#[actix_web::main]
async fn main() -> Result<(), StartUpError> {
    env_logger::init();

    trace!("TRACE level enabled");

    dotenv().ok();

    let database_url = std::env::vars()
        .find(|(k, _)| k == "DATABASE_URL")
        .map(|(_, v)| v)
        .ok_or(StartUpError::DatabaseUrlIsNotSet)?;

    let steam_key = std::env::vars()
        .find(|(k, _)| k == "STEAM_API_KEY")
        .map(|(_, v)| &*v.leak());

    let db = Database::connect(database_url).await?;
    Migrator::up(&db, None).await?;

    let registry = Data::new(Registry { steam_key, db });

    let srv = HttpServer::new(move || {
        app!()
            .wrap(from_fn(middleware::auth))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), key())
                    .session_lifecycle(BrowserSession::default())
                    .cookie_name(String::from("session"))
                    .cookie_http_only(true)
                    .cookie_same_site(SameSite::Lax)
                    .build(),
            )
            .app_data(Data::clone(&registry))
    })
    .workers(2)
    .bind(("0.0.0.0", 8080))?
    .run();

    srv.await?;

    Ok(())
}
