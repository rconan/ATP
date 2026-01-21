//! TIC Catalog Query Example
//!
//! Demonstrates querying the TESS Input Catalog (TIC) via MAST API
//! and building a star field for guide star selection.

use atp_rs::{Config, Observatory, Target};
use atp_rs::catalog::TICCatalog;
use atp_rs::guidestar::select_all_guide_stars;
use atp_rs::probe::create_probe_array;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== ATP-RS TIC Catalog Query Example ===\n");

    // Load configuration
    let config = create_test_config();

    println!("Configuration:");
    println!("  V magnitude limit: {}", config.star_catalog.v_magnitude_limit);
    println!("  Search radius:     {} arcmin", config.star_catalog.radius.0);
    println!("  Exclude radius:    {} arcmin", config.star_catalog.exclude_radius.0);
    println!();

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
    println!("  LST:  {:.4} rad\n", obs.sidereal_time());

    // Create target - use M45 (Pleiades) as a test case
    println!("Target: M45 (Pleiades)");
    let target = Target::from_radec(
        56.75,  // RA in degrees
        24.12,  // Dec in degrees
        0.0,    // Rotator angle
        &obs,
    )?;

    println!("  RA/Dec:  {:.4}°, {:.4}°", target.ra_deg(), target.dec_deg());
    println!("  Alt/Az:  {:.2}°, {:.2}°", target.alt_deg(), target.az_deg());
    println!();

    // Create TIC catalog client
    println!("Creating TIC catalog client...");
    let catalog = TICCatalog::new(Some(60))?;
    println!("  Timeout: 60 seconds\n");

    // Query TIC catalog
    println!("Querying MAST TIC catalog...");
    println!("  Center: RA={:.4}°, Dec={:.4}°", target.ra_deg(), target.dec_deg());
    println!("  Radius: {} arcmin", config.star_catalog.radius.0);
    println!("  (This may take 10-30 seconds...)\n");

    let entries = catalog
        .query_around_target(&target, config.star_catalog.radius.0)
        .await;

    match entries {
        Ok(tic_entries) => {
            println!("✓ Query successful!");
            println!("  Retrieved {} TIC entries\n", tic_entries.len());

            // Show sample entries
            println!("Sample TIC entries:");
            println!("{:>6} {:>10} {:>10} {:>6} {:>6} {:>6} {:>12}",
                     "Index", "RA (°)", "Dec (°)", "V mag", "J mag", "T mag", "Type");
            println!("{}", "-".repeat(75));

            for (i, entry) in tic_entries.iter().take(10).enumerate() {
                println!("{:>6} {:>10.5} {:>10.5} {:>6} {:>6} {:>6} {:>12}",
                         i,
                         entry.ra,
                         entry.dec,
                         entry.v_mag.map_or("--".to_string(), |v| format!("{:.2}", v)),
                         entry.j_mag.map_or("--".to_string(), |j| format!("{:.2}", j)),
                         entry.t_mag.map_or("--".to_string(), |t| format!("{:.2}", t)),
                         entry.obj_type.as_ref().unwrap_or(&"UNKNOWN".to_string()));
            }

            if tic_entries.len() > 10 {
                println!("  ... and {} more entries", tic_entries.len() - 10);
            }
            println!();

            // Build star field from TIC entries
            println!("Building star field...");
            let mut field = catalog.build_starfield(
                tic_entries,
                config.star_catalog.v_magnitude_limit,
                Some(config.star_catalog.exclude_radius.0),
                Some(&target),
            )?;

            println!("  Stars in field: {}", field.len());

            if let Some((min, max)) = field.v_mag_range() {
                println!("  V mag range: {:.2} - {:.2}", min, max);
            }
            println!();

            // Update to local coordinates
            field.update(&obs, &target);

            // Perform guide star selection
            if field.len() >= 4 {
                println!("Performing guide star selection...");

                let mut probes = create_probe_array(Some(2.0));
                for probe in &mut probes {
                    probe.reach_for_the_stars(&field);
                }

                let total_reachable: usize = probes.iter().map(|p| p.num_reachable()).sum();
                println!("  Total reachable stars: {}", total_reachable);

                if total_reachable >= 4 {
                    let result = select_all_guide_stars(
                        &mut probes,
                        &field,
                        &target,
                        &config,
                        10,
                    );

                    match result {
                        Ok((tt7, sh)) => {
                            println!("\n✓ Guide star selection successful!\n");

                            println!("TT7 Guide Star:");
                            println!("  Star index:  {}", tt7.star_idx);
                            println!("  V magnitude: {:.2}", field.stars[tt7.star_idx].v_mag);
                            println!("  RMS error:   {:.2} mas", tt7.rms_error_mas);
                            println!("  Probe:       {}\n", tt7.probe_idx);

                            println!("SH Guide Star Triplet:");
                            for (i, &idx) in sh.star_indices.iter().enumerate() {
                                println!("  Star {}: index={}, V={:.2}, probe={}",
                                         i + 1,
                                         idx,
                                         field.stars[idx].v_mag,
                                         sh.probe_indices[i]);
                            }
                            println!("  WFE estimate: {:.1} nm RMS", sh.median_wfe_nm);
                        }
                        Err(e) => {
                            println!("✗ Guide star selection failed: {}", e);
                        }
                    }
                } else {
                    println!("  ✗ Not enough reachable stars for guide star selection");
                }
            } else {
                println!("✗ Not enough stars in field (need at least 4)");
            }

            println!("\n=== Query Complete! ===");
        }
        Err(e) => {
            println!("✗ TIC query failed: {}\n", e);
            println!("This is expected if:");
            println!("  • No internet connection");
            println!("  • MAST API is down");
            println!("  • Firewall blocking HTTPS");
            println!("\nTo test without network, use synthetic star fields:");
            println!("  cargo run --example guide_star_selection");
        }
    }

    Ok(())
}

fn create_test_config() -> atp_rs::Config {
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
  V magnitude limit: 16
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
