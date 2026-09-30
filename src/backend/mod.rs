pub mod application;
pub mod domain;
mod persistence;

pub use persistence::migrator::run_migrations;
pub use persistence::seeder::run_seeds;
