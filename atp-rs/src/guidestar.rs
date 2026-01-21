//! Guide star selection algorithms
//!
//! This module implements the optimal guide star selection for both TT7 and SH sensors.

use crate::config::Config;
use crate::errors::{AtpError, Result};
use crate::probe::Probe;
use crate::starfield::StarField;
use crate::target::Target;
use crate::wavefront::tt7_tt_error;
use std::f64::consts::PI;

/// Result of TT7 guide star selection
#[derive(Debug, Clone)]
pub struct TT7GuideStarResult {
    /// Index of selected star in starfield
    pub star_idx: usize,
    /// Index of assigned probe
    pub probe_idx: usize,
    /// Expected RMS tilt error (mas)
    pub rms_error_mas: f64,
    /// Distance from probe center (arcmin)
    pub distance_arcmin: f64,
}

/// Result of SH guide star triplet selection
#[derive(Debug, Clone)]
pub struct SHGuideStarResult {
    /// Indices of selected stars (3 stars)
    pub star_indices: [usize; 3],
    /// Assigned probe indices (3 probes)
    pub probe_indices: [usize; 3],
    /// Median wavefront error from Monte Carlo (nm RMS)
    pub median_wfe_nm: f64,
    /// Azimuthal separation quality metric
    pub azimuth_metric: f64,
}

/// Find optimal TT7 guide star
///
/// Equivalent to Python's tt7 guide star selection (atp.py:402-413)
///
/// # Arguments
/// * `probes` - Available AGWS probes (will be searched for reachable stars)
/// * `stars` - Star field with local coordinates
/// * `target` - Target for zenith distance calculation
/// * `config` - Configuration for TT7 parameters
///
/// # Returns
/// Best TT7 guide star selection result
pub fn find_tt7_guide_star(
    probes: &[Probe],
    stars: &StarField,
    target: &Target,
    config: &Config,
) -> Result<TT7GuideStarResult> {
    if stars.is_empty() {
        return Err(AtpError::Catalog("No stars available".to_string()));
    }

    let zenith_distance = target.zenith_distance();
    let mut best_error = f64::INFINITY;
    let mut best_star_idx = 0;
    let mut best_probe_idx = 0;

    // Calculate TT7 error for all stars
    let mut tt_errors: Vec<(usize, f64)> = Vec::new();

    for (star_idx, star) in stars.stars.iter().enumerate() {
        // Calculate angular separation from target in arcmin
        let dx = star.local_x;
        let dy = star.local_y;
        let separation_rad = (dx * dx + dy * dy).sqrt();
        let separation_arcmin = separation_rad * 180.0 * 60.0 / PI;

        // Calculate expected error
        let error = tt7_tt_error(
            separation_arcmin,
            star.v_mag,
            zenith_distance,
            config,
        )?;

        tt_errors.push((star_idx, error));
    }

    // Sort by error (ascending)
    tt_errors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Find best star that is reachable by a probe
    for (star_idx, error) in tt_errors {
        // Check which probes can reach this star
        for (probe_idx, probe) in probes.iter().enumerate() {
            if probe.can_reach(stars, star_idx) {
                best_star_idx = star_idx;
                best_error = error;
                best_probe_idx = probe_idx;

                // Calculate distance from probe
                let distance = probe.distance_to_star(stars, star_idx)
                    .unwrap_or(0.0);
                let distance_arcmin = distance * 180.0 * 60.0 / PI;

                return Ok(TT7GuideStarResult {
                    star_idx,
                    probe_idx,
                    rms_error_mas: best_error,
                    distance_arcmin,
                });
            }
        }
    }

    Err(AtpError::Catalog(
        "No reachable TT7 guide star found".to_string(),
    ))
}

