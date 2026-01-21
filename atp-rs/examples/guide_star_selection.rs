//! Complete guide star selection example
//!
//! Demonstrates the full ATP workflow:
//! 1. Load configuration
//! 2. Create observatory and target
//! 3. Generate/load star field
//! 4. Find optimal TT7 guide star
//! 5. Find optimal SH triplet
//! 6. Visualize results

use atp_rs::{Config, Observatory, Target, StarField};
use atp_rs::guidestar::{select_all_guide_stars, find_tt7_guide_star, find_sh_guide_stars};
use atp_rs::probe::create_probe_array;
use std::f64::consts::PI;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== ATP-RS Complete Guide Star Selection ===\n");

    // Load configuration
    let config_path = "../atp.yaml";
    let config = if std::path::Path::new(config_path).exists() {
        println!("Loading configuration from {}", config_path);
        Config::from_file(config_path)?
    } else {
        println!("Using embedded test configuration");
        create_test_config()
    };

    println!("Configuration:");
    println!("  Telescope: {}m", config.telescope.diameter.0);
    println!("  Atmosphere: r0={}cm @ {}nm",
             config.atmosphere.r0.0,
             config.atmosphere.wavelength.0);
    println!("  TT7: {} lenslets", config.tt7.optics.lenslet.array);
    println!("  SH:  {} lenslets\n", config.sh.optics.lenslet.array);

    // Create observatory
    let obs = Observatory::new(
        config.observatory.latitude.0,
        config.observatory.longitude.0,
        config.observatory.height.0,
        &config.observation.time,
        config.time_resolution_sec(),
    )?;

    println!("Observatory: Las Campanas, Chile");
    println!("  Time: {}", obs.current_time);
    println!("  LST:  {:.4} rad ({:.2}°)\n",
             obs.sidereal_time(),
             obs.sidereal_time().to_degrees());

    // Create target
    let target = match &config.target.pointing_altaz {
        Some(altaz) => {
            println!("Target from config:");
            Target::from_altaz(
                altaz.alt.0,
                altaz.az.0,
                config.target.rotator_angle.0,
                &obs,
            )?
        }
        None => {
            println!("Using default target:");
            Target::from_altaz(45.0, 0.0, 0.0, &obs)?
        }
    };

    println!("  RA/Dec:  {:.4}°, {:.4}°", target.ra_deg(), target.dec_deg());
    println!("  Alt/Az:  {:.2}°, {:.2}°", target.alt_deg(), target.az_deg());
    println!("  Zenith:  {:.2}°\n", target.zenith_distance().to_degrees());

    // Create star field
    println!("Generating synthetic star field...");
    let mut field = create_synthetic_starfield(&target, 50);

    field.update(&obs, &target);
    field.apply_constraints(&target);

    println!("  Total stars: {}", field.len());
    if let Some((min, max)) = field.v_mag_range() {
        println!("  V mag range: {:.2} - {:.2}", min, max);
    }
    println!();

    // Create AGWS probe array
    let mut probes = create_probe_array(Some(2.0));
    println!("AGWS Probes: {}", probes.len());
    for (i, probe) in probes.iter_mut().enumerate() {
        probe.reach_for_the_stars(&field);
        println!("  Probe {}: {} reachable stars @ az={:.0}°",
                 i,
                 probe.num_reachable(),
                 probe.azimuth_deg());
    }
    println!();

    // Option 1: Complete workflow
    println!("=== METHOD 1: Complete Workflow ===");
    let (tt7_result, sh_result) = select_all_guide_stars(
        &mut probes,
        &field,
        &target,
        &config,
        10, // Test 10 SH candidates
    )?;

    println!("\nTT7 Guide Star:");
    print_tt7_result(&tt7_result, &field);

    println!("\nSH Guide Star Triplet:");
    print_sh_result(&sh_result, &field);

    // Option 2: Step-by-step
    println!("\n=== METHOD 2: Step-by-Step Selection ===");

    // Reset probes
    let mut probes2 = create_probe_array(Some(2.0));
    for probe in &mut probes2 {
        probe.reach_for_the_stars(&field);
    }

    // Find TT7
    println!("\nStep 1: Finding TT7 guide star...");
    let tt7_step = find_tt7_guide_star(&probes2, &field, &target, &config)?;
    println!("  Selected star {}: V={:.2}, error={:.2} mas",
             tt7_step.star_idx,
             field.stars[tt7_step.star_idx].v_mag,
             tt7_step.rms_error_mas);

    // Mark TT7 probe as used
    probes2[tt7_step.probe_idx].assign_guide_star(tt7_step.star_idx);

    // Get remaining probes
    let sh_probes: Vec<_> = probes2
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != tt7_step.probe_idx)
        .map(|(_, p)| p.clone())
        .collect();

    // Find SH triplet
    println!("\nStep 2: Finding SH triplet...");
    let sh_step = find_sh_guide_stars(&sh_probes, &field, &target, &config, 10)?;
    println!("  Selected stars: {:?}", sh_step.star_indices);
    println!("  Magnitudes: {:.2}, {:.2}, {:.2}",
             field.stars[sh_step.star_indices[0]].v_mag,
             field.stars[sh_step.star_indices[1]].v_mag,
             field.stars[sh_step.star_indices[2]].v_mag);
    println!("  WFE estimate: {:.1} nm RMS", sh_step.median_wfe_nm);

    // Summary
    println!("\n=== Selection Summary ===");
    print_field_map(&field, &tt7_result, &sh_result);

    println!("\n=== Performance Metrics ===");
    println!("TT7:");
    println!("  RMS tilt error: {:.2} mas", tt7_result.rms_error_mas);
    println!("  Guide star V mag: {:.2}", field.stars[tt7_result.star_idx].v_mag);
    println!("\nSH:");
    println!("  Wavefront error: {:.1} nm RMS", sh_result.median_wfe_nm);
    println!("  Avg magnitude: {:.2}",
             sh_result.star_indices.iter()
                 .map(|&i| field.stars[i].v_mag)
                 .sum::<f64>() / 3.0);
    println!("  Azimuth quality: {:.3} rad", sh_result.azimuth_metric);

    println!("\n=== Complete! ===");

    Ok(())
}

