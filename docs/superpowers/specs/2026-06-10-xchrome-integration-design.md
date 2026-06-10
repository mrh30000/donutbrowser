# XChrome Integration Design

Date: 2026-06-10

## Summary

Integrate the useful workflow ideas from XChrome into Donut Browser without copying
XChrome source code. Donut Browser remains a Tauri/Rust and Next.js application
using its existing profiles, groups, proxies, Wayfern fingerprinting, API, MCP,
and sync foundations.

The integration is phased. Phase 1 focuses on capabilities that are valuable,
low-coupling, and visible quickly: batch profile launch/stop, group-based batch
operations, Windows window arrangement, proxy diagnostics, and a batch task result
panel. Later phases add batch profile creation, automation APIs, script execution,
advanced fingerprint editing, and cross-platform window arrangement.

## Constraints

- XChrome is licensed under CC BY-NC 4.0. Donut Browser is AGPL-3.0. The
  implementation must not copy XChrome source code into Donut Browser.
- XChrome ideas should be reimplemented from behavior and product requirements,
  using Donut Browser's architecture and coding standards.
- User-facing strings must use Donut Browser's i18n system and be added to all
  locale files.
- Rust commands that can surface user-facing errors must return Donut Browser
  backend error JSON codes, not raw English strings.
- Phase 1 should not introduce a separate "XChrome mode" or duplicate Donut's
  existing profile, group, and proxy management surfaces.

## Goals

- Add batch workflows that make profile operations efficient at scale.
- Add XChrome-style window arrangement for running profiles, starting with
  Windows.
- Improve proxy troubleshooting with multi-source diagnostics.
- Preserve Donut Browser's existing single-profile workflows.
- Keep each new subsystem isolated behind small, testable modules.

## Non-Goals

- Do not copy XChrome C# code, WPF UI, database schema, or embedded scripts.
- Do not implement synchronized click or keyboard group control in Phase 1.
- Do not change Wayfern's fingerprinting internals in Phase 1.
- Do not directly import XChrome's database in Phase 1.
- Do not add a full script marketplace or script manager in Phase 1.

## Recommended Approach

Use an incremental integration path:

1. Phase 1 fills Donut Browser's current operational gaps: batch launch/stop,
   window arrangement, and proxy diagnostics.
2. Phase 2 expands the automation surface: batch creation, batch API/MCP
   commands, and simple scripts.
3. Phase 3 adds advanced fingerprint editing, version-selection improvements,
   cross-platform window arrangement, and optional import tooling.

This avoids a parallel product area and lets the new workflows reuse Donut's
existing profile table, group filtering, proxy management, and browser runner.

## Frontend Design

Add batch operation entry points to the existing interface.

### Profile List Toolbar

When multiple profiles are selected, expose batch actions:

- Batch launch
- Batch stop
- Arrange windows
- Diagnose proxies
- Existing bulk assignment actions for groups, proxies, and extension groups

### Group Context

For the current selected group, provide group-level actions:

- Launch group
- Stop group
- Diagnose group proxies
- Launch and arrange

### Batch Operations Dialog

Add a focused dialog or sub-page dialog for configuring a batch task.

Configuration fields:

- Target: selected profiles, current group, or all profiles
- Concurrency: default 3
- Launch interval: default 1 second
- Failure policy: continue or stop on first error
- Post-launch action: none, arrange windows, or diagnose proxies before launch

### Window Arrangement Controls

Initial controls:

- Layout mode: automatic grid, horizontal tile, vertical tile
- Display target: primary display or selected display
- Gap: 0, 8, or 16 px
- Preserve aspect ratio: enabled or disabled

Phase 1 does not need a visual preview. Users can apply a layout and rerun
arrangement if needed.

### Proxy Diagnostics Results

Show a per-profile result table with:

- Profile name
- Proxy name
- Exit IP
- Country or region
- Detection source
- Latency
- Error code or failure reason

## Backend Design

Add isolated Rust modules rather than placing all logic in existing large files.

### `window_layout.rs`

Responsibilities:

- Resolve running profile windows.
- Read display work areas.
- Calculate grid, horizontal, and vertical layouts.
- Move and resize windows.
- Report platform capabilities.

Phase 1 support:

- Windows implementation.
- macOS/Linux return explicit unsupported capability responses until implemented.

### `batch_runner.rs`

Responsibilities:

- Accept profile IDs and batch options.
- Read profile configuration through existing profile managers.
- Call the existing launch/stop path.
- Apply concurrency and launch interval limits.
- Emit per-profile task status events.
- Respect failure policy.

