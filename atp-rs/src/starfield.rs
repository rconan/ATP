//! Star catalog management and coordinate transformations

use crate::coordinates::{rotation_y, rotation_z};
use crate::errors::{AtpError, Result};
use crate::observatory::Observatory;
use crate::target::Target;
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// A single star entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Star {
    /// Right ascension in radians
    pub ra: f64,
    /// Declination in radians
    pub dec: f64,
    /// V-band magnitude
    pub v_mag: f64,
    /// J-band magnitude
    pub j_mag: f64,
    /// Local x-coordinate (radians, relative to target)
    pub local_x: f64,
    /// Local y-coordinate (radians, relative to target)
    pub local_y: f64,
    /// Local z-coordinate (radians, relative to target)
    pub local_z: f64,
}

/// Star field catalog and transformations
///
/// Equivalent to Python's `StarField` class (atp.py:268-376)
#[derive(Debug, Clone)]
pub struct StarField {
    /// Stars in the field
    pub stars: Vec<Star>,
    /// V magnitude limit for filtering
    pub v_mag_limit: f64,
    /// Exclude radius from target in radians
    pub exclude_radius: Option<f64>,
    /// Color bands required
    pub color_bands: Vec<String>,
}

impl StarField {
    /// Create a new star field
    ///
    /// # Arguments
    /// * `v_mag_limit` - Maximum V magnitude
    /// * `exclude_radius_arcmin` - Optional exclusion radius in arcminutes
    /// * `color_bands` - Required color bands (e.g., ["V", "J"])
    pub fn new(
        v_mag_limit: f64,
        exclude_radius_arcmin: Option<f64>,
        color_bands: Vec<String>,
    ) -> Self {
        let exclude_radius = exclude_radius_arcmin.map(|r| r * PI / (180.0 * 60.0));

        Self {
            stars: Vec::new(),
            v_mag_limit,
            exclude_radius,
            color_bands,
        }
    }

    /// Add a star to the field
    pub fn add_star(&mut self, ra: f64, dec: f64, v_mag: f64, j_mag: f64) {
        self.stars.push(Star {
            ra,
            dec,
            v_mag,
            j_mag,
            local_x: 0.0,
            local_y: 0.0,
            local_z: 0.0,
        });
    }

    /// Query TIC catalog (async)
    ///
    /// This would typically use reqwest to query MAST/TIC
    /// For now, this is a placeholder for the async implementation
    #[allow(dead_code)]
    pub async fn query_catalog(
        &mut self,
        target: &Target,
        radius_arcmin: f64,
    ) -> Result<()> {
        // Placeholder for TIC catalog query
        // In production, this would:
        // 1. Query https://mast.stsci.edu/api/v0/invoke
        // 2. Parse the response
        // 3. Filter by magnitude and color
        // 4. Add stars to self.stars

        log::info!("Querying TIC catalog at RA={:.4}°, Dec={:.4}° with radius={:.2}' ",
                   target.ra_deg(), target.dec_deg(), radius_arcmin);

        // TODO: Implement actual HTTP query
        Err(AtpError::Catalog(
            "TIC catalog query not yet implemented".to_string(),
        ))
    }

    /// Load stars from YAML file (for testing/offline use)
    pub fn from_yaml(path: &str) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        let data: StarFieldYaml = serde_yaml::from_reader(file)?;

        let mut field = Self::new(
            18.0, // default
            None,
            vec!["V".to_string(), "J".to_string()],
        );

        for i in 0..data.ra.len() {
            field.add_star(
                data.ra[i].to_radians(),
                data.dec[i].to_radians(),
                data.v_mag[i],
                data.j_mag[i],
            );
        }

