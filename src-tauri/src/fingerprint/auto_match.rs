use crate::camoufox::geolocation::{self, GeolocationError};
use crate::fingerprint::profile::{AutoOrManual, FingerprintProfile};

/// Resolve all Auto-mode fields in a FingerprintProfile using IP geolocation.
///
/// For fields where `mode == Auto`, fetches the public IP (optionally through proxy),
/// looks up geolocation, and fills in language, timezone, and geolocation fields.
/// If `ip_api_url` is configured in app settings, it is tried first.
pub async fn resolve_auto_fields(
    profile: &mut FingerprintProfile,
    proxy_url: Option<&str>,
) -> Result<(), GeolocationError> {
    let settings = crate::settings_manager::SettingsManager::instance()
        .load_settings()
        .ok();

    let ip = if let Some(ref custom_url) = settings.as_ref().and_then(|s| s.ip_api_url.clone()) {
        match fetch_ip_from_custom(custom_url, settings.as_ref().and_then(|s| s.ip_api_key.as_deref()), proxy_url).await {
            Ok(ip) => ip,
            Err(_) => geolocation::fetch_public_ip(proxy_url).await?,
        }
    } else {
        geolocation::fetch_public_ip(proxy_url).await?
    };
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

async fn fetch_ip_from_custom(
    url: &str,
    api_key: Option<&str>,
    proxy_url: Option<&str>,
) -> Result<String, String> {
    let client_builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5));
    let client = if let Some(proxy) = proxy_url {
        let p = reqwest::Proxy::all(proxy).map_err(|e| format!("Invalid proxy: {e}"))?;
        client_builder.no_proxy().proxy(p).build().map_err(|e| e.to_string())?
    } else {
        client_builder.build().map_err(|e| e.to_string())?
    };

    let mut req = client.get(url);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let ip = text.trim().to_string();
    if geolocation::validate_ip(&ip) {
        Ok(ip)
    } else {
        Err(format!("Invalid IP from custom API: {ip}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_fields_skips_manual_mode() {
        let profile = FingerprintProfile {
            language: Some(crate::fingerprint::profile::LanguageConfig {
                mode: AutoOrManual::Manual,
                language: Some("en-US".to_string()),
                languages: Some(vec!["en-US".to_string()]),
            }),
            ..Default::default()
        };
        assert!(matches!(
            profile.language.as_ref().unwrap().mode,
            AutoOrManual::Manual
        ));
    }
}