The module should not create a second profile store or bypass existing launch
validation.

### `proxy_diagnostics.rs`

Responsibilities:

- Reuse existing proxy parsing and connection test logic where possible.
- Query multiple detection sources.
- Normalize output into one result model.
- Apply per-source timeout and total timeout.
- Return structured error codes.

Initial detection sources:

- `ip-api`
- `ip.sb`
- `ipapi.co`

## Tauri Commands

Phase 1 commands:

- `batch_launch_profiles(profile_ids, options)`
- `batch_stop_profiles(profile_ids)`
- `arrange_running_profile_windows(profile_ids, options)`
- `diagnose_profile_proxies(profile_ids, options)`
- `get_window_layout_capabilities()`

Commands should validate profile IDs, option bounds, and platform support before
starting work.

## Events

Batch work should report status through Tauri events keyed by task ID.

Per-profile states:

- `queued`
- `launching`
- `running`
- `failed`
- `stopped`
- `arranging`
- `completed`

The frontend aggregates these events in the batch result panel.

## API and MCP

REST API and MCP expansion should wait until Phase 2, after Phase 1 behavior and
types are stable.

Phase 2 API/MCP candidates:

- Batch launch
- Batch stop
- Proxy diagnostics
- Script automation
- Batch profile creation

## Script Automation

Phase 2 should provide a small automation model before any full script manager.

Initial actions:

- Open URL
- Execute JavaScript
- Wait for a duration
- Take screenshot
- Close profile

Automation should run through existing browser debugging or automation surfaces
and must respect Donut Browser's commercial-license and entitlement checks where
they already apply.

## Advanced Fingerprint Editing

Phase 3 should add UI for editing fingerprint details already represented in
Donut Browser's Wayfern and Camoufox profile configuration types.

Candidate fields:

- Timezone
- Language and languages
- Screen and window dimensions
- WebGL vendor and renderer
- Canvas seed
- Font list
- Geolocation

Phase 3 should avoid changing low-level fingerprint engine behavior unless a
specific gap is identified and separately designed.

## Error Handling

Backend errors that can reach the UI must use JSON error codes.

Examples:

- `BATCH_NO_PROFILES_SELECTED`
- `BATCH_PROFILE_NOT_FOUND`
- `BATCH_LAUNCH_FAILED`
- `WINDOW_LAYOUT_UNSUPPORTED_PLATFORM`
- `WINDOW_LAYOUT_NO_RUNNING_WINDOWS`
- `PROXY_DIAGNOSTIC_TIMEOUT`
- `PROXY_DIAGNOSTIC_SOURCE_FAILED`

Each new error code requires:

- Rust JSON error emission.
- `BackendErrorCode` update.
- `translateBackendError` switch case.
- Locale entries for all supported languages.

## Testing Plan

### Rust Unit Tests

- Layout calculation for automatic grid, horizontal tile, and vertical tile.
- Bounds checking for concurrency and launch interval options.
- Failure policy behavior in batch queue logic.
- Proxy diagnostic response parsing.

### Rust Integration Tests

- Tauri command parameter validation.
- JSON backend error format.
- Unsupported platform responses for window layout.

### Frontend Tests

- Batch operations dialog renders selected target and options.
- Batch result panel updates for queued, running, failed, and completed states.
- Proxy diagnostic table renders success and failure rows.

### E2E Tests

- Select multiple profiles and start a batch launch.
- Confirm task status updates appear.
- Confirm existing single-profile launch still works.
- On Windows, verify arrange windows command reports success for running
  profile windows.

## Phasing

### Phase 1

- Batch launch and stop.
- Group-based batch launch and stop.
- Windows window arrangement.
- Proxy diagnostics.
- Batch task result panel.

### Phase 2

- Batch profile creation.
- Batch proxy import and profile assignment.
- REST API and MCP batch commands.
- Simple script automation actions.

### Phase 3

- Advanced fingerprint editor.
- Browser version selection improvements.
- Cross-platform window arrangement.
- Automation templates.
- Optional XChrome export-format importer.

## Implementation Defaults

- Group-level quick actions should be added near the existing group navigation
  and profile filtering controls, reusing the current page layout instead of
  adding a separate top-level mode.
- Windows window discovery should start from Donut's existing running-profile
  launch metadata and process IDs. If a direct process-to-window mapping is not
  reliable, the Windows implementation should add a small internal mapping from
  launched profile ID to discovered top-level browser window.
- API/MCP command shapes are intentionally deferred to Phase 2. Phase 1 must keep
  the Rust option and result structs stable enough to reuse from API/MCP later.
