//! ATP Web UI - Main entry point

mod app;
mod components;
mod models;
mod mock_data;

use app::App;
use leptos::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    // Set up logging
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).expect("Failed to initialize logger");

    log::info!("ATP Web UI starting...");

    mount_to_body(|| view! { <App/> })
}
