use living_voices::{
    app::App,
    backend::{open_pool, run_migrations, seed_titles},
    cli,
    config::Config,
};

#[derive(Clone, Copy, Debug)]
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
        Frontend::Cli
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log_panics::init();

    let frontend = Frontend::from_args();

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

    let app = App::new(
        pool,
        config.settings.models.memory.clone(),
        config.settings.models.chat.clone(),
        config.settings.memory.clone(),
        config.settings.director.clone(),
    );

    #[cfg(debug_assertions)]
    {
        use living_voices::backend::run_seeds;

        log::info!("Seeding demo world: {}", seed_titles().join(", "));

        match run_seeds(&config.db_path.to_string_lossy().as_ref()).await {
            Ok(adventures) => {
                log::info!("Seeded {} adventures", adventures.len());
                if let Some(first) = adventures.into_iter().next() {
                    if let Err(err) = app.load_adventure(first) {
                        eprintln!("Could not open seeded adventure: {err}");
                        std::process::exit(1);
                    }
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
        Frontend::Cli => cli::init::run(&app).map_err(Into::into),
    };

    if let Err(err) = result {
        eprintln!("Application error: {err}");
        std::process::exit(1);
    }
}
