use living_voices::backend::PersistenceError;
#[cfg(debug_assertions)]
use living_voices::backend::application::adventure_manager::AdventureManagerError;
use living_voices::backend::application::port::image_port::ImageError;
use living_voices::provision::entrypoint::{
    default_media_progress_logger, default_text_progress_logger, start_provisioning,
};
use living_voices::provision::{Gpu, start_engines};
use living_voices::{
    app::App,
    backend::{open_pool, run_migrations},
    cli,
    config::Config,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Frontend {
    Cli,
    Ui,
}

impl Frontend {
    fn from_args() -> Self {
        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--ui" | "-u" | "ui" => return Frontend::Ui,
                "--cli" | "-c" | "cli" => return Frontend::Cli,
                _ => {}
            }
        }
        Frontend::Ui
    }

    fn setup() -> bool {
        std::env::args().skip(1).any(|arg| arg == "--setup")
    }
}

#[derive(thiserror::Error, Debug)]
enum StartupError {
    #[error("failed to initialize config: {0}")]
    Config(#[source] std::io::Error),
    #[error("failed to open database pool: {0}")]
    Pool(#[source] PersistenceError),
    #[error("database migration failed: {0}")]
    Migration(#[source] PersistenceError),
    #[cfg(debug_assertions)]
    #[error("could not seed demo world: {0}")]
    Seed(#[source] PersistenceError),
    #[error("failed to initialize the app context: {0}")]
    Compose(#[source] ImageError),
    #[cfg(debug_assertions)]
    #[error("could not open seeded adventure: {0}")]
    LoadAdventure(#[source] AdventureManagerError),
    #[error("could not run the desktop frontend: {0}")]
    Ui(#[source] iced::Error),
    #[error("could not run the terminal frontend: {0}")]
    Cli(#[source] std::io::Error),
    #[error("failed to start engines: {0}")]
    StartEngines(String),
}

async fn prepare_database(config: &Config) -> Result<sqlx::SqlitePool, StartupError> {
    let pool = open_pool(config.db_path.to_string_lossy().as_ref())
        .await
        .map_err(StartupError::Pool)?;
    run_migrations(&pool)
        .await
        .map_err(StartupError::Migration)?;
    Ok(pool)
}

#[cfg(debug_assertions)]
async fn seed_and_load(config: &Config, app: &App) -> Result<(), StartupError> {
    use living_voices::backend::{run_seeds, seed_titles};

    log::info!("Seeding demo world: {}", seed_titles().join(", "));

    let adventures = run_seeds(config.db_path.to_string_lossy().as_ref())
        .await
        .map_err(StartupError::Seed)?;
    log::info!("Seeded {} adventures", adventures.len());

    Ok(())
}

// The terminal has no setup page, so it brings the engines up before drawing
async fn run_cli(app: &App, config: Config) -> Result<(), StartupError> {
    let mut settings = config.settings.clone();
    settings.models.image.enabled &= !config.needs_setup();

    start_provisioning(
        &settings,
        default_text_progress_logger(),
        default_media_progress_logger(),
    )
    .await
    .map_err(StartupError::StartEngines)?;
    let engines = start_engines(&settings)
        .await
        .map_err(StartupError::StartEngines)?;

    let ctx = app
        .context_for(&engines.stamp(config))
        .map_err(StartupError::Compose)?;
    tokio::task::block_in_place(|| cli::init::run(&ctx)).map_err(StartupError::Cli)
}

async fn run() -> Result<(), StartupError> {
    let config = Config::new().map_err(StartupError::Config)?;
    let frontend = Frontend::from_args();
    log::info!("Config loaded from {}", config.config_file.display());

    let gpu = Gpu::detect();
    log::info!("graphics: {} {}", gpu.card_label(), gpu.vram_label());

    let pool = prepare_database(&config).await?;
    let app = App::new(pool, &config).map_err(StartupError::Compose)?;

    #[cfg(debug_assertions)]
    seed_and_load(&config, &app).await?;

    log::info!("Starting Living Voices ({frontend:?})");
    match frontend {
        Frontend::Ui => {
            let open_setup = Frontend::setup() || config.needs_setup();
            app.run(config, gpu, open_setup).map_err(StartupError::Ui)
        }
        Frontend::Cli => run_cli(&app, config).await,
    }
}

#[tokio::main]
async fn main() {
    Config::init_logging();
    log_panics::init();

    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
