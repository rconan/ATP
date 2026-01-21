//! Astronomical catalog queries (TIC/MAST)
//!
//! This module provides asynchronous queries to the TESS Input Catalog (TIC)
//! via the MAST (Mikulski Archive for Space Telescopes) API.

use crate::errors::{AtpError, Result};
use crate::starfield::StarField;
use crate::target::Target;
use reqwest;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// MAST API base URL
const MAST_API_URL: &str = "https://mast.stsci.edu/api/v0/invoke";

/// TIC catalog entry from MAST query
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TICEntry {
    /// TIC ID
    #[serde(rename = "ID")]
    pub id: Option<String>,

    /// Right ascension (degrees)
    #[serde(rename = "ra")]
    pub ra: f64,

    /// Declination (degrees)
    #[serde(rename = "dec")]
    pub dec: f64,

    /// TESS magnitude
    #[serde(rename = "Tmag")]
    pub t_mag: Option<f64>,

    /// V-band magnitude
    #[serde(rename = "Vmag")]
    pub v_mag: Option<f64>,

    /// J-band magnitude (2MASS)
    #[serde(rename = "Jmag")]
    pub j_mag: Option<f64>,

    /// H-band magnitude (2MASS)
    #[serde(rename = "Hmag")]
    pub h_mag: Option<f64>,

    /// K-band magnitude (2MASS)
    #[serde(rename = "Kmag")]
    pub k_mag: Option<f64>,

    /// Object type
    #[serde(rename = "objType")]
    pub obj_type: Option<String>,

    /// Disposition flag
    #[serde(rename = "disposition")]
    pub disposition: Option<String>,

    /// Proper motion in RA (mas/yr)
    #[serde(rename = "pmRA")]
    pub pm_ra: Option<f64>,

    /// Proper motion in Dec (mas/yr)
    #[serde(rename = "pmDEC")]
    pub pm_dec: Option<f64>,
}

/// MAST API request for catalog cone search
#[derive(Debug, Serialize)]
struct MastConeSearchRequest {
    service: String,
    params: ConeSearchParams,
    format: String,
    timeout: u32,
}

#[derive(Debug, Serialize)]
struct ConeSearchParams {
    ra: f64,
    dec: f64,
    radius: f64,
}

/// MAST API response wrapper
#[derive(Debug, Deserialize)]
struct MastResponse {
    status: String,
    msg: Option<String>,
    data: Option<Vec<TICEntry>>,
}

/// TIC catalog query client
pub struct TICCatalog {
    client: reqwest::Client,
    timeout_sec: u64,
}

