use living_voices::backend::PersistenceError;
#[cfg(debug_assertions)]
use living_voices::backend::application::adventure_manager::AdventureManagerError;
use living_voices::provision::Gpu;
use living_voices::provision::entrypoint::{
    Engines, default_media_progress_logger, default_text_progress_logger, start_provisioning,
};
use living_voices::{
    app::App,
    backend::{open_pool, run_migrations},
    cli,
    config::Config,
    ui,
};
use std::time::Duration;

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
    Compose(#[source] living_voices::backend::application::port::image_port::ImageError),
    #[cfg(debug_assertions)]
    #[error("could not open seeded adventure: {0}")]
    LoadAdventure(#[source] AdventureManagerError),
    #[error("application error: {0}")]
    Run(#[source] Box<dyn std::error::Error>),
}

fn stamp_endpoints(config: Config, engines: &Engines) -> Config {
    let config = match (engines.chat_url(), engines.memory_url()) {
        (Some(chat), Some(memory)) => config.with_text_endpoints(chat, memory),
        _ => config,
    };

    match engines.media_url() {
        Some(url) => config.with_media_engine(url),
        None => config,
    }
}

async fn resolve_engines(config: &Config, frontend: Frontend, gpu: &Gpu) -> Option<Engines> {
    if frontend == Frontend::Ui && (Frontend::setup() || config.needs_setup()) {
        match ui::gate::gate_run(config.clone(), gpu) {
            Ok(Some(outcome)) => return outcome.engines,
            Ok(None) => log::warn!("gate closed without an answer; using config.toml"),
            Err(err) => log::warn!("gate window failed: {err}"),
        }
    }

    match start_provisioning(
        &config.settings,
        default_text_progress_logger(),
        default_media_progress_logger(),
    )
    .await
    {
        Ok(engines) => Some(engines),
        Err(err) => {
            log::warn!("provisioning failed: {err}");
            None
        }
    }
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

    if let Some(adventure) = adventures.into_iter().next() {
        app.load_adventure(adventure)
            .await
            .map_err(StartupError::LoadAdventure)?;
    }

    Ok(())
}

fn compose(pool: &sqlx::SqlitePool, config: &Config) -> Result<App, StartupError> {
    App::new(
        pool.clone(),
        config.settings.models.memory.clone(),
        config.settings.models.chat.clone(),
        config.settings.models.image.clone(),
        config.settings.memory.clone(),
        config.settings.director.clone(),
    )
    .map_err(StartupError::Compose)
}

fn launch(app: &App, frontend: Frontend) -> Result<(), StartupError> {
    let result: Result<(), Box<dyn std::error::Error>> = match frontend {
        Frontend::Ui => app.run().map_err(Into::into),
        Frontend::Cli => tokio::task::block_in_place(|| cli::init::run(app)).map_err(Into::into),
    };
    result.map_err(StartupError::Run)
}

async fn run() -> Result<(), StartupError> {
    let mut config = Config::new().map_err(StartupError::Config)?;
    let frontend = Frontend::from_args();
    log::info!("Config loaded from {}", config.config_file.display());

    let gpu = Gpu::detect();
    log::info!("graphics: {} {}", gpu.card_label(), gpu.vram_label());

    if let Some(engines) = resolve_engines(&config, frontend, &gpu).await {
        config = stamp_endpoints(config, &engines);
    }

    let pool = prepare_database(&config).await?;
    let app = compose(&pool, &config)?;

    #[cfg(debug_assertions)]
    seed_and_load(&config, &app).await?;

    log::info!("Starting Living Voices ({frontend:?})");
    launch(&app, frontend)
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
