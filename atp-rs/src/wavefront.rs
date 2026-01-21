//! Wavefront sensing error calculations
//!
//! This module implements the noise models and atmospheric error calculations
//! for the AGWS wavefront sensors (TT7 and SH).

use crate::config::Config;
use crate::constants::ARCMIN2RAD;
use crate::errors::Result;
use std::f64::consts::PI;

// Bessel functions using series approximation
// For better accuracy, we could use a numerical library, but for now simple approximations

/// Bessel function J0 (order 0)
fn bessel_j0(x: f64) -> f64 {
    if x.abs() < 8.0 {
        // Small x approximation
        let y = x * x;
        let ans1 = 57568490574.0 + y * (-13362590354.0 + y * (651619640.7
            + y * (-11214424.18 + y * (77392.33017 + y * (-184.9052456)))));
        let ans2 = 57568490411.0 + y * (1029532985.0 + y * (9494680.718
            + y * (59272.64853 + y * (267.8532712 + y))));
        ans1 / ans2
    } else {
        // Large x approximation
        let z = 8.0 / x;
        let y = z * z;
        let xx = x - 0.785398164;
        let ans1 = 1.0 + y * (-0.1098628627e-2 + y * (0.2734510407e-4
            + y * (-0.2073370639e-5 + y * 0.2093887211e-6)));
        let ans2 = -0.1562499995e-1 + y * (0.1430488765e-3
            + y * (-0.6911147651e-5 + y * (0.7621095161e-6
                - y * 0.934945152e-7)));
        (2.0 / PI / x).sqrt() * (ans1 * xx.cos() - z * ans2 * xx.sin())
    }
}

/// Bessel function J1 (order 1)
fn bessel_j1(x: f64) -> f64 {
    if x.abs() < 8.0 {
        // Small x approximation
        let y = x * x;
        let ans1 = x * (72362614232.0 + y * (-7895059235.0 + y * (242396853.1
            + y * (-2972611.439 + y * (15704.48260 + y * (-30.16036606))))));
        let ans2 = 144725228442.0 + y * (2300535178.0 + y * (18583304.74
            + y * (99447.43394 + y * (376.9991397 + y))));
        ans1 / ans2
    } else {
        // Large x approximation
        let z = 8.0 / x;
        let y = z * z;
        let xx = x - 2.356194491;
        let ans1 = 1.0 + y * (0.183105e-2 + y * (-0.3516396496e-4
            + y * (0.2457520174e-5 + y * (-0.240337019e-6))));
        let ans2 = 0.04687499995 + y * (-0.2002690873e-3
            + y * (0.8449199096e-5 + y * (-0.88228987e-6
                + y * 0.105787412e-6)));
        (2.0 / PI / x).sqrt() * (ans1 * xx.cos() - z * ans2 * xx.sin())
            * x.signum()
    }
}

/// Gamma function using Lanczos approximation
fn gamma(z: f64) -> f64 {
    const G: f64 = 7.0;
    const COEF: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];

    if z < 0.5 {
        PI / ((PI * z).sin() * gamma(1.0 - z))
    } else {
        let z = z - 1.0;
        let mut x = COEF[0];
        for i in 1..9 {
            x += COEF[i] / (z + i as f64);
        }
        let t = z + G + 0.5;
        (2.0 * PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * x
    }
}

/// Calculate photon noise variance for centroid estimation
///
/// Equivalent to Python's `photon_noise_variance` (atp.py:21-22)
///
/// # Arguments
/// * `fwhm` - Full width at half maximum in radians
/// * `n_photons` - Number of photons per lenslet
///
/// # Returns
/// Centroid variance in rad²
pub fn photon_noise_variance(fwhm: f64, n_photons: f64) -> f64 {
    0.5 * fwhm * fwhm / n_photons / (2.0_f64.ln())
}