impl TICCatalog {
    /// Create a new TIC catalog client
    ///
    /// # Arguments
    /// * `timeout_sec` - HTTP request timeout in seconds (default: 30)
    pub fn new(timeout_sec: Option<u64>) -> Result<Self> {
        let timeout = timeout_sec.unwrap_or(30);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout))
            .user_agent("ATP-RS/0.1.0")
            .build()
            .map_err(|e| AtpError::Http(e))?;

        Ok(Self {
            client,
            timeout_sec: timeout,
        })
    }

    /// Query TIC catalog for stars in a cone
    ///
    /// # Arguments
    /// * `ra` - Right ascension in degrees
    /// * `dec` - Declination in degrees
    /// * `radius` - Search radius in degrees
    ///
    /// # Returns
    /// Vector of TIC entries
    pub async fn cone_search(
        &self,
        ra: f64,
        dec: f64,
        radius: f64,
    ) -> Result<Vec<TICEntry>> {
        log::info!("Querying TIC: RA={:.4}°, Dec={:.4}°, radius={:.4}°", ra, dec, radius);

        let request = MastConeSearchRequest {
            service: "Mast.Catalogs.Filtered.Tic.Cone".to_string(),
            params: ConeSearchParams { ra, dec, radius },
            format: "json".to_string(),
            timeout: self.timeout_sec as u32,
        };

        let response = self
            .client
            .post(MAST_API_URL)
            .json(&request)
            .send()
            .await
            .map_err(|e| AtpError::Catalog(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AtpError::Catalog(format!(
                "MAST API returned status: {}",
                response.status()
            )));
        }

        let mast_response: MastResponse = response
            .json()
            .await
            .map_err(|e| AtpError::Catalog(format!("Failed to parse response: {}", e)))?;

        if mast_response.status != "COMPLETE" && mast_response.status != "EXECUTING" {
            return Err(AtpError::Catalog(format!(
                "MAST query failed: {}",
                mast_response.msg.unwrap_or_else(|| "Unknown error".to_string())
            )));
        }

        let entries = mast_response.data.unwrap_or_default();
        log::info!("Retrieved {} TIC entries", entries.len());

        Ok(entries)
    }

    /// Query TIC catalog around a target
    ///
    /// # Arguments
    /// * `target` - Target for coordinate center
    /// * `radius_arcmin` - Search radius in arcminutes
    ///
    /// # Returns
    /// Vector of TIC entries
    pub async fn query_around_target(
        &self,
        target: &Target,
        radius_arcmin: f64,
    ) -> Result<Vec<TICEntry>> {
        let radius_deg = radius_arcmin / 60.0;
        self.cone_search(target.ra_deg(), target.dec_deg(), radius_deg)
            .await
    }

    /// Build a StarField from TIC query results
    ///
    /// # Arguments
    /// * `entries` - TIC catalog entries
    /// * `v_mag_limit` - Maximum V magnitude to include
    /// * `exclude_radius_arcmin` - Exclusion radius from target (arcmin)
    /// * `target` - Target for filtering
    ///
    /// # Returns
    /// Populated StarField
    pub fn build_starfield(
        &self,
        entries: Vec<TICEntry>,
        v_mag_limit: f64,
        exclude_radius_arcmin: Option<f64>,
        target: Option<&Target>,
    ) -> Result<StarField> {
        let mut field = StarField::new(
            v_mag_limit,
            exclude_radius_arcmin,
            vec!["V".to_string(), "J".to_string()],
        );

        let mut added = 0;
        let mut filtered = 0;

        for entry in entries {
            // Filter by disposition (exclude problematic stars)
            if let Some(ref disp) = entry.disposition {
                if disp == "SPLIT" || disp.starts_with("DUPLICATE") {
                    filtered += 1;
                    continue;
                }
            }

            // Require V and J magnitudes
            let v_mag = match entry.v_mag {
                Some(v) if !v.is_nan() => v,
                _ => {
                    filtered += 1;
                    continue;
                }
            };

            let j_mag = match entry.j_mag {
                Some(j) if !j.is_nan() => j,
                _ => {
                    filtered += 1;
                    continue;
                }
            };

            // Check V magnitude limit
            if v_mag > v_mag_limit {
                filtered += 1;
                continue;
            }

            // Check exclusion radius
            if let Some(tgt) = target {
                if let Some(exclude_rad) = exclude_radius_arcmin {
                    let dx = (entry.ra - tgt.ra_deg()) * tgt.dec_deg().to_radians().cos();
                    let dy = entry.dec - tgt.dec_deg();
                    let separation = (dx * dx + dy * dy).sqrt() * 60.0; // degrees to arcmin

                    if separation < exclude_rad {
                        filtered += 1;
                        continue;
                    }
                }
            }

            field.add_star(
                entry.ra.to_radians(),
                entry.dec.to_radians(),
                v_mag,
                j_mag,
            );
            added += 1;
        }

        log::info!(
            "Built starfield: {} stars added, {} filtered",
            added,
            filtered
        );

        Ok(field)
    }

    /// Complete workflow: query TIC and build starfield
    ///
    /// # Arguments
    /// * `target` - Target for cone search center
    /// * `radius_arcmin` - Search radius (arcmin)
    /// * `v_mag_limit` - Maximum V magnitude
    /// * `exclude_radius_arcmin` - Exclusion radius from target (arcmin)
    ///
    /// # Returns
    /// Populated and filtered StarField
    pub async fn query_and_build_starfield(
        &self,
        target: &Target,
        radius_arcmin: f64,
        v_mag_limit: f64,
        exclude_radius_arcmin: Option<f64>,
    ) -> Result<StarField> {
        let entries = self.query_around_target(target, radius_arcmin).await?;
        self.build_starfield(entries, v_mag_limit, exclude_radius_arcmin, Some(target))
    }
}

