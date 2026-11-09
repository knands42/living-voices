use living_voices::provision::entrypoint::{
    Engines, default_media_progress_logger, default_text_progress_logger, start_provisioning,
};
use living_voices::{
    app::App,
    backend::{open_pool, run_migrations, seed_titles},
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
        for arg in std::env::args().skip(1) {
            if arg == "--setup" {
                return true;
            }
        }
        false
    }
}

/// The endpoints returned from text and media engines.
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

fn compose(pool: &sqlx::SqlitePool, config: &Config) -> Result<App, String> {
    App::new(
        pool.clone(),
        config.settings.models.memory.clone(),
        config.settings.models.chat.clone(),
        config.settings.models.image.clone(),
        config.settings.memory.clone(),
        config.settings.director.clone(),
    )
    .map_err(|err| err.to_string())
}

#[tokio::main]
async fn main() {
    Config::init_logging();
    log_panics::init();

    let mut config = match Config::new() {
        Ok(config) => config,
        Err(err) => {
            log::error!("Failed to initialize config: {err}");
            std::process::exit(1);
        }
    };

    let frontend = Frontend::from_args();
    log::info!("Config loaded from {}", config.config_file.display());

    let gpu = living_voices::provision::Gpu::detect();
    log::info!("graphics: {} {}", gpu.card_label(), gpu.vram_label());

    let mut engines = None;
    if frontend == Frontend::Ui && (Frontend::setup() || config.needs_setup()) {
        match ui::gate::gate_run(config.clone(), &gpu) {
            Ok(Some(outcome)) => {
                config = outcome.config;
                engines = outcome.engines;
            }
            Ok(None) => log::warn!("gate closed without an answer; using config.toml"),
            Err(err) => log::warn!("gate window failed: {err}"),
        }
    }

    // No gate, or it could not answer: start whatever the config asks for.
    if engines.is_none() {
        engines = match start_provisioning(
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
        };
    }

    let config = match engines.as_ref() {
        Some(engines) => stamp_endpoints(config, engines),
        None => config,
    };

    let pool = match open_pool(config.db_path.to_string_lossy().as_ref()).await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Failed to open database pool: {err}");
            std::process::exit(1);
        }
    };

    if let Err(err) = run_migrations(&pool).await {
        eprintln!("Database migration failed: {err}");
        std::process::exit(1);
    }

    #[cfg(debug_assertions)]
    let seeded_first = {
        use living_voices::backend::run_seeds;

        log::info!("Seeding demo world: {}", seed_titles().join(", "));

        match run_seeds(config.db_path.to_string_lossy().as_ref()).await {
            Ok(adventures) => {
                log::info!("Seeded {} adventures", adventures.len());
                adventures.into_iter().next()
            }
            Err(err) => {
                eprintln!("Could not seed demo world: {err}");
                std::process::exit(1);
            }
        }
    };
    #[cfg(not(debug_assertions))]
    let seeded_first = None;

    let app = match compose(&pool, &config) {
        Ok(app) => app,
        Err(err) => {
            eprintln!("Failed to initialize the app context: {err}");
            std::process::exit(1);
        }
    };

    #[cfg(debug_assertions)]
    if let Some(first) = seeded_first
        && let Err(err) = app.load_adventure(first).await
    {
        eprintln!("Could not open seeded adventure: {err}");
        std::process::exit(1);
    }

    log::info!("Starting Living Voices ({:?})", frontend);

    let result: Result<(), Box<dyn std::error::Error>> = match frontend {
        Frontend::Ui => app.run().map_err(Into::into),
        Frontend::Cli => tokio::task::block_in_place(|| cli::init::run(&app)).map_err(Into::into),
    };

    if let Err(err) = result {
        eprintln!("Application error: {err}");
        std::process::exit(1);
    }
}
