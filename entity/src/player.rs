use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "player")]
pub struct Model {
    #[sea_orm(primary_key, unique)]
    pub id: u32,
    pub user_id: u32,
    pub game_id: u32,
    pub pers_id: u32,
    pub car_id: u32,
    pub fwheel_id: u32,
    pub bwheel_id: u32,
    #[sea_orm(default_expr = "now()", not_null)]
    pub created_at: ::chrono::NaiveDateTime,
    #[sea_orm(default_expr = "now()", not_null)]
    pub updated_at: ::chrono::NaiveDateTime,
}

impl Model {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::game::Entity",
        from = "Column::GameId",
        to = "super::game::Column::Id"
    )]
    Game,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::game::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
