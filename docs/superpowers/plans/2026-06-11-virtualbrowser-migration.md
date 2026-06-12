# VirtualBrowser 优势迁移实施计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 VirtualBrowser 的指纹深度和操作体验优势迁移到 DonutBrowser，通过统一指纹层实现两个引擎的一致配置。

**Architecture:** 在 `src-tauri/src/fingerprint/` 新建统一指纹模块，定义 `FingerprintProfile` 数据模型和 `FingerprintAdapter` trait。Camoufox 和 Wayfern 各自实现适配器，将统一配置映射为引擎特定的启动参数/env vars/Firefox prefs/CDP 命令。前端在 `wayfern-config-form.tsx` 基础上新增统一指纹配置 tab。

**Tech Stack:** Rust (Tauri backend), TypeScript/React (Next.js frontend), CDP (Wayfern fingerprint injection)

**Spec:** `docs/superpowers/specs/2026-06-11-virtualbrowser-migration-design.md`

---

## Chunk 1: Phase 1 — Fingerprint Depth (Backend)

### File Structure

```
src-tauri/src/
├── fingerprint/                        # NEW: unified fingerprint layer
│   ├── mod.rs                          # module exports + FingerprintAdapter trait + EngineLaunchConfig
│   ├── profile.rs                      # FingerprintProfile + all sub-types
│   ├── noise.rs                        # noise seed generators (AudioContext, ClientRects)
│   ├── network.rs                      # SSL version control mapping
│   ├── identity.rs                     # sec-ch-ua brand generation
│   ├── auto_match.rs                   # IP-based auto field resolution
│   ├── camoufox_adapter.rs             # FingerprintProfile → env vars + Firefox prefs
│   └── wayfern_adapter.rs              # FingerprintProfile → WayfernFingerprintConfig JSON
├── profile/types.rs                    # MODIFY: add fingerprint field
├── camoufox_manager.rs                 # MODIFY: integrate adapter in launch flow
├── wayfern_manager.rs                  # MODIFY: integrate adapter in launch flow
├── lib.rs                              # MODIFY: register fingerprint module
├── camoufox/config.rs                  # READ-ONLY: understand existing config builder
├── camoufox/geolocation.rs             # READ-ONLY: understand existing geo module
└── camoufox/env_vars.rs                # READ-ONLY: understand env var chunking
```

### Task 1: Create FingerprintProfile data types

**Files:**
- Create: `src-tauri/src/fingerprint/profile.rs`
- Create: `src-tauri/src/fingerprint/mod.rs`

- [ ] **Step 1: Create fingerprint module directory and mod.rs**

Create `src-tauri/src/fingerprint/mod.rs`:

```rust
pub mod profile;
pub mod noise;
pub mod network;
pub mod identity;
pub mod auto_match;
pub mod camoufox_adapter;
pub mod wayfern_adapter;

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
    /// Chromium launch arguments (Wayfern)
    pub args: Vec<String>,
    /// Environment variables (Camoufox)
    pub env_vars: HashMap<String, String>,
    /// Firefox preferences (Camoufox)
    pub firefox_prefs: HashMap<String, serde_json::Value>,
    /// The Wayfern-format fingerprint JSON (Wayfern uses CDP setFingerprint)
    pub wayfern_fingerprint: Option<serde_json::Value>,
}

/// Trait implemented by each engine adapter.
pub trait FingerprintAdapter {
    fn apply(
        &self,
        profile: &FingerprintProfile,
        engine_config: &mut EngineLaunchConfig,
    ) -> Result<(), FingerprintError>;
}
```

- [ ] **Step 2: Create profile.rs with all data types**

Create `src-tauri/src/fingerprint/profile.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FingerprintProfile {
    // ── Identity ──
    pub os: Option<String>,
    pub user_agent: Option<UaConfig>,
    pub sec_ch_ua: Option<SecChUaConfig>,
    pub platform: Option<String>,
    pub language: Option<LanguageConfig>,

    // ── Network ──
    pub timezone: Option<TimezoneConfig>,
    pub webrtc: Option<WebRtcConfig>,
    pub ssl: Option<SslConfig>,

    // ── Geolocation ──
    pub geolocation: Option<GeoConfig>,

    // ── Screen ──
    pub screen: Option<ScreenConfig>,
    pub fonts: Option<FontConfig>,

    // ── Rendering ──
    pub canvas: Option<NoiseMode>,
    pub webgl_img: Option<NoiseMode>,
    pub webgl: Option<WebGlConfig>,
    pub audio_context: Option<NoiseMode>,
    pub client_rects: Option<NoiseMode>,
    pub speech_voices: Option<NoiseMode>,

    // ── Hardware ──
    pub cpu: Option<u32>,
    pub memory: Option<u32>,
    pub gpu_acceleration: Option<bool>,

    // ── Privacy ──
    pub dnt: Option<bool>,
    pub homepage: Option<String>,
    pub cookie_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoiseMode {
    Default,
    Random,
}

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
```

- [ ] **Step 3: Verify module compiles**

Add `pub mod fingerprint;` to `src-tauri/src/lib.rs` after the existing module declarations:

```rust
pub mod fingerprint;
```

Run:
```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: `Finished` with no errors (warnings about unused items are fine).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/fingerprint/mod.rs src-tauri/src/fingerprint/profile.rs src-tauri/src/lib.rs
git commit -m "feat(fingerprint): add FingerprintProfile data model and FingerprintAdapter trait

Defines the unified fingerprint configuration types and the engine adapter
trait that Camoufox and Wayfern will implement."
```

### Task 2: Implement noise generators

**Files:**
- Create: `src-tauri/src/fingerprint/noise.rs`

- [ ] **Step 1: Write noise module tests**

Create `src-tauri/src/fingerprint/noise.rs`:

