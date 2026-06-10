# XChrome Phase 1 Integration Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Phase 1 XChrome-inspired workflows to Donut Browser: batch profile launch/stop, Windows window arrangement, proxy diagnostics, and a batch operations UI.

**Architecture:** Keep the implementation inside Donut Browser's existing Tauri/Rust backend and Next.js frontend. Add focused Rust modules for batch orchestration, window layout calculation/platform actions, and proxy diagnostics; expose them through Tauri commands; add a small frontend dialog and action-bar entries that reuse existing profile selection state.

**Tech Stack:** Rust, Tauri 2, Tokio, Serde, reqwest, Windows API through the existing `windows` crate, Next.js 16, React 19, TypeScript, i18next, shadcn/Radix UI primitives, pnpm.

---

## Spec Reference

Read first:

- `docs/superpowers/specs/2026-06-10-xchrome-integration-design.md`
- `AGENTS.md`, especially translation rules, backend error-code rules, and GitNexus rules.

Phase 1 only:

- Batch launch and stop.
- Group-based batch launch and stop through selected/current profiles.
- Windows window arrangement.
- Proxy diagnostics.
- Batch task result panel.

Do not implement:

- Synchronized click/keyboard group control.
- Script automation.
- API/MCP expansion.
- Advanced fingerprint editing.
- XChrome database import.
- Any copied XChrome source code.

## File Structure

Create:

- `src-tauri/src/batch_runner.rs`  
  Batch options, task events, validation, profile ID resolution, and launch/stop orchestration.

- `src-tauri/src/window_layout.rs`  
  Cross-platform layout types, pure layout calculation, capability reporting, and Windows-specific top-level window arrangement.

- `src-tauri/src/proxy_diagnostics.rs`  
  Multi-source proxy diagnostics types and orchestration over stored proxy settings.

- `src/components/batch-operations-dialog.tsx`  
  Dialog for configuring batch launch/stop/arrange/diagnose work and showing task results.

- `src/hooks/use-batch-task-events.ts`  
  Tauri event listener and reducer for batch task status updates.

Modify:

- `src-tauri/src/lib.rs`  
  Register modules and Tauri commands.

- `src-tauri/Cargo.toml`  
  Add missing Windows API features if required by `window_layout.rs`.

- `src/app/page.tsx`  
  Own dialog state, selected profile actions, command invocations, and event-driven feedback.

- `src/components/profile-data-table.tsx`  
  Add action-bar buttons for batch launch, stop, arrange, and diagnose.

- `src/types.ts`  
  Add frontend types matching backend batch options/results.

- `src/lib/backend-errors.ts`  
  Add structured backend error codes and translation mapping.

- `src/i18n/locales/*.json`  
  Add new UI and backend error strings to all locales.

Test:

- Rust unit tests in the new backend modules.
- Existing Rust command registration tests in `src-tauri/src/lib.rs`.
- Frontend typecheck via `pnpm lint:js`.
- Full validation via `pnpm format && pnpm lint && pnpm test`.

---

## Chunk 1: Backend Pure Logic

### Task 1: Add Window Layout Calculation

**Files:**

