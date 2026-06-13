use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::browser::GithubRelease;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionComponent {
  pub major: u32,
  pub minor: u32,
  pub patch: u32,
  pub build: u32,
  pub pre_release: Option<PreRelease>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreRelease {
  pub kind: PreReleaseKind,
  pub number: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PreReleaseKind {
  Alpha,
  Beta,
  RC,
  Dev,
  Pre,
}

impl VersionComponent {
  pub fn parse(version: &str) -> Self {
    let version = version.trim();
    // Normalize common tag prefixes like 'v1.2.3' -> '1.2.3'
    let version = if version.starts_with('v') || version.starts_with('V') {
      &version[1..]
    } else {
      version
    };

    // Handle special case for Zen Browser twilight releases
    if version.to_lowercase() == "twilight" {
      // Pure twilight release without base version
      return VersionComponent {
        major: 999, // High major version to indicate it's a rolling release
        minor: 0,
        patch: 0,
        build: 0,
        pre_release: Some(PreRelease {
          kind: PreReleaseKind::Alpha,
          number: Some(999), // High number to indicate it's a rolling release
        }),
      };
    }

    // Split version into numeric and pre-release parts
    let (numeric_part, pre_release_part) = Self::split_version(version);

    // Parse numeric parts (major.minor.patch)
    let parts: Vec<u32> = numeric_part
      .split('.')
      .filter_map(|part| part.parse().ok())
      .collect();

    let major = parts.first().copied().unwrap_or(0);
    let minor = parts.get(1).copied().unwrap_or(0);
    let patch = parts.get(2).copied().unwrap_or(0);
    let build = parts.get(3).copied().unwrap_or(0);

    // Parse pre-release part
    let pre_release = pre_release_part
      .as_deref()
      .and_then(Self::parse_pre_release);

    VersionComponent {
      major,
      minor,
      patch,
      build,
      pre_release,
    }
  }

  fn split_version(version: &str) -> (String, Option<String>) {
    let version = version.to_lowercase();

    // Look for pre-release indicators
    for (i, ch) in version.char_indices() {
      if ch.is_alphabetic() && i > 0 {
        // Check if this is a pre-release indicator
        let remaining = &version[i..];
        if remaining.starts_with('a')
          || remaining.starts_with('b')
          || remaining.starts_with("alpha")
          || remaining.starts_with("beta")
          || remaining.starts_with("rc")
          || remaining.starts_with("dev")
          || remaining.starts_with("pre")
        {
          return (version[..i].to_string(), Some(remaining.to_string()));
        }
      }
    }

    (version, None)
  }

  fn parse_pre_release(pre_release: &str) -> Option<PreRelease> {
    let pre_release = pre_release.trim().to_lowercase();

    if pre_release.is_empty() {
      return None;
    }

    // Extract kind and number
    let (kind, number) = if let Some(stripped) = pre_release.strip_prefix("alpha") {
      (PreReleaseKind::Alpha, Self::extract_number(stripped))
    } else if let Some(stripped) = pre_release.strip_prefix("beta") {
      (PreReleaseKind::Beta, Self::extract_number(stripped))
    } else if let Some(stripped) = pre_release.strip_prefix("rc") {
      (PreReleaseKind::RC, Self::extract_number(stripped))
    } else if let Some(stripped) = pre_release.strip_prefix("dev") {
      (PreReleaseKind::Dev, Self::extract_number(stripped))
    } else if let Some(stripped) = pre_release.strip_prefix("pre") {
      (PreReleaseKind::Pre, Self::extract_number(stripped))
    } else if let Some(stripped) = pre_release.strip_prefix('a') {
      (PreReleaseKind::Alpha, Self::extract_number(stripped))
    } else if let Some(stripped) = pre_release.strip_prefix('b') {
      (PreReleaseKind::Beta, Self::extract_number(stripped))
    } else {
      return None;
    };

    Some(PreRelease { kind, number })
  }

  fn extract_number(s: &str) -> Option<u32> {
    let numeric_part: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    numeric_part.parse().ok()
  }
}

impl PartialOrd for VersionComponent {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for VersionComponent {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    // Check for twilight versions
    let self_is_twilight = self
      .pre_release
      .as_ref()
      .map(|pr| pr.kind == PreReleaseKind::Alpha && pr.number == Some(999))
      .unwrap_or(false);
    let other_is_twilight = other
      .pre_release
      .as_ref()
      .map(|pr| pr.kind == PreReleaseKind::Alpha && pr.number == Some(999))
      .unwrap_or(false);

    // If one is twilight and the other isn't, twilight always has priority
    if self_is_twilight && !other_is_twilight {
      return Ordering::Greater; // twilight > non-twilight
    }
    if !self_is_twilight && other_is_twilight {
      return Ordering::Less; // non-twilight < twilight
    }

    // Both are twilight or both are not twilight - use normal comparison
    match (self_is_twilight, other_is_twilight) {
      (true, true) => {
        // Both are twilight, compare by base version
        return (self.major, self.minor, self.patch, self.build).cmp(&(
          other.major,
          other.minor,
          other.patch,
          other.build,
        ));
      }
      (false, false) => {
        // Neither is twilight, continue with normal comparison
      }
      _ => unreachable!(), // Already handled above
    }

    // Compare major.minor.patch.build first
    match (self.major, self.minor, self.patch, self.build).cmp(&(
      other.major,
      other.minor,
      other.patch,
      other.build,
    )) {
      Ordering::Equal => {
        // If numeric parts are equal, compare pre-release
        match (&self.pre_release, &other.pre_release) {
          (None, None) => Ordering::Equal,
          (None, Some(_)) => Ordering::Greater, // Stable > pre-release
          (Some(_), None) => Ordering::Less,    // Pre-release < stable
          (Some(a), Some(b)) => {
            // Compare pre-release kinds first
            match a.kind.cmp(&b.kind) {
              Ordering::Equal => {
                // Same kind, compare numbers
                match (&a.number, &b.number) {
                  (None, None) => Ordering::Equal,
                  (None, Some(_)) => Ordering::Less,
                  (Some(_), None) => Ordering::Greater,
                  (Some(a_num), Some(b_num)) => a_num.cmp(b_num),
                }
              }
              other => other,
            }
          }
        }
      }
      other => other,
    }
  }
}

// Helper function to sort versions properly
pub fn sort_versions(versions: &mut [String]) {
  versions.sort_by(|a, b| {
    let version_a = VersionComponent::parse(a);
    let version_b = VersionComponent::parse(b);
    version_b.cmp(&version_a) // Descending order (newest first)
  });
}

// Helper function to compare two versions
pub fn compare_versions(version1: &str, version2: &str) -> std::cmp::Ordering {
  let version_a = VersionComponent::parse(version1);
  let version_b = VersionComponent::parse(version2);
  version_a.cmp(&version_b)
}

pub fn is_version_newer(version1: &str, version2: &str) -> bool {
  // Use the proper VersionComponent comparison from api_client.rs
  let version_a = VersionComponent::parse(version1);
  let version_b = VersionComponent::parse(version2);
  version_a > version_b
}

// Helper function to sort GitHub releases
pub fn sort_github_releases(releases: &mut [GithubRelease]) {
  releases.sort_by(|a, b| {
    // Normalize tags like "v1.81.9" -> "1.81.9" for correct ordering
    let tag_a = a.tag_name.trim_start_matches('v');
    let tag_b = b.tag_name.trim_start_matches('v');
    let version_a = VersionComponent::parse(tag_a);
    let version_b = VersionComponent::parse(tag_b);
    version_b.cmp(&version_a) // Descending order (newest first)
  });
}

pub fn is_nightly_version(version: &str) -> bool {
  let version_comp = VersionComponent::parse(version);
  version_comp.pre_release.is_some()
}


// Global singleton instance
lazy_static::lazy_static! {
  static ref API_CLIENT: ApiClient = ApiClient::new();
}

#[cfg(test)]
mod tests {
  use super::*;
  use wiremock::matchers::{method, path, query_param};
  use wiremock::{Mock, MockServer, ResponseTemplate};

  async fn setup_mock_server() -> MockServer {
    MockServer::start().await
  }

  fn create_test_client(server: &MockServer) -> ApiClient {
    let base_url = server.uri();
    ApiClient::new_with_base_urls(
      base_url.clone(), // firefox_api_base
      base_url.clone(), // firefox_dev_api_base
      base_url.clone(), // github_api_base
      base_url.clone(), // chromium_api_base
    )
  }

  #[test]
  fn test_version_parsing() {
    // Test basic version parsing
    let v1 = VersionComponent::parse("1.2.3");
    assert_eq!(v1.major, 1);
    assert_eq!(v1.minor, 2);
    assert_eq!(v1.patch, 3);
    assert!(v1.pre_release.is_none());

    // Test alpha version
    let v2 = VersionComponent::parse("1.2.3a1");
    assert_eq!(v2.major, 1);
    assert_eq!(v2.minor, 2);
    assert_eq!(v2.patch, 3);
    assert!(v2.pre_release.is_some());
    let pre = v2.pre_release.unwrap();
    assert_eq!(pre.kind, PreReleaseKind::Alpha);
    assert_eq!(pre.number, Some(1));

    // Test beta version
    let v3 = VersionComponent::parse("137.0b5");
    assert_eq!(v3.major, 137);
    assert_eq!(v3.minor, 0);
    assert_eq!(v3.patch, 0);
    assert!(v3.pre_release.is_some());
    let pre = v3.pre_release.unwrap();
    assert_eq!(pre.kind, PreReleaseKind::Beta);
    assert_eq!(pre.number, Some(5));

    // Test twilight version (Zen Browser)
    let v4 = VersionComponent::parse("twilight");
    assert_eq!(v4.major, 999);
    assert_eq!(v4.minor, 0);
    assert_eq!(v4.patch, 0);
    assert!(v4.pre_release.is_some());
    let pre = v4.pre_release.unwrap();
    assert_eq!(pre.kind, PreReleaseKind::Alpha);
    assert_eq!(pre.number, Some(999));
  }

  #[test]
  fn test_version_comparison() {
    // Test basic version comparison
    let v1 = VersionComponent::parse("1.2.3");
    let v2 = VersionComponent::parse("1.2.4");
    assert!(v2 > v1);

    // Test major version difference
    let v3 = VersionComponent::parse("2.0.0");
    let v4 = VersionComponent::parse("1.9.9");
    assert!(v3 > v4);

    // Test stable vs pre-release
    let v5 = VersionComponent::parse("1.2.3");
    let v6 = VersionComponent::parse("1.2.3b1");
    assert!(v5 > v6); // Stable > beta

    // Test different pre-release types
    let v7 = VersionComponent::parse("1.2.3a1");
    let v8 = VersionComponent::parse("1.2.3b1");
    assert!(v8 > v7); // Beta > alpha

    // Test pre-release numbers
    let v9 = VersionComponent::parse("137.0b4");
    let v10 = VersionComponent::parse("137.0b5");
    assert!(v10 > v9); // b5 > b4

    // Test twilight version (should have highest priority)
    let v11 = VersionComponent::parse("twilight");
    let v12 = VersionComponent::parse("1.0.0");
    assert!(v11 > v12); // twilight > stable due to high major version

    // Test twilight vs other pre-releases
    let v13 = VersionComponent::parse("twilight");
    let v14 = VersionComponent::parse("1.0.0a1");
    assert!(v13 > v14); // twilight > a1 due to high major version
  }

  #[test]
  fn test_version_sorting() {
    let mut versions = vec![
      "1.9.9b".to_string(),
      "1.12.6b".to_string(),
      "1.10.0".to_string(),
      "137.0b4".to_string(),
      "137.0b5".to_string(),
      "137.0".to_string(),
      "twilight".to_string(),
      "2.0.0a1".to_string(),
    ];

    sort_versions(&mut versions);

    // Expected order with twilight priority: twilight first due to high major version (999), then normal semantic versioning
    assert_eq!(versions[0], "twilight");
    assert_eq!(versions[1], "137.0");
    assert_eq!(versions[2], "137.0b5");
    assert_eq!(versions[3], "137.0b4");
    assert_eq!(versions[4], "2.0.0a1");
    assert_eq!(versions[5], "1.12.6b");
    assert_eq!(versions[6], "1.10.0");
    assert_eq!(versions[7], "1.9.9b");
  }

  #[test]
  fn test_sort_versions_comprehensive() {
    let mut versions = vec![
      "1.0.0".to_string(),
      "1.0.1".to_string(),
      "1.1.0".to_string(),
      "2.0.0a1".to_string(),
      "2.0.0b1".to_string(),
      "2.0.0rc1".to_string(),
      "2.0.0".to_string(),
      "10.0.0".to_string(),
      "twilight".to_string(),
    ];

    sort_versions(&mut versions);

    // Expected order with twilight priority: twilight first due to high major version (999), then normal semantic versioning
    assert_eq!(versions[0], "twilight");
    assert_eq!(versions[1], "10.0.0");
    assert_eq!(versions[2], "2.0.0");
    assert_eq!(versions[3], "2.0.0rc1");
    assert_eq!(versions[4], "2.0.0b1");
    assert_eq!(versions[5], "2.0.0a1");
  }

  #[tokio::test]
  async fn test_firefox_api() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    let mock_response = r#"{
      "releases": {
        "firefox-139.0": {
          "build_number": 1,
          "category": "major",
          "date": "2024-01-15",
          "description": "Firefox 139.0 Release",
          "is_security_driven": false,
          "product": "firefox",
          "version": "139.0"
        },
        "firefox-138.0": {
          "build_number": 1,
          "category": "major",
          "date": "2024-01-01",
          "description": "Firefox 138.0 Release",
          "is_security_driven": false,
          "product": "firefox",
          "version": "138.0"
        }
      }
    }"#;

    Mock::given(method("GET"))
      .and(path("/firefox.json"))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_string(mock_response)
          .insert_header("content-type", "application/json"),
      )
      .mount(&server)
      .await;

    let result = client.fetch_firefox_releases_with_caching(true).await;

    if let Err(e) = &result {
      log::info!("Firefox API test error: {e}");
    }
    assert!(result.is_ok());
    let releases = result.unwrap();
    assert!(!releases.is_empty());
    assert_eq!(releases[0].version, "139.0");
  }

  #[tokio::test]
  async fn test_firefox_developer_api() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    let mock_response = r#"{
      "releases": {
        "devedition-140.0b1": {
          "build_number": 1,
          "category": "major",
          "date": "2024-01-20",
          "description": "Firefox Developer Edition 140.0b1",
          "is_security_driven": false,
          "product": "devedition",
          "version": "140.0b1"
        }
      }
    }"#;

    Mock::given(method("GET"))
      .and(path("/devedition.json"))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_string(mock_response)
          .insert_header("content-type", "application/json"),
      )
      .mount(&server)
      .await;

    let result = client
      .fetch_firefox_developer_releases_with_caching(true)
      .await;

    if let Err(e) = &result {
      log::info!("Firefox Developer API test error: {e}");
    }
    assert!(result.is_ok());
    let releases = result.unwrap();
    assert!(!releases.is_empty());
    assert_eq!(releases[0].version, "140.0b1");
  }

  #[tokio::test]
  async fn test_zen_api() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    let mock_response = r#"[
      {
        "tag_name": "twilight",
        "name": "Zen Browser Twilight",
        "prerelease": false,
        "published_at": "2024-01-15T10:00:00Z",
        "assets": [
          {
            "name": "zen.macos-universal.dmg",
            "browser_download_url": "https://example.com/zen-twilight.dmg",
            "size": 120000000
          }
        ]
      }
    ]"#;

    Mock::given(method("GET"))
      .and(path("/repos/zen-browser/desktop/releases"))
      .and(query_param("per_page", "100"))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_string(mock_response)
          .insert_header("content-type", "application/json"),
      )
      .mount(&server)
      .await;

    let result = client.fetch_zen_releases_with_caching(true).await;

    assert!(result.is_ok());
    let releases = result.unwrap();
    assert!(!releases.is_empty());
    assert_eq!(releases[0].tag_name, "twilight");
  }

  #[tokio::test]
  async fn test_brave_api() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    let mock_response = r#"[
      {
        "tag_name": "v1.81.9",
        "name": "Release v1.81.9 (Chromium 137.0.7151.104)",
        "prerelease": false,
        "published_at": "2024-01-15T10:00:00Z",
        "draft": false,
        "assets": [
          {
            "name": "brave-v1.81.9-universal.dmg",
            "browser_download_url": "https://example.com/brave-1.81.9-universal.dmg",
            "size": 200000000
          },
          {
            "name": "brave-browser-1.81.9-linux-amd64.zip",
            "browser_download_url": "https://example.com/brave-1.81.9-linux-amd64.zip",
            "size": 180000000
          },
          {
            "name": "BraveBrowserStandaloneSetup.exe",
            "browser_download_url": "https://example.com/brave-1.81.9-setup.exe",
            "size": 150000000
          }
        ]
      }
    ]"#;

    Mock::given(method("GET"))
      .and(path("/repos/brave/brave-browser/releases"))
      .and(query_param("per_page", "100"))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_string(mock_response)
          .insert_header("content-type", "application/json"),
      )
      .mount(&server)
      .await;

    let result = client.fetch_brave_releases_with_caching(true).await;

    if let Err(e) = &result {
      log::info!("Brave API test error: {e}");
    }
    assert!(result.is_ok());
    let releases = result.unwrap();
    assert!(!releases.is_empty());
    assert_eq!(releases[0].tag_name, "v1.81.9");
    assert!(!releases[0].is_nightly); // "Release v1.81.9 (Chromium 137.0.7151.104)" starts with "Release" so it should be stable
  }

  #[tokio::test]
  async fn test_chromium_api() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    let (os, arch) = ApiClient::get_platform_info();
    let platform_str = match (&os[..], &arch[..]) {
      ("windows", "x64") => "Win_x64",
      ("windows", "arm64") => "Win_Arm64",
      ("linux", "x64") => "Linux_x64",
      ("linux", "arm64") => return,
      ("macos", "x64") => "Mac",
      ("macos", "arm64") => "Mac_Arm",
      _ => return,
    };

    Mock::given(method("GET"))
      .and(path(format!("/{platform_str}/LAST_CHANGE")))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_string("1465660")
          .insert_header("content-type", "text/plain"),
      )
      .mount(&server)
      .await;

    let result = client.fetch_chromium_latest_version().await;

    assert!(result.is_ok());
    let version = result.unwrap();
    assert_eq!(version, "1465660");
  }

  #[tokio::test]
  async fn test_chromium_releases_with_caching() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    let (os, arch) = ApiClient::get_platform_info();
    let platform_str = match (&os[..], &arch[..]) {
      ("windows", "x64") => "Win_x64",
      ("windows", "arm64") => "Win_Arm64",
      ("linux", "x64") => "Linux_x64",
      ("linux", "arm64") => return,
      ("macos", "x64") => "Mac",
      ("macos", "arm64") => "Mac_Arm",
      _ => return,
    };

    Mock::given(method("GET"))
      .and(path(format!("/{platform_str}/LAST_CHANGE")))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_string("1465660")
          .insert_header("content-type", "text/plain"),
      )
      .mount(&server)
      .await;

    let result = client.fetch_chromium_releases_with_caching(true).await;

    assert!(result.is_ok());
    let releases = result.unwrap();
    assert!(!releases.is_empty());
    assert_eq!(releases[0].version, "1465660");
    assert!(!releases[0].is_prerelease);
  }

  #[test]
  fn test_is_nightly_version() {
    assert!(is_nightly_version("1.2.3a1"));
    assert!(is_nightly_version("137.0b5"));
    assert!(is_nightly_version("140.0rc1"));
    assert!(!is_nightly_version("139.0"));
    assert!(!is_nightly_version("1.2.3"));
  }

  #[test]
  fn test_is_zen_nightly_version() {
    // Only "twilight" should be considered nightly for Zen Browser
    assert!(is_browser_version_nightly("zen", "twilight", None));
    assert!(is_browser_version_nightly("zen", "TWILIGHT", None)); // Case insensitive

    // Versions with "b" should NOT be considered nightly for Zen Browser
    assert!(!is_browser_version_nightly("zen", "1.12.8b", None));
    assert!(!is_browser_version_nightly("zen", "1.0.0b1", None));
    assert!(!is_browser_version_nightly("zen", "2.0.0", None));
  }

  #[tokio::test]
  async fn test_error_handling_404() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    Mock::given(method("GET"))
      .and(path("/firefox.json"))
      .respond_with(ResponseTemplate::new(404))
      .mount(&server)
      .await;

    let result = client.fetch_firefox_releases_with_caching(true).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_error_handling_invalid_json() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    Mock::given(method("GET"))
      .and(path("/firefox.json"))
      .respond_with(
        ResponseTemplate::new(200)
          .set_body_string("invalid json")
          .insert_header("content-type", "application/json"),
      )
      .mount(&server)
      .await;

    let result = client.fetch_firefox_releases_with_caching(true).await;
    assert!(result.is_err());
  }

  #[tokio::test]
  async fn test_github_api_rate_limit() {
    let server = setup_mock_server().await;
    let client = create_test_client(&server);

    Mock::given(method("GET"))
      .and(path("/repos/zen-browser/desktop/releases"))
      .and(query_param("per_page", "100"))
      .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "60"))
      .mount(&server)
      .await;

    let result = client.fetch_zen_releases_with_caching(true).await;
    assert!(result.is_err());
  }

  #[test]
  fn test_camoufox_beta_version_parsing() {
    // Test specific Camoufox beta versions that are causing issues
    let v22 = VersionComponent::parse("135.0.5beta22");
    let v24 = VersionComponent::parse("135.0.5beta24");

    log::info!("v22: {v22:?}");
    log::info!("v24: {v24:?}");

    // v24 should be greater than v22
    assert!(
      v24 > v22,
      "135.0.5beta24 should be greater than 135.0.5beta22"
    );

    // Test other beta version combinations
    let v1 = VersionComponent::parse("135.0.5beta1");
    let v2 = VersionComponent::parse("135.0.5beta2");
    assert!(v2 > v1, "135.0.5beta2 should be greater than 135.0.5beta1");

    // Test sorting of multiple versions
    let mut versions = vec![
      "135.0.5beta22".to_string(),
      "135.0.5beta24".to_string(),
      "135.0.5beta23".to_string(),
      "135.0.5beta21".to_string(),
    ];

    sort_versions(&mut versions);

    log::info!("Sorted versions: {versions:?}");

    // Should be sorted from newest to oldest
    assert_eq!(versions[0], "135.0.5beta24");
    assert_eq!(versions[1], "135.0.5beta23");
    assert_eq!(versions[2], "135.0.5beta22");
    assert_eq!(versions[3], "135.0.5beta21");
  }

  #[test]
  fn test_camoufox_user_reported_versions() {
    // Test the exact versions reported by the user: 135.0.1beta24 vs 135.0beta22
    let v22 = VersionComponent::parse("135.0beta22");
    let v24 = VersionComponent::parse("135.0.1beta24");

    log::info!("User reported v22: {v22:?}");
    log::info!("User reported v24: {v24:?}");

    // 135.0.1beta24 should be greater than 135.0beta22 (newer patch version)
    assert!(
      v24 > v22,
      "135.0.1beta24 should be greater than 135.0beta22, but got: v24={v24:?} vs v22={v22:?}"
    );

    // Test sorting of the exact user-reported versions
    let mut versions = vec!["135.0beta22".to_string(), "135.0.1beta24".to_string()];

    sort_versions(&mut versions);

    log::info!("User reported sorted versions: {versions:?}");

    // Should be sorted from newest to oldest
    assert_eq!(
      versions[0], "135.0.1beta24",
      "135.0.1beta24 should be first (newest)"
    );
    assert_eq!(
      versions[1], "135.0beta22",
      "135.0beta22 should be second (older)"
    );
  }

  #[test]
  fn test_camoufox_version_classification() {
    // Test that Camoufox beta versions are now correctly classified as stable (not nightly)
    assert!(
      !is_browser_version_nightly("camoufox", "135.0beta22", None),
      "135.0beta22 should be classified as stable for Camoufox"
    );
    assert!(
      !is_browser_version_nightly("camoufox", "135.0.1beta24", None),
      "135.0.1beta24 should be classified as stable for Camoufox"
    );

    // Test with release names too - beta releases should be stable
    assert!(
      !is_browser_version_nightly("camoufox", "135.0beta22", Some("Release Beta 22")),
      "Release with 'Beta' in name should be classified as stable for Camoufox"
    );

    // Test that stable versions are not classified as nightly
    assert!(
      !is_browser_version_nightly("camoufox", "135.0", None),
      "135.0 should be classified as stable"
    );
    assert!(
      !is_browser_version_nightly("camoufox", "135.0.1", None),
      "135.0.1 should be classified as stable"
    );

    // Test alpha and RC versions are still considered nightly
    assert!(
      !is_browser_version_nightly("camoufox", "136.0alpha1", None),
      "136.0alpha1 should not be classified as nightly/prerelease"
    );
    assert!(
      !is_browser_version_nightly("camoufox", "136.0rc1", None),
      "136.0rc1 should not be classified as nightly/prerelease"
    );
  }
}
