//! Physical and conversion constants

use std::f64::consts::PI;

/// Radians to milliarcseconds
pub const RAD2MAS: f64 = 180.0 * 3600.0 * 1000.0 / PI;

/// Radians to arcseconds
pub const RAD2ARCSEC: f64 = 180.0 * 3600.0 / PI;

/// Arcseconds to radians
pub const ARCSEC2RAD: f64 = PI / (180.0 * 3600.0);

/// Arcminutes to radians
pub const ARCMIN2RAD: f64 = PI / (180.0 * 60.0);

/// Degrees to radians
pub const DEG2RAD: f64 = PI / 180.0;

/// Radians to degrees
pub const RAD2DEG: f64 = 180.0 / PI;
