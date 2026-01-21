//! Shared data models for ATP web UI

use serde::{Deserialize, Serialize};

/// Query parameters for star field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldQueryParams {
    /// Observatory time (UTC)
    pub time: String,
    /// Time resolution in seconds
    pub time_resolution: f64,
    /// Target name or coordinates
    pub target_name: Option<String>,
    /// Telescope altitude in degrees
    pub altitude: Option<f64>,
    /// Telescope azimuth in degrees
    pub azimuth: Option<f64>,
    /// V magnitude limit
    pub v_mag_limit: f64,
    /// Search radius in arcminutes
    pub radius: f64,
    /// Exclusion radius in arcminutes
    pub exclude_radius: f64,
}

impl Default for FieldQueryParams {
    fn default() -> Self {
        Self {
            time: "2018-01-01T04:00:00Z".to_string(),
            time_resolution: 60.0,
            target_name: None,
            altitude: Some(45.0),
            azimuth: Some(0.0),
            v_mag_limit: 16.0,
            radius: 10.0,
            exclude_radius: 3.0,
        }
    }
}

/// Star data for plotting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StarData {
    /// Star index
    pub idx: usize,
    /// Local X coordinate (arcmin)
    pub x: f64,
    /// Local Y coordinate (arcmin)
    pub y: f64,
    /// V magnitude
    pub v_mag: f64,
    /// J magnitude
    pub j_mag: f64,
    /// RA in degrees
    pub ra: f64,
    /// Dec in degrees
    pub dec: f64,
}

/// Probe data for visualization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProbeData {
    /// Probe index (0-3)
    pub idx: usize,
    /// Probe position X (arcmin)
    pub x: f64,
    /// Probe position Y (arcmin)
    pub y: f64,
    /// Assigned guide star index (if any)
    pub star_idx: Option<usize>,
    /// Probe type: "TT7" or "SH"
    pub probe_type: String,
}

/// Complete star field response
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldResponse {
    /// All stars in field
    pub stars: Vec<StarData>,
    /// Probe positions and assignments
    pub probes: Vec<ProbeData>,
    /// Target coordinates
    pub target_ra: f64,
    pub target_dec: f64,
    pub target_alt: f64,
    pub target_az: f64,
    /// Observatory time
    pub obs_time: String,
    /// Sidereal time (radians)
    pub sidereal_time: f64,
}

/// Guide star selection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideStarResult {
    /// TT7 guide star index
    pub tt7_star_idx: usize,
    /// TT7 probe index
    pub tt7_probe_idx: usize,
    /// TT7 RMS error (mas)
    pub tt7_error_mas: f64,
    /// SH guide star indices
    pub sh_star_indices: Vec<usize>,
    /// SH probe indices
    pub sh_probe_indices: Vec<usize>,
    /// SH median WFE (nm)
    pub sh_wfe_nm: f64,
    /// Updated field with probe assignments
    pub field: FieldResponse,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub message: String,
}
