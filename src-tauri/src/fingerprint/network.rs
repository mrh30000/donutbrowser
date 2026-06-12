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
