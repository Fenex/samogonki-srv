use std::{
    fmt::{Debug, Display, Write},
    str::FromStr,
    vec,
};

use ::aw::{
    body::EitherBody,
    http::header::{self, HeaderValue},
    HttpResponse, Responder,
};
use ::enum_primitive_derive::Primitive;
use ::log::{info, warn};
use ::num_traits::FromPrimitive;
use ::serde::{Deserialize, Serialize};
use entity::game::GameType;

#[derive(Debug, Primitive, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
pub enum Language {
    Ru = 0,
    En = 1,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Primitive, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
pub enum PacketType {
    /// Пакеты, передаваемые от сервера в игру с заголовком и информацией о сделанных ходах.
    OG_GAME_PACKET = 1,
    /// Ответы со стороны игрока о получении пакета и результатах обсчета хода.
    OG_CONTROL_PACKET = 2,
    /// Ходы игроков.
    OG_SEEDS_PACKET = 3,
    /// Информация по состоявшейся игре.
    OG_COMPLETED_GAME_PACKET = 4,
    /// Административный тип пакетов для проверки корректности игры.
    OG_SYS_PACKET = 5,
    /// Обновление информации о ходе для экспресс-игры
    OG_REFRESH_PACKET = 6,
    /// Ответ на 6й пакет
    OG_REFRESH_ANSWER_PACKET = 7,
    /// Запуск тестовой версии с сервера
    OG_ARCADE_GAME_PACKET = 8,
}

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct UrlProperty {
    pub post: String,
    pub post_port: u16,
    pub post_path: String,
    pub sreturn: String,
}

impl Debug for UrlProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(' ')
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Player {
    pub uid: u32,
    pub nickname: String,
    pub pers_car_comp_id: u32,
    pub front_car_comp_id: u32,
    pub fwheel_car_comp_id: u32,
    pub bwheel_car_comp_id: u32,
    pub is_robot: bool,
    pub password: Option<String>,
}

impl Player {
    pub fn new(uid: u32, nickname: &str) -> Self {
        Self {
            uid,
            nickname: nickname.into(),
            pers_car_comp_id: 1,
            front_car_comp_id: 1,
            fwheel_car_comp_id: 1,
            bwheel_car_comp_id: 1,
            is_robot: false,
            password: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    /// Номер версии API
    pub version: usize,
    /// Тип пакета
    pub t_type: PacketType,
    /// Уникальный идентификатор Игры
    pub gmid: u32,
    /// Язык игры (зачем ?)
    pub language: Language,
    /// Уникальный номер игрока владельца игры.
    pub game_owner_pid: u32,
    /// Уникальный номер владельца пакета. Выдается сервером.
    pub packet_owner_pid: u32,
    /// Пароль доступа для владельца макета.
    /// Иначе говоря, идентификатор подтверждения прав на работу под данным OWNER_UID.
    /// Уникальная строка для каждого игрока в каждой новой игре.
    pub password: String,
    /// Номер мира
    pub kd_world_id: u8,
    /// Номер трассы
    pub kd_route_id: u8,
    /// Случайное число для игры
    pub game_rnd: u16, // TODO: change to i32 (?)
    /// 'W' - до победителя или 'A' - полный проход
    pub game_type: GameType,
    /// Число кругов
    pub laps: u32,
    // Число семян на ход
    pub seeds: u32,
    // Длительность хода в минутах
    pub duration: u32,
    // Число полных сделанных ходов
    pub move_cnt: u32,
    // экспресс игра или нет
    pub is_express: bool,
    // Параметры адреса сервера
    pub url: UrlProperty,
    pub players: Vec<Player>,
    pub steps: Vec<PlayerTurnInfo>,
}

impl KdlabCodec for Packet {
    fn encode(&self) -> String {
        use PacketType::*;

        let mut ret = vec![
            String::from("KDLAB"),
            self.version.to_string(),
            (self.t_type as usize).to_string(),
            self.gmid.to_string(),
            (self.language as usize).to_string(),
            self.game_owner_pid.to_string(),
            self.packet_owner_pid.to_string(),
            self.password.clone(),
            self.kd_world_id.to_string(),
            self.kd_route_id.to_string(),
            self.game_rnd.to_string(),
            format!("{}", self.game_type),
            self.laps.to_string(),
            self.seeds.to_string(),
            self.duration.to_string(),
            self.move_cnt.to_string(),
        ];

        let mut players = vec![];

        if !self.players.is_empty()
            && !matches!(
                self.t_type,
                OG_SEEDS_PACKET | OG_REFRESH_PACKET | OG_REFRESH_ANSWER_PACKET
            )
        {
            players = self.players.iter().collect();
        }

        let max_step = self
            .steps
            .iter()
            .map(|s| s.step_number)
            .max()
            .unwrap_or_default();

        let steps = self
            .steps
            .iter()
            .filter(|s| s.step_number == max_step)
            .collect::<Vec<_>>();

        ret.append(&mut vec![
            players.len().to_string(),
            steps.len().to_string(),
            YesNo(self.is_express).to_string(),
            self.url.post.clone(),
            self.url.post_port.to_string(),
            self.url.post_path.clone(),
            self.url.sreturn.clone(),
        ]);

        for p in players {
            ret.push(p.uid.to_string());
            ret.push(p.nickname.clone());
            ret.push(p.pers_car_comp_id.to_string());
            ret.push(p.front_car_comp_id.to_string());
            ret.push(p.fwheel_car_comp_id.to_string());
            ret.push(p.bwheel_car_comp_id.to_string());
            ret.push(YesNo(p.is_robot).to_string());
        }

        if !steps.is_empty() {
            // for step_number in 0..max_step {
            // ret.push(step_number.to_string());
            ret.push(max_step.to_string());
            ret.push(steps.len().to_string());
            for step in steps {
                ret.push(step.player_id.to_string());
                ret.push(YesNo(step.is_finished).to_string());
                ret.push(step.rank.to_string());
                ret.push(step.move_time.to_string());
                ret.push(step.move_steps.to_string());
                ret.push(step.bottles_cnt.to_string());
                ret.push(step.total_seeds_cnt.to_string());
                ret.push(step.arcanes_cnt.to_string());
                ret.push(step.destroys_cnt.to_string());
                ret.push(step.user_seeds_cnt.to_string());
                ret.push(step.seeds.to_string());
            }
        }

        ret.push(String::from("BITRIX"));
        ret.join(";")
    }

    fn decode(input: &str) -> Option<Self> {
        use PacketType::*;

        let mut iter = input.split(';');

        if "KDLAB" != iter.next()? {
            None?
        }
        let version = next(&mut iter)?;
        let t_type = PacketType::from_u8(next(&mut iter)?)?;
        let id = next(&mut iter)?;
        let language = next(&mut iter)?;
        let game_owner_uid = next(&mut iter)?;
        let owner_uid = next(&mut iter)?;
        let password = iter.next().map(ToOwned::to_owned)?;
        let kd_world_id = next(&mut iter)?;
        let kd_route_id = next(&mut iter)?;
        let game_rnd = next(&mut iter)?;
        let game_type = next(&mut iter)?;
        let laps = next(&mut iter)?;
        let seeds = next(&mut iter)?;
        let duration = next(&mut iter)?;
        let move_cnt = next(&mut iter)?;
        let players_cnt: usize = next(&mut iter)?;
        let steps_cnt: usize = next(&mut iter)?;
        let is_express = iter.next()? == "Y";

        info!("steps_cnt is {steps_cnt}");

        let mut iter = iter.skip(4); // possible url property

        let mut players = Vec::with_capacity(players_cnt);
        if players_cnt > 0
            && !matches!(
                t_type,
                OG_SEEDS_PACKET | OG_REFRESH_PACKET | OG_REFRESH_ANSWER_PACKET
            )
        {
            for _ in 0..players_cnt {
                let player = Player {
                    uid: next(&mut iter)?,
                    nickname: iter.next().map(ToOwned::to_owned)?,
                    pers_car_comp_id: next(&mut iter)?,
                    front_car_comp_id: next(&mut iter)?,
                    fwheel_car_comp_id: next(&mut iter)?,
                    bwheel_car_comp_id: next(&mut iter)?,
                    is_robot: iter.next()? == "Y",
                    password: None,
                };

                players.push(player);
            }
        }

        //  * step - один ход игры, в котором содержатся ходы игроков для данного хода игры.
        // если в step содержится количество turn равное количеству игроков в игре,
        // то этот ход игры (step) считается завершённым
        //  * turn - один ход игрока, в котром могут содержаться несколько действий игрока (seeds)

        let mut steps = vec![];
        for _ in 0..steps_cnt {
            steps.append(&mut PlayerTurnInfo::from_raw(&mut iter)?);
        }

        let undecoded = iter.collect::<Vec<_>>().join(";");
        //dbg!(&undecoded.trim(), "BITRIX");
        if undecoded.trim() != "BITRIX" {
            warn!("undecoded tail: {undecoded}");
        }

        let p = Packet {
            version,
            t_type,
            gmid: id,
            language: Language::from_u8(language)?,
            game_owner_pid: game_owner_uid,
            packet_owner_pid: owner_uid,
            password,
            kd_world_id,
            kd_route_id,
            game_rnd,
            game_type,
            laps,
            seeds,
            duration,
            move_cnt,
            is_express,
            url: UrlProperty::default(),
            players,
            steps,
        };

        Some(p)
    }
}

pub trait KdlabCodec
where
    Self: Sized,
{
    fn encode(&self) -> String;
    fn decode(input: &str) -> Option<Self>;
}

pub struct KdlabNetObject<T>(pub T);

impl<T> Responder for KdlabNetObject<T>
where
    T: KdlabCodec,
{
    type Body = EitherBody<String>;

    fn respond_to(self, _: &::aw::HttpRequest) -> HttpResponse<Self::Body> {
        match HttpResponse::Ok().message_body(self.0.encode()) {
            Ok(mut res) => {
                res.headers_mut()
                    .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
                res.map_into_left_body()
            }
            Err(err) => HttpResponse::from_error(err).map_into_right_body(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerTurnInfo {
    pub step_number: u32,
    pub player_id: u32,
    pub is_finished: bool,
    pub rank: u32,
    pub move_time: u32,
    pub move_steps: u32,
    pub bottles_cnt: u32,
    pub total_seeds_cnt: u32,
    pub arcanes_cnt: u32,
    pub destroys_cnt: u32,
    pub user_seeds_cnt: u32,
    pub seeds: String,
}

impl PlayerTurnInfo {
    fn from_raw<'a>(mut iter: impl Iterator<Item = &'a str>) -> Option<Vec<Self>> {
        let step_number = next(&mut iter)?;
        let turns_cnt = next(&mut iter)?;

        let mut ret = vec![];
        for _ in 0..turns_cnt {
            ret.push(Self {
                step_number,
                player_id: next(&mut iter)?,
                is_finished: iter.next()? == "Y",
                rank: next(&mut iter)?,
                move_time: next(&mut iter)?,
                move_steps: next(&mut iter)?,
                bottles_cnt: next(&mut iter)?,
                total_seeds_cnt: next(&mut iter)?,
                arcanes_cnt: next(&mut iter)?,
                destroys_cnt: next(&mut iter)?,
                user_seeds_cnt: next(&mut iter)?,
                seeds: iter.next()?.to_owned(),
            })
        }

        Some(ret)
    }
}

struct YesNo(bool);

impl Display for YesNo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(match self {
            Self(true) => 'Y',
            Self(false) => 'N',
        })
    }
}

fn next<'a, F: FromStr>(mut iter: impl Iterator<Item = &'a str>) -> Option<F> {
    iter.next()?.parse::<F>().ok()
}

#[cfg(test)]
mod tests {
    use crate::data::{GameType, Language, PacketType, Player, PlayerTurnInfo};

    use super::{KdlabCodec, Packet, UrlProperty};

    #[test]
    fn packet_bothcode_type2_players_2() {
        let input = "KDLAB;104;2;1;0;0;1;password;0;0;47792;A;3;100;10;0;2;0;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;BITRIX";
        let p = Packet::decode(&input).unwrap();
        assert_eq!(p.version, 104);
        assert_eq!(p.t_type, PacketType::OG_CONTROL_PACKET);
        assert_eq!(p.gmid, 1);
        assert_eq!(p.language, Language::Ru);
        assert_eq!(p.game_owner_pid, 0);
        assert_eq!(p.packet_owner_pid, 1);
        assert_eq!(p.password.as_str(), "password");
        assert_eq!(p.kd_world_id, 0);
        assert_eq!(p.kd_route_id, 0);
        assert_eq!(p.game_rnd, 47792);
        assert_eq!(p.game_type, GameType::All);
        assert_eq!(p.laps, 3);
        assert_eq!(p.seeds, 100);
        assert_eq!(p.duration, 10);
        assert_eq!(p.move_cnt, 0);
        // assert_eq!(p.players_cnt);
        // assert_eq!(p.steps_cnt);
        assert!(p.url.post.is_empty());
        assert_eq!(p.url.post_port, 0);
        assert!(p.url.post.is_empty());
        assert!(p.url.post.is_empty());
        assert_eq!(p.is_express, true);
        assert_eq!(
            p.players,
            vec![
                Player {
                    uid: 0,
                    nickname: "player".to_owned(),
                    pers_car_comp_id: 1,
                    front_car_comp_id: 1,
                    fwheel_car_comp_id: 1,
                    bwheel_car_comp_id: 1,
                    is_robot: false,
                    password: None
                },
                Player {
                    uid: 1,
                    nickname: "player2".to_owned(),
                    pers_car_comp_id: 1,
                    front_car_comp_id: 1,
                    fwheel_car_comp_id: 1,
                    bwheel_car_comp_id: 1,
                    is_robot: false,
                    password: None
                }
            ]
        );

        assert_eq!(input, &p.encode());
    }

    #[test]
    fn packet_encode_type7_without_players_with_single_step() {
        let packet = Packet {
            version: 104,
            t_type: PacketType::OG_REFRESH_ANSWER_PACKET,
            gmid: 0,
            language: Language::Ru,
            game_owner_pid: 0,
            packet_owner_pid: 0,
            password: String::from("password"),
            kd_world_id: 0,
            kd_route_id: 0,
            game_rnd: 0,
            game_type: GameType::All,
            laps: 5,
            seeds: 200,
            duration: 10,
            move_cnt: 0,
            is_express: true,
            url: UrlProperty::default(),
            players: vec![],
            steps: vec![PlayerTurnInfo {
                step_number: 1,
                player_id: 0,
                is_finished: false,
                rank: 0,
                move_time: 0,
                move_steps: 0,
                bottles_cnt: 0,
                total_seeds_cnt: 0,
                arcanes_cnt: 0,
                destroys_cnt: 0,
                user_seeds_cnt: 4,
                seeds: String::from("661#348#50#-1#1181#291#51#-1#1616#423#51#-1#1879#702#51#-1"),
            }],
        };

        assert_eq!(packet.encode(), "KDLAB;104;7;0;0;0;0;password;0;0;0;A;5;200;10;0;0;1;Y;;0;;;1;1;0;N;0;0;0;0;0;0;0;4;661#348#50#-1#1181#291#51#-1#1616#423#51#-1#1879#702#51#-1;BITRIX");
        // TODO: ts-server encode that:
        // assert_eq!(packet.encode(), "KDLAB;104;7;0;0;0;0;password;0;0;0;A;5;200;10;0;2;0;Y;;0;;;1;1;0;N;0;0;0;0;0;0;0;4;661#348#50#-1#1181#291#51#-1#1616#423#51#-1#1879#702#51#-1;BITRIX");
    }
}
