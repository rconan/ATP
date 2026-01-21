//! Target (pointing) management and coordinate transformations

use crate::coordinates::{altaz_to_equatorial, equatorial_to_altaz, parallactic_angle};
use crate::errors::{AtpError, Result};
use crate::observatory::Observatory;
use std::f64::consts::PI;

/// Target pointing and coordinate tracking
///
/// Equivalent to Python's `Target` class (atp.py:226-267)
#[derive(Debug, Clone)]
pub struct Target {
    /// Right ascension in radians (ICRS frame)
    pub ra: f64,
    /// Declination in radians (ICRS frame)
    pub dec: f64,
    /// Altitude in radians (horizontal frame)
    pub alt: f64,
    /// Azimuth in radians (horizontal frame)
    pub az: f64,
    /// Parallactic angle in radians
    pub parallactic_angle: f64,
    /// Rotator angle in radians
    pub rotator_angle: f64,
}

impl Target {
    /// Create target from RA/Dec coordinates
    ///
    /// # Arguments
    /// * `ra` - Right ascension in degrees
    /// * `dec` - Declination in degrees
    /// * `rotator_angle` - Initial rotator angle in degrees
    /// * `obs` - Observatory for coordinate transformation
    pub fn from_radec(
        ra: f64,
        dec: f64,
        rotator_angle: f64,
        obs: &Observatory,
    ) -> Result<Self> {
        let ra_rad = ra.to_radians();
        let dec_rad = dec.to_radians();
        let lst = obs.sidereal_time();

        let (alt, az) = equatorial_to_altaz(ra_rad, dec_rad, lst, obs.latitude);

        let ha = lst - ra_rad;
        let pa = parallactic_angle(ha, dec_rad, obs.latitude);

        Ok(Self {
            ra: ra_rad,
            dec: dec_rad,
            alt,
            az,
            parallactic_angle: pa,
            rotator_angle: rotator_angle.to_radians(),
        })
    }

    /// Create target from Alt/Az coordinates
    ///
    /// # Arguments
    /// * `alt` - Altitude in degrees
    /// * `az` - Azimuth in degrees
    /// * `rotator_angle` - Initial rotator angle in degrees
    /// * `obs` - Observatory for coordinate transformation
    pub fn from_altaz(
        alt: f64,
        az: f64,
        rotator_angle: f64,
        obs: &Observatory,
    ) -> Result<Self> {
        let alt_rad = alt.to_radians();
        let az_rad = az.to_radians();
        let lst = obs.sidereal_time();

        let (ra, dec) = altaz_to_equatorial(alt_rad, az_rad, lst, obs.latitude);

        let ha = lst - ra;
        let pa = parallactic_angle(ha, dec, obs.latitude);

        Ok(Self {
            ra,
            dec,
            alt: alt_rad,
            az: az_rad,
            parallactic_angle: pa,
            rotator_angle: rotator_angle.to_radians(),
        })
    }

    /// Update target coordinates for current observatory time
    ///
    /// # Arguments
    /// * `obs` - Observatory with current time
    /// * `update_rotator` - Whether to update rotator angle for field rotation
    pub fn update(&mut self, obs: &Observatory, update_rotator: bool) {
        let lst = obs.sidereal_time();

        // Update Alt-Az from RA-Dec
        let (alt, az) = equatorial_to_altaz(self.ra, self.dec, lst, obs.latitude);
        self.alt = alt;
        self.az = az;

        // Update parallactic angle
        let ha = lst - self.ra;
        self.parallactic_angle = parallactic_angle(ha, self.dec, obs.latitude);

        // Update rotator angle for field rotation
        if update_rotator {
            // Field rotation rate depends on telescope motion
            // Simplified calculation for alt-az mount
            let dt = obs.time_resolution.num_milliseconds() as f64 / 1000.0; // seconds
            let rotation_rate = obs.latitude.cos() * self.az.cos() / self.alt.cos();
            let delta_angle = rotation_rate * dt * (15.0_f64.to_radians() / 3600.0); // 15 arcsec/sec
            self.rotator_angle += delta_angle;
        }
    }

    /// Get RA in degrees
    pub fn ra_deg(&self) -> f64 {
        self.ra.to_degrees()
    }

    /// Get Dec in degrees
    pub fn dec_deg(&self) -> f64 {
        self.dec.to_degrees()
    }

    /// Get altitude in degrees
    pub fn alt_deg(&self) -> f64 {
        self.alt.to_degrees()
    }

    /// Get azimuth in degrees
    pub fn az_deg(&self) -> f64 {
        self.az.to_degrees()
    }

    /// Get rotator angle in degrees
    pub fn rotator_angle_deg(&self) -> f64 {
        self.rotator_angle.to_degrees()
    }

    /// Get parallactic angle in degrees
    pub fn parallactic_angle_deg(&self) -> f64 {
        self.parallactic_angle.to_degrees()
    }

    /// Get zenith distance in radians
    pub fn zenith_distance(&self) -> f64 {
        PI / 2.0 - self.alt
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@(Target)> RA/Dec: {:.4}°, {:.4}°\n\
             @(Target)> Alt/Az: {:.4}°, {:.4}°\n\
             @(Target)> Parallactic angle: {:.4}°",
            self.ra_deg(),
            self.dec_deg(),
            self.alt_deg(),
            self.az_deg(),
            self.parallactic_angle_deg()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_target_from_altaz() {
        let obs = Observatory::new(
            -29.049,
            -70.682,
            2514.0,
            "2018-01-01T04:00:00Z",
            60.0,
        )
        .unwrap();

        let target = Target::from_altaz(45.0, 0.0, 0.0, &obs).unwrap();

        assert!((target.alt_deg() - 45.0).abs() < 0.01);
        assert!((target.az_deg() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_target_update() {
        let mut obs = Observatory::new(
            -29.049,
            -70.682,
            2514.0,
            "2018-01-01T04:00:00Z",
            60.0,
        )
        .unwrap();

        let mut target = Target::from_altaz(45.0, 0.0, 0.0, &obs).unwrap();

        let initial_alt = target.alt;

        // Advance time by 1 hour
        for _ in 0..60 {
            obs.update();
            target.update(&obs, false);
        }

        // Target should have moved (alt/az changed)
        assert!((target.alt - initial_alt).abs() > 1e-6);
    }

    #[test]
    fn test_coordinate_round_trip() {
        let obs = Observatory::new(
            -29.049,
            -70.682,
            2514.0,
            "2018-01-01T04:00:00Z",
            60.0,
        )
        .unwrap();

        // Create from alt/az
        let target1 = Target::from_altaz(45.0, 30.0, 0.0, &obs).unwrap();

        // Create from resulting ra/dec
        let target2 = Target::from_radec(
            target1.ra_deg(),
            target1.dec_deg(),
            0.0,
            &obs,
        )
        .unwrap();

        // Should get back to same alt/az
        assert_relative_eq!(target1.alt, target2.alt, epsilon = 1e-8);
        assert_relative_eq!(target1.az, target2.az, epsilon = 1e-8);
    }
}
