use std::fmt::Write;

use super::*;

mod world;
pub use world::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "game")]
pub struct Model {
    #[sea_orm(primary_key, unique)]
    pub id: u32,
    #[sea_orm(default_expr = "now()", not_null)]
    pub created_at: ChronoDateTime,
    #[sea_orm(default_expr = "now()", not_null)]
    pub updated_at: ChronoDateTime,
    pub owner_id: u32,
    #[sea_orm(default_value = "0")]
    pub world_id: u32,
    #[sea_orm(default_value = "0")]
    pub track_id: u32,
    pub rnd: i32,
    #[sea_orm(default_value = "1")]
    pub game_type: GameType,
    #[sea_orm(default_value = "1")]
    pub laps: u32,
    #[sea_orm(default_value = "10")]
    pub seeds: u32,
    #[sea_orm(default_value = "100")]
    pub duration: u32,
    #[sea_orm(default_value = "true")]
    pub is_express: bool,
    #[sea_orm(default_value = "2")]
    pub players_cnt: u32,
}

impl Model {
    pub fn world(&self) -> World {
        (self.world_id, self.track_id).try_into().unwrap()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id"
    )]
    User,
    // #[sea_orm(has_many = "super::player::Entity")]
    // Player,
    #[sea_orm(has_many = "super::turn::Entity")]
    Turn,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

// impl Related<super::player::Entity> for Entity {
//     fn to() -> RelationDef {
//         Relation::Player.def()
//     }
// }

impl Related<super::turn::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Turn.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        let reject = || match insert {
            true => Err(DbErr::RecordNotInserted),
            false => Err(DbErr::RecordNotUpdated),
        };

        // below validate fields:

        {
            if let ActiveValue::Set(world_id) = self.world_id {
                if !World::available_world_ids().contains(&world_id) {
                    ::log::warn!("reject save a game with incorrect world_id: {}", world_id);
                    return reject();
                }
            }
        }

        {
            if let ActiveValue::Set(players_cnt) = self.players_cnt {
                if !(2..=5).contains(&players_cnt) {
                    ::log::warn!(
                        "reject save a game with incorrect players_cnt: {}",
                        players_cnt
                    );
                    return reject();
                }
            }
        }

        if insert && self.rnd.is_not_set() {
            ::log::trace!("generate new game.rnd before insert a row");
            self.rnd = ActiveValue::Set(::rand::random());
        }

        self.updated_at = ActiveValue::NotSet;

        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum GameWorld {
    Mountain = 0,
    Water = 1,
    Forest = 2,
    Town = 3,
    Lava = 4,
    Dolly = 5,
    Mechanic = 6,
    Interface = 7,
    Watch = 8,
    Vid = 9,
    Forests = 10,
    Waters = 11,
    Mounts = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum GameType {
    Winner = 1,
    All = 2,
}

impl std::str::FromStr for GameType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_uppercase().as_str() {
            "W" => GameType::Winner,
            "A" => GameType::All,
            _ => Err(())?,
        })
    }
}

impl std::fmt::Display for GameType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(match self {
            GameType::Winner => 'W',
            GameType::All => 'A',
        })
    }
}
