//! Binary entry point. Setup lives in `lib.rs`.

// Release builds on Windows don't open a console window next to the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cactus_app::run();
}
