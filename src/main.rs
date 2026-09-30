use living_voices::{app::App, config::Config};

fn main() {
    env_logger::init();
    log_panics::init();

    let config = Config::new().expect("Failed to initialize config");
    log::info!("Config path: {}", config.config_path);

    let app = App::new();

    log::info!("Starting Living Voices");
    let demo = std::env::args().any(|arg| arg == "--demo");
    if demo && let Err(err) = app.seed_demo() {
        eprintln!("Could not seed demo world: {err}");
        std::process::exit(1);
    }

    if let Err(err) = app.run() {
        eprintln!("Application error: {err}");
        std::process::exit(1);
    }
}
