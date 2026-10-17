use sqlx::{SqlitePool, migrate::Migrator};

use crate::backend::persistence::error::PersistenceError;

static MIGRATIONS: Migrator = sqlx::migrate!("./migrations");

pub async fn run_migrations(pool: SqlitePool) -> Result<(), PersistenceError> {
    MIGRATIONS.run(&pool).await?;
    pool.close().await;

    Ok(())
}
