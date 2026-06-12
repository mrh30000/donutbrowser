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
