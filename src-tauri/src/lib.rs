pub mod auth;
pub mod cloud;
#[cfg(feature = "desktop")]
pub mod commands;
pub mod database;
pub mod downloads;
pub mod errors;
pub mod events;
pub mod sync;
#[cfg(feature = "desktop")]
pub fn run() {
    commands::run();
}
