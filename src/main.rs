use living_voices::{app::App, backend::{run_migrations, run_seeds}, config::Config};

#[tokio::main]
async fn main() {
    env_logger::init();
    log_panics::init();

    let config = Config::new().expect("Failed to initialize config");
    log::info!("Config loaded from {}", config.config_dir);

    // Run migrations before anything else touches the DB
    if let Err(e) = run_migrations(&config.db_path).await {
        eprintln!("Database migration failed: {e}");
        std::process::exit(1);
    }

    let app = App::new();
    log::info!("Starting Living Voices");

    let demo = std::env::args().any(|arg| arg == "--demo");
    if demo {
        if let Err(err) = run_seeds() {
            eprintln!("Could not seed demo world: {err}");
            std::process::exit(1);
        }
    }

    if let Err(err) = app.run() {
        eprintln!("Application error: {err}");
        std::process::exit(1);
    }
}