fn print_tt7_result(result: &atp_rs::guidestar::TT7GuideStarResult, field: &StarField) {
    let star = &field.stars[result.star_idx];
    println!("  Star index:    {}", result.star_idx);
    println!("  Probe:         {}", result.probe_idx);
    println!("  V magnitude:   {:.2}", star.v_mag);
    println!("  RMS error:     {:.2} mas", result.rms_error_mas);
    println!("  Distance:      {:.2} arcmin", result.distance_arcmin);
    println!("  Position:      ({:.3}, {:.3}) arcmin",
             star.local_x * 180.0 * 60.0 / PI,
             star.local_y * 180.0 * 60.0 / PI);
}

fn print_sh_result(result: &atp_rs::guidestar::SHGuideStarResult, field: &StarField) {
    for (i, &star_idx) in result.star_indices.iter().enumerate() {
        let star = &field.stars[star_idx];
        println!("  Star {} (index {}):", i + 1, star_idx);
        println!("    V magnitude: {:.2}", star.v_mag);
        println!("    Probe:       {}", result.probe_indices[i]);
        println!("    Position:    ({:.3}, {:.3}) arcmin",
                 star.local_x * 180.0 * 60.0 / PI,
                 star.local_y * 180.0 * 60.0 / PI);
    }
    println!("\n  WFE estimate:  {:.1} nm RMS", result.median_wfe_nm);
    println!("  Az quality:    {:.3} rad", result.azimuth_metric);
}

fn print_field_map(
    field: &StarField,
    tt7: &atp_rs::guidestar::TT7GuideStarResult,
    sh: &atp_rs::guidestar::SHGuideStarResult,
) {
    println!("\nField Map (local coordinates in arcmin):");
    println!("  TT7 (★): Star {}", tt7.star_idx);
    println!("  SH  (●): Stars {:?}", sh.star_indices);
    println!();

    // Simple ASCII map
    let scale = 25.0; // arcmin
    let width = 40;
    let height = 20;

    for y in 0..height {
        for x in 0..width {
            let fx = (x as f64 / width as f64 - 0.5) * 2.0 * scale;
            let fy = (0.5 - y as f64 / height as f64) * 2.0 * scale * 0.5;

            let mut symbol = ' ';

            // Check if any star is near this position
            for (idx, star) in field.stars.iter().enumerate() {
                let sx = star.local_x * 180.0 * 60.0 / PI;
                let sy = star.local_y * 180.0 * 60.0 / PI;

                if (sx - fx).abs() < 1.0 && (sy - fy).abs() < 1.0 {
                    if idx == tt7.star_idx {
                        symbol = '★';
                    } else if sh.star_indices.contains(&idx) {
                        symbol = '●';
                    } else {
                        symbol = '·';
                    }
                }
            }

            // Draw axes
            if fx.abs() < 0.5 {
                symbol = '│';
            }
            if fy.abs() < 0.5 {
                symbol = '─';
            }
            if fx.abs() < 0.5 && fy.abs() < 0.5 {
                symbol = '+';
            }

            print!("{}", symbol);
        }
        println!();
    }
    println!("\n  Scale: ±{} arcmin", scale);
}

fn create_synthetic_starfield(target: &Target, n_stars: usize) -> StarField {
    let mut field = StarField::new(18.0, Some(3.0), vec!["V".to_string(), "J".to_string()]);

    use rand::Rng;
    let mut rng = rand::thread_rng();

    for _ in 0..n_stars {
        // Random position within 10 arcmin
        let r = rng.gen::<f64>().sqrt() * 10.0 / 60.0; // arcmin to degrees
        let theta = rng.gen::<f64>() * 2.0 * PI;

        let ra = target.ra_deg() + r * theta.cos();
        let dec = target.dec_deg() + r * theta.sin();

        // Random magnitude 10-16
        let v_mag = 10.0 + rng.gen::<f64>() * 6.0;
        let j_mag = v_mag - 0.5;

        field.add_star(ra.to_radians(), dec.to_radians(), v_mag, j_mag);
    }

    field
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