        Ok(field)
    }

    /// Apply magnitude and distance constraints
    pub fn apply_constraints(&mut self, target: &Target) {
        self.stars.retain(|star| {
            // Check V magnitude limit
            if star.v_mag > self.v_mag_limit {
                return false;
            }

            // Check exclude radius
            if let Some(exclude_rad) = self.exclude_radius {
                let dx = star.local_x;
                let dy = star.local_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < exclude_rad {
                    return false;
                }
            }

            // Check valid magnitudes (not NaN)
            if star.v_mag.is_nan() || star.j_mag.is_nan() {
                return false;
            }

            true
        });

        log::info!("Stars after constraints: {}", self.stars.len());
    }

    /// Update local coordinates for all stars
    ///
    /// Transforms stars from equatorial (RA/Dec) to local frame
    /// relative to target pointing (atp.py:355-369)
    pub fn update(&mut self, obs: &Observatory, target: &Target) {
        let lst = obs.sidereal_time();

        // Rotation matrix: local frame relative to target
        // R = Rz(rotator) * Ry(π/2 - alt) * Rz(az)
        let r = rotation_z(target.rotator_angle)
            * rotation_y(PI / 2.0 - target.alt)
            * rotation_z(target.az);

        for star in &mut self.stars {
            // Convert RA/Dec to Alt/Az
            let ha = lst - star.ra; // Hour angle

            let sin_alt = star.dec.sin() * obs.latitude.sin()
                + star.dec.cos() * obs.latitude.cos() * ha.cos();
            let alt = sin_alt.asin();

            let cos_az = (star.dec.sin() - obs.latitude.sin() * alt.sin())
                / (obs.latitude.cos() * alt.cos());
            let sin_az = -ha.sin() * star.dec.cos() / alt.cos();
            let az = sin_az.atan2(cos_az);

            // Convert to Cartesian
            let x = az.cos() * alt.cos();
            let y = az.sin() * alt.cos();
            let z = alt.sin();

            // Apply rotation matrix
            let v = Vector3::new(x, y, z);
            let local = r * v;

            star.local_x = local[0];
            star.local_y = local[1];
            star.local_z = local[2];
        }
    }

    /// Get distance of star(s) from origin in local coordinates
    ///
    /// # Arguments
    /// * `origin` - Origin in local frame [x, y] in radians
    /// * `star_indices` - Optional indices of stars to compute distance for
    ///
    /// # Returns
    /// Vector of distances in radians
    pub fn distance_from(&self, origin: [f64; 2], star_indices: Option<&[usize]>) -> Vec<f64> {
        let indices: Vec<usize> = match star_indices {
            Some(idx) => idx.to_vec(),
            None => (0..self.stars.len()).collect(),
        };

        indices
            .iter()
            .map(|&i| {
                let dx = self.stars[i].local_x - origin[0];
                let dy = self.stars[i].local_y - origin[1];
                (dx * dx + dy * dy).sqrt()
            })
            .collect()
    }

    /// Get number of stars in field
    pub fn len(&self) -> usize {
        self.stars.len()
    }

    /// Check if field is empty
    pub fn is_empty(&self) -> bool {
        self.stars.is_empty()
    }

    /// Get V magnitude range
    pub fn v_mag_range(&self) -> Option<(f64, f64)> {
        if self.stars.is_empty() {
            return None;
        }

        let min = self
            .stars
            .iter()
            .map(|s| s.v_mag)
            .fold(f64::INFINITY, f64::min);
        let max = self
            .stars
            .iter()
            .map(|s| s.v_mag)
            .fold(f64::NEG_INFINITY, f64::max);

        Some((min, max))
    }
}

/// YAML format for star field data
#[derive(Debug, Deserialize, Serialize)]
struct StarFieldYaml {
    ra: Vec<f64>,   // degrees
    dec: Vec<f64>,  // degrees
    v_mag: Vec<f64>,
    j_mag: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_starfield_creation() {
        let field = StarField::new(18.0, Some(3.0), vec!["V".to_string(), "J".to_string()]);

        assert_eq!(field.v_mag_limit, 18.0);
        assert!(field.exclude_radius.is_some());
        assert_eq!(field.color_bands.len(), 2);
        assert!(field.is_empty());
    }

    #[test]
    fn test_add_star() {
        let mut field = StarField::new(18.0, None, vec!["V".to_string()]);

        field.add_star(0.0, 0.0, 10.0, 9.5);
        assert_eq!(field.len(), 1);

        field.add_star(1.0, 0.5, 12.0, 11.5);
        assert_eq!(field.len(), 2);
    }

    #[test]
    fn test_magnitude_constraints() {
        let obs = Observatory::new(-29.049, -70.682, 2514.0, "2018-01-01T04:00:00Z", 60.0)
            .unwrap();
        let target = Target::from_altaz(45.0, 0.0, 0.0, &obs).unwrap();

        let mut field = StarField::new(15.0, None, vec!["V".to_string()]);

        field.add_star(0.0, 0.0, 10.0, 9.5);
        field.add_star(1.0, 0.5, 16.0, 11.5); // Too faint
        field.add_star(0.5, 0.2, 12.0, 11.0);

        field.update(&obs, &target);
        field.apply_constraints(&target);

        // Should have filtered out the faint star
        assert_eq!(field.len(), 2);
    }
}
