//! ATP-RS: AGWS Target Practice (Rust implementation)
//!
//! This library provides tools for selecting optimal guide stars for the
//! Giant Magellan Telescope's Acquisition, Guiding & Wavefront Sensing system.

pub mod constants;
pub mod coordinates;
pub mod observatory;
pub mod target;
pub mod starfield;
pub mod probe;
pub mod config;
pub mod errors;
pub mod wavefront;
pub mod guidestar;
pub mod catalog;
pub mod monte_carlo;

// Re-export main types
pub use observatory::Observatory;
pub use target::Target;
pub use starfield::StarField;
pub use probe::Probe;
pub use config::Config;
pub use errors::{AtpError, Result};