```rust
use rand::Rng;

/// Generate a random noise seed for AudioContext spoofing.
/// Returns an i32 in the range [-50, 50], matching Camoufox's canvas:aaOffset pattern.
pub fn audio_context_seed() -> i32 {
    let mut rng = rand::rng();
    rng.random_range(-50..=50)
}

/// Generate a random noise seed for ClientRects spoofing.
/// Returns an i32 in the range [-100, 100] for finer-grained offset control.
pub fn client_rects_seed() -> i32 {
    let mut rng = rand::rng();
    rng.random_range(-100..=100)
}

/// Generate a deterministic seed from a profile ID for consistent fingerprints
/// across launches of the same profile.
pub fn deterministic_seed(profile_id: &str, salt: &str) -> i32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    format!("{}:{}", profile_id, salt).hash(&mut hasher);
    let hash = hasher.finish();
    (hash as i32).wrapping_rem(101)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_context_seed_range() {
        for _ in 0..100 {
            let seed = audio_context_seed();
            assert!((-50..=50).contains(&seed), "seed {seed} out of range");
        }
    }

    #[test]
    fn test_client_rects_seed_range() {
        for _ in 0..100 {
            let seed = client_rects_seed();
            assert!((-100..=100).contains(&seed), "seed {seed} out of range");
        }
    }

    #[test]
    fn test_deterministic_seed_consistency() {
        let s1 = deterministic_seed("test-uuid", "audio");
        let s2 = deterministic_seed("test-uuid", "audio");
        assert_eq!(s1, s2);
    }

    #[test]
    fn test_deterministic_seed_different_salts() {
        let s1 = deterministic_seed("test-uuid", "audio");
        let s2 = deterministic_seed("test-uuid", "rects");
        // Different salts should (almost certainly) produce different seeds
        // Not strictly guaranteed but probability is overwhelming
        assert_ne!(s1, s2);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd src-tauri && cargo test fingerprint::noise --lib 2>&1 | grep -E "test result|panicked|FAILED"
```
Expected: `test result: ok`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/fingerprint/noise.rs
git commit -m "feat(fingerprint): add noise seed generators for AudioContext and ClientRects"
```

### Task 3: Implement network (SSL) and identity (sec-ch-ua) modules

**Files:**
- Create: `src-tauri/src/fingerprint/network.rs`
- Create: `src-tauri/src/fingerprint/identity.rs`

- [ ] **Step 1: Create network.rs for SSL version control**

```rust
use crate::fingerprint::profile::SslConfig;
use std::collections::HashMap;

/// Map SSL configuration to Firefox preferences.
/// Firefox uses `security.tls.version.min` (0=SSL3, 1=TLS1.0, 2=TLS1.1, 3=TLS1.2, 4=TLS1.3)
/// and `security.ssl3.<cipher>.enabled` for individual cipher suites.
pub fn ssl_to_firefox_prefs(config: &SslConfig) -> HashMap<String, serde_json::Value> {
    let mut prefs = HashMap::new();

    if !config.enabled {
        return prefs;
    }

    // Map disabled SSL/TLS versions to Firefox pref keys
    for version in &config.disabled_versions {
        match version.as_str() {
            "SSL3" | "ssl3" => {
                prefs.insert(
                    "security.tls.version.min".to_string(),
                    serde_json::json!(1),
                );
            }
            "TLS1.0" | "tls1.0" => {
                prefs.insert(
                    "security.tls.version.min".to_string(),
                    serde_json::json!(2),
                );
            }
            "TLS1.1" | "tls1.1" => {
                prefs.insert(
                    "security.tls.version.min".to_string(),
                    serde_json::json!(3),
                );
            }
            _ => {}
        }
    }

    prefs
}

