use ::sea_orm::{prelude::*, DbBackend, Schema};

use entity::prelude::*;

pub async fn setup_schema(db: &DbConn) -> Result<(), DbErr> {
    let schema = Schema::new(DbBackend::Sqlite);

    let stmts = [
        schema.create_table_from_entity(User),
        schema.create_table_from_entity(Game),
        // schema.create_table_from_entity(Player),
        schema.create_table_from_entity(Turn),
    ];

    for stmt in stmts {
        db.execute(db.get_database_backend().build(&stmt)).await?;
    }

    Ok(())
}