/// Find optimal SH guide star triplet
///
/// Equivalent to Python's `SH_GSs` function (atp.py:75-196)
///
/// This is a simplified version without Monte Carlo simulation.
/// The full implementation with crseo would be added when CUDA is available.
///
/// # Arguments
/// * `probes` - Available AGWS probes (3 probes after TT7 assignment)
/// * `stars` - Star field with local coordinates
/// * `target` - Target for coordinate calculations
/// * `config` - Configuration for SH parameters
/// * `n_candidates` - Number of triplet candidates to evaluate
///
/// # Returns
/// Best SH guide star triplet selection result
pub fn find_sh_guide_stars(
    probes: &[Probe],
    stars: &StarField,
    target: &Target,
    config: &Config,
    n_candidates: usize,
) -> Result<SHGuideStarResult> {
    if probes.len() < 3 {
        return Err(AtpError::Config(
            "Need at least 3 probes for SH triplet".to_string(),
        ));
    }

    // Get stars reachable by any probe
    let mut reachable_mask = vec![false; stars.len()];
    for probe in probes {
        for &idx in &probe.reachable_stars {
            reachable_mask[idx] = true;
        }
    }

    let reachable_indices: Vec<usize> = reachable_mask
        .iter()
        .enumerate()
        .filter_map(|(i, &r)| if r { Some(i) } else { None })
        .collect();

    if reachable_indices.len() < 3 {
        return Err(AtpError::Catalog(
            "Not enough reachable stars for SH triplet".to_string(),
        ));
    }

    // Find triplets with ~120° azimuthal separation
    let target_separation = 2.0 * PI / 3.0; // 120 degrees

    let mut best_triplet: Option<SHGuideStarResult> = None;
    let mut n_tested = 0;

    for &idx1 in &reachable_indices {
        if n_tested >= n_candidates {
            break;
        }

        let star1 = &stars.stars[idx1];
        let azimuth1 = star1.local_y.atan2(star1.local_x);

        // Find stars near +120° and -120°
        let mut best_az_error = f64::INFINITY;
        let mut best_idx2 = 0;
        let mut best_idx3 = 0;

        for &idx2 in &reachable_indices {
            if idx2 == idx1 {
                continue;
            }

            let star2 = &stars.stars[idx2];
            let azimuth2 = star2.local_y.atan2(star2.local_x);
            let delta_az2 = normalize_angle(azimuth2 - azimuth1);

            for &idx3 in &reachable_indices {
                if idx3 == idx1 || idx3 == idx2 {
                    continue;
                }

                let star3 = &stars.stars[idx3];
                let azimuth3 = star3.local_y.atan2(star3.local_x);
                let delta_az3 = normalize_angle(azimuth3 - azimuth1);

                // Check if roughly 120° separated
                let error2 = (delta_az2 - target_separation).abs();
                let error3 = (delta_az3 + target_separation).abs();
                let az_error = error2 + error3;

                if az_error < best_az_error {
                    best_az_error = az_error;
                    best_idx2 = idx2;
                    best_idx3 = idx3;
                }
            }
        }

        // Check if this triplet can be assigned to probes
        let indices = [idx1, best_idx2, best_idx3];
        if let Some(probe_assignment) = assign_triplet_to_probes(&indices, probes, stars) {
            // Estimate wavefront error (simplified without Monte Carlo)
            let wfe_estimate = estimate_triplet_wfe(&indices, stars, target, config)?;

            if best_triplet.is_none()
                || wfe_estimate < best_triplet.as_ref().unwrap().median_wfe_nm
            {
                best_triplet = Some(SHGuideStarResult {
                    star_indices: indices,
                    probe_indices: probe_assignment,
                    median_wfe_nm: wfe_estimate,
                    azimuth_metric: best_az_error,
                });
            }

            n_tested += 1;
        }
    }

    best_triplet.ok_or_else(|| {
        AtpError::Catalog("No valid SH triplet found".to_string())
    })
}

/// Normalize angle to [-π, π]
fn normalize_angle(angle: f64) -> f64 {
    let mut a = angle % (2.0 * PI);
    if a > PI {
        a -= 2.0 * PI;
    } else if a < -PI {
        a += 2.0 * PI;
    }
    a
}

/// Assign triplet of stars to probes
///
/// Returns probe indices if assignment is possible, None otherwise
fn assign_triplet_to_probes(
    star_indices: &[usize; 3],
    probes: &[Probe],
    stars: &StarField,
) -> Option<[usize; 3]> {
    let mut probe_assignment = [0usize; 3];
    let mut used_probes = vec![false; probes.len()];

    for (i, &star_idx) in star_indices.iter().enumerate() {
        // Find nearest unused probe that can reach this star
        let mut best_distance = f64::INFINITY;
        let mut best_probe = None;

        for (probe_idx, probe) in probes.iter().enumerate() {
            if used_probes[probe_idx] {
                continue;
            }

            if probe.can_reach(stars, star_idx) {
                if let Some(dist) = probe.distance_to_star(stars, star_idx) {
                    if dist < best_distance {
                        best_distance = dist;
                        best_probe = Some(probe_idx);
                    }
                }
            }
        }

        if let Some(probe_idx) = best_probe {
            probe_assignment[i] = probe_idx;
            used_probes[probe_idx] = true;
        } else {
            return None; // Cannot assign this star
        }
    }

    Some(probe_assignment)
}

