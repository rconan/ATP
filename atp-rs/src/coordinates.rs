//! Coordinate transformations and rotation matrices

use nalgebra::{Matrix3, Vector3};
use std::f64::consts::PI;

/// Rotation matrix around Z-axis
pub fn rotation_z(angle: f64) -> Matrix3<f64> {
    let c = angle.cos();
    let s = angle.sin();
    Matrix3::new(
        c, s, 0.0,
        -s, c, 0.0,
        0.0, 0.0, 1.0,
    )
}

/// Rotation matrix around Y-axis
pub fn rotation_y(angle: f64) -> Matrix3<f64> {
    let c = angle.cos();
    let s = angle.sin();
    Matrix3::new(
        c, 0.0, -s,
        0.0, 1.0, 0.0,
        s, 0.0, c,
    )
}

/// Rotation matrix around X-axis
pub fn rotation_x(angle: f64) -> Matrix3<f64> {
    let c = angle.cos();
    let s = angle.sin();
    Matrix3::new(
        1.0, 0.0, 0.0,
        0.0, c, s,
        0.0, -s, c,
    )
}

/// Convert equatorial coordinates (RA, Dec) to Alt-Az
///
/// # Arguments
/// * `ra` - Right ascension in radians
/// * `dec` - Declination in radians
/// * `lst` - Local sidereal time in radians
/// * `lat` - Observer latitude in radians
///
/// # Returns
/// (altitude, azimuth) in radians
pub fn equatorial_to_altaz(ra: f64, dec: f64, lst: f64, lat: f64) -> (f64, f64) {
    let ha = lst - ra; // Hour angle

    let sin_alt = dec.sin() * lat.sin() + dec.cos() * lat.cos() * ha.cos();
    let alt = sin_alt.asin();

    let cos_az = (dec.sin() - lat.sin() * alt.sin()) / (lat.cos() * alt.cos());
    let sin_az = -ha.sin() * dec.cos() / alt.cos();

    let az = sin_az.atan2(cos_az);

    (alt, az)
}

/// Convert Alt-Az coordinates to equatorial (RA, Dec)
///
/// # Arguments
/// * `alt` - Altitude in radians
/// * `az` - Azimuth in radians
/// * `lst` - Local sidereal time in radians
/// * `lat` - Observer latitude in radians
///
/// # Returns
/// (right ascension, declination) in radians
pub fn altaz_to_equatorial(alt: f64, az: f64, lst: f64, lat: f64) -> (f64, f64) {
    let sin_dec = alt.sin() * lat.sin() + alt.cos() * lat.cos() * az.cos();
    let dec = sin_dec.asin();

    let cos_ha = (alt.sin() - lat.sin() * dec.sin()) / (lat.cos() * dec.cos());
    let sin_ha = -az.sin() * alt.cos() / dec.cos();

    let ha = sin_ha.atan2(cos_ha);
    let ra = lst - ha;

    // Normalize RA to [0, 2π]
    let ra = if ra < 0.0 { ra + 2.0 * PI } else { ra };
    let ra = if ra >= 2.0 * PI { ra - 2.0 * PI } else { ra };

    (ra, dec)
}

/// Calculate parallactic angle
///
/// # Arguments
/// * `ha` - Hour angle in radians
/// * `dec` - Declination in radians
/// * `lat` - Observer latitude in radians
///
/// # Returns
/// Parallactic angle in radians
pub fn parallactic_angle(ha: f64, dec: f64, lat: f64) -> f64 {
    let numerator = lat.sin() * dec.cos() - lat.cos() * ha.cos() * dec.sin();
    let denominator = -lat.cos() * ha.sin();
    numerator.atan2(denominator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_rotation_matrices() {
        let identity = Matrix3::identity();

        // Rotation by 0 should give identity
        assert_relative_eq!(rotation_x(0.0), identity, epsilon = 1e-10);
        assert_relative_eq!(rotation_y(0.0), identity, epsilon = 1e-10);
        assert_relative_eq!(rotation_z(0.0), identity, epsilon = 1e-10);

        // Rotation by 2π should give identity
        assert_relative_eq!(rotation_z(2.0 * PI), identity, epsilon = 1e-10);
    }

    #[test]
    fn test_coordinate_conversions() {
        let lat = 0.5; // ~28.6 degrees
        let lst = 1.0;
        let ra = 0.8;
        let dec = 0.3;

        // Convert to alt-az and back
        let (alt, az) = equatorial_to_altaz(ra, dec, lst, lat);
        let (ra2, dec2) = altaz_to_equatorial(alt, az, lst, lat);

        assert_relative_eq!(ra, ra2, epsilon = 1e-10);
        assert_relative_eq!(dec, dec2, epsilon = 1e-10);
    }
}
