//! Configuration file parsing (YAML)

use crate::errors::Result;
use serde::{Deserialize, Serialize};

/// Complete ATP configuration
///
/// Matches the structure of atp.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "Observation")]
    pub observation: ObservationConfig,

    #[serde(rename = "Observatory")]
    pub observatory: ObservatoryConfig,

    #[serde(rename = "Target")]
    pub target: TargetConfig,

    #[serde(rename = "Star Catalog")]
    pub star_catalog: StarCatalogConfig,

    #[serde(rename = "Telescope")]
    pub telescope: TelescopeConfig,

    #[serde(rename = "Atmosphere")]
    pub atmosphere: AtmosphereConfig,

    #[serde(rename = "TT7")]
    pub tt7: WfsConfig,

    #[serde(rename = "SH")]
    pub sh: WfsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationConfig {
    #[serde(rename = "time scale")]
    pub time_scale: String,

    pub time: String,

    #[serde(rename = "time resolution")]
    pub time_resolution: (Option<f64>, String),

    pub duration: (Option<f64>, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservatoryConfig {
    pub latitude: (f64, String),
    pub longitude: (f64, String),
    pub height: (f64, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    #[serde(rename = "pointing target")]
    pub pointing_target: Option<String>,

    #[serde(rename = "pointing ra/dec")]
    pub pointing_radec: Option<RaDecConfig>,

    #[serde(rename = "pointing alt/az")]
    pub pointing_altaz: Option<AltAzConfig>,

    #[serde(rename = "pointing accuracy")]
    pub pointing_accuracy: Option<f64>,

    #[serde(rename = "rotator angle")]
    pub rotator_angle: (f64, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaDecConfig {
    pub ra: (f64, String),
    pub dec: (f64, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltAzConfig {
    pub alt: (f64, String),
    pub az: (f64, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarCatalogConfig {
    #[serde(rename = "ra/dec error rms")]
    pub radec_error_rms: Option<f64>,

    pub radius: (f64, String),

    #[serde(rename = "exclude radius")]
    pub exclude_radius: (f64, String),

    #[serde(rename = "V magnitude limit")]
    pub v_magnitude_limit: f64,

    pub color: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelescopeConfig {
    pub diameter: (f64, String),
    pub area: f64, // m^2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtmosphereConfig {
    pub wavelength: (f64, String),
    pub r0: (f64, String),
    #[serde(rename = "L0")]
    pub l0: (f64, String),
    pub altitude: (Vec<f64>, String),
    pub fr0: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WfsConfig {
    #[serde(rename = "guide star")]
    pub guide_star: GuideStarConfig,

    pub optics: OpticsConfig,

    pub detector: DetectorConfig,

    pub control: ControlConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideStarConfig {
    pub wavelength: (f64, String),

    #[serde(rename = "zero point")]
    pub zero_point: f64, // ph/m^2/s
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpticsConfig {
    pub lenslet: LensletConfig,
    pub throughput: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LensletConfig {
    pub array: u32,

    #[serde(rename = "spot size")]
    pub spot_size: String,

    #[serde(rename = "pixel scale")]
    pub pixel_scale: Option<f64>,

    pub pixels: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    pub exposure: (f64, String),

    #[serde(rename = "quantum efficiency")]
    pub quantum_efficiency: f64,

    #[serde(rename = "read-out noise")]
    pub read_out_noise: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlConfig {
    pub gain: f64,
    pub latency: f64,
}

impl Config {
    /// Load configuration from YAML file
    pub fn from_file(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let config: Config = serde_yaml::from_reader(file)?;
        Ok(config)
    }

    /// Save configuration to YAML file
    pub fn to_file(&self, path: &str) -> Result<()> {
        let file = std::fs::File::create(path)?;
        serde_yaml::to_writer(file, self)?;
        Ok(())
    }

    /// Get time resolution in seconds
    pub fn time_resolution_sec(&self) -> f64 {
        let (value, unit) = &self.observation.time_resolution;
        let value = value.unwrap_or(60.0);
        match unit.as_str() {
            "second" => value,
            "minute" => value * 60.0,
            "hour" => value * 3600.0,
            _ => value,
        }
    }

    /// Get exclude radius in arcminutes
    pub fn exclude_radius_arcmin(&self) -> f64 {
        let (value, unit) = &self.star_catalog.exclude_radius;
        match unit.as_str() {
            "arcmin" => *value,
            "degree" => value * 60.0,
            "arcsec" => value / 60.0,
            _ => *value,
        }
    }

    /// Get catalog search radius in arcminutes
    pub fn catalog_radius_arcmin(&self) -> f64 {
        let (value, unit) = &self.star_catalog.radius;
        match unit.as_str() {
            "arcmin" => *value,
            "degree" => value * 60.0,
            "arcsec" => value / 60.0,
            _ => *value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        // Test with the actual atp.yaml file
        let config_path = "../atp.yaml";

        if std::path::Path::new(config_path).exists() {
            let config = Config::from_file(config_path).unwrap();

            assert_eq!(config.observation.time_scale, "UTC");
            assert!(config.observatory.latitude.0 < 0.0); // Southern hemisphere
            assert_eq!(config.telescope.diameter.1, "m");
            assert_eq!(config.star_catalog.color.len(), 2);
        }
    }

    #[test]
    fn test_time_resolution_conversion() {
        let yaml_str = r#"
Observation:
  time scale: 'UTC'
  time: '2018-01-01T04:00:00'
  time resolution: [60, 'second']
  duration: [null, 'second']
Observatory:
  latitude: [-29.049, 'degree']
  longitude: [-70.682, 'degree']
  height: [2514, 'meter']
Target:
  pointing target: null
  pointing ra/dec: null
  pointing alt/az:
    alt: [45, 'degree']
    az: [0, 'degree']
  pointing accuracy: null
  rotator angle: [0, 'degree']
Star Catalog:
  ra/dec error rms: null
  radius: [10, 'arcmin']
  exclude radius: [3, 'arcmin']
  V magnitude limit: 18
  color: [V, J]
Telescope:
  diameter: [25.5, 'm']
  area: 367
Atmosphere:
  wavelength: [500, 'nm']
  r0: [16, 'cm']
  L0: [25, 'm']
  altitude: [[25, 275], 'm']
  fr0: [0.1, 0.9]
TT7:
  guide star:
    wavelength: [715, 'nm']
    zero point: 24.46e9
  optics:
    lenslet:
      array: 7
      spot size: seeing limited
      pixel scale: null
      pixels: null
    throughput: 0.48
  detector:
    exposure: [5, 'ms']
    quantum efficiency: 0.8
    read-out noise: 0
  control:
    gain: 0
    latency: 0
SH:
  guide star:
    wavelength: [715, 'nm']
    zero point: 24.46e9
  optics:
    lenslet:
      array: 48
      spot size: seeing limited
      pixel scale: null
      pixels: null
    throughput: 0.48
  detector:
    exposure: [30, 's']
    quantum efficiency: 0.8
    read-out noise: 0
  control:
    gain: 0
    latency: 0
"#;

        let config: Config = serde_yaml::from_str(yaml_str).unwrap();
        assert_eq!(config.time_resolution_sec(), 60.0);
        assert_eq!(config.exclude_radius_arcmin(), 3.0);
        assert_eq!(config.catalog_radius_arcmin(), 10.0);
    }
}
