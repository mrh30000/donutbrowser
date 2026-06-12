use crate::fingerprint::profile::{AutoOrManual, SecChUaBrand, SecChUaConfig};

/// Generate default sec-ch-ua brands for a given user agent string.
/// Extracts the Chrome major version from the UA and creates standard brand entries.
pub fn default_brands_from_ua(user_agent: &str) -> Vec<SecChUaBrand> {
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
        AutoOrManual::Auto => return prefs,
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
