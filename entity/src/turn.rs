use std::ops::{Deref, DerefMut};

use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "turn")]
pub struct Model {
    #[sea_orm(primary_key, unique, not_null)]
    pub id: u32,
    #[sea_orm(not_null)]
    pub game_id: u32,
    #[sea_orm(not_null)]
    pub user_id: u32,
    #[sea_orm(not_null)]
    pub player_number: u32,
    #[sea_orm(default_value = "1", not_null)]
    pub step_number: u32,
    #[sea_orm(default_value = false, not_null)]
    pub is_finished: bool,
    #[sea_orm(default_value = "0", not_null)]
    pub rank: u32,
    #[sea_orm(default_value = "0", not_null)]
    pub move_time: u32,
    #[sea_orm(default_value = "0", not_null)]
    pub move_steps: u32,
    #[sea_orm(default_value = "0", not_null)]
    pub bottles_cnt: u32,
    #[sea_orm(default_value = "0", not_null)]
    pub total_seeds_cnt: u32,
    #[sea_orm(default_value = "0", not_null)]
    pub arcanes_cnt: u32,
    #[sea_orm(default_value = "0", not_null)]
    pub destroys_cnt: u32,
    #[sea_orm(default_value = "0", not_null)]
    pub user_seeds_cnt: u32,
    #[sea_orm(null)]
    pub seeds: Option<String>,
    #[sea_orm(default_value = "1", not_null)]
    /// персонаж
    pub prop_pers: u32,
    #[sea_orm(default_value = "1", not_null)]
    /// персонаж
    pub prop_car: u32,
    #[sea_orm(default_value = "1", not_null)]
    /// передние колёса
    pub prop_fwheel: u32,
    #[sea_orm(default_value = "1", not_null)]
    /// задние колёса
    pub prop_bwheel: u32,
    /// Отправлено ли пользователю `self.player_number` все события этого хода
    /// т.е. все события с такими же `self.game_id` и `self.step_number`
    #[sea_orm(default_value = false, not_null)]
    pub is_received: bool,
    #[sea_orm(default_expr = "now()", not_null)]
    pub created_at: ::chrono::NaiveDateTime,
    #[sea_orm(default_expr = "now()", not_null)]
    pub updated_at: ::chrono::NaiveDateTime,
}

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

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.updated_at = ActiveValue::NotSet;

        Ok(self)
    }
}