- Create: `src-tauri/src/window_layout.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact before editing `lib.rs`**

Run:

```bash
# From repo root
```

Use GitNexus:

```text
impact target=lib.rs direction=upstream repo=donutbrowser
```

Expected: report risk before editing. If HIGH or CRITICAL, stop and review the blast radius.

- [ ] **Step 2: Write failing tests for layout calculation**

Add this test module to the bottom of `src-tauri/src/window_layout.rs` before implementation:

```rust
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn grid_layout_fills_rows_and_columns() {
    let work_area = Rect {
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
    };

    let layouts = calculate_layouts(
      5,
      work_area,
      WindowLayoutOptions {
        mode: WindowLayoutMode::Grid,
        gap: 0,
        preserve_aspect_ratio: false,
      },
    )
    .unwrap();

    assert_eq!(layouts.len(), 5);
    assert_eq!(layouts[0], Rect { x: 0, y: 0, width: 400, height: 400 });
    assert_eq!(layouts[4], Rect { x: 400, y: 400, width: 400, height: 400 });
  }

  #[test]
  fn horizontal_layout_respects_gap() {
    let layouts = calculate_layouts(
      3,
      Rect {
        x: 10,
        y: 20,
        width: 930,
        height: 500,
      },
      WindowLayoutOptions {
        mode: WindowLayoutMode::Horizontal,
        gap: 10,
        preserve_aspect_ratio: false,
      },
    )
    .unwrap();

    assert_eq!(layouts[0], Rect { x: 10, y: 20, width: 303, height: 500 });
    assert_eq!(layouts[1].x, 323);
    assert_eq!(layouts[2].x, 636);
  }

  #[test]
  fn zero_windows_returns_error() {
    let err = calculate_layouts(
      0,
      Rect {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
      },
      WindowLayoutOptions::default(),
    )
    .unwrap_err();

    assert_eq!(err.code(), "WINDOW_LAYOUT_NO_RUNNING_WINDOWS");
  }
}
```

- [ ] **Step 3: Run the failing test**

Run:

```bash
cd src-tauri
cargo test window_layout::tests --lib
```

Expected: FAIL because `window_layout` types/functions do not exist yet.

- [ ] **Step 4: Add the minimal layout module**

Create `src-tauri/src/window_layout.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
  pub x: i32,
  pub y: i32,
  pub width: i32,
  pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowLayoutMode {
  Grid,
  Horizontal,
  Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowLayoutOptions {
  pub mode: WindowLayoutMode,
  pub gap: i32,
  pub preserve_aspect_ratio: bool,
}

impl Default for WindowLayoutOptions {
  fn default() -> Self {
    Self {
      mode: WindowLayoutMode::Grid,
      gap: 8,
      preserve_aspect_ratio: false,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowLayoutError {
  code: &'static str,
}

impl WindowLayoutError {
  pub fn new(code: &'static str) -> Self {
    Self { code }
  }

  pub fn code(&self) -> &'static str {
    self.code
  }

  pub fn to_backend_error(&self) -> String {
    serde_json::json!({ "code": self.code }).to_string()
  }
}

pub fn calculate_layouts(
  count: usize,
  work_area: Rect,
  options: WindowLayoutOptions,
) -> Result<Vec<Rect>, WindowLayoutError> {
  if count == 0 {
    return Err(WindowLayoutError::new("WINDOW_LAYOUT_NO_RUNNING_WINDOWS"));
  }

  let gap = options.gap.max(0);
  let mut rects = Vec::with_capacity(count);

  match options.mode {
    WindowLayoutMode::Grid => {
      let columns = (count as f64).sqrt().ceil() as i32;
      let rows = ((count as f64) / columns as f64).ceil() as i32;
      let cell_width = (work_area.width - gap * (columns - 1)) / columns;
      let cell_height = (work_area.height - gap * (rows - 1)) / rows;

      for index in 0..count {
        let row = index as i32 / columns;
        let col = index as i32 % columns;
        rects.push(Rect {
          x: work_area.x + col * (cell_width + gap),
          y: work_area.y + row * (cell_height + gap),
          width: cell_width,
          height: cell_height,
        });
      }
    }
    WindowLayoutMode::Horizontal => {
      let columns = count as i32;
      let width = (work_area.width - gap * (columns - 1)) / columns;
      for index in 0..count {
        let col = index as i32;
        rects.push(Rect {
          x: work_area.x + col * (width + gap),
          y: work_area.y,
          width,
          height: work_area.height,
        });
      }
    }
    WindowLayoutMode::Vertical => {
      let rows = count as i32;
      let height = (work_area.height - gap * (rows - 1)) / rows;
      for index in 0..count {
        let row = index as i32;
        rects.push(Rect {
          x: work_area.x,
          y: work_area.y + row * (height + gap),
          width: work_area.width,
          height,
        });
      }
    }
  }

  Ok(rects)
}
```

In `src-tauri/src/lib.rs`, add:

```rust
mod window_layout;
```

- [ ] **Step 5: Run tests**

Run:

```bash
cd src-tauri
cargo test window_layout::tests --lib
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src-tauri/src/window_layout.rs src-tauri/src/lib.rs
git commit -m "feat: add window layout calculations"
```

### Task 2: Add Batch Runner Validation and Status Types

**Files:**

- Create: `src-tauri/src/batch_runner.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add tests for option bounds and event payload serialization:

```rust
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_options_are_conservative() {
    let options = BatchLaunchOptions::default();
    assert_eq!(options.concurrency, 3);
    assert_eq!(options.launch_interval_ms, 1000);
    assert_eq!(options.failure_policy, FailurePolicy::Continue);
  }

  #[test]
  fn validation_rejects_empty_profile_list() {
    let err = validate_profile_ids(&[]).unwrap_err();
    assert_eq!(err.code(), "BATCH_NO_PROFILES_SELECTED");
  }

  #[test]
  fn validation_clamps_concurrency() {
    let mut options = BatchLaunchOptions {
      concurrency: 0,
      launch_interval_ms: 1000,
      failure_policy: FailurePolicy::Continue,
      post_launch_action: PostLaunchAction::None,
    };

    options.normalize();
    assert_eq!(options.concurrency, 1);
  }

  #[test]
  fn status_event_serializes_expected_shape() {
    let event = BatchTaskEvent {
      task_id: "task-1".to_string(),
      profile_id: "profile-1".to_string(),
      profile_name: "Profile 1".to_string(),
      status: BatchProfileStatus::Launching,
      error: None,
    };

    let value = serde_json::to_value(event).unwrap();
    assert_eq!(value["status"], "launching");
  }
}
```

- [ ] **Step 2: Run failing tests**

Run:

```bash
cd src-tauri
cargo test batch_runner::tests --lib
```

Expected: FAIL because module/types do not exist.

- [ ] **Step 3: Implement types and validation**

Create `src-tauri/src/batch_runner.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchError {
  code: &'static str,
}

impl BatchError {
  pub fn new(code: &'static str) -> Self {
    Self { code }
  }

  pub fn code(&self) -> &'static str {
    self.code
  }

  pub fn to_backend_error(&self) -> String {
    serde_json::json!({ "code": self.code }).to_string()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailurePolicy {
  Continue,
  StopOnFirstError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostLaunchAction {
  None,
  ArrangeWindows,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchLaunchOptions {
  pub concurrency: usize,
  pub launch_interval_ms: u64,
  pub failure_policy: FailurePolicy,
  pub post_launch_action: PostLaunchAction,
}

impl Default for BatchLaunchOptions {
  fn default() -> Self {
    Self {
      concurrency: 3,
      launch_interval_ms: 1000,
      failure_policy: FailurePolicy::Continue,
      post_launch_action: PostLaunchAction::None,
    }
  }
}

impl BatchLaunchOptions {
  pub fn normalize(&mut self) {
    self.concurrency = self.concurrency.clamp(1, 10);
    self.launch_interval_ms = self.launch_interval_ms.min(30_000);
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchProfileStatus {
  Queued,
  Launching,
  Running,
  Failed,
  Stopped,
  Arranging,
  Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchTaskEvent {
  pub task_id: String,
  pub profile_id: String,
  pub profile_name: String,
  pub status: BatchProfileStatus,
  pub error: Option<String>,
}

pub fn validate_profile_ids(profile_ids: &[String]) -> Result<(), BatchError> {
  if profile_ids.is_empty() {
    return Err(BatchError::new("BATCH_NO_PROFILES_SELECTED"));
  }
  Ok(())
}
```

In `src-tauri/src/lib.rs`, add:

```rust
mod batch_runner;
```

- [ ] **Step 4: Run tests**

Run:

```bash
cd src-tauri
cargo test batch_runner::tests --lib
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src-tauri/src/batch_runner.rs src-tauri/src/lib.rs
git commit -m "feat: add batch runner types"
```

### Task 3: Add Proxy Diagnostics Types and Source Parsing

**Files:**

- Create: `src-tauri/src/proxy_diagnostics.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add tests for normalizing detector responses:

```rust
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
    let err = parse_source_response(ProxyDiagnosticSource::IpApi, serde_json::json!({}))
      .unwrap_err();
    assert_eq!(err.code(), "PROXY_DIAGNOSTIC_SOURCE_FAILED");
  }
}
```

- [ ] **Step 2: Run failing tests**

Run:

```bash
cd src-tauri
cargo test proxy_diagnostics::tests --lib
```

Expected: FAIL because module/types do not exist.

- [ ] **Step 3: Implement diagnostic types and parsing**

Create `src-tauri/src/proxy_diagnostics.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyDiagnosticError {
  code: &'static str,
}

impl ProxyDiagnosticError {
  pub fn new(code: &'static str) -> Self {
    Self { code }
  }

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
  .and_then(|v| v.as_str())
  .ok_or_else(|| ProxyDiagnosticError::new("PROXY_DIAGNOSTIC_SOURCE_FAILED"))?;

  let country = match source {
    ProxyDiagnosticSource::IpApi => value.get("country"),
    ProxyDiagnosticSource::IpSb => value.get("country"),
    ProxyDiagnosticSource::IpapiCo => value.get("country_name"),
  }
  .and_then(|v| v.as_str())
  .map(ToOwned::to_owned);

  let country_code = match source {
    ProxyDiagnosticSource::IpApi => value.get("countryCode"),
    ProxyDiagnosticSource::IpSb => value.get("country_code"),
    ProxyDiagnosticSource::IpapiCo => value.get("country_code"),
  }
  .and_then(|v| v.as_str())
  .map(ToOwned::to_owned);

  Ok(ProxyDiagnosticSourceResult {
    source,
    ip: ip.to_string(),
    country,
    country_code,
  })
}
```

In `src-tauri/src/lib.rs`, add:

```rust
mod proxy_diagnostics;
```

- [ ] **Step 4: Run tests**

Run:

```bash
cd src-tauri
cargo test proxy_diagnostics::tests --lib
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src-tauri/src/proxy_diagnostics.rs src-tauri/src/lib.rs
git commit -m "feat: add proxy diagnostics types"
```

---

## Chunk 2: Backend Commands and Platform Integration

### Task 4: Implement Batch Launch and Stop Commands

**Files:**

- Modify: `src-tauri/src/batch_runner.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/batch_runner.rs`

- [ ] **Step 1: Run GitNexus impact for launch/kill symbols**

Use GitNexus:

```text
impact target=launch_browser_profile_impl direction=upstream repo=donutbrowser
impact target=kill_browser_profile direction=upstream repo=donutbrowser
```

Expected: report blast radius. If HIGH or CRITICAL, surface it before editing.

- [ ] **Step 2: Add tests for profile resolution**

Add a pure helper in `batch_runner.rs`:

```rust
pub fn profiles_by_ids(
  profiles: Vec<crate::profile::BrowserProfile>,
  profile_ids: &[String],
) -> Result<Vec<crate::profile::BrowserProfile>, BatchError> {
  validate_profile_ids(profile_ids)?;
  let mut resolved = Vec::with_capacity(profile_ids.len());

  for id in profile_ids {
    let profile = profiles
      .iter()
      .find(|profile| profile.id.to_string() == *id)
      .cloned()
      .ok_or_else(|| BatchError::new("BATCH_PROFILE_NOT_FOUND"))?;
    resolved.push(profile);
  }

  Ok(resolved)
}
```

Write tests that pass two profile IDs and fail for a missing ID. Use `BrowserProfile::default()` with explicit UUIDs.

- [ ] **Step 3: Run failing tests**

Run:

```bash
cd src-tauri
cargo test batch_runner::tests --lib
```

Expected: FAIL until helper is implemented.

- [ ] **Step 4: Implement orchestration functions**

Add to `batch_runner.rs`:

```rust
pub async fn batch_launch_profiles(
  app_handle: tauri::AppHandle,
  profile_ids: Vec<String>,
  mut options: BatchLaunchOptions,
) -> Result<String, String> {
  validate_profile_ids(&profile_ids).map_err(|e| e.to_backend_error())?;
  options.normalize();

  let task_id = uuid::Uuid::new_v4().to_string();
  let profiles = crate::profile::ProfileManager::instance()
    .list_profiles()
    .map_err(|_| serde_json::json!({ "code": "INTERNAL_ERROR" }).to_string())?;
  let profiles = profiles_by_ids(profiles, &profile_ids).map_err(|e| e.to_backend_error())?;

  for profile in profiles {
    emit_status(&task_id, &profile, BatchProfileStatus::Queued, None);
    emit_status(&task_id, &profile, BatchProfileStatus::Launching, None);

    let result = crate::browser_runner::launch_browser_profile_impl(
      app_handle.clone(),
      profile.clone(),
      None,
      None,
      false,
      false,
    )
    .await;

    match result {
      Ok(updated) => {
        emit_status(&task_id, &updated, BatchProfileStatus::Running, None);
        emit_status(&task_id, &updated, BatchProfileStatus::Completed, None);
      }
      Err(err) => {
        emit_status(&task_id, &profile, BatchProfileStatus::Failed, Some(err));
        if options.failure_policy == FailurePolicy::StopOnFirstError {
          break;
        }
      }
    }

    tokio::time::sleep(std::time::Duration::from_millis(options.launch_interval_ms)).await;
  }

  Ok(task_id)
}
```

Implement `batch_stop_profiles` similarly, calling `crate::browser_runner::kill_browser_profile`.

Important: this first pass may be sequential. Add concurrency only after the sequential path is working and tested. If adding concurrency in the same task, use a bounded `tokio::sync::Semaphore`.

- [ ] **Step 5: Implement `emit_status`**

Add:

```rust
fn emit_status(
  task_id: &str,
  profile: &crate::profile::BrowserProfile,
  status: BatchProfileStatus,
  error: Option<String>,
) {
  let event = BatchTaskEvent {
    task_id: task_id.to_string(),
    profile_id: profile.id.to_string(),
    profile_name: profile.name.clone(),
    status,
    error,
  };

  if let Err(err) = crate::events::emit("batch-task-status", event) {
    log::warn!("Failed to emit batch task status: {err}");
  }
}
```

- [ ] **Step 6: Register Tauri commands**

In `src-tauri/src/lib.rs`, add wrapper commands near related profile commands:

```rust
#[tauri::command]
async fn batch_launch_profiles(
  app_handle: tauri::AppHandle,
  profile_ids: Vec<String>,
  options: Option<crate::batch_runner::BatchLaunchOptions>,
) -> Result<String, String> {
  crate::batch_runner::batch_launch_profiles(
    app_handle,
    profile_ids,
    options.unwrap_or_default(),
  )
  .await
}

#[tauri::command]
async fn batch_stop_profiles(
  app_handle: tauri::AppHandle,
  profile_ids: Vec<String>,
) -> Result<String, String> {
  crate::batch_runner::batch_stop_profiles(app_handle, profile_ids).await
}
```

Add both to `tauri::generate_handler![...]`.

- [ ] **Step 7: Run command registration and Rust tests**

Run:

```bash
pnpm check-unused-commands
cd src-tauri && cargo test batch_runner::tests --lib
```

Expected: PASS. If `check-unused-commands` flags the new commands before frontend is wired, add them to the test allowlist only temporarily, or do the frontend wiring before running the check as final validation.

- [ ] **Step 8: Commit**

Run:

```bash
git add src-tauri/src/batch_runner.rs src-tauri/src/lib.rs
git commit -m "feat: add batch profile commands"
```

### Task 5: Implement Window Arrangement Command

**Files:**

- Modify: `src-tauri/src/window_layout.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml` if Windows features are missing

- [ ] **Step 1: Add capability tests**

Add tests:

```rust
#[test]
fn capabilities_report_current_platform() {
  let capabilities = get_window_layout_capabilities();
  if cfg!(target_os = "windows") {
    assert!(capabilities.supported);
  } else {
    assert!(!capabilities.supported);
  }
}
```

- [ ] **Step 2: Implement capability type**

Add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayoutCapabilities {
  pub supported: bool,
  pub platform: String,
}

pub fn get_window_layout_capabilities() -> WindowLayoutCapabilities {
  WindowLayoutCapabilities {
    supported: cfg!(target_os = "windows"),
    platform: std::env::consts::OS.to_string(),
  }
}
```

- [ ] **Step 3: Add Windows API features if needed**

The current `windows` dependency has many features but may need:

```toml
"Win32_UI_WindowsAndMessaging",
"Win32_Graphics_Gdi",
```

Add only if compilation proves they are required.

- [ ] **Step 4: Implement platform functions**

Add:

```rust
pub async fn arrange_running_profile_windows(
  profile_ids: Vec<String>,
  options: WindowLayoutOptions,
) -> Result<(), String> {
  if !cfg!(target_os = "windows") {
    return Err(
      serde_json::json!({ "code": "WINDOW_LAYOUT_UNSUPPORTED_PLATFORM" }).to_string(),
    );
  }

  arrange_running_profile_windows_platform(profile_ids, options).await
}
```

For non-Windows:

```rust
#[cfg(not(target_os = "windows"))]
async fn arrange_running_profile_windows_platform(
  _profile_ids: Vec<String>,
  _options: WindowLayoutOptions,
) -> Result<(), String> {
  Err(serde_json::json!({ "code": "WINDOW_LAYOUT_UNSUPPORTED_PLATFORM" }).to_string())
}
```

For Windows:

- Resolve profiles by ID.
- Keep only profiles with `process_id`.
- Enumerate top-level windows with `EnumWindows`.
- Match window process ID with `GetWindowThreadProcessId`.
- Get primary monitor work area.
- Call `calculate_layouts`.
- Move each window with `SetWindowPos`.

Use a tiny internal struct for discovered windows:

```rust
#[cfg(target_os = "windows")]
struct ProfileWindow {
  profile_id: String,
  process_id: u32,
  hwnd: windows::Win32::Foundation::HWND,
}
```

- [ ] **Step 5: Register commands**

In `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
fn get_window_layout_capabilities() -> crate::window_layout::WindowLayoutCapabilities {
  crate::window_layout::get_window_layout_capabilities()
}

#[tauri::command]
async fn arrange_running_profile_windows(
  profile_ids: Vec<String>,
  options: crate::window_layout::WindowLayoutOptions,
) -> Result<(), String> {
  crate::window_layout::arrange_running_profile_windows(profile_ids, options).await
}
```

Add both to `generate_handler`.

- [ ] **Step 6: Run tests**

Run:

```bash
cd src-tauri
cargo test window_layout::tests --lib
cargo check --target x86_64-pc-windows-msvc
```

Expected: unit tests pass. Windows target check may fail if the toolchain is unavailable on this machine; if so, run normal `cargo check` and document the missing target.

- [ ] **Step 7: Commit**

Run:

```bash
git add src-tauri/src/window_layout.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: arrange running profile windows"
```

### Task 6: Implement Proxy Diagnostics Command

**Files:**

- Modify: `src-tauri/src/proxy_diagnostics.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Run GitNexus impact for proxy logic**

Use GitNexus:

```text
impact target=check_proxy_validity direction=upstream repo=donutbrowser
impact target=ProxyManager direction=upstream repo=donutbrowser
```

Expected: report blast radius before editing proxy-related behavior.

- [ ] **Step 2: Add tests for missing proxy handling**

Add a pure helper:

```rust
pub fn missing_proxy_result(profile_id: String, profile_name: String) -> ProfileProxyDiagnosticResult {
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
```

Test that it returns `is_valid = false` and `PROXY_NOT_FOUND`.

- [ ] **Step 3: Implement source URLs**

Add:

```rust
impl ProxyDiagnosticSource {
  fn url(self) -> &'static str {
    match self {
      ProxyDiagnosticSource::IpApi => "http://ip-api.com/json/",
      ProxyDiagnosticSource::IpSb => "https://api.ip.sb/geoip/",
      ProxyDiagnosticSource::IpapiCo => "https://ipapi.co/json/",
    }
  }
}
```

- [ ] **Step 4: Implement diagnostics orchestration**

Add `diagnose_profile_proxies`:

```rust
pub async fn diagnose_profile_proxies(
  profile_ids: Vec<String>,
  options: ProxyDiagnosticOptions,
) -> Result<Vec<ProfileProxyDiagnosticResult>, String> {
  crate::batch_runner::validate_profile_ids(&profile_ids)
    .map_err(|e| e.to_backend_error())?;

  let profiles = crate::profile::ProfileManager::instance()
    .list_profiles()
    .map_err(|_| serde_json::json!({ "code": "INTERNAL_ERROR" }).to_string())?;
  let profiles = crate::batch_runner::profiles_by_ids(profiles, &profile_ids)
    .map_err(|e| e.to_backend_error())?;

  let mut results = Vec::with_capacity(profiles.len());
  for profile in profiles {
    results.push(diagnose_one_profile(profile, &options).await);
  }
  Ok(results)
}
```

`diagnose_one_profile` should:

- Return `missing_proxy_result` if `profile.proxy_id` is empty.
- Resolve stored proxy through `PROXY_MANAGER.get_stored_proxy`.
- Reuse `ProxyManager::build_proxy_url` if made public, or add a small method that returns the same URL safely.
- Use `reqwest::Proxy::all(proxy_url)` to route each source request.
- Timeout each source.
- Return first successful source result.
- Return `PROXY_DIAGNOSTIC_TIMEOUT` or `PROXY_DIAGNOSTIC_SOURCE_FAILED` after all sources fail.

Avoid changing existing `check_proxy_validity` behavior in this task.

- [ ] **Step 5: Register command**

In `src-tauri/src/lib.rs`:

```rust
#[tauri::command]
async fn diagnose_profile_proxies(
  profile_ids: Vec<String>,
  options: Option<crate::proxy_diagnostics::ProxyDiagnosticOptions>,
) -> Result<Vec<crate::proxy_diagnostics::ProfileProxyDiagnosticResult>, String> {
  crate::proxy_diagnostics::diagnose_profile_proxies(
    profile_ids,
    options.unwrap_or_default(),
  )
  .await
}
```

Add it to `generate_handler`.

- [ ] **Step 6: Run tests**

Run:

```bash
cd src-tauri
cargo test proxy_diagnostics::tests --lib
cargo test batch_runner::tests --lib
```

Expected: PASS.

- [ ] **Step 7: Commit**

Run:

```bash
git add src-tauri/src/proxy_diagnostics.rs src-tauri/src/lib.rs
git commit -m "feat: add profile proxy diagnostics"
```

---

## Chunk 3: Frontend Types, Events, and Dialog

### Task 7: Add Frontend Types and Batch Event Hook

**Files:**

- Modify: `src/types.ts`
- Create: `src/hooks/use-batch-task-events.ts`

- [ ] **Step 1: Add frontend types**

In `src/types.ts`, add:

```ts
export type BatchProfileStatus =
  | "queued"
  | "launching"
  | "running"
  | "failed"
  | "stopped"
  | "arranging"
  | "completed";

export type FailurePolicy = "continue" | "stop_on_first_error";
export type PostLaunchAction = "none" | "arrange_windows";
export type WindowLayoutMode = "grid" | "horizontal" | "vertical";

export interface BatchLaunchOptions {
  concurrency: number;
  launch_interval_ms: number;
  failure_policy: FailurePolicy;
  post_launch_action: PostLaunchAction;
}

export interface BatchTaskEvent {
  task_id: string;
  profile_id: string;
  profile_name: string;
  status: BatchProfileStatus;
  error?: string;
}

export interface WindowLayoutOptions {
  mode: WindowLayoutMode;
  gap: number;
  preserve_aspect_ratio: boolean;
}

export interface WindowLayoutCapabilities {
  supported: boolean;
  platform: string;
}

export interface ProfileProxyDiagnosticResult {
  profile_id: string;
  profile_name: string;
  proxy_id?: string;
  proxy_name?: string;
  is_valid: boolean;
  latency_ms?: number;
  source?: "ip_api" | "ip_sb" | "ipapi_co";
  ip?: string;
  country?: string;
  country_code?: string;
  error?: string;
}
```

- [ ] **Step 2: Create event hook**

Create `src/hooks/use-batch-task-events.ts`:

```ts
"use client";

import { listen } from "@tauri-apps/api/event";
import * as React from "react";
import type { BatchTaskEvent } from "@/types";

export function useBatchTaskEvents() {
  const [eventsByTask, setEventsByTask] = React.useState<
    Record<string, Record<string, BatchTaskEvent>>
  >({});

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;

    void (async () => {
      unlisten = await listen<BatchTaskEvent>("batch-task-status", (event) => {
        const payload = event.payload;
        setEventsByTask((current) => ({
          ...current,
          [payload.task_id]: {
            ...(current[payload.task_id] ?? {}),
            [payload.profile_id]: payload,
          },
        }));
      });
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  const clearTask = React.useCallback((taskId: string) => {
    setEventsByTask((current) => {
      const next = { ...current };
      delete next[taskId];
      return next;
    });
  }, []);

  return { eventsByTask, clearTask };
}
```

- [ ] **Step 3: Run JS typecheck**

Run:

```bash
pnpm lint:js
```

Expected: PASS. If it fails because `donut-sync` has unrelated issues, record the exact failure and continue only if app code typechecks independently.

- [ ] **Step 4: Commit**

Run:

```bash
git add src/types.ts src/hooks/use-batch-task-events.ts
git commit -m "feat: add batch task frontend types"
```

### Task 8: Add Batch Operations Dialog

**Files:**

- Create: `src/components/batch-operations-dialog.tsx`
- Modify: `src/i18n/locales/*.json`

- [ ] **Step 1: Add translation keys to all locale files**

Use one script to update all locale files. Add these keys with English text for non-English locales if translations are not available:

```json
{
  "batchOperations": {
    "title": "Batch Operations",
    "description": "Run actions across selected profiles.",
    "targetSelected": "Selected profiles",
    "concurrency": "Concurrency",
    "launchInterval": "Launch interval",
    "failurePolicy": "Failure policy",
    "continueOnError": "Continue on error",
    "stopOnFirstError": "Stop on first error",
    "postLaunchAction": "After launch",
    "postLaunchNone": "Do nothing",
    "postLaunchArrange": "Arrange windows",
    "layoutMode": "Window layout",
    "layoutGrid": "Grid",
    "layoutHorizontal": "Horizontal",
    "layoutVertical": "Vertical",
    "gap": "Gap",
    "preserveAspectRatio": "Preserve aspect ratio",
    "launchSelected": "Launch selected",
    "stopSelected": "Stop selected",
    "arrangeWindows": "Arrange windows",
    "diagnoseProxies": "Diagnose proxies",
    "results": "Results",
    "noResults": "No results yet",
    "profile": "Profile",
    "result": "Result",
    "exitIp": "Exit IP",
    "country": "Country",
    "latency": "Latency",
    "source": "Source"
  }
}
```

- [ ] **Step 2: Create dialog component**

Implement `BatchOperationsDialog` with props:

```ts
interface BatchOperationsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  selectedCount: number;
  activeTaskEvents: BatchTaskEvent[];
  diagnosticResults: ProfileProxyDiagnosticResult[];
  isRunning: boolean;
  capabilities?: WindowLayoutCapabilities;
  onLaunch: (options: BatchLaunchOptions) => Promise<void>;
  onStop: () => Promise<void>;
  onArrange: (options: WindowLayoutOptions) => Promise<void>;
  onDiagnose: () => Promise<void>;
}
```

Use existing components from `src/components/ui/` where available:

- `Dialog`
- `Button`
- `Select`
- `Input`
- `Checkbox`
- `Table`

Do not hardcode visible English strings; all copy must use `useTranslation()`.

- [ ] **Step 3: Keep controls compact**

Use:

- Numeric input for concurrency.
- Numeric input for launch interval seconds.
- Select for failure policy.
- Select for post-launch action.
- Select for layout mode.
- Select or simple buttons for gap values.
- Checkbox for preserve aspect ratio.

- [ ] **Step 4: Run frontend lint/typecheck**

Run:

```bash
pnpm lint:js
```

Expected: PASS.

- [ ] **Step 5: Commit**

Run:

```bash
git add src/components/batch-operations-dialog.tsx src/i18n/locales
git commit -m "feat: add batch operations dialog"
```

### Task 9: Wire Dialog State in Main Page

**Files:**

- Modify: `src/app/page.tsx`
- Modify: `src/lib/backend-errors.ts`
- Modify: `src/i18n/locales/*.json`

- [ ] **Step 1: Add backend error translations**

Add codes to `BackendErrorCode`:

```ts
| "BATCH_NO_PROFILES_SELECTED"
| "BATCH_PROFILE_NOT_FOUND"
| "BATCH_LAUNCH_FAILED"
| "WINDOW_LAYOUT_UNSUPPORTED_PLATFORM"
| "WINDOW_LAYOUT_NO_RUNNING_WINDOWS"
| "PROXY_DIAGNOSTIC_TIMEOUT"
| "PROXY_DIAGNOSTIC_SOURCE_FAILED"
```

Add switch cases using `t("backendErrors...")`.

Add locale keys for all supported locales:

```json
{
  "backendErrors": {
    "batchNoProfilesSelected": "Select at least one profile.",
    "batchProfileNotFound": "One of the selected profiles no longer exists.",
    "batchLaunchFailed": "Failed to launch one or more profiles.",
    "windowLayoutUnsupportedPlatform": "Window arrangement is not supported on this platform yet.",
    "windowLayoutNoRunningWindows": "No running profile windows were found.",
    "proxyDiagnosticTimeout": "Proxy diagnostic timed out.",
    "proxyDiagnosticSourceFailed": "Proxy diagnostic source failed."
  }
}
```

- [ ] **Step 2: Import dialog and hook in `page.tsx`**

Add imports:

```ts
import { BatchOperationsDialog } from "@/components/batch-operations-dialog";
import { useBatchTaskEvents } from "@/hooks/use-batch-task-events";
import type {
  BatchLaunchOptions,
  ProfileProxyDiagnosticResult,
  WindowLayoutCapabilities,
  WindowLayoutOptions,
} from "@/types";
```

- [ ] **Step 3: Add page state**

Near other dialog state:

```ts
const [batchOperationsDialogOpen, setBatchOperationsDialogOpen] =
  useState(false);
const [activeBatchTaskId, setActiveBatchTaskId] = useState<string | null>(null);
const [batchDiagnosticResults, setBatchDiagnosticResults] = useState<
  ProfileProxyDiagnosticResult[]
>([]);
const [batchActionRunning, setBatchActionRunning] = useState(false);
const [windowLayoutCapabilities, setWindowLayoutCapabilities] =
  useState<WindowLayoutCapabilities | undefined>(undefined);
const { eventsByTask } = useBatchTaskEvents();
```

- [ ] **Step 4: Load window layout capabilities**

Add an effect:

```ts
useEffect(() => {
  void (async () => {
    try {
      const capabilities =
        await invoke<WindowLayoutCapabilities>("get_window_layout_capabilities");
      setWindowLayoutCapabilities(capabilities);
    } catch (error) {
      console.error("Failed to load window layout capabilities:", error);
    }
  })();
}, []);
```

- [ ] **Step 5: Add handlers**

Add handlers:

```ts
const handleOpenBatchOperations = useCallback(() => {
  if (selectedProfiles.length === 0) return;
  setBatchOperationsDialogOpen(true);
}, [selectedProfiles.length]);

const handleBatchLaunch = useCallback(
  async (options: BatchLaunchOptions) => {
    if (selectedProfiles.length === 0) return;
    setBatchActionRunning(true);
    try {
      const taskId = await invoke<string>("batch_launch_profiles", {
        profileIds: selectedProfiles,
        options,
      });
      setActiveBatchTaskId(taskId);
    } catch (error) {
      showErrorToast(translateBackendError(t, error));
    } finally {
      setBatchActionRunning(false);
    }
  },
  [selectedProfiles, t],
);
```

Add equivalent handlers for:

- `batch_stop_profiles`
- `arrange_running_profile_windows`
- `diagnose_profile_proxies`

For diagnostics, store returned `ProfileProxyDiagnosticResult[]` in `batchDiagnosticResults`.

- [ ] **Step 6: Render dialog**

Near other dialogs:

```tsx
<BatchOperationsDialog
  open={batchOperationsDialogOpen}
  onOpenChange={setBatchOperationsDialogOpen}
  selectedCount={selectedProfiles.length}
  activeTaskEvents={
    activeBatchTaskId
      ? Object.values(eventsByTask[activeBatchTaskId] ?? {})
      : []
  }
  diagnosticResults={batchDiagnosticResults}
  isRunning={batchActionRunning}
  capabilities={windowLayoutCapabilities}
  onLaunch={handleBatchLaunch}
  onStop={handleBatchStop}
  onArrange={handleBatchArrange}
  onDiagnose={handleBatchDiagnose}
/>
```

- [ ] **Step 7: Run lint/typecheck**

Run:

```bash
pnpm lint:js
```

Expected: PASS.

- [ ] **Step 8: Commit**

Run:

```bash
git add src/app/page.tsx src/lib/backend-errors.ts src/i18n/locales
git commit -m "feat: wire batch operations page state"
```

---

## Chunk 4: Profile Table Entry Points

### Task 10: Add Batch Action-Bar Buttons

**Files:**

- Modify: `src/components/profile-data-table.tsx`
- Modify: `src/app/page.tsx`
- Modify: `src/i18n/locales/*.json`

- [ ] **Step 1: Add props to `ProfilesDataTableProps`**

Add:

```ts
onOpenBatchOperations?: () => void;
onBatchLaunchSelected?: () => void;
onBatchStopSelected?: () => void;
onBatchArrangeSelected?: () => void;
onBatchDiagnoseSelected?: () => void;
```

Thread them into `ProfilesDataTable`.

- [ ] **Step 2: Add action-bar buttons**

In the existing `<DataTableActionBar>` near `profile-data-table.tsx:3095`, add buttons using lucide icons:

- `LuPlay` for launch selected.
- `LuSquare` for stop selected.
- Use `LuPanelTop` or another existing lucide/react-icons layout icon if available for arrange.
- Use `FiWifi` for diagnose proxies.

Use tooltip strings from i18n:

```json
{
  "profiles": {
    "actionBar": {
      "launchSelected": "Launch selected",
      "stopSelected": "Stop selected",
      "arrangeWindows": "Arrange windows",
      "diagnoseProxies": "Diagnose proxies",
      "batchOperations": "Batch operations"
    }
  }
}
```

- [ ] **Step 3: Disable unsafe actions**

Disable:

- Launch selected if no selected profiles.
- Stop selected if no selected profiles.
- Arrange selected if window layout capability is unsupported.
- Diagnose selected if no selected profiles.

Do not silently clear selection after batch launch/stop; users need to see results.

- [ ] **Step 4: Wire props from `page.tsx`**

In the `<ProfilesDataTable>` call around `src/app/page.tsx:1546`, pass:

```tsx
onOpenBatchOperations={handleOpenBatchOperations}
onBatchLaunchSelected={() => void handleBatchLaunch(defaultBatchLaunchOptions)}
onBatchStopSelected={() => void handleBatchStop()}
onBatchArrangeSelected={() =>
  void handleBatchArrange(defaultWindowLayoutOptions)
}
onBatchDiagnoseSelected={() => void handleBatchDiagnose()}
```

Define `defaultBatchLaunchOptions` and `defaultWindowLayoutOptions` with `useMemo`.

- [ ] **Step 5: Run lint/typecheck**

Run:

```bash
pnpm lint:js
```

Expected: PASS.

- [ ] **Step 6: Commit**

Run:

```bash
git add src/components/profile-data-table.tsx src/app/page.tsx src/i18n/locales
git commit -m "feat: add batch profile actions"
```

---

## Chunk 5: Final Validation and Review

### Task 11: Run Full Validation

**Files:**

- No planned edits unless validation fails.

- [ ] **Step 1: Format**

Run:

```bash
pnpm format
```

Expected: completes successfully. Review formatting changes before committing.

- [ ] **Step 2: Lint**

Run:

```bash
pnpm lint
```

Expected: PASS.

- [ ] **Step 3: Test**

Run:

```bash
pnpm test
```

Expected: PASS. If output is too noisy, use:

```bash
pnpm test 2>&1 | Select-String -Pattern "test result|panicked|FAILED"
```

- [ ] **Step 4: Check unused commands**

Run:

```bash
pnpm check-unused-commands
```

Expected: PASS. New commands must be used by frontend or intentionally allowlisted with a comment explaining why.

- [ ] **Step 5: GitNexus detect changes**

Use GitNexus:

```text
detect_changes scope=all repo=donutbrowser
```

Expected: affected flows match batch/profile/proxy/window work. Investigate unexpected affected flows before final commit.

- [ ] **Step 6: Review diff**

Run:

```bash
git status --short
git diff --stat
git diff
```

Expected:

- No unrelated `AGENTS.md` changes included unless the user explicitly asked.
- No copied XChrome source.
- All user-facing strings translated through i18n.
- All backend UI errors are structured JSON codes.

- [ ] **Step 7: Final commit**

Run:

```bash
git add src-tauri/src src-tauri/Cargo.toml src app package.json pnpm-lock.yaml
git commit -m "feat: add phase 1 batch browser workflows"
```

Only include files actually changed by the implementation.

### Task 12: Manual Smoke Test

**Files:**

- No planned edits unless smoke test finds bugs.

- [ ] **Step 1: Start dev app**

Run:

```bash
pnpm tauri dev
```

Expected: Donut Browser dev app starts.

- [ ] **Step 2: Test batch launch**

In the UI:

1. Select two stopped profiles.
2. Click batch launch.
3. Confirm events appear in the batch results.
4. Confirm each profile reaches running/completed or a clear failure state.

- [ ] **Step 3: Test batch stop**

In the UI:

1. Select the same running profiles.
2. Click batch stop.
3. Confirm profiles stop and result statuses update.

- [ ] **Step 4: Test proxy diagnostics**

In the UI:

1. Select profiles with and without proxies.
2. Run proxy diagnostics.
3. Confirm profiles without proxies show a localized failure.
4. Confirm valid proxies show IP/country/source/latency.

- [ ] **Step 5: Test window arrangement on Windows**

On Windows:

1. Launch at least two profiles.
2. Select them.
3. Run arrange windows.
4. Confirm windows tile inside the primary display work area.

On non-Windows:

1. Click arrange windows.
2. Confirm the UI shows the localized unsupported-platform error.

- [ ] **Step 6: Stop dev app**

Stop the dev server/session cleanly before final response.

---

## Implementation Notes

- Keep Phase 1 sequential first. Add bounded concurrency only after basic launch/stop events are working.
- Do not mutate profile metadata for batch operations unless the existing launch/stop path already does so.
- Avoid raw `format!("Failed to ...")` errors for new user-facing Tauri commands. Return JSON error codes.
- Do not add visible instructional text that explains how to use the UI. Use labels, controls, and tooltips.
- Keep UI compact; this is an operational tool, not a landing page.
- Because `AGENTS.md` is already modified in the working tree, do not stage it unless the user explicitly requests it.