/// Convenience function to query TIC catalog
///
/// # Arguments
/// * `ra` - Right ascension (degrees)
/// * `dec` - Declination (degrees)
/// * `radius_arcmin` - Search radius (arcminutes)
/// * `timeout_sec` - HTTP timeout (seconds)
///
/// # Returns
/// Vector of TIC entries
pub async fn query_tic(
    ra: f64,
    dec: f64,
    radius_arcmin: f64,
    timeout_sec: Option<u64>,
) -> Result<Vec<TICEntry>> {
    let catalog = TICCatalog::new(timeout_sec)?;
    catalog.cone_search(ra, dec, radius_arcmin / 60.0).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tic_catalog_creation() {
        let catalog = TICCatalog::new(Some(30));
        assert!(catalog.is_ok());

        let cat = catalog.unwrap();
        assert_eq!(cat.timeout_sec, 30);
    }

    #[test]
    fn test_build_starfield() {
        let catalog = TICCatalog::new(Some(30)).unwrap();

        // Create mock TIC entries
        let entries = vec![
            TICEntry {
                id: Some("12345".to_string()),
                ra: 90.0,
                dec: 16.0,
                t_mag: Some(10.0),
                v_mag: Some(10.5),
                j_mag: Some(9.8),
                h_mag: Some(9.5),
                k_mag: Some(9.3),
                obj_type: Some("STAR".to_string()),
                disposition: None,
                pm_ra: Some(5.0),
                pm_dec: Some(-3.0),
            },
            TICEntry {
                id: Some("67890".to_string()),
                ra: 90.1,
                dec: 16.1,
                t_mag: Some(12.0),
                v_mag: Some(12.5),
                j_mag: Some(11.8),
                h_mag: Some(11.5),
                k_mag: Some(11.3),
                obj_type: Some("STAR".to_string()),
                disposition: None,
                pm_ra: Some(-2.0),
                pm_dec: Some(4.0),
            },
            // Bad entry - no V mag
            TICEntry {
                id: Some("99999".to_string()),
                ra: 90.2,
                dec: 16.2,
                t_mag: Some(11.0),
                v_mag: None,
                j_mag: Some(10.5),
                h_mag: None,
                k_mag: None,
                obj_type: Some("STAR".to_string()),
                disposition: None,
                pm_ra: None,
                pm_dec: None,
            },
        ];

        let field = catalog.build_starfield(entries, 18.0, None, None);
        assert!(field.is_ok());

        let starfield = field.unwrap();
        // Should have 2 stars (third filtered due to missing V mag)
        assert_eq!(starfield.len(), 2);
    }

    // Note: Actual HTTP tests are skipped by default to avoid network dependency
    // Run with: cargo test -- --ignored
    #[tokio::test]
    #[ignore]
    async fn test_real_tic_query() {
        let catalog = TICCatalog::new(Some(60)).unwrap();

        // Query a known region (near M45 - Pleiades)
        let result = catalog.cone_search(56.75, 24.12, 0.1).await;

        match result {
            Ok(entries) => {
                println!("Retrieved {} TIC entries", entries.len());
                assert!(!entries.is_empty());

                // Print first few entries
                for (i, entry) in entries.iter().take(5).enumerate() {
                    println!("  [{}] RA={:.4}, Dec={:.4}, V={:?}",
                             i, entry.ra, entry.dec, entry.v_mag);
                }
            }
            Err(e) => {
                println!("Query failed (this is OK if no internet): {}", e);
            }
        }
    }
}