/// Calculate readout noise variance for centroid estimation
///
/// Equivalent to Python's `readout_noise_variance` (atp.py:23-25)
///
/// # Arguments
/// * `n_photons` - Number of photons per lenslet
/// * `ron2` - Readout noise variance (electrons²)
/// * `pixel_scale` - Pixel scale in radians/pixel
/// * `n_subap_pixels2` - Number of pixels in subaperture squared (Ns²)
/// * `n_background` - Background photons (default: 0)
///
/// # Returns
/// Centroid variance in rad²
pub fn readout_noise_variance(
    n_photons: f64,
    ron2: f64,
    pixel_scale: f64,
    n_subap_pixels2: f64,
    n_background: f64,
) -> f64 {
    let sig2 = ron2 + n_background;
    sig2 * (pixel_scale * n_subap_pixels2 / n_photons).powi(2) / 12.0
}

/// Calculate r0 scaling with wavelength and zenith distance
///
/// Equivalent to Python's `r0_scaling` (atp.py:26-27)
///
/// # Arguments
/// * `atm_wavelength` - Atmospheric reference wavelength (m)
/// * `gs_wavelength` - Guide star wavelength (m)
/// * `zenith_distance` - Zenith distance (radians, default: 0)
///
/// # Returns
/// r0 scaling factor
pub fn r0_scaling(atm_wavelength: f64, gs_wavelength: f64, zenith_distance: f64) -> f64 {
    (gs_wavelength / atm_wavelength).powf(1.2) * zenith_distance.cos().powf(0.6)
}

/// Calculate tilt anisoplanatism variance
///
/// Equivalent to Python's `tilt_anisoplanatism` (atp.py:54-74)
///
/// Computes the angular anisoplanatism for tilt using the structure function
/// integrated over spatial frequencies.
///
/// # Arguments
/// * `separation` - Angular separation in radians
/// * `r0` - Fried parameter at guide star wavelength (m)
/// * `wavelength` - Guide star wavelength (m)
/// * `l0` - Outer scale (m)
/// * `diameter` - Telescope diameter (m)
/// * `altitude` - Layer altitudes (m)
/// * `fr0` - Fractional r0 per layer
///
/// # Returns
/// Tilt anisoplanatism variance in rad²
pub fn tilt_anisoplanatism(
    separation: f64,
    r0: f64,
    wavelength: f64,
    l0: f64,
    diameter: f64,
    altitude: &[f64],
    fr0: &[f64],
) -> Result<f64> {
    // Telescope transfer function G(f)
    let g_function = |f: f64, d: f64| -> f64 {
        if f == 0.0 {
            1.0
        } else {
            let red = PI * d * f;
            let j1 = bessel_j1(red);
            (2.0 * j1 / red).powi(2)
        }
    };

    // Structure function integrand
    let integrand = |f: f64| -> f64 {
        let f0 = 1.0 / l0;

        // von Karman spectrum constant
        let gamma_val = 11.0 / 6.0;
        let gamma_factor = gamma(gamma_val);
        let cst = PI * gamma_factor * gamma_factor / (2.0 * PI.powf(11.0 / 3.0))
            * (24.0 * gamma(6.0 / 5.0) / 5.0).powf(5.0 / 6.0)
            * wavelength * wavelength
            * r0.powf(-5.0 / 3.0);

        let mut sum = 0.0;
        for k in 0..altitude.len() {
            let rho = separation * altitude[k];
            let red = 2.0 * PI * rho * f;
            let j0_val = bessel_j0(red);

            sum += fr0[k]
                * f.powi(3)
                * (f * f + f0 * f0).powf(-11.0 / 6.0)
                * g_function(f, diameter)
                * (1.0 - j0_val);
        }

        4.0 * cst * sum
    };

    // Numerical integration using adaptive Simpson's rule
    let variance = adaptive_simpson(integrand, 0.0, 100.0, 1e-6, 10)?;

    Ok(variance)
}