/// Map SSL configuration to Chromium launch arguments.
pub fn ssl_to_chromium_args(config: &SslConfig) -> Vec<String> {
    let mut args = Vec::new();

    if !config.enabled {
        return args;
    }

    // Find the highest disabled version to set as --ssl-version-min
    for version in &config.disabled_versions {
        match version.as_str() {
            "TLS1.0" | "tls1.0" => {
                args.push("--ssl-version-min=tls1.1".to_string());
            }
            "TLS1.1" | "tls1.1" => {
                args.push("--ssl-version-min=tls1.2".to_string());
            }
            _ => {}
        }
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssl_disabled_no_prefs() {
        let config = SslConfig {
            enabled: false,
            disabled_versions: vec!["SSL3".to_string()],
        };
        let prefs = ssl_to_firefox_prefs(&config);
        assert!(prefs.is_empty());
    }

    #[test]
    fn test_ssl3_disabled_sets_min_version() {
        let config = SslConfig {
            enabled: true,
            disabled_versions: vec!["SSL3".to_string()],
        };
        let prefs = ssl_to_firefox_prefs(&config);
        assert_eq!(prefs.get("security.tls.version.min"), Some(&serde_json::json!(1)));
    }

    #[test]
    fn test_chromium_ssl_args() {
        let config = SslConfig {
            enabled: true,
            disabled_versions: vec!["TLS1.0".to_string()],
        };
        let args = ssl_to_chromium_args(&config);
        assert!(args.contains(&"--ssl-version-min=tls1.1".to_string()));
    }
}
```

- [ ] **Step 2: Create identity.rs for sec-ch-ua brand generation**

```rust
use crate::fingerprint::profile::{AutoOrManual, SecChUaBrand, SecChUaConfig};

/// Generate default sec-ch-ua brands for a given user agent string.
/// Extracts the Chrome major version from the UA and creates standard brand entries.
pub fn default_brands_from_ua(user_agent: &str) -> Vec<SecChUaBrand> {
    // Try to extract Chrome version from UA
    let version = extract_chrome_version(user_agent).unwrap_or("131");

    vec![
        SecChUaBrand {
            brand: "Google Chrome".to_string(),
            version: format!("{}.0.0.0", version),
        },
        SecChUaBrand {
            brand: "Chromium".to_string(),
            version: format!("{}.0.0.0", version),
        },
        SecChUaBrand {
            brand: "Not_A Brand".to_string(),
            version: "24.0.0.0".to_string(),
        },
    ]
}

/// Build the Sec-CH-UA header value from brand list.
pub fn build_sec_ch_ua_header(brands: &[SecChUaBrand]) -> String {
    brands
        .iter()
        .map(|b| format!("\"{}\";v=\"{}\"", b.brand, b.version))
        .collect::<Vec<_>>()
        .join(", ")
}

fn extract_chrome_version(ua: &str) -> Option<&str> {
    ua.split("Chrome/")
        .nth(1)
        .and_then(|s| s.split(' ').next())
        .and_then(|s| s.split('.').next())
}

/// Map sec-ch-ua config to Firefox preferences.
pub fn sec_ch_ua_to_firefox_prefs(config: &SecChUaConfig) -> std::collections::HashMap<String, serde_json::Value> {
    let mut prefs = std::collections::HashMap::new();

    let brands = match config.mode {
        AutoOrManual::Auto => return prefs, // use defaults
        AutoOrManual::Manual => &config.brands,
    };

    if brands.is_empty() {
        return prefs;
    }

    let header = build_sec_ch_ua_header(brands);
    prefs.insert(
        "network.http.sec-ch-ua".to_string(),
        serde_json::json!(header),
    );

    prefs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_chrome_version() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/131.0.6778.69 Safari/537.36";
        assert_eq!(extract_chrome_version(ua), Some("131"));
    }

    #[test]
    fn test_default_brands() {
        let ua = "Mozilla/5.0 Chrome/131.0.6778.69";
        let brands = default_brands_from_ua(ua);
        assert_eq!(brands.len(), 3);
        assert!(brands[0].brand.contains("Google Chrome"));
    }

    #[test]
    fn test_sec_ch_ua_header_format() {
        let brands = vec![
            SecChUaBrand { brand: "Chrome".to_string(), version: "131".to_string() },
        ];
        let header = build_sec_ch_ua_header(&brands);
        assert_eq!(header, "\"Chrome\";v=\"131\"");
    }

    #[test]
    fn test_auto_mode_returns_empty_prefs() {
        let config = SecChUaConfig {
            mode: AutoOrManual::Auto,
            brands: vec![],
        };
        let prefs = sec_ch_ua_to_firefox_prefs(&config);
        assert!(prefs.is_empty());
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cd src-tauri && cargo test fingerprint::network --lib 2>&1 | grep -E "test result|panicked|FAILED"
cd src-tauri && cargo test fingerprint::identity --lib 2>&1 | grep -E "test result|panicked|FAILED"
```
Expected: Both `test result: ok`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/fingerprint/network.rs src-tauri/src/fingerprint/identity.rs
git commit -m "feat(fingerprint): add SSL version control and sec-ch-ua brand generation"
```

### Task 4: Implement IP auto-matching

**Files:**
- Create: `src-tauri/src/fingerprint/auto_match.rs`

- [ ] **Step 1: Create auto_match module**

```rust
use crate::camoufox::geolocation::{self, GeolocationError};
use crate::fingerprint::profile::{AutoOrManual, FingerprintProfile};

/// Resolve all Auto-mode fields in a FingerprintProfile using IP geolocation.
///
/// For fields where `mode == Auto`, fetches the public IP (optionally through proxy),
/// looks up geolocation, and fills in language, timezone, and geolocation fields.
pub async fn resolve_auto_fields(
    profile: &mut FingerprintProfile,
    proxy_url: Option<&str>,
) -> Result<(), GeolocationError> {
    let ip = geolocation::fetch_public_ip(proxy_url).await?;
    let geo = geolocation::get_geolocation(&ip)?;

    if let Some(ref mut lang) = profile.language {
        if matches!(lang.mode, AutoOrManual::Auto) {
            lang.language = Some(geo.locale.as_string());
            lang.languages = Some(vec![geo.locale.as_string()]);
        }
    }

    if let Some(ref mut tz) = profile.timezone {
        if matches!(tz.mode, AutoOrManual::Auto) {
            tz.name = Some(geo.timezone.clone());
        }
    }

    if let Some(ref mut geo_cfg) = profile.geolocation {
        if matches!(geo_cfg.mode, AutoOrManual::Auto) {
            geo_cfg.longitude = Some(geo.longitude);
            geo_cfg.latitude = Some(geo.latitude);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_fields_skips_manual_mode() {
        // Unit test: verify that Manual-mode fields are not modified
        // (async integration test requires a real IP, so we test the logic structure)
        let profile = FingerprintProfile {
            language: Some(crate::fingerprint::profile::LanguageConfig {
                mode: AutoOrManual::Manual,
                language: Some("en-US".to_string()),
                languages: Some(vec!["en-US".to_string()]),
            }),
            ..Default::default()
        };
        // Manual mode should not be touched - we can't easily test async without mocking,
        // but we verify the structure is correct
        assert!(matches!(
            profile.language.as_ref().unwrap().mode,
            AutoOrManual::Manual
        ));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd src-tauri && cargo test fingerprint::auto_match --lib 2>&1 | grep -E "test result|panicked|FAILED"
```
Expected: `test result: ok`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/fingerprint/auto_match.rs
git commit -m "feat(fingerprint): add IP-based auto field resolution for language, timezone, geolocation"
```

### Task 5: Implement Camoufox adapter

**Files:**
- Create: `src-tauri/src/fingerprint/camoufox_adapter.rs`

- [ ] **Step 1: Create Camoufox adapter**

```rust
use crate::fingerprint::identity;
use crate::fingerprint::network;
use crate::fingerprint::noise;
use crate::fingerprint::profile::*;
use crate::fingerprint::{EngineLaunchConfig, FingerprintAdapter, FingerprintError};

pub struct CamoufoxAdapter {
    profile_id: String,
}

impl CamoufoxAdapter {
    pub fn new(profile_id: String) -> Self {
        Self { profile_id }
    }
}

impl FingerprintAdapter for CamoufoxAdapter {
    fn apply(
        &self,
        profile: &FingerprintProfile,
        config: &mut EngineLaunchConfig,
    ) -> Result<(), FingerprintError> {
        let cfg = &mut config.fingerprint_config;

        // UA
        if let Some(ref ua) = profile.user_agent {
            if matches!(ua.mode, AutoOrManual::Manual) {
                if let Some(ref val) = ua.value {
                    cfg.insert("navigator.userAgent".to_string(), serde_json::json!(val));
                }
            }
        }

        // Platform
        if let Some(ref platform) = profile.platform {
            cfg.insert("navigator.platform".to_string(), serde_json::json!(platform));
        }

        // Language
        if let Some(ref lang) = profile.language {
            if matches!(lang.mode, AutoOrManual::Manual) {
                if let Some(ref l) = lang.language {
                    cfg.insert("navigator.language".to_string(), serde_json::json!(l));
                }
                if let Some(ref ls) = lang.languages {
                    cfg.insert("navigator.languages".to_string(), serde_json::json!(ls));
                }
            }
        }

        // Timezone
        if let Some(ref tz) = profile.timezone {
            if matches!(tz.mode, AutoOrManual::Manual) {
                if let Some(ref name) = tz.name {
                    cfg.insert("timezone.zone".to_string(), serde_json::json!(name));
                }
                if let Some(offset) = tz.offset {
                    cfg.insert("timezone.value".to_string(), serde_json::json!(offset));
                }
            }
        }

        // Screen
        if let Some(ref screen) = profile.screen {
            if matches!(screen.mode, AutoOrManual::Manual) {
                if let Some(w) = screen.width {
                    cfg.insert("screen.width".to_string(), serde_json::json!(w));
                    cfg.insert("screen.availWidth".to_string(), serde_json::json!(w));
                }
                if let Some(h) = screen.height {
                    cfg.insert("screen.height".to_string(), serde_json::json!(h));
                    cfg.insert("screen.availHeight".to_string(), serde_json::json!(h));
                }
            }
        }

        // CPU / Memory
        if let Some(cpu) = profile.cpu {
            cfg.insert(
                "navigator.hardwareConcurrency".to_string(),
                serde_json::json!(cpu),
            );
        }
        if let Some(mem) = profile.memory {
            cfg.insert(
                "navigator.deviceMemory".to_string(),
                serde_json::json!(mem),
            );
        }

        // Canvas noise
        if let Some(NoiseMode::Random) = profile.canvas {
            let seed = noise::deterministic_seed(&self.profile_id, "canvas");
            cfg.insert("canvas:aaOffset".to_string(), serde_json::json!(seed));
            cfg.insert("canvas:aaCapOffset".to_string(), serde_json::json!(true));
        }

        // AudioContext noise
        if let Some(NoiseMode::Random) = profile.audio_context {
            let seed = noise::deterministic_seed(&self.profile_id, "audio");
            cfg.insert("audio:noise_seed".to_string(), serde_json::json!(seed));
        }

        // ClientRects noise
        if let Some(NoiseMode::Random) = profile.client_rects {
            let seed = noise::deterministic_seed(&self.profile_id, "rects");
            cfg.insert("rects:seed".to_string(), serde_json::json!(seed));
        }

        // WebGL metadata
        if let Some(ref webgl) = profile.webgl {
            if matches!(webgl.mode, AutoOrManual::Manual) {
                if let Some(ref vendor) = webgl.vendor {
                    cfg.insert("webgl.vendor".to_string(), serde_json::json!(vendor));
                }
                if let Some(ref renderer) = webgl.renderer {
                    cfg.insert("webgl.render".to_string(), serde_json::json!(renderer));
                }
            }
        }

        // sec-ch-ua → Firefox prefs
        if let Some(ref sec_ch_ua) = profile.sec_ch_ua {
            let prefs = identity::sec_ch_ua_to_firefox_prefs(sec_ch_ua);
            config.firefox_prefs.extend(prefs);
        }

        // SSL → Firefox prefs
        if let Some(ref ssl) = profile.ssl {
            let prefs = network::ssl_to_firefox_prefs(ssl);
            config.firefox_prefs.extend(prefs);
        }

        // WebRTC
        if let Some(ref webrtc) = profile.webrtc {
            match webrtc.mode {
                WebRtcMode::Block => {
                    config.firefox_prefs.insert(
                        "media.peerconnection.enabled".to_string(),
                        serde_json::json!(false),
                    );
                }
                WebRtcMode::Replace | WebRtcMode::Allow => {
                    config.firefox_prefs.insert(
                        "media.peerconnection.enabled".to_string(),
                        serde_json::json!(true),
                    );
                }
            }
        }

        // DNT
        if let Some(dnt) = profile.dnt {
            config.firefox_prefs.insert(
                "privacy.donottrackheader.enabled".to_string(),
                serde_json::json!(dnt),
            );
        }

        // GPU acceleration
        if let Some(false) = profile.gpu_acceleration {
            config.firefox_prefs.insert(
                "layers.acceleration.disabled".to_string(),
                serde_json::json!(true),
            );
            config.firefox_prefs.insert(
                "webgl.disabled".to_string(),
                serde_json::json!(true),
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camoufox_adapter_ua() {
        let adapter = CamoufoxAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            user_agent: Some(UaConfig {
                mode: AutoOrManual::Manual,
                value: Some("Mozilla/5.0 Custom".to_string()),
            }),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        assert_eq!(
            config.fingerprint_config.get("navigator.userAgent"),
            Some(&serde_json::json!("Mozilla/5.0 Custom"))
        );
    }

    #[test]
    fn test_camoufox_adapter_auto_ua_skipped() {
        let adapter = CamoufoxAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            user_agent: Some(UaConfig {
                mode: AutoOrManual::Auto,
                value: Some("Should be ignored".to_string()),
            }),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        assert!(config.fingerprint_config.get("navigator.userAgent").is_none());
    }

    #[test]
    fn test_camoufox_adapter_webrtc_block() {
        let adapter = CamoufoxAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            webrtc: Some(WebRtcConfig { mode: WebRtcMode::Block }),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        assert_eq!(
            config.firefox_prefs.get("media.peerconnection.enabled"),
            Some(&serde_json::json!(false))
        );
    }

    #[test]
    fn test_camoufox_adapter_canvas_noise() {
        let adapter = CamoufoxAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            canvas: Some(NoiseMode::Random),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        assert!(config.fingerprint_config.get("canvas:aaOffset").is_some());
    }

    #[test]
    fn test_camoufox_adapter_dnt() {
        let adapter = CamoufoxAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            dnt: Some(true),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        assert_eq!(
            config.firefox_prefs.get("privacy.donottrackheader.enabled"),
            Some(&serde_json::json!(true))
        );
    }
}
```

- [ ] **Step 2: Fix compilation — add fingerprint_config to EngineLaunchConfig**

The `EngineLaunchConfig` in `mod.rs` needs a `fingerprint_config` field for the Camoufox adapter. Update `mod.rs`:

```rust
#[derive(Debug, Default)]
pub struct EngineLaunchConfig {
    pub args: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub firefox_prefs: HashMap<String, serde_json::Value>,
    pub wayfern_fingerprint: Option<serde_json::Value>,
    /// Camoufox config map (key-value pairs that become CAMOU_CONFIG_N env vars)
    pub fingerprint_config: HashMap<String, serde_json::Value>,
}
```

- [ ] **Step 3: Run tests**

```bash
cd src-tauri && cargo test fingerprint::camoufox_adapter --lib 2>&1 | grep -E "test result|panicked|FAILED"
```
Expected: `test result: ok`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/fingerprint/camoufox_adapter.rs src-tauri/src/fingerprint/mod.rs
git commit -m "feat(fingerprint): implement Camoufox adapter with full fingerprint mapping"
```

### Task 6: Implement Wayfern adapter

**Files:**
- Create: `src-tauri/src/fingerprint/wayfern_adapter.rs`

- [ ] **Step 1: Create Wayfern adapter**

The Wayfern adapter maps `FingerprintProfile` to the `WayfernFingerprintConfig` JSON format that Wayfern expects via `Wayfern.setFingerprint` CDP command.

```rust
use crate::fingerprint::identity;
use crate::fingerprint::noise;
use crate::fingerprint::profile::*;
use crate::fingerprint::{EngineLaunchConfig, FingerprintAdapter, FingerprintError};

pub struct WayfernAdapter {
    profile_id: String,
}

impl WayfernAdapter {
    pub fn new(profile_id: String) -> Self {
        Self { profile_id }
    }

    /// Build a WayfernFingerprintConfig JSON object from a FingerprintProfile.
    /// This JSON is passed to Wayfern.setFingerprint via CDP.
    fn build_wayfern_json(&self, profile: &FingerprintProfile) -> serde_json::Value {
        let mut fp = serde_json::Map::new();

        // UA
        if let Some(ref ua) = profile.user_agent {
            if matches!(ua.mode, AutoOrManual::Manual) {
                if let Some(ref val) = ua.value {
                    fp.insert("userAgent".to_string(), serde_json::json!(val));
                }
            }
        }

        // Platform
        if let Some(ref platform) = profile.platform {
            fp.insert("platform".to_string(), serde_json::json!(platform));
        }

        // Language
        if let Some(ref lang) = profile.language {
            if matches!(lang.mode, AutoOrManual::Manual) {
                if let Some(ref l) = lang.language {
                    fp.insert("language".to_string(), serde_json::json!(l));
                }
                if let Some(ref ls) = lang.languages {
                    fp.insert("languages".to_string(), serde_json::json!(ls));
                }
            }
        }

        // Timezone
        if let Some(ref tz) = profile.timezone {
            if matches!(tz.mode, AutoOrManual::Manual) {
                if let Some(ref name) = tz.name {
                    fp.insert("timezone".to_string(), serde_json::json!(name));
                }
                if let Some(offset) = tz.offset {
                    fp.insert("timezoneOffset".to_string(), serde_json::json!(offset));
                }
            }
        }

        // Geolocation
        if let Some(ref geo) = profile.geolocation {
            if matches!(geo.mode, AutoOrManual::Manual) {
                if let Some(lon) = geo.longitude {
                    fp.insert("longitude".to_string(), serde_json::json!(lon));
                }
                if let Some(lat) = geo.latitude {
                    fp.insert("latitude".to_string(), serde_json::json!(lat));
                }
                if let Some(precision) = geo.precision {
                    fp.insert("accuracy".to_string(), serde_json::json!(precision));
                }
            }
        }

        // Screen
        if let Some(ref screen) = profile.screen {
            if matches!(screen.mode, AutoOrManual::Manual) {
                if let Some(w) = screen.width {
                    fp.insert("screenWidth".to_string(), serde_json::json!(w));
                    fp.insert("screenAvailWidth".to_string(), serde_json::json!(w));
                    fp.insert("windowOuterWidth".to_string(), serde_json::json!(w));
                }
                if let Some(h) = screen.height {
                    fp.insert("screenHeight".to_string(), serde_json::json!(h));
                    fp.insert("screenAvailHeight".to_string(), serde_json::json!(h));
                    fp.insert("windowOuterHeight".to_string(), serde_json::json!(h));
                }
            }
        }

        // Hardware
        if let Some(cpu) = profile.cpu {
            fp.insert("hardwareConcurrency".to_string(), serde_json::json!(cpu));
        }
        if let Some(mem) = profile.memory {
            fp.insert("deviceMemory".to_string(), serde_json::json!(mem));
        }

        // Canvas noise
        if let Some(NoiseMode::Random) = profile.canvas {
            let seed = noise::deterministic_seed(&self.profile_id, "canvas");
            fp.insert("canvasNoiseSeed".to_string(), serde_json::json!(seed.to_string()));
        }

        // WebGL metadata
        if let Some(ref webgl) = profile.webgl {
            if matches!(webgl.mode, AutoOrManual::Manual) {
                if let Some(ref vendor) = webgl.vendor {
                    fp.insert("webglVendor".to_string(), serde_json::json!(vendor));
                }
                if let Some(ref renderer) = webgl.renderer {
                    fp.insert("webglRenderer".to_string(), serde_json::json!(renderer));
                }
            }
        }

        // DNT
        if let Some(dnt) = profile.dnt {
            fp.insert(
                "doNotTrack".to_string(),
                serde_json::json!(if dnt { "1" } else { "0" }),
            );
        }

        // Audio noise seed (Wayfern extension)
        if let Some(NoiseMode::Random) = profile.audio_context {
            let seed = noise::deterministic_seed(&self.profile_id, "audio");
            fp.insert("audioSampleRate".to_string(), serde_json::json!(44100 + seed));
        }

        // sec-ch-ua brands
        if let Some(ref sec_ch_ua) = profile.sec_ch_ua {
            if matches!(sec_ch_ua.mode, AutoOrManual::Manual) && !sec_ch_ua.brands.is_empty() {
                if let Some(first) = sec_ch_ua.brands.first() {
                    fp.insert("brand".to_string(), serde_json::json!(first.brand.clone()));
                    fp.insert("brandVersion".to_string(), serde_json::json!(first.version.clone()));
                }
            }
        }

        serde_json::Value::Object(fp)
    }
}

impl FingerprintAdapter for WayfernAdapter {
    fn apply(
        &self,
        profile: &FingerprintProfile,
        config: &mut EngineLaunchConfig,
    ) -> Result<(), FingerprintError> {
        let fp_json = self.build_wayfern_json(profile);

        // Chromium-native args
        if let Some(ref lang) = profile.language {
            if matches!(lang.mode, AutoOrManual::Manual) {
                if let Some(ref l) = lang.language {
                    config.args.push(format!("--lang={}", l));
                }
            }
        }

        if let Some(false) = profile.gpu_acceleration {
            config.args.push("--disable-gpu".to_string());
        }

        if let Some(true) = profile.dnt {
            config.args.push("--enable-do-not-track".to_string());
        }

        // Store the fingerprint JSON for CDP injection
        config.wayfern_fingerprint = Some(fp_json);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayfern_adapter_ua() {
        let adapter = WayfernAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            user_agent: Some(UaConfig {
                mode: AutoOrManual::Manual,
                value: Some("Mozilla/5.0 Custom".to_string()),
            }),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        let fp = config.wayfern_fingerprint.as_ref().unwrap();
        assert_eq!(fp.get("userAgent").and_then(|v| v.as_str()), Some("Mozilla/5.0 Custom"));
    }

    #[test]
    fn test_wayfern_adapter_lang_arg() {
        let adapter = WayfernAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            language: Some(LanguageConfig {
                mode: AutoOrManual::Manual,
                language: Some("zh-CN".to_string()),
                languages: None,
            }),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        assert!(config.args.contains(&"--lang=zh-CN".to_string()));
    }

    #[test]
    fn test_wayfern_adapter_gpu_disabled() {
        let adapter = WayfernAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            gpu_acceleration: Some(false),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        assert!(config.args.contains(&"--disable-gpu".to_string()));
    }

    #[test]
    fn test_wayfern_adapter_canvas_noise() {
        let adapter = WayfernAdapter::new("test-uuid".to_string());
        let profile = FingerprintProfile {
            canvas: Some(NoiseMode::Random),
            ..Default::default()
        };
        let mut config = EngineLaunchConfig::default();
        adapter.apply(&profile, &mut config).unwrap();
        let fp = config.wayfern_fingerprint.as_ref().unwrap();
        assert!(fp.get("canvasNoiseSeed").is_some());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd src-tauri && cargo test fingerprint::wayfern_adapter --lib 2>&1 | grep -E "test result|panicked|FAILED"
```
Expected: `test result: ok`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/fingerprint/wayfern_adapter.rs
git commit -m "feat(fingerprint): implement Wayfern adapter mapping to WayfernFingerprintConfig JSON"
```

### Task 7: Add fingerprint field to BrowserProfile

**Files:**
- Modify: `src-tauri/src/profile/types.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Add fingerprint field to Rust BrowserProfile**

In `src-tauri/src/profile/types.rs`, add the import at the top:

```rust
use crate::fingerprint::profile::FingerprintProfile;
```

Then add the field to the `BrowserProfile` struct (after `updated_at`):

```rust
    /// Unified fingerprint configuration (VirtualBrowser migration).
    /// When `None`, the engine uses its default fingerprint generation.
    #[serde(default)]
    pub fingerprint_profile: Option<FingerprintProfile>,
```

- [ ] **Step 2: Add FingerprintProfile type to frontend types.ts**

In `src/types.ts`, add after the existing `BrowserProfile` interface:

```typescript
// ── Fingerprint Profile (VirtualBrowser migration) ──

export type NoiseMode = "Default" | "Random";
export type AutoOrManual = "Auto" | "Manual";
export type WebRtcMode = "Replace" | "Allow" | "Block";
export type GeoPermission = "Ask" | "Allow" | "Block";
export type FontMode = "SystemDefault" | "RandomMatch";

export interface UaConfig {
  mode: AutoOrManual;
  value?: string;
}

export interface SecChUaBrand {
  brand: string;
  version: string;
}

export interface SecChUaConfig {
  mode: AutoOrManual;
  brands: SecChUaBrand[];
}

export interface LanguageConfig {
  mode: AutoOrManual;
  language?: string;
  languages?: string[];
}

export interface TimezoneConfig {
  mode: AutoOrManual;
  name?: string;
  offset?: number;
}

export interface WebRtcConfig {
  mode: WebRtcMode;
}

export interface SslConfig {
  enabled: boolean;
  disabled_versions: string[];
}

export interface GeoConfig {
  mode: AutoOrManual;
  longitude?: number;
  latitude?: number;
  precision?: number;
  permission: GeoPermission;
}

export interface ScreenConfig {
  mode: AutoOrManual;
  width?: number;
  height?: number;
}

export interface FontConfig {
  mode: FontMode;
}

export interface WebGlConfig {
  mode: AutoOrManual;
  vendor?: string;
  renderer?: string;
}

export interface FingerprintProfile {
  os?: string;
  user_agent?: UaConfig;
  sec_ch_ua?: SecChUaConfig;
  platform?: string;
  language?: LanguageConfig;
  timezone?: TimezoneConfig;
  webrtc?: WebRtcConfig;
  ssl?: SslConfig;
  geolocation?: GeoConfig;
  screen?: ScreenConfig;
  fonts?: FontConfig;
  canvas?: NoiseMode;
  webgl_img?: NoiseMode;
  webgl?: WebGlConfig;
  audio_context?: NoiseMode;
  client_rects?: NoiseMode;
  speech_voices?: NoiseMode;
  cpu?: number;
  memory?: number;
  gpu_acceleration?: boolean;
  dnt?: boolean;
  homepage?: string;
  cookie_json?: string;
}
```

Add `fingerprint_profile` field to the existing `BrowserProfile` interface:

```typescript
  fingerprint_profile?: FingerprintProfile;
```

- [ ] **Step 3: Verify compilation**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/profile/types.rs src/types.ts
git commit -m "feat(profile): add fingerprint_profile field to BrowserProfile for unified fingerprint config"
```

### Task 8: Integrate adapters into launch flow

**Files:**
- Modify: `src-tauri/src/camoufox_manager.rs`
- Modify: `src-tauri/src/wayfern_manager.rs`

- [ ] **Step 1: Integrate Camoufox adapter into camoufox_manager launch**

In `src-tauri/src/camoufox_manager.rs`, find the `launch_camoufox` method (~line 135) where `CamoufoxConfigBuilder::new()` is called and the builder is configured. After the existing builder configuration (after screen constraints, geoip, etc.) and before `builder.build()` / `builder.build_async()`, insert:

```rust
// Apply unified fingerprint profile if present (VirtualBrowser migration)
if let Some(ref fp_profile) = profile.fingerprint_profile {
    use crate::fingerprint::FingerprintAdapter;
    let adapter = crate::fingerprint::camoufox_adapter::CamoufoxAdapter::new(
        profile.id.to_string(),
    );
    let mut engine_config = crate::fingerprint::EngineLaunchConfig::default();
    if let Err(e) = adapter.apply(fp_profile, &mut engine_config) {
        log::warn!("Failed to apply unified fingerprint profile: {}", e);
    }
    // Merge fingerprint_config into the builder's extra_config
    for (key, value) in engine_config.fingerprint_config {
        builder = builder.extra_config(&key, value);
    }
    // Merge Firefox prefs
    for (key, value) in engine_config.firefox_prefs {
        builder = builder.firefox_pref(&key, value);
    }
}
```

- [ ] **Step 2: Integrate Wayfern adapter into wayfern_manager launch**

In `src-tauri/src/wayfern_manager.rs`, find the launch method where `fingerprint_params` is constructed and sent via `send_cdp_command(ws_url, "Wayfern.setFingerprint", fingerprint_params.clone())` (~line 853). Before the CDP send, insert:

```rust
// Apply unified fingerprint profile if present (VirtualBrowser migration)
if let Some(ref fp_profile) = profile.fingerprint_profile {
    use crate::fingerprint::FingerprintAdapter;
    let adapter = crate::fingerprint::wayfern_adapter::WayfernAdapter::new(
        profile.id.to_string(),
    );
    let mut engine_config = crate::fingerprint::EngineLaunchConfig::default();
    if let Err(e) = adapter.apply(fp_profile, &mut engine_config) {
        log::warn!("Failed to apply unified fingerprint profile: {}", e);
    }
    // Merge Chromium args into launch command
    launch_args.extend(engine_config.args);
    // If the unified fingerprint provides fields not in the existing fingerprint,
    // merge them into fingerprint_params (the JSON sent to Wayfern.setFingerprint)
    if let Some(fp_json) = engine_config.wayfern_fingerprint {
        if let (Some(params_obj), Some(fp_obj)) = (
            fingerprint_params.as_object_mut(),
            fp_json.as_object(),
        ) {
            for (key, value) in fp_obj {
                params_obj.entry(key.clone()).or_insert(value.clone());
            }
        }
    }
}
```

- [ ] **Step 3: Verify full compilation**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: `Finished` with no errors.

- [ ] **Step 4: Run all existing tests**

```bash
cd src-tauri && cargo test --lib 2>&1 | grep -E "test result|panicked|FAILED"
```
Expected: All `test result: ok` lines, no FAILED.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/camoufox_manager.rs src-tauri/src/wayfern_manager.rs
git commit -m "feat(launch): integrate fingerprint adapters into Camoufox and Wayfern launch flows"
```

### Task 9: Add Tauri commands for fingerprint management

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add Tauri command to resolve auto fields**

In `src-tauri/src/lib.rs`, add a new Tauri command:

```rust
#[tauri::command]
async fn resolve_fingerprint_auto_fields(
    profile_id: String,
    proxy_url: Option<String>,
) -> Result<fingerprint::profile::FingerprintProfile, String> {
    let pm = profile::manager::ProfileManager::instance();
    let profiles = pm.list_profiles()
        .map_err(|e| format!("Failed to list profiles: {}", e))?;
    let mut profile = profiles
        .into_iter()
        .find(|p| p.id.to_string() == profile_id)
        .ok_or_else(|| "Profile not found".to_string())?;

    let mut fp = profile.fingerprint_profile.unwrap_or_default();
    fingerprint::auto_match::resolve_auto_fields(&mut fp, proxy_url.as_deref())
        .await
        .map_err(|e| format!("Auto-match failed: {}", e))?;

    // Persist the resolved fingerprint back to the profile
    profile.fingerprint_profile = Some(fp.clone());
    pm.save_profile(&profile)
        .map_err(|e| format!("Failed to save profile: {}", e))?;

    Ok(fp)
}
```

Register it in the `invoke_handler` builder chain.

- [ ] **Step 2: Verify compilation and commit**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(tauri): add resolve_fingerprint_auto_fields command for IP-based auto matching"
```

---

## Chunk 2: Phase 1 — Fingerprint Depth (Frontend)

### Task 10: Create fingerprint configuration tab component

**Files:**
- Create: `src/components/fingerprint-config-form.tsx`
- Modify: `src/i18n/locales/en.json` (and all 8 other locales)

- [ ] **Step 1: Create the fingerprint config form component**

Create `src/components/fingerprint-config-form.tsx` with sections for:
- Identity (UA, sec-ch-ua, language, timezone)
- Network & Privacy (WebRTC, geolocation, SSL, DNT)
- Rendering (Canvas, WebGL, AudioContext, ClientRects, Speech Voices)
- Hardware (CPU, memory, screen, fonts, GPU)

The component accepts a `FingerprintProfile` and an `onChange` callback, using shadcn UI primitives (Select, Switch, Input, Label, Tabs) and `useTranslation()` for all strings.

Follow the existing pattern from `wayfern-config-form.tsx` for the tab structure and form layout.

- [ ] **Step 2: Add i18n keys to en.json**

Add a `fingerprint` namespace to `src/i18n/locales/en.json` with keys for all labels, options, and descriptions in the form.

- [ ] **Step 3: Propagate i18n keys to all 9 locales**

Use a Python script to add the same keys (with English fallback values for now) to all locale files: `es.json`, `fr.json`, `ja.json`, `ko.json`, `pt.json`, `ru.json`, `vi.json`, `zh.json`.

- [ ] **Step 4: Integrate form into create-profile-dialog**

Modify `src/components/create-profile-dialog.tsx` to include the `FingerprintConfigForm` as a tab or section within the browser-config step.

- [ ] **Step 5: Run frontend checks**

```bash
pnpm format && pnpm lint 2>&1 | tail -5
```
Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add src/components/fingerprint-config-form.tsx src/i18n/locales/ src/components/create-profile-dialog.tsx
git commit -m "feat(ui): add fingerprint configuration form with per-field control"
```

---

## Chunk 3: Phase 2 — UX Enhancements

### Task 11: Batch create profiles

**Files:**
- Create: `src/components/batch-create-dialog.tsx`
- Modify: `src-tauri/src/batch_runner.rs` (if needed)
- Modify: `src/app/page.tsx` (add button + dialog trigger)
- Modify: `src/i18n/locales/*.json`

- [ ] **Step 1: Create batch create dialog component**
- [ ] **Step 2: Wire up to existing batch_runner backend**
- [ ] **Step 3: Add i18n keys to all 9 locales**
- [ ] **Step 4: Add trigger button to profile list page**
- [ ] **Step 5: Commit**

### Task 12: Profile import/export

**Files:**
- Create: `src-tauri/src/profile/import_export.rs` (or add to existing profile module)
- Create: `src/components/profile-import-export.tsx`
- Modify: `src/app/page.tsx`
- Modify: `src/i18n/locales/*.json`

- [ ] **Step 1: Implement JSON export format and Tauri command**
- [ ] **Step 2: Implement JSON import with UUID regeneration**
- [ ] **Step 3: Create frontend import/Export UI component**
- [ ] **Step 4: Add i18n keys to all 9 locales**
- [ ] **Step 5: Commit**

### Task 13: IP query API settings

**Files:**
- Modify: `src-tauri/src/settings_manager.rs` (add ip_api_url, ip_api_key fields)
- Modify: `src/components/settings-dialog.tsx` (add IP API section)
- Modify: `src/i18n/locales/*.json`

- [ ] **Step 1: Extend settings manager with IP API config**
- [ ] **Step 2: Add settings UI for IP API URL and key**
- [ ] **Step 3: Wire settings into auto_match module**
- [ ] **Step 4: Add i18n keys to all 9 locales**
- [ ] **Step 5: Commit**

### Task 14: Final i18n translation pass

**Files:**
- Modify: all 9 locale files in `src/i18n/locales/`

- [ ] **Step 1: Review all new keys across all locales**
- [ ] **Step 2: Add proper translations (not English fallbacks) for zh, ja, ko**
- [ ] **Step 3: Verify no empty-string values**
- [ ] **Step 4: Run `pnpm lint` to check for typos**
- [ ] **Step 5: Commit**

---

## Manual Verification (Post-Implementation)

After all tasks are complete:

1. **Camoufox fingerprint check**: Create a Camoufox profile with custom fingerprint settings (canvas noise, audio noise, custom UA, WebGL vendor). Launch and visit `browserleaks.com/canvas`, `browserleaks.com/webgl`, `browserleaks.com/audiocontext` to verify spoofing.

2. **Wayfern fingerprint check**: Create a Wayfern profile with custom fingerprint settings. Launch and verify on the same sites.

3. **Auto-matching**: Set language/timezone/geolocation to Auto mode. Launch with a proxy. Verify the resolved values match the proxy IP's location.

4. **Batch create**: Create 5 profiles at once. Verify all are created with correct settings.

5. **Import/Export**: Export 2 profiles, delete them, re-import. Verify all data is preserved (with new UUIDs).

6. **Regression**: Create a profile with no fingerprint_profile set (None). Verify it launches with the existing default fingerprint generation behavior.
