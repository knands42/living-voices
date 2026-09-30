use living_voices::{
    app::App,
    backend::{run_migrations, run_seeds, seed_titles},
    config::Config,
};

#[tokio::main]
async fn main() {
    env_logger::init();
    log_panics::init();

    let config = Config::new().expect("Failed to initialize config");
    log::info!("Config loaded from {}", config.config_file.display());

    // Run migrations before anything else touches the DB
    if let Err(e) = run_migrations(&config.db_path.to_string_lossy().as_ref()).await {
        eprintln!("Database migration failed: {e}");
        std::process::exit(1);
    }

    let app = App::new();
    log::info!("Starting Living Voices");

    let demo = std::env::args().any(|arg| arg == "--demo");
    if demo {
        log::info!("Seeding demo world: {}", seed_titles().join(", "));

        match run_seeds(&config.db_path.to_string_lossy().as_ref()).await {
            Ok(ids) => log::info!("Seeded {} adventures", ids.len()),
            Err(err) => {
                eprintln!("Could not seed demo world: {err}");
                std::process::exit(1);
            }
        }
    }

    if let Err(err) = app.run() {
        eprintln!("Application error: {err}");
        std::process::exit(1);
    }
}