/// Adaptive Simpson's rule for numerical integration
///
/// Replaces scipy.integrate.quad for our specific use case
fn adaptive_simpson<F>(
    f: F,
    a: f64,
    b: f64,
    tol: f64,
    max_depth: usize,
) -> Result<f64>
where
    F: Fn(f64) -> f64,
{
    fn simpson_recursive<F>(
        f: &F,
        a: f64,
        b: f64,
        fa: f64,
        fb: f64,
        fm: f64,
        s: f64,
        tol: f64,
        depth: usize,
        max_depth: usize,
    ) -> Result<f64>
    where
        F: Fn(f64) -> f64,
    {
        if depth >= max_depth {
            return Ok(s);
        }

        let m = (a + b) / 2.0;
        let h = (b - a) / 2.0;
        let lm = (a + m) / 2.0;
        let rm = (m + b) / 2.0;

        let flm = f(lm);
        let frm = f(rm);

        let s_left = h / 6.0 * (fa + 4.0 * flm + fm);
        let s_right = h / 6.0 * (fm + 4.0 * frm + fb);
        let s2 = s_left + s_right;

        if (s2 - s).abs() <= 15.0 * tol {
            Ok(s2 + (s2 - s) / 15.0)
        } else {
            let left = simpson_recursive(
                f,
                a,
                m,
                fa,
                fm,
                flm,
                s_left,
                tol / 2.0,
                depth + 1,
                max_depth,
            )?;
            let right = simpson_recursive(
                f,
                m,
                b,
                fm,
                fb,
                frm,
                s_right,
                tol / 2.0,
                depth + 1,
                max_depth,
            )?;
            Ok(left + right)
        }
    }

    let fa = f(a);
    let fb = f(b);
    let m = (a + b) / 2.0;
    let fm = f(m);
    let h = (b - a) / 2.0;
    let s = h / 3.0 * (fa + 4.0 * fm + fb);

    simpson_recursive(&f, a, b, fa, fb, fm, s, tol, 0, max_depth)
}

