use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyDiagnosticError {
  code: &'static str,
}

impl ProxyDiagnosticError {
  pub fn new(code: &'static str) -> Self {
    Self { code }
  }

  #[allow(dead_code)]
  pub fn code(&self) -> &'static str {
    self.code
  }

  pub fn to_backend_error(&self) -> String {
    serde_json::json!({ "code": self.code }).to_string()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyDiagnosticSource {
  IpApi,
  IpSb,
  IpapiCo,
}

impl ProxyDiagnosticSource {
  fn url(self) -> &'static str {
    match self {
      ProxyDiagnosticSource::IpApi => "http://ip-api.com/json/",
      ProxyDiagnosticSource::IpSb => "https://api.ip.sb/geoip/",
      ProxyDiagnosticSource::IpapiCo => "https://ipapi.co/json/",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyDiagnosticOptions {
  pub timeout_ms: u64,
  pub sources: Vec<ProxyDiagnosticSource>,
}

impl Default for ProxyDiagnosticOptions {
  fn default() -> Self {
    Self {
      timeout_ms: 10_000,
      sources: vec![
        ProxyDiagnosticSource::IpApi,
        ProxyDiagnosticSource::IpSb,
        ProxyDiagnosticSource::IpapiCo,
      ],
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyDiagnosticSourceResult {
  pub source: ProxyDiagnosticSource,
  pub ip: String,
  pub country: Option<String>,
  pub country_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProxyDiagnosticResult {
  pub profile_id: String,
  pub profile_name: String,
  pub proxy_id: Option<String>,
  pub proxy_name: Option<String>,
  pub is_valid: bool,
  pub latency_ms: Option<u128>,
  pub source: Option<ProxyDiagnosticSource>,
  pub ip: Option<String>,
  pub country: Option<String>,
  pub country_code: Option<String>,
  pub error: Option<String>,
}

pub fn parse_source_response(
  source: ProxyDiagnosticSource,
  value: serde_json::Value,
) -> Result<ProxyDiagnosticSourceResult, ProxyDiagnosticError> {
  let ip = match source {
    ProxyDiagnosticSource::IpApi => value.get("query"),
    ProxyDiagnosticSource::IpSb => value.get("ip"),
    ProxyDiagnosticSource::IpapiCo => value.get("ip"),
  }
  .and_then(|value| value.as_str())
  .ok_or_else(|| ProxyDiagnosticError::new("PROXY_DIAGNOSTIC_SOURCE_FAILED"))?;

  let country = match source {
    ProxyDiagnosticSource::IpApi => value.get("country"),
    ProxyDiagnosticSource::IpSb => value.get("country"),
    ProxyDiagnosticSource::IpapiCo => value.get("country_name"),
  }
  .and_then(|value| value.as_str())
  .map(ToOwned::to_owned);

  let country_code = match source {
    ProxyDiagnosticSource::IpApi => value.get("countryCode"),
    ProxyDiagnosticSource::IpSb => value.get("country_code"),
    ProxyDiagnosticSource::IpapiCo => value.get("country_code"),
  }
  .and_then(|value| value.as_str())
  .map(ToOwned::to_owned);

  Ok(ProxyDiagnosticSourceResult {
    source,
    ip: ip.to_string(),
    country,
    country_code,
  })
}

pub fn missing_proxy_result(
  profile_id: String,
  profile_name: String,
) -> ProfileProxyDiagnosticResult {
  ProfileProxyDiagnosticResult {
    profile_id,
    profile_name,
    proxy_id: None,
    proxy_name: None,
    is_valid: false,
    latency_ms: None,
    source: None,
    ip: None,
    country: None,
    country_code: None,
    error: Some("PROXY_NOT_FOUND".to_string()),
  }
}

pub async fn diagnose_profile_proxies(
  profile_ids: Vec<String>,
  options: ProxyDiagnosticOptions,
) -> Result<Vec<ProfileProxyDiagnosticResult>, String> {
  crate::batch_runner::validate_profile_ids(&profile_ids)
    .map_err(|error| error.to_backend_error())?;

  let profiles = crate::profile::ProfileManager::instance()
    .list_profiles()
    .map_err(|_| serde_json::json!({ "code": "INTERNAL_ERROR" }).to_string())?;
  let profiles = crate::batch_runner::profiles_by_ids(profiles, &profile_ids)
    .map_err(|error| error.to_backend_error())?;

  let mut results = Vec::with_capacity(profiles.len());
  for profile in profiles {
    results.push(diagnose_one_profile(profile, &options).await);
  }

  Ok(results)
}

async fn diagnose_one_profile(
  profile: crate::profile::BrowserProfile,
  options: &ProxyDiagnosticOptions,
) -> ProfileProxyDiagnosticResult {
  let Some(proxy_id) = profile.proxy_id.clone() else {
    return missing_proxy_result(profile.id.to_string(), profile.name);
  };

  let proxy = crate::proxy_manager::PROXY_MANAGER
    .get_stored_proxies()
    .into_iter()
    .find(|proxy| proxy.id == proxy_id);
  let Some(proxy) = proxy else {
    return missing_proxy_result(profile.id.to_string(), profile.name);
  };

  let proxy_url = crate::proxy_manager::ProxyManager::build_proxy_url(&proxy.proxy_settings);
  let timeout = Duration::from_millis(options.timeout_ms.clamp(1000, 30_000));
  let sources = if options.sources.is_empty() {
    ProxyDiagnosticOptions::default().sources
  } else {
    options.sources.clone()
  };

  let start = Instant::now();
  let proxy_config = match reqwest::Proxy::all(proxy_url) {
    Ok(proxy_config) => proxy_config,
    Err(_) => {
      return failed_result(
        profile,
        Some(proxy.id),
        Some(proxy.name),
        Some(start.elapsed().as_millis()),
        "PROXY_DIAGNOSTIC_SOURCE_FAILED",
      );
    }
  };
  let client = match reqwest::Client::builder()
    .proxy(proxy_config)
    .timeout(timeout)
    .build()
  {
    Ok(client) => client,
    Err(_) => {
      return failed_result(
        profile,
        Some(proxy.id),
        Some(proxy.name),
        Some(start.elapsed().as_millis()),
        "PROXY_DIAGNOSTIC_SOURCE_FAILED",
      );
    }
  };

  let mut timed_out = false;
  for source in sources {
    match client.get(source.url()).send().await {
      Ok(response) => match response.json::<serde_json::Value>().await {
        Ok(value) => {
          if let Ok(parsed) = parse_source_response(source, value) {
            return ProfileProxyDiagnosticResult {
              profile_id: profile.id.to_string(),
              profile_name: profile.name,
              proxy_id: Some(proxy.id),
              proxy_name: Some(proxy.name),
              is_valid: true,
              latency_ms: Some(start.elapsed().as_millis()),
              source: Some(parsed.source),
              ip: Some(parsed.ip),
              country: parsed.country,
              country_code: parsed.country_code,
              error: None,
            };
          }
        }
        Err(error) => {
          timed_out |= error.is_timeout();
        }
      },
      Err(error) => {
        timed_out |= error.is_timeout();
      }
    }
  }

  failed_result(
    profile,
    Some(proxy.id),
    Some(proxy.name),
    Some(start.elapsed().as_millis()),
    if timed_out {
      "PROXY_DIAGNOSTIC_TIMEOUT"
    } else {
      "PROXY_DIAGNOSTIC_SOURCE_FAILED"
    },
  )
}

fn failed_result(
  profile: crate::profile::BrowserProfile,
  proxy_id: Option<String>,
  proxy_name: Option<String>,
  latency_ms: Option<u128>,
  code: &'static str,
) -> ProfileProxyDiagnosticResult {
  ProfileProxyDiagnosticResult {
    profile_id: profile.id.to_string(),
    profile_name: profile.name,
    proxy_id,
    proxy_name,
    is_valid: false,
    latency_ms,
    source: None,
    ip: None,
    country: None,
    country_code: None,
    error: Some(code.to_string()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_ip_api_response() {
    let value = serde_json::json!({
      "query": "1.2.3.4",
      "country": "United States",
      "countryCode": "US"
    });

    let parsed = parse_source_response(ProxyDiagnosticSource::IpApi, value).unwrap();
    assert_eq!(parsed.ip, "1.2.3.4");
    assert_eq!(parsed.country.as_deref(), Some("United States"));
    assert_eq!(parsed.country_code.as_deref(), Some("US"));
  }

  #[test]
  fn parses_ip_sb_response() {
    let value = serde_json::json!({
      "ip": "1.2.3.4",
      "country": "United States",
      "country_code": "US"
    });

    let parsed = parse_source_response(ProxyDiagnosticSource::IpSb, value).unwrap();
    assert_eq!(parsed.ip, "1.2.3.4");
  }

  #[test]
  fn missing_ip_returns_source_failed() {
    let err =
      parse_source_response(ProxyDiagnosticSource::IpApi, serde_json::json!({})).unwrap_err();
    assert_eq!(err.code(), "PROXY_DIAGNOSTIC_SOURCE_FAILED");
  }

  #[test]
  fn missing_proxy_result_is_invalid() {
    let result = missing_proxy_result("profile-1".to_string(), "Profile 1".to_string());
    assert!(!result.is_valid);
    assert_eq!(result.error.as_deref(), Some("PROXY_NOT_FOUND"));
  }
}