/// Estimate wavefront error for a triplet (simplified)
///
/// This is a simplified version that doesn't use Monte Carlo.
/// The full implementation would use crseo for proper simulation.
fn estimate_triplet_wfe(
    star_indices: &[usize; 3],
    stars: &StarField,
    _target: &Target,
    config: &Config,
) -> Result<f64> {
    // Simplified estimate based on magnitude and seeing
    let gs_wavelength = config.sh.guide_star.wavelength.0 * 1e-9;
    let r0 = config.atmosphere.r0.0 / 100.0; // cm to m
    let seeing_nm = gs_wavelength / r0 * 1e9; // Convert to nm

    // Get average magnitude
    let avg_mag: f64 = star_indices
        .iter()
        .map(|&idx| stars.stars[idx].v_mag)
        .sum::<f64>()
        / 3.0;

    // Simple model: WFE scales with seeing and magnitude
    // Brighter stars = better correction
    let magnitude_factor = 10.0_f64.powf((avg_mag - 10.0) / 5.0);
    let wfe_estimate = seeing_nm * 0.1 * magnitude_factor;

    Ok(wfe_estimate)
}

/// Complete guide star selection workflow
///
/// Selects TT7 guide star first, then SH triplet from remaining probes
pub fn select_all_guide_stars(
    probes: &mut [Probe],
    stars: &StarField,
    target: &Target,
    config: &Config,
    n_sh_candidates: usize,
) -> Result<(TT7GuideStarResult, SHGuideStarResult)> {
    // Ensure all probes know about reachable stars
    for probe in probes.iter_mut() {
        probe.reach_for_the_stars(stars);
    }

    // Select TT7 guide star
    let tt7_result = find_tt7_guide_star(probes, stars, target, config)?;

    // Assign TT7 guide star to probe
    probes[tt7_result.probe_idx].assign_guide_star(tt7_result.star_idx);

    // Get remaining probes for SH
    let sh_probes: Vec<Probe> = probes
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            if i != tt7_result.probe_idx {
                Some(p.clone())
            } else {
                None
            }
        })
        .collect();

    if sh_probes.len() < 3 {
        return Err(AtpError::Config(
            "Not enough probes remaining for SH".to_string(),
        ));
    }

    // Select SH triplet
    let sh_result = find_sh_guide_stars(&sh_probes, stars, target, config, n_sh_candidates)?;

    Ok((tt7_result, sh_result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observatory::Observatory;
    use crate::probe::create_probe_array;

    #[test]
    fn test_normalize_angle() {
        use approx::assert_relative_eq;

        assert_relative_eq!(normalize_angle(0.0), 0.0, epsilon = 1e-10);
        assert_relative_eq!(normalize_angle(PI), PI, epsilon = 1e-10);
        assert_relative_eq!(normalize_angle(-PI), -PI, epsilon = 1e-10);
        assert_relative_eq!(normalize_angle(3.0 * PI), PI, epsilon = 1e-10);
        assert_relative_eq!(normalize_angle(-3.0 * PI), -PI, epsilon = 1e-10);
    }

    #[test]
    fn test_find_tt7_guide_star() {
        // Create test configuration
        let config = create_test_config();
        let obs = Observatory::new(
            -29.049,
            -70.682,
            2514.0,
            "2018-01-01T04:00:00Z",
            60.0,
        )
        .unwrap();
        let target = Target::from_altaz(45.0, 0.0, 0.0, &obs).unwrap();

        // Create star field with some stars
        let mut field = StarField::new(18.0, None, vec!["V".to_string()]);

        // Add stars at various positions and magnitudes
        for i in 0..10 {
            let offset = (i as f64 - 5.0) * 0.01;
            let ra = target.ra_deg() + offset;
            let dec = target.dec_deg() + offset;
            let v_mag = 10.0 + i as f64 * 0.5;
            field.add_star(ra.to_radians(), dec.to_radians(), v_mag, 9.5);
        }

        field.update(&obs, &target);

        // Create probes
        let mut probes = create_probe_array(Some(2.0));
        for probe in &mut probes {
            probe.reach_for_the_stars(&field);
        }

        // Find TT7 guide star
        let result = find_tt7_guide_star(&probes, &field, &target, &config);

        // Should find a guide star
        assert!(result.is_ok());

        let tt7 = result.unwrap();
        assert!(tt7.star_idx < field.len());
        assert!(tt7.probe_idx < probes.len());
        assert!(tt7.rms_error_mas > 0.0);
    }

    fn create_test_config() -> Config {
        serde_yaml::from_str(
            r#"
Observation:
  time scale: 'UTC'
  time: '2018-01-01T04:00:00Z'
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
  altitude: [[25, 275, 425, 1250, 4000, 8000, 13000], 'm']
  fr0: [0.1257, 0.0874, 0.0666, 0.3498, 0.2273, 0.0681, 0.0751]
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
"#,
        )
        .unwrap()
    }
}
