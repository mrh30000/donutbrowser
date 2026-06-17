use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FingerprintProfile {
  // -- Identity --
  pub os: Option<String>,
  pub user_agent: Option<UaConfig>,
  pub sec_ch_ua: Option<SecChUaConfig>,
  pub platform: Option<String>,
  pub language: Option<LanguageConfig>,

  // -- Network --
  pub timezone: Option<TimezoneConfig>,
  pub webrtc: Option<WebRtcConfig>,
  pub ssl: Option<SslConfig>,

  // -- Geolocation --
  pub geolocation: Option<GeoConfig>,

  // -- Screen --
  pub screen: Option<ScreenConfig>,
  pub fonts: Option<FontConfig>,

  // -- Rendering --
  pub canvas: Option<NoiseMode>,
  pub webgl_img: Option<NoiseMode>,
  pub webgl: Option<WebGlConfig>,
  pub audio_context: Option<NoiseMode>,
  pub client_rects: Option<NoiseMode>,
  pub speech_voices: Option<NoiseMode>,

  // -- Hardware --
  pub cpu: Option<u32>,
  pub memory: Option<u32>,
  pub gpu_acceleration: Option<bool>,

  // -- Privacy --
  pub dnt: Option<bool>,
  pub homepage: Option<String>,
  pub cookie_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoiseMode {
  Default,
  Random,
}

#[allow(clippy::derivable_impls)]
impl Default for NoiseMode {
  fn default() -> Self {
    Self::Default
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoOrManual {
  Auto,
  Manual,
}

#[allow(clippy::derivable_impls)]
impl Default for AutoOrManual {
  fn default() -> Self {
    Self::Manual
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UaConfig {
  pub mode: AutoOrManual,
  pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecChUaConfig {
  pub mode: AutoOrManual,
  pub brands: Vec<SecChUaBrand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecChUaBrand {
  pub brand: String,
  pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguageConfig {
  pub mode: AutoOrManual,
  pub language: Option<String>,
  pub languages: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimezoneConfig {
  pub mode: AutoOrManual,
  pub name: Option<String>,
  pub offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebRtcMode {
  Replace,
  Allow,
  Block,
}

#[allow(clippy::derivable_impls)]
impl Default for WebRtcMode {
  fn default() -> Self {
    Self::Allow
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebRtcConfig {
  pub mode: WebRtcMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SslConfig {
  pub enabled: bool,
  pub disabled_versions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeoConfig {
  pub mode: AutoOrManual,
  pub longitude: Option<f64>,
  pub latitude: Option<f64>,
  pub precision: Option<u32>,
  pub permission: GeoPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoPermission {
  Ask,
  Allow,
  Block,
}

#[allow(clippy::derivable_impls)]
impl Default for GeoPermission {
  fn default() -> Self {
    Self::Ask
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScreenConfig {
  pub mode: AutoOrManual,
  pub width: Option<u32>,
  pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FontConfig {
  pub mode: FontMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FontMode {
  SystemDefault,
  RandomMatch,
}

#[allow(clippy::derivable_impls)]
impl Default for FontMode {
  fn default() -> Self {
    Self::SystemDefault
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebGlConfig {
  pub mode: AutoOrManual,
  pub vendor: Option<String>,
  pub renderer: Option<String>,
}
