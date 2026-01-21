//! Error types for ATP-RS (AGWS Target Practice)

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AtpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Time parsing error: {0}")]
    Time(String),

    #[error("Coordinate transformation error: {0}")]
    Coordinate(String),

    #[error("Catalog query error: {0}")]
    Catalog(String),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("CEO/crseo error: {0}")]
    Ceo(String),

    #[error("Numerical error: {0}")]
    Numerical(String),

    #[error("Guide star selection error: {0}")]
    GuideStarSelection(String),
}

pub type Result<T> = std::result::Result<T, AtpError>;
