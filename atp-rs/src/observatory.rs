//! Observatory location and time management

use crate::errors::{AtpError, Result};
use chrono::{DateTime, Duration, Utc};
use std::f64::consts::PI;

/// Observatory location and time tracking
///
/// Equivalent to Python's `Observatory` class (atp.py:197-225)
#[derive(Debug, Clone)]
pub struct Observatory {
    /// Latitude in radians
    pub latitude: f64,
    /// Longitude in radians
    pub longitude: f64,
    /// Height above sea level in meters
    pub height: f64,
    /// Current observation time (UTC)
    pub current_time: DateTime<Utc>,
    /// Start time of observation
    pub start_time: DateTime<Utc>,
    /// Time resolution for updates
    pub time_resolution: Duration,
}

impl Observatory {
    /// Create a new Observatory
    ///
    /// # Arguments
    /// * `latitude` - Latitude in degrees
    /// * `longitude` - Longitude in degrees
    /// * `height` - Height above sea level in meters
    /// * `time` - ISO 8601 datetime string (e.g., "2018-01-01T04:00:00")
    /// * `time_resolution_sec` - Time step in seconds
    pub fn new(
        latitude: f64,
        longitude: f64,
        height: f64,
        time: &str,
        time_resolution_sec: f64,
    ) -> Result<Self> {
        // Try parsing with timezone first, then without
        let current_time = DateTime::parse_from_rfc3339(time)
            .or_else(|_| {
                // Try adding 'Z' suffix for UTC
                let time_with_z = format!("{}Z", time);
                DateTime::parse_from_rfc3339(&time_with_z)
            })
            .map_err(|e| AtpError::Time(format!("Failed to parse time '{}': {}", time, e)))?
            .with_timezone(&Utc);

        let time_resolution = Duration::milliseconds((time_resolution_sec * 1000.0) as i64);

        Ok(Self {
            latitude: latitude.to_radians(),
            longitude: longitude.to_radians(),
            height,
            current_time,
            start_time: current_time,
            time_resolution,
        })
    }

    /// Update the current time by one time resolution step
    pub fn update(&mut self) {
        self.current_time = self.current_time + self.time_resolution;
    }

    /// Get Local Sidereal Time (LST) in radians
    ///
    /// This is a simplified calculation. For higher precision, consider using
    /// a full astronomical library.
    pub fn sidereal_time(&self) -> f64 {
        // Julian Date
        let jd = self.julian_date();

        // Greenwich Mean Sidereal Time (GMST) at 0h UT
        let t = (jd - 2451545.0) / 36525.0;
        let gmst0 = 280.46061837
            + 360.98564736629 * (jd - 2451545.0)
            + 0.000387933 * t * t
            - t * t * t / 38710000.0;

        // Normalize to [0, 360)
        let gmst0 = gmst0 % 360.0;
        let gmst0 = if gmst0 < 0.0 { gmst0 + 360.0 } else { gmst0 };

        // Convert to radians and add longitude
        let gmst_rad = gmst0.to_radians();
        let lst = gmst_rad + self.longitude;

        // Normalize to [0, 2π)
        let lst = lst % (2.0 * PI);
        if lst < 0.0 { lst + 2.0 * PI } else { lst }
    }

    /// Calculate Julian Date from current time
    fn julian_date(&self) -> f64 {
        // Unix timestamp to JD conversion
        let unix_timestamp = self.current_time.timestamp() as f64;
        let jd = (unix_timestamp / 86400.0) + 2440587.5;
        jd
    }

    /// Get latitude in degrees
    pub fn latitude_deg(&self) -> f64 {
        self.latitude.to_degrees()
    }

    /// Get longitude in degrees
    pub fn longitude_deg(&self) -> f64 {
        self.longitude.to_degrees()
    }
}

impl std::fmt::Display for Observatory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@(Observatory)> Location: lat={:.4}°, lon={:.4}°, height={:.1}m\n\
             @(Observatory)> Time: {}",
            self.latitude_deg(),
            self.longitude_deg(),
            self.height,
            self.current_time
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observatory_creation() {
        // Las Campanas Observatory coordinates
        let obs = Observatory::new(
            -29.049,
            -70.682,
            2514.0,
            "2018-01-01T04:00:00Z",
            60.0,
        )
        .unwrap();

        assert!((obs.latitude_deg() - (-29.049)).abs() < 1e-6);
        assert!((obs.longitude_deg() - (-70.682)).abs() < 1e-6);
        assert_eq!(obs.height, 2514.0);
    }

    #[test]
    fn test_time_update() {
        let mut obs = Observatory::new(
            -29.049,
            -70.682,
            2514.0,
            "2018-01-01T04:00:00Z",
            60.0,
        )
        .unwrap();

        let initial_time = obs.current_time;
        obs.update();

        let elapsed = obs.current_time - initial_time;
        assert_eq!(elapsed.num_seconds(), 60);
    }

    #[test]
    fn test_sidereal_time() {
        let obs = Observatory::new(
            -29.049,
            -70.682,
            2514.0,
            "2018-01-01T04:00:00Z",
            60.0,
        )
        .unwrap();

        let lst = obs.sidereal_time();

        // LST should be in [0, 2π]
        assert!(lst >= 0.0);
        assert!(lst < 2.0 * PI);
    }
}
