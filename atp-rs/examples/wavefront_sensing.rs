//! Wavefront sensing error calculation example
//!
//! Demonstrates TT7 tilt error calculation for guide stars

use atp_rs::{Config, Observatory, Target, StarField};
use atp_rs::wavefront::tt7_tt_error;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== ATP-RS Wavefront Sensing Example ===\n");

    // Load configuration
    let config_path = "../atp.yaml";
    let config = if std::path::Path::new(config_path).exists() {
        Config::from_file(config_path)?
    } else {
        println!("Warning: atp.yaml not found, using embedded test config");
        create_test_config()
    };

    println!("Configuration loaded:");
    println!("  Telescope: {:.1}m diameter", config.telescope.diameter.0);
    println!("  Atmosphere: r0={:.1}cm @ {}nm",
             config.atmosphere.r0.0,
             config.atmosphere.wavelength.0);
    println!("  TT7: {} lenslets, {} exposure",
             config.tt7.optics.lenslet.array,
             format_exposure(&config.tt7.detector.exposure));
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
    println!("  LST: {:.2} rad\n", obs.sidereal_time());

    // Create target
    let target = match &config.target.pointing_altaz {
        Some(altaz) => Target::from_altaz(
            altaz.alt.0,
            altaz.az.0,
            config.target.rotator_angle.0,
            &obs,
        )?,
        None => {
            println!("Using default target: Alt=45°, Az=0°");
            Target::from_altaz(45.0, 0.0, 0.0, &obs)?
        }
    };

    println!("Target pointing:");
    println!("  Alt/Az: {:.2}°, {:.2}°", target.alt_deg(), target.az_deg());
    println!("  Zenith distance: {:.2}°\n", target.zenith_distance().to_degrees());

    // Calculate TT7 errors for various guide stars
    println!("TT7 Tilt Error Analysis:");
    println!("{:>10} {:>10} {:>15}", "Separation", "Magnitude", "RMS Error");
    println!("{:>10} {:>10} {:>15}", "(arcmin)", "(V-band)", "(mas)");
    println!("{}", "-".repeat(40));

    // Test range: on-axis to 10 arcmin, magnitude 8 to 18
    let separations = vec![0.0, 1.0, 3.0, 5.0, 10.0];
    let magnitudes = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0];

    // For each separation
    for &sep in &separations {
        println!("\nSeparation = {:.1}' from target:", sep);

        for &mag in &magnitudes {
            let error_mas = tt7_tt_error(
                sep,
                mag,
                target.zenith_distance(),
                &config,
            )?;

            println!("  {:>8.1}   {:>8.1}   {:>13.2}",
                     sep, mag, error_mas);

            // Highlight good guide stars (< 5 mas error)
            if error_mas < 5.0 {
                println!("    └─> Good guide star! (< 5 mas)");
            }
        }
    }

    println!("\n=== Summary ===");
    println!("TT7 performance depends on:");
    println!("  • Guide star brightness (fainter = more noise)");
    println!("  • Angular separation (larger = more anisoplanatism)");
    println!("  • Zenith distance (larger = worse seeing)");
    println!("\nFor best performance:");
    println!("  • Use bright stars (V < 14)");
    println!("  • Keep separation < 5 arcmin");
    println!("  • Observe near zenith when possible");

    Ok(())
}

fn format_exposure(exp: &(f64, String)) -> String {
    format!("{:.0}{}", exp.0, exp.1)
}

fn create_test_config() -> Config {
    // Minimal test configuration
    serde_yaml::from_str(r#"
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
"#).unwrap()
}
