use ::sea_orm::DbConn;

#[derive(Debug, Default, Clone)]
pub struct Registry {
    pub db: DbConn,
    pub steam_key: Option<&'static str>,
}
