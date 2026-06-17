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
      cfg.insert(
        "navigator.platform".to_string(),
        serde_json::json!(platform),
      );
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
      cfg.insert("navigator.deviceMemory".to_string(), serde_json::json!(mem));
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

    // sec-ch-ua -> Firefox prefs
    if let Some(ref sec_ch_ua) = profile.sec_ch_ua {
      let prefs = identity::sec_ch_ua_to_firefox_prefs(sec_ch_ua);
      config.firefox_prefs.extend(prefs);
    }

    // SSL -> Firefox prefs
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
      config
        .firefox_prefs
        .insert("webgl.disabled".to_string(), serde_json::json!(true));
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
    assert!(!config
      .fingerprint_config
      .contains_key("navigator.userAgent"));
  }

  #[test]
  fn test_camoufox_adapter_webrtc_block() {
    let adapter = CamoufoxAdapter::new("test-uuid".to_string());
    let profile = FingerprintProfile {
      webrtc: Some(WebRtcConfig {
        mode: WebRtcMode::Block,
      }),
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
    assert!(config.fingerprint_config.contains_key("canvas:aaOffset"));
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
