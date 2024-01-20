use ::sea_orm::{
    ActiveModelTrait, ActiveValue, DbConn, DbErr, EntityTrait, IntoActiveModel, ModelTrait,
};

use crate::data::{Language, Packet, PacketType, Player, PlayerTurnInfo};

#[derive(Debug, ::thiserror::Error)]
pub enum GameManagerError {
    #[error("Game `{0}` not found")]
    GameNotFound(u32),
    #[error("Game `{0}` not active")]
    GameNotActive(u32),
    #[error("Player with pid=`{0}` not found at this game")]
    IncorrectPlayerId(u32),
    #[error("Incorrect step number")]
    IncorrectStepNumber,
    #[error("Incorrect income steps")]
    IncorrectIncomeSteps,
    #[error("Incorrect income players")]
    IncorrectIncomePlayers,
    #[error("DbErr: `{0}`")]
    DbErr(#[from] DbErr),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Open,
    Started,
    Finished,
    Cancelled,
}

#[derive(Debug)]
pub struct GameManager<'a> {
    db: &'a DbConn,
    pub game: entity::game::Model,
    pub turns: Vec<(entity::turn::Model, entity::user::Model)>,
    active_pid: Option<u32>,
}

impl<'db> GameManager<'db> {
    pub async fn load_game(db: &'db DbConn, gmid: u32) -> Result<Self, GameManagerError> {
        let game = entity::game::Entity::find_by_id(gmid)
            .one(db)
            .await?
            .ok_or(GameManagerError::GameNotFound(gmid))?;

        let turns = game
            .find_related(entity::turn::Entity)
            .find_also_related(entity::user::Entity)
            .all(db)
            .await?
            .into_iter()
            .map(|(turn, user)| (turn, user.unwrap()))
            .collect::<Vec<_>>();

        Ok(Self {
            db,
            game,
            turns,
            active_pid: None,
        })
    }
}

