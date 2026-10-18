use living_voices::{
    app::App,
    backend::{open_pool, run_migrations, seed_titles},
    config::Config,
};

#[tokio::main]
async fn main() {
    env_logger::init();
    log_panics::init();

    let config = Config::new().expect("Failed to initialize config");
    log::info!("Config loaded from {}", config.config_file.display());

    let pool = match open_pool(&config.db_path.to_string_lossy().as_ref()).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to open database pool: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = run_migrations(pool.clone()).await {
        eprintln!("Database migration failed: {e}");
        std::process::exit(1);
    }

    #[cfg(debug_assertions)]
    {
        use living_voices::backend::run_seeds;

        log::info!("Seeding demo world: {}", seed_titles().join(", "));

        match run_seeds(&config.db_path.to_string_lossy().as_ref()).await {
            Ok(ids) => log::info!("Seeded {} adventures", ids.len()),
            Err(err) => {
                eprintln!("Could not seed demo world: {err}");
                std::process::exit(1);
            }
        }

        log::info!("Demo mode: debug build — exiting after seed.");
    }

    let app = App::new(pool, config.settings.models.memory.clone());
    log::info!("Starting Living Voices");

    if let Err(err) = app.run() {
        eprintln!("Application error: {err}");
        std::process::exit(1);
    }
}
