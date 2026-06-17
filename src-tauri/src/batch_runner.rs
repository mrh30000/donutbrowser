use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchError {
  code: &'static str,
}

impl BatchError {
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
    log::warn!("Failed to emit batch task event: {err}");
  }
}

pub async fn batch_launch_profiles(
  app_handle: tauri::AppHandle,
  profile_ids: Vec<String>,
  mut options: BatchLaunchOptions,
) -> Result<String, String> {
  validate_profile_ids(&profile_ids).map_err(|error| error.to_backend_error())?;
  options.normalize();

  let task_id = uuid::Uuid::new_v4().to_string();
  let profiles = crate::profile::ProfileManager::instance()
    .list_profiles()
    .map_err(|_| serde_json::json!({ "code": "INTERNAL_ERROR" }).to_string())?;
  let profiles =
    profiles_by_ids(profiles, &profile_ids).map_err(|error| error.to_backend_error())?;

  for profile in &profiles {
    emit_status(&task_id, profile, BatchProfileStatus::Queued, None);
  }

  let concurrency = options.concurrency.max(1);
  let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(concurrency));
  let mut handles: Vec<tokio::task::JoinHandle<Result<(), String>>> =
    Vec::with_capacity(profiles.len());

  for profile in profiles {
    let permit = semaphore
      .clone()
      .acquire_owned()
      .await
      .expect("batch semaphore closed unexpectedly");
    let handle = app_handle.clone();
    let interval = options.launch_interval_ms;
    let task_id_for_spawn = task_id.clone();
    handles.push(tokio::spawn(async move {
      let _permit = permit;
      emit_status(
        &task_id_for_spawn,
        &profile,
        BatchProfileStatus::Launching,
        None,
      );

      let result = crate::browser_runner::launch_browser_profile_impl(
        handle,
        profile.clone(),
        None,
        None,
        false,
        false,
      )
      .await;

      let outcome = match result {
        Ok(updated) => {
          emit_status(
            &task_id_for_spawn,
            &updated,
            BatchProfileStatus::Running,
            None,
          );
          emit_status(
            &task_id_for_spawn,
            &updated,
            BatchProfileStatus::Completed,
            None,
          );
          Ok(())
        }
        Err(error) => {
          emit_status(
            &task_id_for_spawn,
            &profile,
            BatchProfileStatus::Failed,
            Some("BATCH_LAUNCH_FAILED".to_string()),
          );
          Err(error)
        }
      };

      if interval > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
      }
      outcome
    }));
  }

  let mut had_failure = false;
  for handle in handles {
    if let Ok(Err(_)) = handle.await {
      had_failure = true;
    }
  }

  if had_failure && options.failure_policy == FailurePolicy::StopOnFirstError {
    log::warn!("Batch launch stopped after first failure");
  }

  if options.post_launch_action == PostLaunchAction::ArrangeWindows {
    let _ = crate::window_layout::arrange_running_profile_windows(
      profile_ids,
      crate::window_layout::WindowLayoutOptions::default(),
    )
    .await;
  }

  Ok(task_id)
}

pub async fn batch_stop_profiles(
  app_handle: tauri::AppHandle,
  profile_ids: Vec<String>,
) -> Result<String, String> {
  validate_profile_ids(&profile_ids).map_err(|error| error.to_backend_error())?;

  let task_id = uuid::Uuid::new_v4().to_string();
  let profiles = crate::profile::ProfileManager::instance()
    .list_profiles()
    .map_err(|_| serde_json::json!({ "code": "INTERNAL_ERROR" }).to_string())?;
  let profiles =
    profiles_by_ids(profiles, &profile_ids).map_err(|error| error.to_backend_error())?;

  for profile in profiles {
    emit_status(&task_id, &profile, BatchProfileStatus::Queued, None);
    let result =
      crate::browser_runner::kill_browser_profile(app_handle.clone(), profile.clone()).await;
    match result {
      Ok(()) => {
        emit_status(&task_id, &profile, BatchProfileStatus::Stopped, None);
        emit_status(&task_id, &profile, BatchProfileStatus::Completed, None);
      }
      Err(_) => {
        emit_status(
          &task_id,
          &profile,
          BatchProfileStatus::Failed,
          Some("BATCH_STOP_FAILED".to_string()),
        );
      }
    }
  }

  Ok(task_id)
}

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

  #[test]
  fn profiles_by_ids_resolves_requested_order() {
    let first_id = uuid::Uuid::new_v4();
    let second_id = uuid::Uuid::new_v4();
    let profiles = vec![
      crate::profile::BrowserProfile {
        id: first_id,
        name: "First".to_string(),
        ..Default::default()
      },
      crate::profile::BrowserProfile {
        id: second_id,
        name: "Second".to_string(),
        ..Default::default()
      },
    ];

    let resolved =
      profiles_by_ids(profiles, &[second_id.to_string(), first_id.to_string()]).unwrap();

    assert_eq!(resolved[0].name, "Second");
    assert_eq!(resolved[1].name, "First");
  }

  #[test]
  fn profiles_by_ids_rejects_missing_profile() {
    let err = profiles_by_ids(
      vec![crate::profile::BrowserProfile::default()],
      &[uuid::Uuid::new_v4().to_string()],
    )
    .unwrap_err();

    assert_eq!(err.code(), "BATCH_PROFILE_NOT_FOUND");
  }
}
