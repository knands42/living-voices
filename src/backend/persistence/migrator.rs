use sqlx::migrate::Migrator;

use crate::backend::persistence::{db::open_pool, error::PersistenceError};

static MIGRATIONS: Migrator = sqlx::migrate!("./migrations");

pub async fn run_migrations(db_path: &str) -> Result<(), PersistenceError> {
    let pool = open_pool(db_path).await?;

    MIGRATIONS.run(&pool).await?;
    pool.close().await;

    Ok(())
}
