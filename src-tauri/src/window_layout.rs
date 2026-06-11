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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayoutCapabilities {
  pub supported: bool,
  pub platform: String,
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

pub fn get_window_layout_capabilities() -> WindowLayoutCapabilities {
  WindowLayoutCapabilities {
    supported: cfg!(target_os = "windows"),
    platform: std::env::consts::OS.to_string(),
  }
}

pub async fn arrange_running_profile_windows(
  profile_ids: Vec<String>,
  options: WindowLayoutOptions,
) -> Result<(), String> {
  crate::batch_runner::validate_profile_ids(&profile_ids)
    .map_err(|error| error.to_backend_error())?;

  arrange_platform_windows(profile_ids, options).map_err(|error| error.to_backend_error())
}

#[cfg(not(target_os = "windows"))]
fn arrange_platform_windows(
  _profile_ids: Vec<String>,
  _options: WindowLayoutOptions,
) -> Result<(), WindowLayoutError> {
  Err(WindowLayoutError::new("WINDOW_LAYOUT_UNSUPPORTED_PLATFORM"))
}

#[cfg(target_os = "windows")]
fn arrange_platform_windows(
  profile_ids: Vec<String>,
  options: WindowLayoutOptions,
) -> Result<(), WindowLayoutError> {
  use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
  use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetSystemMetrics, GetWindowThreadProcessId, IsWindowVisible, SetWindowPos,
    HWND_TOP, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOZORDER,
  };

  struct EnumContext {
    process_ids: std::collections::HashSet<u32>,
    windows: Vec<HWND>,
  }

  unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let context = &mut *(lparam.0 as *mut EnumContext);
    if !IsWindowVisible(hwnd).as_bool() {
      return BOOL(1);
    }

    let mut process_id = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    if context.process_ids.contains(&process_id) {
      context.windows.push(hwnd);
    }

    BOOL(1)
  }

  let profiles = crate::profile::ProfileManager::instance()
    .list_profiles()
    .map_err(|_| WindowLayoutError::new("WINDOW_LAYOUT_NO_RUNNING_WINDOWS"))?;
  let profiles = crate::batch_runner::profiles_by_ids(profiles, &profile_ids)
    .map_err(|_| WindowLayoutError::new("WINDOW_LAYOUT_NO_RUNNING_WINDOWS"))?;
  let process_ids: std::collections::HashSet<u32> = profiles
    .iter()
    .filter_map(|profile| profile.process_id)
    .collect();

  if process_ids.is_empty() {
    return Err(WindowLayoutError::new("WINDOW_LAYOUT_NO_RUNNING_WINDOWS"));
  }

  let mut context = EnumContext {
    process_ids,
    windows: Vec::new(),
  };

  unsafe {
    let _ = EnumWindows(
      Some(enum_windows_proc),
      LPARAM(&mut context as *mut _ as isize),
    );
  }

  if context.windows.is_empty() {
    return Err(WindowLayoutError::new("WINDOW_LAYOUT_NO_RUNNING_WINDOWS"));
  }

  let work_area = Rect {
    x: 0,
    y: 0,
    width: unsafe { GetSystemMetrics(SM_CXSCREEN) },
    height: unsafe { GetSystemMetrics(SM_CYSCREEN) },
  };
  let layouts = calculate_layouts(context.windows.len(), work_area, options)?;

  for (hwnd, rect) in context.windows.into_iter().zip(layouts) {
    unsafe {
      let _ = SetWindowPos(
        hwnd,
        HWND_TOP,
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        SWP_NOZORDER | SWP_NOACTIVATE,
      );
    }
  }

  Ok(())
}

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
    assert_eq!(
      layouts[0],
      Rect {
        x: 0,
        y: 0,
        width: 400,
        height: 400
      }
    );
    assert_eq!(
      layouts[4],
      Rect {
        x: 400,
        y: 400,
        width: 400,
        height: 400
      }
    );
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

    assert_eq!(
      layouts[0],
      Rect {
        x: 10,
        y: 20,
        width: 303,
        height: 500
      }
    );
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