impl GameManager<'_> {
    /// число сделанных полных ходов
    pub fn move_cnt(&self) -> u32 {
        if self.status() == GameStatus::Open {
            return 0;
        }

        if self.turns.len() == self.game.players_cnt as usize {
            if self.turns.iter().any(|(t, _)| t.seeds.is_none()) {
                return 0;
            } else {
                return 1;
            }
        }

        let max_step = self
            .turns
            .iter()
            .map(|(t, _)| t.step_number)
            .max()
            .unwrap_or(1);

        if self
            .turns
            .iter()
            .filter(|(t, _)| t.step_number == max_step)
            .count()
            < self.game.players_cnt as usize
        {
            return max_step - 1;
        }

        let unfinished_step = self
            .turns
            .iter()
            .filter(|(t, _)| t.seeds.is_none())
            .map(|(t, _)| t.step_number - 1)
            .min()
            .unwrap_or(max_step);

        return unfinished_step;
    }

    pub fn status(&self) -> GameStatus {
        if { self.game.players_cnt as usize } <= self.turns.len() {
            GameStatus::Started
        } else {
            GameStatus::Open
        }
    }

    pub async fn apply_results(&mut self, packet: &mut Packet) -> Result<(), GameManagerError> {
        use ActiveValue::*;

        assert!(self.active_pid.is_some());
        assert!(packet.t_type == PacketType::OG_CONTROL_PACKET);

        if self.status() != GameStatus::Started {
            Err(GameManagerError::GameNotActive(self.game.id))? // попытка сделать ход в не начатой игре
        }

        if packet.move_cnt == 0 && packet.steps.is_empty() {
            return Ok(());
        }

        let current_step = { self.move_cnt() + 1 };
        let step_results = packet.move_cnt;

        if step_results + 1 != current_step {
            // позволяем записать результаты обсчёта только последнего завершённого хода (step)
            Err(GameManagerError::IncorrectStepNumber)?
        }

        let income_turns = packet
            .steps
            .iter()
            .filter(|&p| p.step_number == step_results)
            .collect::<Vec<_>>();
        if income_turns.len() != self.game.players_cnt as usize {
            // в пакете должны присутствовать ходы (turns) всех игроков для этого хода (step)
            Err(GameManagerError::IncorrectIncomeSteps)?
        }

        for turn in self
            .turns
            .iter()
            .map(|(t, _)| t)
            .filter(|&t| t.step_number == step_results)
        {
            let income_t = income_turns
                .iter()
                .find(|t| t.player_id == turn.player_number)
                .ok_or(GameManagerError::IncorrectIncomeSteps)?;
            let income_p = packet
                .players
                .iter()
                .find(|p| p.uid == turn.player_number)
                .ok_or(GameManagerError::IncorrectIncomePlayers)?;

            // TODO: check is_finished, add ratings

            let mut turn = turn.clone().into_active_model();
            turn.is_finished = Set(income_t.is_finished);
            turn.rank = Set(income_t.rank);
            turn.move_time = Set(income_t.move_time);
            turn.move_steps = Set(income_t.move_steps);
            turn.bottles_cnt = Set(income_t.bottles_cnt);
            turn.total_seeds_cnt = Set(income_t.total_seeds_cnt);
            turn.arcanes_cnt = Set(income_t.arcanes_cnt);
            turn.destroys_cnt = Set(income_t.destroys_cnt);
            turn.prop_pers = Set(income_p.pers_car_comp_id);
            turn.prop_car = Set(income_p.front_car_comp_id);
            turn.prop_fwheel = Set(income_p.fwheel_car_comp_id);
            turn.prop_bwheel = Set(income_p.bwheel_car_comp_id);
            turn.update(self.db)
                .await
                .map_err(GameManagerError::DbErr)?;
        }

        Ok(())
    }

    pub async fn apply_step(&mut self, packet: &mut Packet) -> Result<(), GameManagerError> {
        use ActiveValue::*;

        assert!(self.active_pid.is_some());
        assert!(packet.t_type == PacketType::OG_SEEDS_PACKET);

        if self.status() != GameStatus::Started {
            Err(GameManagerError::GameNotActive(self.game.id))? // попытка сделать ход в не начатой игре
        }

        let current_step = { self.move_cnt() + 1 };

        let income_step = packet
            .steps
            .iter()
            .find(|step| {
                step.player_id == self.active_pid.unwrap() && step.step_number == current_step
            })
            .ok_or(GameManagerError::IncorrectStepNumber)?;
        let income_player = packet
            .players
            .iter()
            .find(|p| p.uid == self.active_pid.unwrap());

        let (last_turn, _) = self
            .turns
            .iter_mut()
            .filter(|(t, _)| t.player_number == self.active_pid.unwrap())
            .max_by_key(|(t, _)| t.step_number)
            .unwrap();

        let mut turn = last_turn.clone().into_active_model();
        if last_turn.step_number != current_step {
            // we need to insert new row in DB
            turn.id = NotSet;
            turn.is_received = NotSet;
            turn.step_number = Set(current_step);
            turn.user_seeds_cnt = Set(0);
            turn.seeds = Set(None);
        }

        turn.is_finished = Set(income_step.is_finished);
        turn.rank = Set(income_step.rank);
        turn.move_time = Set(income_step.move_time);
        turn.move_steps = Set(income_step.move_steps);
        turn.bottles_cnt = Set(income_step.bottles_cnt);
        turn.total_seeds_cnt = Set(income_step.total_seeds_cnt);
        turn.arcanes_cnt = Set(income_step.arcanes_cnt);
        turn.destroys_cnt = Set(income_step.destroys_cnt);
        turn.user_seeds_cnt = Set(income_step.user_seeds_cnt);
        turn.seeds = Set(match income_step.seeds.as_str() {
            "" => None,
            s => Some(s.to_owned()),
        });

        if let Some(income_player) = income_player {
            turn.prop_pers = Set(income_player.pers_car_comp_id);
            turn.prop_car = Set(income_player.front_car_comp_id);
            turn.prop_fwheel = Set(income_player.fwheel_car_comp_id);
            turn.prop_bwheel = Set(income_player.bwheel_car_comp_id);
        }

        if turn.id.is_not_set() {
            turn.insert(self.db).await?;
        } else {
            turn.update(self.db).await?;
        }

        Ok(())
    }

    pub async fn get_refresh_packet(&self, packet: &Packet) -> Result<Packet, GameManagerError> {
        use ActiveValue::*;

        assert!(self.active_pid.is_some());
        assert!(packet.t_type == PacketType::OG_REFRESH_PACKET);

        let requested_step = packet.move_cnt + 1;
        // let current_step = self.move_cnt() + 1;

        // if current_step - 1 != packet.move_cnt {
        //     Err(GameManagerError::IncorrectStepNumber)?
        // }

        let current_turns = self
            .turns
            .iter()
            .filter(|(t, _)| t.step_number == requested_step && t.seeds.is_some())
            .collect::<Vec<_>>();

        let mut p = packet.clone();

        p.steps = current_turns
            .iter()
            .map(|(t, _)| PlayerTurnInfo {
                step_number: t.step_number,
                player_id: t.player_number,
                is_finished: t.is_finished,
                rank: t.rank,
                move_time: t.move_time,
                move_steps: t.move_steps,
                bottles_cnt: t.bottles_cnt,
                total_seeds_cnt: t.total_seeds_cnt,
                arcanes_cnt: t.arcanes_cnt,
                destroys_cnt: t.destroys_cnt,
                user_seeds_cnt: t.user_seeds_cnt,
                seeds: t.seeds.clone().unwrap_or_default(),
            })
            .collect();

        if current_turns.len() < self.game.players_cnt as usize {
            // не все сделали свои ходы (turns) в этот ход (step)
            p.t_type = PacketType::OG_REFRESH_ANSWER_PACKET;
        } else {
            p.t_type = PacketType::OG_GAME_PACKET;
            p.move_cnt = self.move_cnt();

            p.players = current_turns
                .iter()
                .map(|(t, u)| crate::data::Player {
                    uid: t.player_number,
                    nickname: u.login().to_string(),
                    pers_car_comp_id: t.prop_pers,
                    front_car_comp_id: t.prop_car,
                    fwheel_car_comp_id: t.prop_fwheel,
                    bwheel_car_comp_id: t.prop_bwheel,
                    is_robot: false,
                    password: Some(String::from("")),
                })
                .collect();

            let mut my_turn = current_turns
                .iter()
                .find(|(t, _)| t.player_number == p.packet_owner_pid)
                .unwrap()
                .0
                .clone()
                .into_active_model();

            if !matches!(my_turn.is_received.take(), Some(true)) {
                my_turn.is_received = Set(true);
                my_turn.update(self.db).await?;
            }
        }

        Ok(p)
    }

    pub fn get_info(&self, t_type: PacketType) -> Packet {
        let mut players = vec![];
        while let Some((turn, user)) = self
            .turns
            .iter()
            .filter(|(t, _)| t.player_number as usize == players.len())
            .max_by_key(|(t, _)| t.step_number)
        {
            players.push(Player {
                uid: turn.player_number,
                nickname: user.login().to_string(),
                pers_car_comp_id: turn.prop_car,
                front_car_comp_id: turn.prop_car,
                fwheel_car_comp_id: turn.prop_fwheel,
                bwheel_car_comp_id: turn.prop_bwheel,
                is_robot: false,
                password: None,
            });
        }

        let steps = self
            .turns
            .iter()
            .filter(|(t, _)| t.step_number <= self.move_cnt() && t.seeds.is_some())
            .map(|(t, _)| PlayerTurnInfo {
                step_number: t.step_number,
                player_id: t.player_number,
                is_finished: t.is_finished,
                rank: t.rank,
                move_time: t.move_time,
                move_steps: t.move_steps,
                bottles_cnt: t.bottles_cnt,
                total_seeds_cnt: t.total_seeds_cnt,
                arcanes_cnt: t.arcanes_cnt,
                destroys_cnt: t.destroys_cnt,
                user_seeds_cnt: t.user_seeds_cnt,
                seeds: t.seeds.clone().unwrap_or_default(),
            })
            .collect::<Vec<_>>();

        let game_owner_pid = self
            .turns
            .iter()
            .find(|(_, u)| u.id == self.game.owner_id)
            .map(|(t, _)| t.player_number)
            .unwrap_or_default();

        Packet {
            version: 104,
            t_type,
            gmid: self.game.id,
            language: Language::Ru,
            game_owner_pid: game_owner_pid,
            packet_owner_pid: self.active_pid.unwrap(),
            password: "password".into(),
            kd_world_id: self.game.world_id as u8,
            kd_route_id: self.game.track_id as u8,
            game_rnd: self.game.rnd as u16,
            game_type: self.game.game_type,
            laps: self.game.laps,
            seeds: self.game.seeds,
            duration: self.game.duration,
            move_cnt: self.move_cnt(),
            is_express: true,
            url: crate::data::UrlProperty {
                post: "".into(),
                post_port: 0,
                post_path: "".into(),
                sreturn: "".into(),
            },
            players,
            steps,
        }
    }
}

