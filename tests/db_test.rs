use ::sea_orm::{
    ActiveModelTrait, Database, DbConn, DbErr, EntityTrait, LoaderTrait, ModelTrait, QuerySelect,
    Set,
};

use entity::game::GameType;

mod db;

#[tokio::test]
async fn db_main_seeding() -> Result<(), DbErr> {
    let db = Database::connect("sqlite::memory:").await?;
    db::setup_schema(&db).await?;

    seeding_users(&db).await?;
    seeding_new_game(&db, 1).await?;
    seeding_new_game(&db, 2).await?;
    add_player_to_game(&db, 1, 1).await.unwrap();
    add_player_to_game(&db, 2, 2).await.unwrap();
    add_player_to_game(&db, 1, 2).await.unwrap();
    add_player_to_game(&db, 3, 1).await.unwrap();
    add_player_to_game(&db, 4, 1).await.unwrap();

    view(&db).await.unwrap();

    Ok(())
}

async fn seeding_users(db: &DbConn) -> Result<(), DbErr> {
    let users = (1..6).map(|i| {
        let number = i * 100 + i * 10 + i;
        entity::user::ActiveModel {
            id: Set(i),
            steam_id: Set(number.into()),
            login: Set(Some(format!("login{number}"))),
            ..Default::default()
        }
    });

    for user in users {
        user.insert(db).await?;
    }

    Ok(())
}

async fn seeding_new_game(db: &DbConn, owner_id: usize) -> Result<(), DbErr> {
    let game = entity::game::ActiveModel {
        owner_id: Set(owner_id.try_into().unwrap()),
        world_id: Set(0),
        track_id: Set(0),
        game_type: Set(GameType::Winner),
        laps: Set(3),
        seeds: Set(15),
        duration: Set(100),
        is_express: Set(true),
        players_cnt: Set(5),
        ..Default::default()
    };

    game.insert(db).await?;

    Ok(())
}

async fn add_player_to_game(db: &DbConn, user_id: usize, game_id: u32) -> Result<(), DbErr> {
    let game = entity::game::Entity::find_by_id(game_id)
        .one(db)
        .await?
        .unwrap();
    dbg!("Finded game: {:#?}", &game);

    let turns = game
        .find_related(entity::turn::Entity)
        .find_also_related(entity::user::Entity)
        .group_by(entity::turn::Column::PlayerNumber)
        .all(db)
        .await?;

    // let players = game
    //     .find_related(entity::turn::Entity)
    //     .find_also_related(entity::user::Entity)
    //     // .order_by_desc(entity::turn::Column::StepNumber)
    //     .group_by(entity::turn::Column::PlayerNumber)
    //     .all(db)
    //     .await?;

    dbg!("Finded turns: {:#?}", &turns);

    assert!(game.players_cnt as usize > turns.len());

    let player = entity::turn::ActiveModel {
        user_id: Set(user_id.try_into().unwrap()),
        game_id: Set(game_id.try_into().unwrap()),
        player_number: Set(turns.len() as u32),
        step_number: Set(0),
        ..Default::default()
    };

    player.insert(db).await?;

    Ok(())
}

async fn view(db: &DbConn) -> Result<(), DbErr> {
    let games = entity::game::Entity::find()
        // .find_with_related(entity::turn::Entity)
        // .join(JoinType::LeftJoin, entity::turn::Relation::User.def())
        .all(db)
        .await?;

    let turns = games.load_many(entity::turn::Entity, db).await?;
    for (game, turns) in games.into_iter().zip(turns.into_iter()) {
        let users = turns.load_one(entity::user::Entity, db).await?;
        dbg!(&game);
        for (turn, user) in turns.into_iter().zip(users.into_iter()) {
            dbg!((turn, user));
        }
    }
    Ok(())
}
