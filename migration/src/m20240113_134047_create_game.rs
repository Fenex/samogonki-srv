use super::*;

use entity::game::{GameType, GameWorld};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Game::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Game::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .unique_key()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Game::CreatedAt)
                            .date_time()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Game::UpdatedAt)
                            .date_time()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(ColumnDef::new(Game::OwnerId).integer().not_null())
                    .col(
                        ColumnDef::new(Game::WorldId)
                            .integer()
                            .default(GameWorld::Mountain)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Game::TrackId)
                            .integer()
                            .default(0)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Game::Rnd).integer().not_null())
                    .col(
                        ColumnDef::new(Game::GameType)
                            .integer()
                            .default(GameType::Winner)
                            .not_null(),
                    )
                    .col(ColumnDef::new(Game::Laps).integer().default(1).not_null())
                    .col(ColumnDef::new(Game::Seeds).integer().default(10).not_null())
                    .col(
                        ColumnDef::new(Game::Duration)
                            .integer()
                            .default(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Game::IsExpress)
                            .boolean()
                            .default(true)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Game::PlayersCnt)
                            .integer()
                            .default(2)
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_owner_id")
                            .from(Game::Table, Game::OwnerId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Game::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum User {
    Table,
    Id,
}

#[derive(DeriveIden)]
pub(crate) enum Game {
    Table,
    Id,
    CreatedAt,
    UpdatedAt,
    OwnerId,
    WorldId,
    TrackId,
    Rnd,
    GameType,
    Laps,
    Seeds,
    Duration,
    IsExpress,
    PlayersCnt,
}
