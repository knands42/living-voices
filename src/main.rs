use living_voices::provision::entrypoint::start_provisioning;
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

#[tokio::main]
async fn main() {
    Config::init_logging();
    log_panics::init();
    let config = match Config::new() {
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

    let config = if frontend == Frontend::Ui && (Frontend::setup() || config.needs_setup()) {
        let fallback = config.clone();

        match ui::gate::gate_run(config, &gpu) {
            Ok(config) => config,
            Err(err) => {
                log::warn!("gate window failed: {err}");
                fallback
            }
        }
    } else {
        match start_provisioning(
            config,
            ui::gate::on_text_engine_callback(),
            ui::gate::on_media_engine_callback(),
        )
        .await
        {
            Ok(config) => config,
            Err(err) => {
                log::error!("provisioning failed: {err}");
                std::process::exit(1);
            }
        }
    };

    let pool = match open_pool(config.db_path.to_string_lossy().as_ref()).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Failed to open database pool: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = run_migrations(&pool).await {
        eprintln!("Database migration failed: {e}");
        std::process::exit(1);
    }

    let app = App::new(
        pool,
        config.settings.models.memory.clone(),
        config.settings.models.chat.clone(),
        config.settings.models.image.clone(),
        config.settings.memory.clone(),
        config.settings.director.clone(),
    )
    .expect("Failed to initialize the app context");

    #[cfg(debug_assertions)]
    {
        use living_voices::backend::run_seeds;

        log::info!("Seeding demo world: {}", seed_titles().join(", "));

        match run_seeds(config.db_path.to_string_lossy().as_ref()).await {
            Ok(adventures) => {
                log::info!("Seeded {} adventures", adventures.len());
                if let Some(first) = adventures.into_iter().next()
                    && let Err(err) = app.load_adventure(first).await
                {
                    eprintln!("Could not open seeded adventure: {err}");
                    std::process::exit(1);
                }
            }
            Err(err) => {
                eprintln!("Could not seed demo world: {err}");
                std::process::exit(1);
            }
        }
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
