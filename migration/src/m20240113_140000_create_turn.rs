use super::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Turn::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Turn::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .unique_key()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Turn::GameId).integer().not_null())
                    .col(ColumnDef::new(Turn::UserId).integer().not_null())
                    .col(ColumnDef::new(Turn::PlayerNumber).integer().not_null())
                    .col(ColumnDef::new(Turn::StepNumber).integer().not_null())
                    .col(
                        ColumnDef::new(Turn::IsFinished)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Turn::Rank).integer().not_null().default(0))
                    .col(
                        ColumnDef::new(Turn::MoveTime)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Turn::MoveSteps)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Turn::BottlesCnt)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Turn::TotalSeedsCnt)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Turn::ArcanesCnt)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Turn::DestroysCnt)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Turn::UserSeedsCnt)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Turn::Seeds).text().null())
                    .col(
                        ColumnDef::new(Turn::PropPers)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Turn::PropCar)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Turn::PropFwheel)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Turn::PropBwheel)
                            .integer()
                            .not_null()
                            .default(1),
                    )
                    .col(
                        ColumnDef::new(Turn::IsReceived)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Turn::CreatedAt)
                            .date_time()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Turn::UpdatedAt)
                            .date_time()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_turn-user_id")
                            .from(Turn::Table, Turn::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_turn-game_id")
                            .from(Turn::Table, Turn::GameId)
                            .to(Game::Table, Game::Id)
                            .on_delete(ForeignKeyAction::Restrict)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .index(
                        Index::create()
                            .name("idx_turn-unique")
                            .col(Turn::GameId)
                            .col(Turn::PlayerNumber)
                            .col(Turn::StepNumber)
                            .unique(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Turn::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Game {
    Table,
    Id,
}

#[derive(DeriveIden)]
pub(crate) enum Turn {
    Table,
    Id,
    GameId,
    UserId,
    PlayerNumber,
    StepNumber,
    IsFinished,
    Rank,
    MoveTime,
    MoveSteps,
    BottlesCnt,
    TotalSeedsCnt,
    ArcanesCnt,
    DestroysCnt,
    UserSeedsCnt,
    Seeds,
    PropPers,
    PropCar,
    PropFwheel,
    PropBwheel,
    IsReceived,
    CreatedAt,
    UpdatedAt,
}
