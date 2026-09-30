use sqlx::sqlite::SqlitePoolOptions;

const MIGRATIONS: sqlx::MigrateInstance = sqlx::migrate!("./migrations");

pub async fn run_migrations(db_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(db_path)
        .await?;

    MIGRATIONS.run(&pool).await?;
    pool.close();
    Ok(())
}
