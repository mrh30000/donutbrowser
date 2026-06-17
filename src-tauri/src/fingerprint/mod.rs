pub mod auto_match;
pub mod camoufox_adapter;
pub mod identity;
pub mod network;
pub mod noise;
pub mod profile;

use profile::FingerprintProfile;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FingerprintError {
  #[error("JSON serialization error: {0}")]
  Json(#[from] serde_json::Error),
  #[error("Unsupported feature for engine: {0}")]
  Unsupported(String),
  #[error("Geolocation error: {0}")]
  Geo(String),
}

/// Engine-agnostic launch configuration container.
/// Each engine adapter writes to the fields it uses.
#[derive(Debug, Default)]
pub struct EngineLaunchConfig {
  pub args: Vec<String>,
  /// Environment variables (Camoufox)
  pub env_vars: HashMap<String, String>,
  /// Firefox preferences (Camoufox)
  pub firefox_prefs: HashMap<String, serde_json::Value>,
  /// Camoufox config map (key-value pairs that become CAMOU_CONFIG_N env vars)
  pub fingerprint_config: HashMap<String, serde_json::Value>,
}

/// Trait implemented by each engine adapter.
pub trait FingerprintAdapter {
  fn apply(
    &self,
    profile: &FingerprintProfile,
    engine_config: &mut EngineLaunchConfig,
  ) -> Result<(), FingerprintError>;
}
