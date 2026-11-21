#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    drone_cloud_sync::run();
}
