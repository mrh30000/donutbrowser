use crate::profile::manager::ProfileManager;
use crate::profile::BrowserProfile;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Chromium cookie decryption support for reading existing encrypted cookies.
/// Writes always go through the plaintext `value` column (see `write_chrome_cookies`),
/// so no encryption path is needed here — Chromium reads plaintext when
/// `encrypted_value` is empty, regardless of what other cookies store.
pub mod chrome_decrypt {
  use aes::cipher::{block_padding::Pkcs7, BlockModeDecrypt, KeyIvInit};
  use ring::pbkdf2;
  use sha2::{Digest, Sha256};
  use std::num::NonZeroU32;
  use std::path::Path;

  type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

  /// PBKDF2 iteration count for deriving the AES key from the password stored
  /// in `os_crypt_key`. Must match Chromium's `OSCryptImpl` on each platform:
  /// macOS uses 1003 iterations, Linux uses 1. Getting this wrong produces a
  /// different AES key → silent decryption failure → empty cookie values.
  /// See `components/os_crypt/sync/os_crypt_{mac.mm,linux.cc}` in Chromium.
  #[cfg(target_os = "macos")]
  const PBKDF2_ITERATIONS: u32 = 1003;
  #[cfg(not(target_os = "macos"))]
  const PBKDF2_ITERATIONS: u32 = 1;

  const KEY_LEN: usize = 16; // AES-128
  const SALT: &[u8] = b"saltysalt";
  const IV: [u8; 16] = [b' '; 16]; // 16 spaces
  const HOST_HASH_LEN: usize = 32; // SHA-256 output length

  fn derive_key(password: &[u8]) -> [u8; KEY_LEN] {
    let mut key = [0u8; KEY_LEN];
    // Using ring::pbkdf2 instead of the `pbkdf2` crate to avoid digest
    // version conflicts between sha1 0.11 (digest 0.11) and pbkdf2 0.12
    // (digest 0.10). ring's implementation is self-contained.
    pbkdf2::derive(
      pbkdf2::PBKDF2_HMAC_SHA1,
      NonZeroU32::new(PBKDF2_ITERATIONS).expect("iterations must be non-zero"),
      SALT,
      password,
      &mut key,
    );
    key
  }

  pub fn get_encryption_key(profile_data_path: &Path) -> Option<[u8; KEY_LEN]> {
    let key_file = profile_data_path.join("os_crypt_key");
    // Read as raw bytes and do NOT trim — Chromium's `ReadFileToString`
    // passes the exact file contents to `Pbkdf2(file_contents)`. Any
    // normalisation we do here would produce a different derived key.
    let contents = std::fs::read(&key_file).ok()?;
    if contents.is_empty() {
      return None;
    }
    Some(derive_key(&contents))
  }

  /// Decrypt a Chrome encrypted cookie value.
  ///
  /// Chromium prefixes encrypted values with "v10" / "v11" and, since ~M100,
  /// prepends `SHA-256(host_key)` to the plaintext before encryption as an
  /// integrity check. After decryption we verify and strip those 32 bytes
  /// when present. Passing `host_key` is required to do that verification —
  /// without it we'd return 32 bytes of hash noise plus the actual value,
  /// which is not valid UTF-8 and gets thrown away.
  pub fn decrypt(encrypted: &[u8], host_key: &str, key: &[u8; KEY_LEN]) -> Option<String> {
    if encrypted.len() < 3 {
      return None;
    }
    let prefix = &encrypted[..3];
    if prefix != b"v10" && prefix != b"v11" {
      return None;
    }
    let ciphertext = &encrypted[3..];
    if ciphertext.is_empty() {
      return Some(String::new());
    }

    let mut buf = ciphertext.to_vec();
    let decrypted = Aes128CbcDec::new(key.into(), &IV.into())
      .decrypt_padded::<Pkcs7>(&mut buf)
      .ok()?;

    // Strip the SHA-256(host_key) integrity prefix if present. Older cookies
    // (pre-M100) didn't have this prefix, so we fall back to the raw bytes
    // when the first 32 bytes don't match the expected hash.
    if decrypted.len() >= HOST_HASH_LEN {
      let expected: [u8; HOST_HASH_LEN] = Sha256::digest(host_key.as_bytes()).into();
      if decrypted[..HOST_HASH_LEN] == expected {
        return String::from_utf8(decrypted[HOST_HASH_LEN..].to_vec()).ok();
      }
    }

    String::from_utf8(decrypted.to_vec()).ok()
  }
}

