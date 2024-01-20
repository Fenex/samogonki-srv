#![allow(unused_imports)]

extern crate actix_web as aw;

#[macro_use]
#[path = "../src/main.rs"]
mod main;
use entity::{game::GameType, user::UserBlocked};
pub use main::*;

mod db;

mod common;
use common::*;
use main::state::Registry;
use sea_orm::{ActiveModelTrait, ActiveValue, Database, DbConn, DbErr};

use crate::common::tapi::RetGame;

async fn seed_required_data(db: &DbConn) -> Result<(), DbErr> {
    use ActiveValue::*;

    let _user1 = entity::user::ActiveModel {
        id: Set(1),
        steam_id: Set(111),
        login: Set(Some("player".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let _user2 = entity::user::ActiveModel {
        id: Set(2),
        steam_id: Set(222),
        login: Set(Some("player2".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let _game = entity::game::ActiveModel {
        id: Set(1),
        owner_id: Set(1),
        world_id: Set(0),
        track_id: Set(0),
        rnd: Set(12711),
        game_type: Set(GameType::All),
        laps: Set(1),
        seeds: Set(100),
        duration: Set(10),
        is_express: Set(true),
        players_cnt: Set(2),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let _player1_turn = entity::turn::ActiveModel {
        id: Set(1),
        game_id: Set(1),
        user_id: Set(1),
        player_number: Set(0),
        step_number: Set(1),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let _player2_turn = entity::turn::ActiveModel {
        id: Set(2),
        game_id: Set(1),
        user_id: Set(2),
        player_number: Set(1),
        step_number: Set(1),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

#[derive(Debug)]
struct Action {
    #[allow(dead_code)]
    client: usize,
    url: &'static str,
    payload: Option<&'static str>,
    response: &'static str,
    check_before: Option<Box<fn(&'_ RetGame) -> ()>>,
    check_after: Option<Box<fn(&'_ RetGame) -> ()>>,
}

#[actix_web::test]
async fn test_full_players2_turn2() {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    db::setup_schema(&db).await.unwrap();
    seed_required_data(&db).await.unwrap();

    let registry = Data::new(Registry {
        steam_key: None,
        db,
    });

    let app = app!()
        .route(TEST_URL_GET_GAME, web::get().to(tapi::get_game))
        .app_data(Data::clone(&registry));

    let srv = test::init_service(app).await;

    let get_game = || {
        let req = test::TestRequest::get()
            .uri(&format!("{TEST_URL_GET_GAME}?id=1"))
            .to_request();

        test::call_and_read_body_json(&srv, req)
    };

    let actions = vec![
        // подключается клиент №0
        Action {
            client: 0,
            url: "/game-on-line/default.asp?ID=1&USERID=0",
            payload: None,
            response: "KDLAB;104;2;1;0;0;0;password;0;0;12711;A;1;100;10;0;2;0;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;BITRIX",
            check_before: None,
            check_after: None
        },
        // POST / OG_CONTROL_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;2;1;0;0;0;password;0;0;12711;A;1;100;10;0;2;0;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: None
        },
        // теперь клиенту №0 предлагается нажать Enter для старта редактирования хода
        // ------------------- //
        // подключается клиент №1
        Action {
            client: 1,
            url: "/game-on-line/default.asp?ID=1&USERID=1",
            payload: None,
            response: "KDLAB;104;2;1;0;0;1;password;0;0;12711;A;1;100;10;0;2;0;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;BITRIX",
            check_before: None,
            check_after: None
        },
        // POST / OG_CONTROL_PACKET
        Action {
            client: 1,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;2;1;0;0;1;password;0;0;12711;A;1;100;10;0;2;0;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: None
        },
        // теперь клиенту №1 предлагается нажать Enter для старта редактирования хода

        /* ХОД №1 */

        // клиент №0 нажимает Enter, редактирует ход, снова нажимает Enter и отправляет данные о ходах на сервер:
        // POST / OG_SEEDS_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;3;1;0;0;0;password;0;0;12711;A;1;100;10;0;2;1;Y;;0;;;1;1;0;N;0;0;0;0;0;0;0;14;620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_0_sent_their_own_first_turn))
        },
        // клиент №0 отправляет запрос на обновление данных о текущей игре (возможно, все игроки уже закончили ход?)
        // POST / OG_REFRESH_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;6;1;0;0;0;password;0;0;12711;A;1;100;10;0;2;0;Y;;0;;;BITRIX;0;0;0;0;0;0;0;14;620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "KDLAB;104;7;1;0;0;0;password;0;0;12711;A;1;100;10;0;0;1;Y;;0;;;1;1;0;N;0;0;0;0;0;0;0;14;620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX",
            check_before: None,
            check_after: Some(Box::new(check::client_0_sent_their_own_first_turn)),
        },
        // клиент №0 узнаёт что не все игроки закончили свои ходы, и выводит сообщение чтобы обновили информацию позже нажатием пробела `Space`
        // ------------------- //
        // клиент №1 нажимает Enter, редактирует ход, снова нажимает Enter и отправляет данные о ходах на сервер:
        // POST / OG_SEEDS_PACKET
        Action {
            client: 1,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;3;1;0;0;1;password;0;0;12711;A;1;100;10;0;2;1;Y;;0;;;1;1;1;N;1;0;0;0;0;0;0;12;592#382#51#-1#892#329#61#-1#1534#377#51#-1#1945#949#48#-1#1882#1209#48#-1#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_1_sent_their_own_first_turn))
        },
        // клиент №1 отправляет запрос на обновление данных о текущей игре и получает ответ что все игроки сделали свои ходы
        // POST / OG_REFRESH_PACKET
        Action { // #7
            client: 1,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;6;1;0;0;1;password;0;0;12711;A;1;100;10;0;2;0;Y;;0;;;BITRIX;1;0;0;0;0;0;0;12;592#382#51#-1#892#329#61#-1#1534#377#51#-1#1945#949#48#-1#1882#1209#48#-1#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX"),
            response: "KDLAB;104;1;1;0;0;1;password;0;0;12711;A;1;100;10;1;2;2;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;1;2;0;N;0;0;0;0;0;0;0;14;620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;1;N;1;0;0;0;0;0;0;12;592#382#51#-1#892#329#61#-1#1534#377#51#-1#1945#949#48#-1#1882#1209#48#-1#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX",
            check_before: None,
            check_after: Some(Box::new(check::client_1_refresh_after_their_own_first_turn)),
        },
        // клиент №1 уведомляет сервер о том, что он принял сделанные ходы игроков, воспроизвёл и передаёт данные обсчёта
        // TODO: пока что этот пакет полностью игнорируется,данные обсчёта всех клиентов надо сравнивать между собой для верификации данных
        // POST / OG_CONTROL_PACKET
        Action {
            client: 1,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;2;1;0;0;1;password;0;0;12711;A;1;100;10;1;2;1;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;1;2;0;N;0;21;0;16;14;0;1;0;;1;N;1;21;0;20;12;0;1;0;;BITRIX#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_1_refresh_after_their_own_first_turn)), // состояние не должно измениться
        },
        // клиент №0 запрашивает сервер о статусе хода и получает информацию о том, что он завершён (начался следующий ход)
        // POST / OG_REFRESH_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;6;1;0;0;0;password;0;0;12711;A;1;100;10;0;2;0;Y;;0;;;BITRIX;0;0;0;0;0;0;0;14;620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "KDLAB;104;1;1;0;0;0;password;0;0;12711;A;1;100;10;1;2;2;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;1;2;0;N;0;0;0;0;0;0;0;14;620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;1;N;1;0;0;0;0;0;0;12;592#382#51#-1#892#329#61#-1#1534#377#51#-1#1945#949#48#-1#1882#1209#48#-1#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX",
            check_before: None,
            check_after: Some(Box::new(check::client_0_refresh_first_turn_and_began_next_step)),
        },
        // клиент №0 уведомляет сервер о том, что он принял сделанные ходы игроков, воспроизвёл и передаёт данные обсчёта
        // TODO:  пока что этот пакет полностью игнорируется,данные обсчёта всех клиентов надо сравнивать между собой для верификации данных
        // POST / OG_CONTROL_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;2;1;0;0;0;password;0;0;12711;A;1;100;10;1;2;1;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;1;2;0;N;0;21;0;16;14;0;1;0;;1;N;1;21;0;20;12;0;1;0;;BITRIX1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_0_refresh_first_turn_and_began_next_step)), // состояние не должно измениться
        },

        /* ХОД №2 */

        // клиент №0 нажимает Enter, редактирует ход, снова нажимает Enter и отправляет данные о ходах на сервер.
        // в данном случае этот ход завершающий (финиш, в игре выставлен 1 круг)
        // POST / OG_SEEDS_PACKET
        Action { // #11
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;3;1;0;0;0;password;0;0;12711;A;1;100;10;1;2;1;Y;;0;;;2;1;0;N;0;21;0;16;14;0;1;2;165#741#51#-1#465#427#51#-1;BITRIX1;0;;1;N;1;21;0;20;12;0;1;0;;BITRIX1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_0_sent_their_own_second_turn)),
        },
        // клиент №0 отправляет запрос на обновление данных о текущей игре (возможно, все игроки уже закончили ход?)
        // POST / OG_REFRESH_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;6;1;0;0;0;password;0;0;12711;A;1;100;10;1;2;0;Y;;0;;;BITRIX;0;21;0;16;14;0;1;2;165#741#51#-1#465#427#51#-1;BITRIX1;0;;1;N;1;21;0;20;12;0;1;0;;BITRIX1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "KDLAB;104;7;1;0;0;0;password;0;0;12711;A;1;100;10;1;0;1;Y;;0;;;2;1;0;N;0;21;0;16;14;0;1;2;165#741#51#-1#465#427#51#-1;BITRIX",
            check_before: None,
            check_after: Some(Box::new(check::client_0_sent_their_own_second_turn)), // состояние не должно измениться
        },
        // клиент №1 нажимает Enter, редактирует ход, снова нажимает Enter и отправляет данные о ходах на сервер.
        // в данном случае этот ход завершающий и для этого клиента тоже (финиш, в игре выставлен 1 круг)
        // POST / OG_SEEDS_PACKET
        Action {
            client: 1,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;3;1;0;0;1;password;0;0;12711;A;1;100;10;1;2;1;Y;;0;;;2;1;1;N;1;21;0;20;12;0;1;4;177#1100#48#-1#204#967#48#-1#214#647#51#-1#379#433#51#-1;BITRIXBITRIX#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_1_sent_their_own_second_turn)),
        },
        // клиент №1 отправляет запрос на получение обновленных данных о текущей игре,
        // в ответ приходят все ходы всех игроков (т.к. все игроки сделали свои ходы)
        // POST / OG_REFRESH_PACKET
        Action { // #14
            client: 1,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;6;1;0;0;1;password;0;0;12711;A;1;100;10;1;2;0;Y;;0;;;BITRIX;1;21;0;20;12;0;1;4;177#1100#48#-1#204#967#48#-1#214#647#51#-1#379#433#51#-1;BITRIXBITRIX#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX"),
            response: "KDLAB;104;1;1;0;0;1;password;0;0;12711;A;1;100;10;2;2;2;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;2;2;0;N;0;21;0;16;14;0;1;2;165#741#51#-1#465#427#51#-1;1;N;1;21;0;20;12;0;1;4;177#1100#48#-1#204#967#48#-1#214#647#51#-1#379#433#51#-1;BITRIX",
            check_before: None,
            check_after: Some(Box::new(check::client_1_refresh_after_their_own_second_turn)),
        },
        // клиент №1 уведомляет сервер о том, что он принял сделанные ходы игроков, воспроизвёл и передаёт данные обсчёта хода
        // TODO: пока что этот пакет полностью игнорируется, данные обсчёта всех клиентов надо сравнивать между собой для верификации данных
        // POST / OG_CONTROL_PACKET
        Action {
            client: 1,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;2;1;0;0;1;password;0;0;12711;A;1;100;10;2;2;1;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;2;2;0;Y;0;23;1;16;15;0;1;0;;1;N;1;23;0;20;12;0;1;0;;BITRIX#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_1_refresh_after_their_own_second_turn)), // ничего не поменялось
        },
        // клиент №0 отправляет запрос на получение обновленных данных о текущей игре,
        // в ответ приходят все ходы всех игроков (т.к. все игроки сделали свои ходы)
        // POST / OG_REFRESH_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;6;1;0;0;0;password;0;0;12711;A;1;100;10;1;2;0;Y;;0;;;BITRIX;0;21;0;16;14;0;1;2;165#741#51#-1#465#427#51#-1;BITRIX1;0;;1;N;1;21;0;20;12;0;1;0;;BITRIX1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "KDLAB;104;1;1;0;0;0;password;0;0;12711;A;1;100;10;2;2;2;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;2;2;0;N;0;21;0;16;14;0;1;2;165#741#51#-1#465#427#51#-1;1;N;1;21;0;20;12;0;1;4;177#1100#48#-1#204#967#48#-1#214#647#51#-1#379#433#51#-1;BITRIX",
            check_before: None,
            check_after: Some(Box::new(check::client_0_refresh_after_their_own_second_turn)),
        },
        // клиент №0 уведомляет сервер о том, что он принял сделанные ходы игроков, воспроизвёл и передаёт данные обсчёта хода
        // TODO: пока что этот пакет полностью игнорируется, данные обсчёта всех клиентов надо сравнивать между собой для верификации данных
        // POST / OG_CONTROL_PACKET
        Action {
            client: 0,
            url: "/game-on-line/default.asp",
            payload: Some("KDLAB;104;2;1;0;0;0;password;0;0;12711;A;1;100;10;2;2;1;Y;;0;;;0;player;1;1;1;1;N;1;player2;1;1;1;1;N;2;2;0;Y;0;23;1;16;15;0;1;0;;1;N;1;23;0;20;12;0;1;0;;BITRIX1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1;BITRIX"),
            response: "OK:KDLAB",
            check_before: None,
            check_after: Some(Box::new(check::client_0_refresh_after_their_own_second_turn)), // ничего не поменялось
        },
    ];

    for (i, action) in actions.into_iter().enumerate() {
        if let Some(ref check) = action.check_before {
            let game: RetGame = get_game().await;
            (check)(&game);
        }

        let req = match action.payload {
            None => test::TestRequest::get(),
            Some(payload) => test::TestRequest::post().set_payload(payload),
        }
        .uri(action.url)
        .to_request();

        let resp = String::from_utf8(test::call_and_read_body(&srv, req).await.to_vec()).unwrap();
        assert_eq!(
            &action.response, &resp,
            "unexpected response on iteration: `{i}` | {:#?}",
            &action
        );

        if let Some(ref check) = action.check_after {
            let game: RetGame = get_game().await;
            (check)(&game);
        }
    }
}

mod check {
    use super::PlayerTurnInfo;
    use super::RetGame;
    use entity::turn::Model as Turn;

    /// состояние игры после отправки хода клиентом №0
    pub(super) fn client_0_sent_their_own_first_turn(rs: &RetGame) {
        assert_eq!(2, rs.turns.len());
        assert_eq!(rs.turns[0].0.step_number, 1);
        assert_eq!(rs.turns[0].0.player_number, 0);
        assert_eq!(rs.turns[0].0.is_finished, false);
        assert_eq!(rs.turns[0].0.rank, 0);
        assert_eq!(rs.turns[0].0.move_time, 0);
        assert_eq!(rs.turns[0].0.move_steps, 0);
        assert_eq!(rs.turns[0].0.bottles_cnt, 0);
        assert_eq!(rs.turns[0].0.total_seeds_cnt, 0);
        assert_eq!(rs.turns[0].0.arcanes_cnt, 0);
        assert_eq!(rs.turns[0].0.destroys_cnt, 0);
        assert_eq!(rs.turns[0].0.user_seeds_cnt, 14);
        assert_eq!(rs.turns[0].0.seeds, Some(String::from("620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1")));
        assert!(rs.turns[1].0.seeds.is_none());
    }

    /// состояние игры сразу после отправки хода клиентом №1
    pub(super) fn client_1_sent_their_own_first_turn(rs: &RetGame) {
        assert_eq!(2, rs.turns.len());
        assert_eq!(rs.turns[0].0.step_number, 1);
        assert_eq!(rs.turns[0].0.player_number, 0);
        assert_eq!(rs.turns[0].0.is_finished, false);
        assert_eq!(rs.turns[0].0.rank, 0);
        assert_eq!(rs.turns[0].0.move_time, 0);
        assert_eq!(rs.turns[0].0.move_steps, 0);
        assert_eq!(rs.turns[0].0.bottles_cnt, 0);
        assert_eq!(rs.turns[0].0.total_seeds_cnt, 0);
        assert_eq!(rs.turns[0].0.arcanes_cnt, 0);
        assert_eq!(rs.turns[0].0.destroys_cnt, 0);
        assert_eq!(rs.turns[0].0.user_seeds_cnt, 14);
        assert_eq!(rs.turns[0].0.seeds, Some(String::from("620#402#51#-1#913#303#51#-1#1190#293#51#-1#1497#402#51#-1#1771#578#51#-1#1955#970#48#-1#1853#1225#48#-1#1727#1506#51#-1#1460#1766#102#-1#1105#3#102#-1#647#39#102#-1#533#1802#102#-1#353#1499#48#-1#211#1059#48#-1")));
        assert_eq!(rs.turns[0].0.is_received, false);
        assert_eq!(rs.turns[1].0.step_number, 1);
        assert_eq!(rs.turns[1].0.player_number, 1);
        assert_eq!(rs.turns[1].0.is_finished, false);
        assert_eq!(rs.turns[1].0.rank, 1);
        assert_eq!(rs.turns[1].0.move_time, 0);
        assert_eq!(rs.turns[1].0.move_steps, 0);
        assert_eq!(rs.turns[1].0.bottles_cnt, 0);
        assert_eq!(rs.turns[1].0.total_seeds_cnt, 0);
        assert_eq!(rs.turns[1].0.arcanes_cnt, 0);
        assert_eq!(rs.turns[1].0.destroys_cnt, 0);
        assert_eq!(rs.turns[1].0.user_seeds_cnt, 12);
        assert_eq!(rs.turns[1].0.seeds, Some(String::from("592#382#51#-1#892#329#61#-1#1534#377#51#-1#1945#949#48#-1#1882#1209#48#-1#1700#1528#51#-1#1564#1702#71#-1#1229#1941#102#-1#925#28#103#-1#756#49#102#-1#563#1842#102#-1#268#1233#48#-1")));
        assert_eq!(rs.turns[1].0.is_received, false);
    }

    /// состояние игры сразу после доставки информации об окончании хода клиенту №1
    pub(super) fn client_1_refresh_after_their_own_first_turn(rs: &RetGame) {
        assert_eq!(rs.turns.len(), 2);
        assert_eq!(rs.turns[0].0.is_received, false);
        assert_eq!(rs.turns[1].0.is_received, true);
    }

    /// состояние игры после отдачи информации о законченных хода клиенту №0
    /// теперь все клиенты уведомлены о ходах других игроков, ход считается завершённым
    pub(super) fn client_0_refresh_first_turn_and_began_next_step(rs: &RetGame) {
        assert_eq!(rs.turns.len(), 2);
        assert_eq!(rs.turns[0].0.is_received, true);
        assert_eq!(rs.turns[1].0.is_received, true);
    }

    /// состояние игры когда клиент #0 отправил свой второй ход, и в этот ход будет достигнут финиш
    pub(super) fn client_0_sent_their_own_second_turn(rs: &RetGame) {
        assert_eq!(3, rs.turns.len());

        assert_eq!(rs.turns[2].0.step_number, 2);
        assert_eq!(rs.turns[2].0.player_number, 0);
        assert_eq!(rs.turns[2].0.is_finished, false);
        assert_eq!(rs.turns[2].0.rank, 0);
        assert_eq!(rs.turns[2].0.move_time, 21);
        assert_eq!(rs.turns[2].0.move_steps, 0);
        assert_eq!(rs.turns[2].0.bottles_cnt, 16);
        assert_eq!(rs.turns[2].0.total_seeds_cnt, 14);
        assert_eq!(rs.turns[2].0.arcanes_cnt, 0);
        assert_eq!(rs.turns[2].0.destroys_cnt, 1);
        assert_eq!(rs.turns[2].0.user_seeds_cnt, 2);
        assert_eq!(
            rs.turns[2].0.seeds,
            Some(String::from("165#741#51#-1#465#427#51#-1"))
        );
        assert_eq!(rs.turns[2].0.is_received, false);
    }

    /// состояние игры когда клиент #1 отправил свой второй ход, и в этот ход будет достигнут финиш
    /// при этом клиент №0 уже отправил ход до этого. т.е. игра готова к обсчитыванию, ход считается завершённым
    /// впрочем, игра также считается завершённой
    pub(super) fn client_1_sent_their_own_second_turn(rs: &RetGame) {
        assert_eq!(4, rs.turns.len());

        assert_eq!(rs.turns[2].0.step_number, 2);
        assert_eq!(rs.turns[2].0.player_number, 0);
        assert_eq!(rs.turns[2].0.is_finished, false);
        assert_eq!(rs.turns[2].0.rank, 0);
        assert_eq!(rs.turns[2].0.move_time, 21);
        assert_eq!(rs.turns[2].0.move_steps, 0);
        assert_eq!(rs.turns[2].0.bottles_cnt, 16);
        assert_eq!(rs.turns[2].0.total_seeds_cnt, 14);
        assert_eq!(rs.turns[2].0.arcanes_cnt, 0);
        assert_eq!(rs.turns[2].0.destroys_cnt, 1);
        assert_eq!(rs.turns[2].0.user_seeds_cnt, 2);
        assert_eq!(
            rs.turns[2].0.seeds,
            Some(String::from("165#741#51#-1#465#427#51#-1"))
        );
        assert_eq!(rs.turns[2].0.is_received, false);
        assert_eq!(rs.turns[3].0.step_number, 2);
        assert_eq!(rs.turns[3].0.player_number, 1);
        assert_eq!(rs.turns[3].0.is_finished, false);
        assert_eq!(rs.turns[3].0.rank, 1);
        assert_eq!(rs.turns[3].0.move_time, 21);
        assert_eq!(rs.turns[3].0.move_steps, 0);
        assert_eq!(rs.turns[3].0.bottles_cnt, 20);
        assert_eq!(rs.turns[3].0.total_seeds_cnt, 12);
        assert_eq!(rs.turns[3].0.arcanes_cnt, 0);
        assert_eq!(rs.turns[3].0.destroys_cnt, 1);
        assert_eq!(rs.turns[3].0.user_seeds_cnt, 4);
        assert_eq!(
            rs.turns[3].0.seeds,
            Some(String::from(
                "177#1100#48#-1#204#967#48#-1#214#647#51#-1#379#433#51#-1"
            ))
        );
        assert_eq!(rs.turns[3].0.is_received, false);
    }

    /// клиент №1 запросил данные о ходе когда этот ход завершили все игроки
    pub(super) fn client_1_refresh_after_their_own_second_turn(rs: &RetGame) {
        assert_eq!(4, rs.turns.len());
        assert_eq!(rs.turns[2].0.is_received, false);
        assert_eq!(rs.turns[3].0.is_received, true);
    }

    /// клиент №0 запросил данные о ходе когда этот ход завершили все игроки
    pub(super) fn client_0_refresh_after_their_own_second_turn(rs: &RetGame) {
        assert_eq!(4, rs.turns.len());
        assert_eq!(rs.turns[2].0.is_received, true);
        assert_eq!(rs.turns[3].0.is_received, true);
    }
}
