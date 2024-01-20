pub use ::sea_orm_migration::prelude::*;

mod m20220101_000001_create_user;
mod m20240113_134047_create_game;
mod m20240113_140000_create_turn;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_user::Migration),
            Box::new(m20240113_134047_create_game::Migration),
            Box::new(m20240113_140000_create_turn::Migration),
        ]
    }
}