/// Unified cookie representation that works across both browser types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedCookie {
  pub name: String,
  pub value: String,
  pub domain: String,
  pub path: String,
  pub expires: i64,
  pub is_secure: bool,
  pub is_http_only: bool,
  pub same_site: i32,
  pub creation_time: i64,
  pub last_accessed: i64,
}

/// Cookies grouped by domain for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCookies {
  pub domain: String,
  pub cookies: Vec<UnifiedCookie>,
  pub cookie_count: usize,
}

/// Result of reading cookies from a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieReadResult {
  pub profile_id: String,
  pub browser_type: String,
  pub domains: Vec<DomainCookies>,
  pub total_count: usize,
}

/// Lightweight cookie metadata for the profile-info dialog. Computed without
/// decrypting any cookie values, so it stays cheap even for multi-MB Chromium
/// cookie stores and never blocks the runtime for noticeable time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieStats {
  pub profile_id: String,
  pub browser_type: String,
  pub total_count: usize,
  /// Every domain the profile has cookies for, sorted by cookie count desc.
  pub domains: Vec<DomainCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainCount {
  pub domain: String,
  pub count: usize,
}

/// Request to copy specific cookies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieCopyRequest {
  pub source_profile_id: String,
  pub target_profile_ids: Vec<String>,
  pub selected_cookies: Vec<SelectedCookie>,
}

/// Identifies a specific cookie to copy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedCookie {
  pub domain: String,
  pub name: String,
}

/// Result of a copy operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieCopyResult {
  pub target_profile_id: String,
  pub cookies_copied: usize,
  pub cookies_replaced: usize,
  pub errors: Vec<String>,
}

/// Result of a cookie import operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieImportResult {
  pub cookies_imported: usize,
  pub cookies_replaced: usize,
  pub errors: Vec<String>,
}

pub struct CookieManager;

impl CookieManager {
  /// Windows epoch offset: seconds between 1601-01-01 and 1970-01-01
  const WINDOWS_EPOCH_DIFF: i64 = 11644473600;

  fn get_chrome_encryption_key(profile: &BrowserProfile, profiles_dir: &Path) -> Option<[u8; 16]> {
    let profile_data_path = profile.get_profile_data_path(profiles_dir);
    chrome_decrypt::get_encryption_key(&profile_data_path)
  }

  
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_netscape_cookies_valid() {
    let content = "# Netscape HTTP Cookie File\n\
      .example.com\tTRUE\t/\tTRUE\t1700000000\tsession_id\tabc123\n\
      example.com\tFALSE\t/path\tFALSE\t0\ttoken\txyz";
    let (cookies, errors) = CookieManager::parse_netscape_cookies(content);
    assert_eq!(cookies.len(), 2);
    assert!(errors.is_empty());

    assert_eq!(cookies[0].domain, ".example.com");
    assert_eq!(cookies[0].name, "session_id");
    assert_eq!(cookies[0].value, "abc123");
    assert_eq!(cookies[0].path, "/");
    assert!(cookies[0].is_secure);
    assert_eq!(cookies[0].expires, 1700000000);

    assert_eq!(cookies[1].domain, "example.com");
    assert!(!cookies[1].is_secure);
    assert_eq!(cookies[1].expires, 0);
  }

  #[test]
  fn test_parse_netscape_cookies_skips_comments_and_blanks() {
    let content = "# Comment line\n\n  \n# Another comment\n\
      .test.com\tTRUE\t/\tFALSE\t0\tname\tvalue\n";
    let (cookies, errors) = CookieManager::parse_netscape_cookies(content);
    assert_eq!(cookies.len(), 1);
    assert!(errors.is_empty());
  }

