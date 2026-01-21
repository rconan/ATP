//! Mock data for ATP web UI development

use crate::models::{FieldResponse, GuideStarResult, ProbeData, StarData};
use rand::Rng;

/// Generate mock star field with realistic data
pub fn generate_mock_field() -> FieldResponse {
    let mut rng = rand::thread_rng();

    // Generate ~50 random stars in field
    let mut stars = Vec::new();
    for i in 0..50 {
        // Random position in ±10 arcmin field
        let r = rng.gen_range(3.5..10.0); // Outside exclusion zone
        let theta = rng.gen_range(0.0..std::f64::consts::TAU);
        let x = r * theta.cos();
        let y = r * theta.sin();

        // Random magnitudes (brighter stars more likely)
        let v_mag = rng.gen_range(10.0..16.0);
        let j_mag = v_mag - rng.gen_range(0.3..0.8); // J typically brighter

        // Random RA/Dec around target
        let ra = 90.0 + rng.gen_range(-0.15..0.15);
        let dec = 16.0 + rng.gen_range(-0.15..0.15);

        stars.push(StarData {
            idx: i,
            x,
            y,
            v_mag,
            j_mag,
            ra,
            dec,
        });
    }

    // Sort by magnitude (brightest first)
    stars.sort_by(|a, b| a.v_mag.partial_cmp(&b.v_mag).unwrap());

    // Create 4 probes at 90° intervals
    let probe_radius = 22.67; // arcmin (1360" / 60)
    let mut probes = Vec::new();
    for i in 0..4 {
        let angle = (i as f64) * std::f64::consts::FRAC_PI_2;
        probes.push(ProbeData {
            idx: i,
            x: probe_radius * angle.cos(),
            y: probe_radius * angle.sin(),
            star_idx: None,
            probe_type: "Available".to_string(),
        });
    }

    FieldResponse {
        stars,
        probes,
        target_ra: 90.0,
        target_dec: 16.0,
        target_alt: 45.0,
        target_az: 0.0,
        obs_time: "2018-01-01T04:00:00Z".to_string(),
        sidereal_time: 1.234,
    }
}

/// Generate mock guide star selection result
pub fn generate_mock_guide_star_result(field: &FieldResponse) -> GuideStarResult {
    if field.stars.len() < 4 {
        panic!("Need at least 4 stars for guide star selection");
    }

    // Select TT7: brightest star (first in sorted list)
    let tt7_star_idx = field.stars[0].idx;
    let tt7_probe_idx = 0;

    // Compute TT7 error based on magnitude (simple model)
    let tt7_v_mag = field.stars[0].v_mag;
    let tt7_error_mas = 0.5 + (tt7_v_mag - 10.0) * 1.5; // ~0.5-9 mas range

    // Select SH triplet: 3 stars roughly 120° apart
    let mut sh_candidates = Vec::new();
    for (i, star) in field.stars.iter().enumerate() {
        if i == 0 {
            continue; // Skip TT7 star
        }
        let theta = star.y.atan2(star.x);
        sh_candidates.push((i, star.idx, theta));
    }

    // Pick 3 stars with good azimuthal distribution
    let sh1 = sh_candidates[0]; // First candidate

    // Find star ~120° away
    let target_angle_1 = sh1.2 + 2.0 * std::f64::consts::PI / 3.0;
    let sh2 = sh_candidates
        .iter()
        .min_by_key(|(_, _, theta)| {
            let diff = (theta - target_angle_1).abs();
            (diff * 1000.0) as i64
        })
        .copied()
        .unwrap();

    // Find star ~240° away
    let target_angle_2 = sh1.2 + 4.0 * std::f64::consts::PI / 3.0;
    let sh3 = sh_candidates
        .iter()
        .filter(|(i, _, _)| *i != sh2.0)
        .min_by_key(|(_, _, theta)| {
            let diff = (theta - target_angle_2).abs();
            (diff * 1000.0) as i64
        })
        .copied()
        .unwrap();

    let sh_star_indices = vec![sh1.1, sh2.1, sh3.1];
    let sh_probe_indices = vec![1, 2, 3];

    // Compute WFE based on average magnitude
    let avg_mag = (field.stars[sh1.0].v_mag +
                   field.stars[sh2.0].v_mag +
                   field.stars[sh3.0].v_mag) / 3.0;
    let sh_wfe_nm = 80.0 + (avg_mag - 10.0) * 15.0; // ~80-170 nm range

    // Update field with probe assignments
    let mut updated_field = field.clone();
    updated_field.probes[tt7_probe_idx].star_idx = Some(tt7_star_idx);
    updated_field.probes[tt7_probe_idx].probe_type = "TT7".to_string();

    for (i, &probe_idx) in sh_probe_indices.iter().enumerate() {
        updated_field.probes[probe_idx].star_idx = Some(sh_star_indices[i]);
        updated_field.probes[probe_idx].probe_type = "SH".to_string();
    }

    GuideStarResult {
        tt7_star_idx,
        tt7_probe_idx,
        tt7_error_mas,
        sh_star_indices,
        sh_probe_indices,
        sh_wfe_nm,
        field: updated_field,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mock_field() {
        let field = generate_mock_field();
        assert_eq!(field.stars.len(), 50);
        assert_eq!(field.probes.len(), 4);

        // Check stars are outside exclusion zone
        for star in &field.stars {
            let r = (star.x * star.x + star.y * star.y).sqrt();
            assert!(r >= 3.0, "Star too close to target: r={}", r);
        }
    }

    #[test]
    fn test_generate_mock_guide_stars() {
        let field = generate_mock_field();
        let result = generate_mock_guide_star_result(&field);

        assert_eq!(result.sh_star_indices.len(), 3);
        assert_eq!(result.sh_probe_indices.len(), 3);
        assert!(result.tt7_error_mas > 0.0);
        assert!(result.sh_wfe_nm > 0.0);
    }
}
