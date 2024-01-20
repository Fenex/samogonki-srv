use std::borrow::Cow;

use super::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, unique)]
    pub id: u32,
    pub steam_id: i64,
    pub login: Option<String>,
    #[sea_orm(default_value = "0")]
    pub is_blocked: UserBlocked,
    #[sea_orm(default_expr = "now()", not_null)]
    pub created_at: ::chrono::NaiveDateTime,
    #[sea_orm(default_expr = "now()", not_null)]
    pub updated_at: ::chrono::NaiveDateTime,
}

impl Model {
    pub fn login<'a>(&'a self) -> Cow<'a, str> {
        self.login
            .as_ref()
            .map(|l| Cow::Borrowed(l.as_str()))
            .unwrap_or(Cow::Owned(format!(r#"№{}"#, self.steam_id)))
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::game::Entity")]
    Game,
    #[sea_orm(has_many = "super::turn::Entity")]
    Turn,
}

impl Related<super::game::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

impl Related<super::turn::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Turn.def()
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum UserBlocked {
    Nope = 0,
    ByAdmin = 1,
    ByModerator = 2,
    BySystem = 3,
}