  #[test]
  fn test_parse_netscape_cookies_malformed_lines() {
    let content = "not\tenough\tfields\n\
      .ok.com\tTRUE\t/\tFALSE\t0\tname\tvalue\n";
    let (cookies, errors) = CookieManager::parse_netscape_cookies(content);
    assert_eq!(cookies.len(), 1);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("expected 7 tab-separated fields"));
  }

  #[test]
  fn test_parse_json_cookies_valid() {
    let content = r#"[
      {
        "name": "sid",
        "value": "abc",
        "domain": ".example.com",
        "path": "/",
        "secure": true,
        "httpOnly": true,
        "sameSite": "lax",
        "expirationDate": 1700000000,
        "session": false
      }
    ]"#;
    let (cookies, errors) = CookieManager::parse_json_cookies(content);
    assert_eq!(cookies.len(), 1);
    assert!(errors.is_empty());
    assert_eq!(cookies[0].name, "sid");
    assert_eq!(cookies[0].domain, ".example.com");
    assert!(cookies[0].is_secure);
    assert!(cookies[0].is_http_only);
    assert_eq!(cookies[0].same_site, 1);
    assert_eq!(cookies[0].expires, 1700000000);
  }

  #[test]
  fn test_parse_json_cookies_session() {
    let content = r#"[{"name": "s", "value": "v", "domain": ".d.com", "session": true, "expirationDate": 9999}]"#;
    let (cookies, errors) = CookieManager::parse_json_cookies(content);
    assert_eq!(cookies.len(), 1);
    assert!(errors.is_empty());
    assert_eq!(cookies[0].expires, 0);
  }

  #[test]
  fn test_parse_json_cookies_same_site_mapping() {
    let content = r#"[
      {"name": "a", "value": "", "domain": ".d.com", "sameSite": "no_restriction"},
      {"name": "b", "value": "", "domain": ".d.com", "sameSite": "lax"},
      {"name": "c", "value": "", "domain": ".d.com", "sameSite": "strict"}
    ]"#;
    let (cookies, _) = CookieManager::parse_json_cookies(content);
    assert_eq!(cookies[0].same_site, 0);
    assert_eq!(cookies[1].same_site, 1);
    assert_eq!(cookies[2].same_site, 2);
  }

  #[test]
  fn test_parse_cookies_auto_detect_json() {
    let content = r#"[{"name": "x", "value": "y", "domain": ".test.com"}]"#;
    let (cookies, _) = CookieManager::parse_cookies(content);
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "x");
  }

  #[test]
  fn test_parse_cookies_auto_detect_netscape() {
    let content = ".test.com\tTRUE\t/\tFALSE\t0\tname\tvalue";
    let (cookies, _) = CookieManager::parse_cookies(content);
    assert_eq!(cookies.len(), 1);
    assert_eq!(cookies[0].name, "name");
  }

  #[test]
  fn test_format_netscape_cookies() {
    let cookies = vec![UnifiedCookie {
      name: "sid".to_string(),
      value: "abc".to_string(),
      domain: ".example.com".to_string(),
      path: "/".to_string(),
      expires: 1700000000,
      is_secure: true,
      is_http_only: false,
      same_site: 0,
      creation_time: 0,
      last_accessed: 0,
    }];
    let output = CookieManager::format_netscape_cookies(&cookies);
    assert!(output.contains("# Netscape HTTP Cookie File"));
    assert!(output.contains(".example.com\tTRUE\t/\tTRUE\t1700000000\tsid\tabc"));
  }

  #[test]
  fn test_format_json_cookies() {
    let cookies = vec![UnifiedCookie {
      name: "sid".to_string(),
      value: "abc".to_string(),
      domain: ".example.com".to_string(),
      path: "/".to_string(),
      expires: 1700000000,
      is_secure: true,
      is_http_only: true,
      same_site: 1,
      creation_time: 0,
      last_accessed: 0,
    }];
    let output = CookieManager::format_json_cookies(&cookies);
    let parsed: Vec<Value> = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["name"], "sid");
    assert_eq!(parsed[0]["sameSite"], "lax");
    assert_eq!(parsed[0]["session"], false);
    assert_eq!(parsed[0]["hostOnly"], false);
  }

  #[test]
  fn test_netscape_roundtrip() {
    let cookies = vec![
      UnifiedCookie {
        name: "a".to_string(),
        value: "1".to_string(),
        domain: ".d.com".to_string(),
        path: "/".to_string(),
        expires: 1700000000,
        is_secure: true,
        is_http_only: false,
        same_site: 0,
        creation_time: 0,
        last_accessed: 0,
      },
      UnifiedCookie {
        name: "b".to_string(),
        value: "2".to_string(),
        domain: "d.com".to_string(),
        path: "/p".to_string(),
        expires: 0,
        is_secure: false,
        is_http_only: false,
        same_site: 0,
        creation_time: 0,
        last_accessed: 0,
      },
    ];
    let formatted = CookieManager::format_netscape_cookies(&cookies);
    let (parsed, errors) = CookieManager::parse_netscape_cookies(&formatted);
    assert!(errors.is_empty());
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].name, "a");
    assert_eq!(parsed[0].domain, ".d.com");
    assert!(parsed[0].is_secure);
    assert_eq!(parsed[1].name, "b");
    assert_eq!(parsed[1].domain, "d.com");
  }

  #[test]
  fn test_json_roundtrip() {
    let cookies = vec![UnifiedCookie {
      name: "tok".to_string(),
      value: "xyz".to_string(),
      domain: ".site.org".to_string(),
      path: "/app".to_string(),
      expires: 1700000000,
      is_secure: false,
      is_http_only: true,
      same_site: 2,
      creation_time: 0,
      last_accessed: 0,
    }];
    let formatted = CookieManager::format_json_cookies(&cookies);
    let (parsed, errors) = CookieManager::parse_json_cookies(&formatted);
    assert!(errors.is_empty());
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].name, "tok");
    assert_eq!(parsed[0].domain, ".site.org");
    assert_eq!(parsed[0].path, "/app");
    assert!(!parsed[0].is_secure);
    assert!(parsed[0].is_http_only);
    assert_eq!(parsed[0].same_site, 2);
    assert_eq!(parsed[0].expires, 1700000000);
  }

  #[test]
  fn test_chrome_time_to_unix() {
    assert_eq!(CookieManager::chrome_time_to_unix(0), 0);
    let chrome_time: i64 = (1700000000 + CookieManager::WINDOWS_EPOCH_DIFF) * 1_000_000;
    assert_eq!(CookieManager::chrome_time_to_unix(chrome_time), 1700000000);
  }

  #[test]
  fn test_unix_to_chrome_time() {
    assert_eq!(CookieManager::unix_to_chrome_time(0), 0);
    let expected = (1700000000 + CookieManager::WINDOWS_EPOCH_DIFF) * 1_000_000;
    assert_eq!(CookieManager::unix_to_chrome_time(1700000000), expected);
  }

  #[test]
  fn test_chrome_time_roundtrip() {
    let unix = 1700000000_i64;
    let chrome = CookieManager::unix_to_chrome_time(unix);
    assert_eq!(CookieManager::chrome_time_to_unix(chrome), unix);
  }

  /// Set up a minimal Chrome cookie SQLite schema for testing writes.
  fn create_chrome_cookies_db(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn
      .execute_batch(
        "CREATE TABLE cookies (
          creation_utc INTEGER NOT NULL,
          host_key TEXT NOT NULL,
          top_frame_site_key TEXT NOT NULL,
          name TEXT NOT NULL,
          value TEXT NOT NULL,
          encrypted_value BLOB NOT NULL DEFAULT '',
          path TEXT NOT NULL,
          expires_utc INTEGER NOT NULL,
          is_secure INTEGER NOT NULL,
          is_httponly INTEGER NOT NULL,
          last_access_utc INTEGER NOT NULL,
          has_expires INTEGER NOT NULL DEFAULT 1,
          is_persistent INTEGER NOT NULL DEFAULT 1,
          priority INTEGER NOT NULL DEFAULT 1,
          samesite INTEGER NOT NULL DEFAULT -1,
          source_scheme INTEGER NOT NULL DEFAULT 0,
          source_port INTEGER NOT NULL DEFAULT -1,
          last_update_utc INTEGER NOT NULL DEFAULT 0,
          source_type INTEGER NOT NULL DEFAULT 0,
          has_cross_site_ancestor INTEGER NOT NULL DEFAULT 0
        );",
      )
      .unwrap();
  }

  /// Set up a minimal Firefox moz_cookies SQLite schema for testing writes.
  fn create_firefox_cookies_db(path: &Path) {
    let conn = Connection::open(path).unwrap();
    conn
      .execute_batch(
        "CREATE TABLE moz_cookies (
          id INTEGER PRIMARY KEY,
          originAttributes TEXT NOT NULL DEFAULT '',
          name TEXT,
          value TEXT,
          host TEXT,
          path TEXT,
          expiry INTEGER,
          lastAccessed INTEGER,
          creationTime INTEGER,
          isSecure INTEGER,
          isHttpOnly INTEGER,
          inBrowserElement INTEGER DEFAULT 0,
          sameSite INTEGER DEFAULT 0,
          rawSameSite INTEGER DEFAULT 0,
          schemeMap INTEGER DEFAULT 0,
          CONSTRAINT moz_uniqueid UNIQUE (name, host, path, originAttributes)
        );",
      )
      .unwrap();
  }

  #[test]
  fn test_write_chrome_cookies_stores_plaintext_values() {
    let tmp = std::env::temp_dir().join(format!("donut_cookie_test_{}.db", uuid::Uuid::new_v4()));
    create_chrome_cookies_db(&tmp);

    let cookies = vec![UnifiedCookie {
      name: "c_user".to_string(),
      value: "100012345".to_string(),
      domain: ".facebook.com".to_string(),
      path: "/".to_string(),
      expires: 1800000000,
      is_secure: true,
      is_http_only: true,
      same_site: 0,
      creation_time: 1700000000,
      last_accessed: 1700000000,
    }];

    let (inserted, replaced) = CookieManager::write_chrome_cookies(&tmp, &cookies).unwrap();
    assert_eq!(inserted, 1);
    assert_eq!(replaced, 0);

    let conn = Connection::open(&tmp).unwrap();
    let (value, encrypted, has_expires, is_persistent, source_scheme, source_port): (
      String,
      Vec<u8>,
      i32,
      i32,
      i32,
      i32,
    ) = conn
      .query_row(
        "SELECT value, encrypted_value, has_expires, is_persistent, source_scheme, source_port
         FROM cookies WHERE name = ?1",
        params!["c_user"],
        |row| {
          Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
          ))
        },
      )
      .unwrap();

    // Core fix: plaintext in value, empty encrypted_value
    assert_eq!(value, "100012345");
    assert!(encrypted.is_empty());
    // Persistent cookie since expires > 0
    assert_eq!(has_expires, 1);
    assert_eq!(is_persistent, 1);
    // Secure cookie gets HTTPS scheme + port 443
    assert_eq!(source_scheme, 2);
    assert_eq!(source_port, 443);

    let _ = std::fs::remove_file(&tmp);
  }

  #[test]
  fn test_write_chrome_cookies_session_cookie_not_expired() {
    let tmp = std::env::temp_dir().join(format!("donut_cookie_test_{}.db", uuid::Uuid::new_v4()));
    create_chrome_cookies_db(&tmp);

    let cookies = vec![UnifiedCookie {
      name: "session".to_string(),
      value: "abc".to_string(),
      domain: ".example.com".to_string(),
      path: "/".to_string(),
      expires: 0, // session cookie
      is_secure: false,
      is_http_only: false,
      same_site: 0,
      creation_time: 1700000000,
      last_accessed: 1700000000,
    }];

    CookieManager::write_chrome_cookies(&tmp, &cookies).unwrap();

    let conn = Connection::open(&tmp).unwrap();
    let (has_expires, is_persistent, source_scheme, source_port): (i32, i32, i32, i32) = conn
      .query_row(
        "SELECT has_expires, is_persistent, source_scheme, source_port
         FROM cookies WHERE name = ?1",
        params!["session"],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
      )
      .unwrap();

    // Session cookie must not be persistent — otherwise Chromium treats
    // expires_utc=0 as 1601-01-01 (immediately expired).
    assert_eq!(has_expires, 0);
    assert_eq!(is_persistent, 0);
    // Non-secure cookie uses HTTP scheme + port 80
    assert_eq!(source_scheme, 1);
    assert_eq!(source_port, 80);

    let _ = std::fs::remove_file(&tmp);
  }

  #[test]
  fn test_write_chrome_cookies_replaces_existing() {
    let tmp = std::env::temp_dir().join(format!("donut_cookie_test_{}.db", uuid::Uuid::new_v4()));
    create_chrome_cookies_db(&tmp);

    let cookie = UnifiedCookie {
      name: "token".to_string(),
      value: "v1".to_string(),
      domain: ".example.com".to_string(),
      path: "/".to_string(),
      expires: 1800000000,
      is_secure: true,
      is_http_only: false,
      same_site: 1,
      creation_time: 1700000000,
      last_accessed: 1700000000,
    };

    let (inserted, _) =
      CookieManager::write_chrome_cookies(&tmp, std::slice::from_ref(&cookie)).unwrap();
    assert_eq!(inserted, 1);

    let mut updated = cookie.clone();
    updated.value = "v2".to_string();
    let (inserted, replaced) =
      CookieManager::write_chrome_cookies(&tmp, std::slice::from_ref(&updated)).unwrap();
    assert_eq!(inserted, 0);
    assert_eq!(replaced, 1);

    let conn = Connection::open(&tmp).unwrap();
    let (value, encrypted): (String, Vec<u8>) = conn
      .query_row(
        "SELECT value, encrypted_value FROM cookies WHERE name = ?1",
        params!["token"],
        |row| Ok((row.get(0)?, row.get(1)?)),
      )
      .unwrap();
    assert_eq!(value, "v2");
    assert!(encrypted.is_empty());

    let _ = std::fs::remove_file(&tmp);
  }

  /// Chrome → Camoufox: write cookies to a Chrome DB, read them back, and
  /// verify they land in a Firefox DB with values intact, correct schemeMap,
  /// and non-expired timestamps. This is the path exercised by the
  /// "copy cookies between profiles of different browser types" feature.
  