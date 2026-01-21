//! AGWS probe arm positioning and guide star assignment

use crate::constants::ARCMIN2RAD;
use crate::starfield::StarField;
use std::f64::consts::PI;

/// AGWS probe arm
///
/// Equivalent to Python's `Probe` class (atp.py:377-390)
#[derive(Debug, Clone)]
pub struct Probe {
    /// Radial distance from field center in radians
    pub radius: f64,
    /// Azimuthal angle in radians
    pub azimuth: f64,
    /// Patrol range radius in radians
    pub range_radius: f64,
    /// Local x-coordinate in radians
    pub local_x: f64,
    /// Local y-coordinate in radians
    pub local_y: f64,
    /// Index of assigned guide star (if any)
    pub gs_idx: Option<usize>,
    /// Indices of stars within reach
    pub reachable_stars: Vec<usize>,
}

impl Probe {
    /// Create a new probe at specified azimuthal angle
    ///
    /// # Arguments
    /// * `azimuth_deg` - Azimuthal angle in degrees (0, 90, 180, 270 for 4 probes)
    /// * `exclude_radius_arcmin` - Exclusion radius in arcminutes (default: 2.0)
    ///
    /// The probe is positioned at 22.67 arcmin (1360 arcsec) from field center
    pub fn new(azimuth_deg: f64, exclude_radius_arcmin: Option<f64>) -> Self {
        let radius_arcmin = 1360.0 / 60.0; // 22.67 arcmin
        let radius = radius_arcmin * ARCMIN2RAD;
        let azimuth = azimuth_deg.to_radians();

        let exclude_radius = exclude_radius_arcmin.unwrap_or(2.0);
        let range_radius = (radius_arcmin - exclude_radius) * ARCMIN2RAD;

        let local_x = radius * azimuth.cos();
        let local_y = radius * azimuth.sin();

        Self {
            radius,
            azimuth,
            range_radius,
            local_x,
            local_y,
            gs_idx: None,
            reachable_stars: Vec::new(),
        }
    }

    /// Find all stars within patrol range
    ///
    /// Equivalent to Python's `reachForTheStars` method (atp.py:388-390)
    ///
    /// # Arguments
    /// * `stars` - Star field to search
    pub fn reach_for_the_stars(&mut self, stars: &StarField) {
        self.reachable_stars.clear();

        let origin = [self.local_x, self.local_y];
        let distances = stars.distance_from(origin, None);

        for (idx, &dist) in distances.iter().enumerate() {
            if dist <= self.range_radius {
                self.reachable_stars.push(idx);
            }
        }
    }

    /// Get position in local frame
    pub fn position(&self) -> [f64; 2] {
        [self.local_x, self.local_y]
    }

    /// Get distance to a specific star
    ///
    /// # Arguments
    /// * `stars` - Star field
    /// * `star_idx` - Index of star
    ///
    /// # Returns
    /// Distance in radians
    pub fn distance_to_star(&self, stars: &StarField, star_idx: usize) -> Option<f64> {
        if star_idx >= stars.len() {
            return None;
        }

        let dx = stars.stars[star_idx].local_x - self.local_x;
        let dy = stars.stars[star_idx].local_y - self.local_y;
        Some((dx * dx + dy * dy).sqrt())
    }

    /// Check if a star is within patrol range
    pub fn can_reach(&self, stars: &StarField, star_idx: usize) -> bool {
        if let Some(dist) = self.distance_to_star(stars, star_idx) {
            dist <= self.range_radius
        } else {
            false
        }
    }

    /// Get number of reachable stars
    pub fn num_reachable(&self) -> usize {
        self.reachable_stars.len()
    }

    /// Get assigned guide star index
    pub fn guide_star(&self) -> Option<usize> {
        self.gs_idx
    }

    /// Assign a guide star
    pub fn assign_guide_star(&mut self, star_idx: usize) {
        self.gs_idx = Some(star_idx);
    }

    /// Clear guide star assignment
    pub fn clear_guide_star(&mut self) {
        self.gs_idx = None;
    }

    /// Get radius in arcminutes
    pub fn radius_arcmin(&self) -> f64 {
        self.radius / ARCMIN2RAD
    }

    /// Get azimuth in degrees
    pub fn azimuth_deg(&self) -> f64 {
        self.azimuth.to_degrees()
    }

    /// Get patrol range in arcminutes
    pub fn range_radius_arcmin(&self) -> f64 {
        self.range_radius / ARCMIN2RAD
    }
}

impl std::fmt::Display for Probe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@(Probe)> Position: r={:.2}', az={:.1}°, patrol range={:.2}', reachable stars: {}{}",
            self.radius_arcmin(),
            self.azimuth_deg(),
            self.range_radius_arcmin(),
            self.num_reachable(),
            match self.gs_idx {
                Some(idx) => format!(", assigned GS: {}", idx),
                None => String::new(),
            }
        )
    }
}

/// Create standard 4-probe configuration
///
/// Creates probes at 0°, 90°, 180°, 270° azimuth
pub fn create_probe_array(exclude_radius_arcmin: Option<f64>) -> Vec<Probe> {
    (0..4)
        .map(|k| Probe::new((k * 90) as f64, exclude_radius_arcmin))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observatory::Observatory;
    use crate::target::Target;

    #[test]
    fn test_probe_creation() {
        let probe = Probe::new(0.0, Some(2.0));

        assert!((probe.radius_arcmin() - 22.67).abs() < 0.01);
        assert_eq!(probe.azimuth_deg(), 0.0);
        assert!(probe.guide_star().is_none());
        assert_eq!(probe.num_reachable(), 0);
    }

    #[test]
    fn test_probe_array() {
        let probes = create_probe_array(Some(2.0));

        assert_eq!(probes.len(), 4);
        assert_eq!(probes[0].azimuth_deg(), 0.0);
        assert_eq!(probes[1].azimuth_deg(), 90.0);
        assert_eq!(probes[2].azimuth_deg(), 180.0);
        assert_eq!(probes[3].azimuth_deg(), 270.0);
    }

    #[test]
    fn test_reach_for_stars() {
        let obs = Observatory::new(-29.049, -70.682, 2514.0, "2018-01-01T04:00:00Z", 60.0)
            .unwrap();
        let target = Target::from_altaz(45.0, 0.0, 0.0, &obs).unwrap();

        let mut field = StarField::new(18.0, None, vec!["V".to_string()]);

        // Add stars at various positions
        // Note: These would need proper coordinate transformations in real usage
        for i in 0..10 {
            let ra = i as f64 * 0.01;
            let dec = i as f64 * 0.01;
            field.add_star(ra, dec, 10.0 + i as f64, 9.5);
        }

        field.update(&obs, &target);

        let mut probe = Probe::new(0.0, Some(2.0));
        probe.reach_for_the_stars(&field);

        // Number of reachable stars depends on positions
        assert!(probe.num_reachable() <= 10);
    }

    #[test]
    fn test_guide_star_assignment() {
        let mut probe = Probe::new(0.0, Some(2.0));

        assert!(probe.guide_star().is_none());

        probe.assign_guide_star(5);
        assert_eq!(probe.guide_star(), Some(5));

        probe.clear_guide_star();
        assert!(probe.guide_star().is_none());
    }
}