/// Calculate TT7 tilt error
///
/// Equivalent to Python's `tt7_tt_error` (atp.py:28-51)
///
/// # Arguments
/// * `separation_arcmin` - Angular separation from target (arcmin)
/// * `magnitude` - Guide star V magnitude
/// * `zenith_distance` - Zenith distance (radians)
/// * `config` - Configuration containing WFS and atmosphere parameters
///
/// # Returns
/// RMS tilt error in milliarcseconds
pub fn tt7_tt_error(
    separation_arcmin: f64,
    magnitude: f64,
    zenith_distance: f64,
    config: &Config,
) -> Result<f64> {
    // Get configuration parameters
    let gs_wavelength = config.tt7.guide_star.wavelength.0 * 1e-9; // nm to m
    let r0_wavelength = config.atmosphere.wavelength.0 * 1e-9; // nm to m
    let mut r0 = config.atmosphere.r0.0 / 100.0; // cm to m

    // Scale r0 for wavelength and zenith distance
    r0 *= r0_scaling(r0_wavelength, gs_wavelength, zenith_distance);

    let diameter = config.telescope.diameter.0;
    let l0 = config.atmosphere.l0.0;
    let altitude: Vec<f64> = config.atmosphere.altitude.0.clone();
    let fr0 = &config.atmosphere.fr0;

    // Calculate anisoplanatism
    let separation_rad = separation_arcmin * ARCMIN2RAD;
    let anisop = tilt_anisoplanatism(
        separation_rad,
        r0,
        gs_wavelength,
        l0,
        diameter,
        &altitude,
        fr0,
    )?;

    // Seeing in arcseconds
    let seeing_arcsec = gs_wavelength / r0;

    // WFS photon budget
    let wfs_gain = config.tt7.optics.throughput * config.tt7.detector.quantum_efficiency;
    let gs_n_photon = config.tt7.guide_star.zero_point * 10.0_f64.powf(-0.4 * magnitude);

    let exposure_sec = match config.tt7.detector.exposure.1.as_str() {
        "ms" => config.tt7.detector.exposure.0 / 1000.0,
        "s" => config.tt7.detector.exposure.0,
        _ => config.tt7.detector.exposure.0 / 1000.0,
    };

    let n_ph_lenslet = exposure_sec * wfs_gain * gs_n_photon * config.telescope.area
        / config.tt7.optics.lenslet.array as f64;

    // Photon noise
    let pn = photon_noise_variance(seeing_arcsec, n_ph_lenslet);

    // Readout noise
    let ron2 = config.tt7.detector.read_out_noise.powi(2);
    let pixel_scale = 0.4 * crate::constants::ARCSEC2RAD;
    let rn = readout_noise_variance(n_ph_lenslet, ron2, pixel_scale, 144.0, 0.0);

    // Total error (2 axes for tilt)
    let total_variance = anisop + 2.0 * (pn + rn);
    let rms_rad = total_variance.sqrt();

    // Convert to milliarcseconds
    Ok(rms_rad * crate::constants::RAD2MAS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_photon_noise_variance() {
        let fwhm = 1.0e-5; // ~2 arcsec in radians
        let n_ph = 1000.0;

        let var = photon_noise_variance(fwhm, n_ph);

        // Should be positive and reasonable
        assert!(var > 0.0);
        assert!(var < fwhm * fwhm);

        // More photons = less noise
        let var_more = photon_noise_variance(fwhm, 10000.0);
        assert!(var_more < var);
    }

    #[test]
    fn test_readout_noise_variance() {
        let n_ph = 1000.0;
        let ron2 = 9.0; // 3 e- RMS readout noise
        let pixel_scale = 0.1e-5; // radians
        let n_subap_px2 = 100.0;

        let var = readout_noise_variance(n_ph, ron2, pixel_scale, n_subap_px2, 0.0);

        // Should be positive
        assert!(var > 0.0);

        // More photons = less readout noise impact
        let var_more = readout_noise_variance(10000.0, ron2, pixel_scale, n_subap_px2, 0.0);
        assert!(var_more < var);
    }

    #[test]
    fn test_r0_scaling() {
        let atm_wl = 500e-9; // 500 nm
        let gs_wl = 700e-9; // 700 nm (R-band)

        // At zenith
        let scale = r0_scaling(atm_wl, gs_wl, 0.0);
        assert!(scale > 1.0); // r0 increases with wavelength

        // At 45 degrees zenith
        let scale_45 = r0_scaling(atm_wl, gs_wl, 45.0_f64.to_radians());
        assert!(scale_45 < scale); // r0 decreases off-zenith
    }

    #[test]
    fn test_adaptive_simpson() {
        // Test with simple function: integral of x^2 from 0 to 1 = 1/3
        let f = |x: f64| x * x;
        let result = adaptive_simpson(f, 0.0, 1.0, 1e-6, 10).unwrap();
        assert_relative_eq!(result, 1.0 / 3.0, epsilon = 1e-5);

        // Test with exponential: integral of e^x from 0 to 1 = e - 1
        let f_exp = |x: f64| x.exp();
        let result_exp = adaptive_simpson(f_exp, 0.0, 1.0, 1e-6, 10).unwrap();
        assert_relative_eq!(result_exp, 1.0_f64.exp() - 1.0, epsilon = 1e-5);
    }

    #[test]
    fn test_tilt_anisoplanatism() {
        // Simple test case with single layer
        let separation = 1.0 * ARCMIN2RAD; // 1 arcmin
        let r0 = 0.16; // 16 cm
        let wavelength = 500e-9;
        let l0 = 25.0;
        let diameter = 25.5;
        let altitude = vec![10000.0]; // 10 km
        let fr0 = vec![1.0];

        let result = tilt_anisoplanatism(
            separation,
            r0,
            wavelength,
            l0,
            diameter,
            &altitude,
            &fr0,
        )
        .unwrap();

        // Should be positive and reasonable magnitude
        assert!(result > 0.0);
        assert!(result < 1e-10); // rad^2, should be small
    }
}
