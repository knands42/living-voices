use living_voices::backend::PersistenceError;
#[cfg(debug_assertions)]
use living_voices::backend::application::adventure_manager::AdventureManagerError;
use living_voices::provision::entrypoint::{
    MediaEngine, TextEngine, default_media_progress_logger, default_text_progress_logger,
    start_provisioning,
};
use living_voices::provision::{Gpu, start_media_engine, start_text_engine};
use living_voices::{
    app::App,
    backend::{open_pool, run_migrations},
    cli,
    config::Config,
    ui,
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
    Compose(#[source] living_voices::backend::application::port::image_port::ImageError),
    #[cfg(debug_assertions)]
    #[error("could not open seeded adventure: {0}")]
    LoadAdventure(#[source] AdventureManagerError),
    #[error("application error: {0}")]
    Run(#[source] Box<dyn std::error::Error>),
    #[error("failed to load engines {0}")]
    BootEngines(String),
    #[error("failed to start engines {0}")]
    StartEngines(String),
}

fn stamp_engine_endpoints(
    config: Config,
    text_engine: &TextEngine,
    media_engine: &MediaEngine,
) -> Config {
    config
        .with_text_endpoints(text_engine.chat_url(), text_engine.memory_url())
        .with_media_engine(media_engine.media_url())
}

async fn provision_engines(
    config: &Config,
    frontend: Frontend,
    gpu: &Gpu,
) -> Result<(), StartupError> {
    if frontend == Frontend::Ui && (Frontend::setup() || config.needs_setup()) {
        match ui::gate::gate_run(config.clone(), gpu) {
            Ok(_) => {
                log::info!("gate window finished successfully");
                return Ok(());
            }
            Err(err) => return Err(StartupError::BootEngines(err.to_string())),
        }
    }

    match start_provisioning(
        &config.settings,
        default_text_progress_logger(),
        default_media_progress_logger(),
    )
    .await
    {
        Ok(_) => {
            log::info!("provisioning finished successfully");
            Ok(())
        }
        Err(err) => Err(StartupError::BootEngines(err.to_string())),
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

    provision_engines(&config, frontend, &gpu).await?;
    let text_engine = start_text_engine()
        .await
        .map_err(StartupError::StartEngines)?;
    let media_engine = start_media_engine()
        .await
        .map_err(StartupError::StartEngines)?;
    let updated_config = stamp_engine_endpoints(config, &text_engine, &media_engine);

    let pool = prepare_database(&updated_config).await?;
    let app = compose(&pool, &updated_config)?;

    #[cfg(debug_assertions)]
    seed_and_load(&updated_config, &app).await?;

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