impl GameManager<'_> {
    /// Установить pid игрока, от лица которого рассматривать эту игру
    pub fn set_pid(&mut self, player_id: u32) -> Result<(), GameManagerError> {
        if self
            .turns
            .iter()
            .find(|(t, _u)| t.player_number == player_id)
            .is_some()
        {
            self.active_pid = Some(player_id);
            Ok(())
        } else {
            Err(GameManagerError::IncorrectPlayerId(player_id))
        }
    }

    /// Установить pid игрока, от лица которого рассматривать эту игру
    pub fn with_pid(mut self, player_id: u32) -> Result<Self, Self> {
        if self
            .turns
            .iter()
            .find(|(t, _u)| t.player_number == player_id)
            .is_some()
        {
            self.active_pid = Some(player_id);
            Ok(self)
        } else {
            Err(self)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::NaiveDateTime;

    fn now() -> NaiveDateTime {
        ::chrono::Utc::now().naive_utc()
    }

    #[test]
    fn empty_open_game() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 0,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Open);
        assert_eq!(manager.move_cnt(), 0);
    }

    #[test]
    fn open_game_with_2_players() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 0,
                        seeds: None,
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 0,
                        seeds: None,
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Open);
        assert_eq!(manager.move_cnt(), 0);
    }

    #[test]
    fn started_game_with_no_steps() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 0,
                        seeds: None,
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 0,
                        seeds: None,
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 3,
                        player_number: 2,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 0,
                        seeds: None,
                        is_received: false,
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 3,
                        steam_id: 333,
                        login: Some(String::from("login333")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Started);
        assert_eq!(manager.move_cnt(), 0);
    }

    #[test]
    fn started_game_with_1_step_by_2nd_player() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 0,
                        seeds: None,
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 10,
                        seeds: Some(String::from("seed")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 3,
                        player_number: 2,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 0,
                        seeds: None,
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 3,
                        steam_id: 333,
                        login: Some(String::from("login333")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Started);
        assert_eq!(manager.move_cnt(), 0);
    }

    #[test]
    fn started_game_with_1_step_by_all_players_without_received() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 1,
                        seeds: Some(String::from("seed1")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 2,
                        seeds: Some(String::from("seed2")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 3,
                        player_number: 2,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 3,
                        seeds: Some(String::from("seed3")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 3,
                        steam_id: 333,
                        login: Some(String::from("login333")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Started);
        assert_eq!(manager.move_cnt(), 1);
    }

    #[test]
    fn started_game_with_1_step_by_all_players_with_received_2nd_player() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 1,
                        seeds: Some(String::from("seed1")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 2,
                        seeds: Some(String::from("seed2")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 3,
                        player_number: 2,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 3,
                        seeds: Some(String::from("seed3")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 3,
                        steam_id: 333,
                        login: Some(String::from("login333")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Started);
        assert_eq!(manager.move_cnt(), 1);
    }

    #[test]
    fn started_game_with_1_step_by_all_players_with_step2_by_2nd_player() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 1,
                        seeds: Some(String::from("seed1")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 2,
                        seeds: Some(String::from("seed2")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 3,
                        player_number: 2,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 3,
                        seeds: Some(String::from("seed3")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 3,
                        steam_id: 333,
                        login: Some(String::from("login333")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 2,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 2,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 22,
                        seeds: Some(String::from("seed2-2")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Started);
        assert_eq!(manager.move_cnt(), 1);
    }

    #[test]
    fn started_game_with_received_1_step_all_players() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 1,
                        seeds: Some(String::from("seed1")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 2,
                        seeds: Some(String::from("seed2")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 3,
                        player_number: 2,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 3,
                        seeds: Some(String::from("seed3")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 3,
                        steam_id: 333,
                        login: Some(String::from("login333")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Started);
        assert_eq!(manager.move_cnt(), 1);
    }

    #[test]
    fn started_game_with_received_1_step_by_all_step_2_by_player_2() {
        let manager = GameManager {
            db: &DbConn::Disconnected,
            game: entity::game::Model {
                id: 1,
                owner_id: 1,
                world_id: 0,
                track_id: 0,
                rnd: 123,
                game_type: entity::game::GameType::Winner,
                laps: 1,
                seeds: 10,
                duration: 100,
                is_express: true,
                players_cnt: 3,
                created_at: now(),
                updated_at: now(),
            },
            turns: vec![
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 1,
                        player_number: 0,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 1,
                        seeds: Some(String::from("seed1")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 1,
                        steam_id: 111,
                        login: Some(String::from("login111")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 2,
                        seeds: Some(String::from("seed2")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 3,
                        player_number: 2,
                        step_number: 1,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 0,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 3,
                        seeds: Some(String::from("seed3")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: true,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 3,
                        steam_id: 333,
                        login: Some(String::from("login333")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
                (
                    entity::turn::Model {
                        id: 0,
                        game_id: 1,
                        user_id: 2,
                        player_number: 1,
                        step_number: 2,
                        is_finished: false,
                        rank: 0,
                        move_time: 0,
                        move_steps: 0,
                        bottles_cnt: 0,
                        total_seeds_cnt: 2,
                        arcanes_cnt: 0,
                        destroys_cnt: 0,
                        user_seeds_cnt: 22,
                        seeds: Some(String::from("seed2-2")),
                        prop_pers: 1,
                        prop_car: 1,
                        prop_fwheel: 1,
                        prop_bwheel: 1,
                        is_received: false,
                        created_at: now(),
                        updated_at: now(),
                    },
                    entity::user::Model {
                        id: 2,
                        steam_id: 222,
                        login: Some(String::from("login222")),
                        is_blocked: entity::user::UserBlocked::Nope,
                        created_at: now(),
                        updated_at: now(),
                    },
                ),
            ],
            active_pid: None,
        };

        assert_eq!(manager.status(), GameStatus::Started);
        assert_eq!(manager.move_cnt(), 1);
    }
}
