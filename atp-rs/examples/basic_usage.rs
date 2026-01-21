//! Basic usage example for ATP-RS
//!
//! This demonstrates creating an observatory, target, and star field

use atp_rs::{Observatory, Target, StarField, Probe};
use atp_rs::probe::create_probe_array;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("=== ATP-RS Basic Usage Example ===\n");

    // Create Las Campanas Observatory
    let mut obs = Observatory::new(
        -29.049,  // latitude (degrees)
        -70.682,  // longitude (degrees)
        2514.0,   // height (meters)
        "2018-01-01T04:00:00Z",  // time (UTC)
        60.0,     // time resolution (seconds)
    )?;

    println!("{}\n", obs);

    // Create target at Alt=45°, Az=0°
    let mut target = Target::from_altaz(
        45.0,  // altitude (degrees)
        0.0,   // azimuth (degrees)
        0.0,   // rotator angle (degrees)
        &obs,
    )?;

    println!("{}\n", target);

    // Create star field
    let mut field = StarField::new(
        18.0,                      // V magnitude limit
        Some(3.0),                 // exclude radius (arcmin)
        vec!["V".to_string(), "J".to_string()],  // color bands
    );

    // Add some example stars (in real usage, query from catalog)
    println!("Adding sample stars...");
    for i in 0..20 {
        let ra = target.ra_deg() + (i as f64 - 10.0) * 0.05;
        let dec = target.dec_deg() + (i as f64 - 10.0) * 0.05;
        let v_mag = 10.0 + (i as f64) * 0.3;
        let j_mag = v_mag - 0.5;
        field.add_star(ra.to_radians(), dec.to_radians(), v_mag, j_mag);
    }

    // Update local coordinates
    field.update(&obs, &target);
    field.apply_constraints(&target);

    println!("Stars in field: {}", field.len());
    if let Some((min, max)) = field.v_mag_range() {
        println!("V magnitude range: {:.2} - {:.2}\n", min, max);
    }

    // Create probe array
    let mut probes = create_probe_array(Some(2.0));
    println!("Created {} probes:", probes.len());

    for (i, probe) in probes.iter_mut().enumerate() {
        probe.reach_for_the_stars(&field);
        println!("  {}", probe);
    }

    // Simulate time evolution
    println!("\n=== Simulating 1 hour time evolution ===");
    for step in 0..6 {
        obs.update();
        target.update(&obs, false);
        field.update(&obs, &target);

        if step % 2 == 0 {
            println!("Step {}: Alt={:.2}°, Az={:.2}°, PA={:.2}°",
                     step,
                     target.alt_deg(),
                     target.az_deg(),
                     target.parallactic_angle_deg());
        }
    }

    println!("\n=== Example complete ===");

    Ok(())
}
