use crate::enums;
use crate::log_error;
use crate::models;
use crate::services;
use crate::utils;
use hardviz_core::settings::CoreSettings;
use tauri_plugin_opener::OpenerExt;

#[derive(Debug)]
pub struct AppState {
  pub settings: std::sync::Mutex<models::settings::Settings>,
  /// Core-owned settings. Loaded from the
  /// same `settings.json` as the App-side `Settings`, but each side only
  /// (de)serializes its own keys so writes don't clobber each other.
  pub core_settings: std::sync::Mutex<CoreSettings>,
}

impl AppState {
  pub fn new() -> Self {
    let settings_path =
      utils::file::get_app_data_dir(services::settings_service::SETTINGS_FILENAME);
    let mut core_settings =
      CoreSettings::load_from_path(&settings_path).unwrap_or_else(|e| {
        log_error!(
          "Failed to load core settings",
          "AppState::new",
          Some(e.to_string())
        );
        CoreSettings::default()
      });
    match core_settings.ensure_storage_health_identity_key() {
      Ok(true) => {
        if let Err(e) = core_settings.save_to_path(&settings_path) {
          log_error!(
            "Failed to save storage health identity key",
            "AppState::new",
            Some(e.to_string())
          );
          core_settings.storage_health_identity.hash_key.clear();
        }
      }
      Ok(false) => {}
      Err(e) => {
        log_error!(
          "Invalid storage health identity key",
          "AppState::new",
          Some(e.to_string())
        );
      }
    }

    Self {
      settings: std::sync::Mutex::from(models::settings::Settings::new()),
      core_settings: std::sync::Mutex::from(core_settings),
    }
  }
}

pub mod commands {

  use super::*;
  use crate::services::external_component_guidance_service::{
    ExternalComponentGuidanceState, normalize_external_component_guidance_key,
  };
  use crate::tray::widget::TrayWidgetSettings;
  use serde_json::json;
  use std::sync::Arc;
  use tauri::{Emitter, EventTarget, Manager, Window};

  const ERROR_TITLE: &str = "Failed to update settings file";

  ///
  /// ## Emit error event to notify frontend
  ///
  fn emit_error(window: &Window) -> Result<(), String> {
    let settings_json_path =
      utils::file::get_app_data_dir(services::settings_service::SETTINGS_FILENAME);

    log_error!(
      "Failed to update settings file",
      "settings.rs",
      Some(settings_json_path.display().to_string())
    );

    window
      .emit_to(
        EventTarget::window(window.label().to_string()),
        "error_event",
        json!({
            "title": ERROR_TITLE,
            "message": format!("If this happens repeatedly, delete {settings_json_path} and restart the app.", settings_json_path = settings_json_path.display())
        }),
      )
      .map_err(|e| format!("Failed to emit event: {e}"))?;

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn get_settings(
    state: tauri::State<'_, AppState>,
  ) -> Result<models::settings::ClientSettings, String> {
    let settings = state.settings.lock().unwrap().clone();
    let core_settings = state.core_settings.lock().unwrap().clone();

    // Convert to comma-separated string for easier handling in frontend
    let color_strings = models::settings::LineGraphColorStringSettings {
      cpu: settings
        .line_graph_color
        .cpu
        .iter()
        .map(|&c| c.to_string())
        .collect::<Vec<String>>()
        .join(","),
      memory: settings
        .line_graph_color
        .memory
        .iter()
        .map(|&c| c.to_string())
        .collect::<Vec<String>>()
        .join(","),
      gpu: settings
        .line_graph_color
        .gpu
        .iter()
        .map(|&c| c.to_string())
        .collect::<Vec<String>>()
        .join(","),
    };

    let client_settings = models::settings::ClientSettings {
      version: settings.version,
      language: settings.language,
      theme: settings.theme,
      navigation_layout: settings.navigation_layout,
      last_acknowledged_announcement: settings.last_acknowledged_announcement,
      display_targets: settings.display_targets,
      graph_size: settings.graph_size,
      graph_fit_to_window: settings.graph_fit_to_window,
      graph_margin_px: settings.graph_margin_px,
      line_graph_type: settings.line_graph_type,
      line_graph_border: settings.line_graph_border,
      line_graph_fill: settings.line_graph_fill,
      line_graph_color: color_strings,
      line_graph_mix: settings.line_graph_mix,
      line_graph_show_legend: settings.line_graph_show_legend,
      line_graph_show_scale: settings.line_graph_show_scale,
      line_graph_show_tooltip: settings.line_graph_show_tooltip,
      background_img_opacity: settings.background_img_opacity,
      selected_background_img: settings.selected_background_img,
      transparent_ui: settings.transparent_ui,
      window_opacity: settings.window_opacity,
      glass_blur: settings.glass_blur,
      temperature_unit: settings.temperature_unit,
      hardware_archive: core_settings.hardware_archive.into(),
      storage_health: core_settings.storage_health.into(),
      burn_in_shift: settings.burn_in_shift,
      burn_in_shift_mode: settings.burn_in_shift_mode,
      burn_in_shift_preset: settings.burn_in_shift_preset,
      burn_in_shift_idle_only: settings.burn_in_shift_idle_only,
      burn_in_shift_options: settings.burn_in_shift_options,
      text_selectable: settings.text_selectable,
      close_to_tray: settings.close_to_tray,
      close_to_tray_choice_made: settings.close_to_tray_choice_made,
      external_component_guidance: settings.external_component_guidance,
      elevated_startup_mode: settings.elevated_startup_mode,
      tray_widget: settings.tray_widget.normalized(),
    };

    Ok(client_settings)
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_language(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_language: String,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_language(new_language) {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_theme(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_theme: enums::settings::Theme,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_theme(new_theme) {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_navigation_layout(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_layout: enums::settings::NavigationLayout,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_navigation_layout(new_layout) {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn acknowledge_navigation_restructure_announcement(
    window: Window,
    state: tauri::State<'_, AppState>,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.acknowledge_navigation_restructure_announcement() {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_display_targets(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_targets: Vec<enums::hardware::HardwareType>,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_display_targets(new_targets) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_graph_size(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_size: enums::settings::GraphSize,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_graph_size(new_size) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_graph_fit_to_window(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_graph_fit_to_window(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_graph_margin_px(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: u32,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_graph_margin_px(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_type(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_type: enums::settings::LineGraphType,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_type(new_type) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_border(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_border(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_fill(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_fill(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_color(
    window: Window,
    state: tauri::State<'_, AppState>,
    target: enums::hardware::HardwareType,
    new_color: String,
  ) -> Result<String, String> {
    let mut settings = state.settings.lock().unwrap();

    match settings.set_line_graph_color(target, new_color) {
      Ok(result) => Ok(result),
      Err(e) => {
        emit_error(&window)?;
        Err(e)
      }
    }
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_mix(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_mix(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_show_legend(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_show_legend(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_show_scale(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_show_scale(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_line_graph_show_tooltip(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_line_graph_show_tooltip(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_background_img_opacity(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: u8,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_background_img_opacity(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_selected_background_img(
    window: Window,
    state: tauri::State<'_, AppState>,
    file_id: Option<String>,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_selected_background_img(file_id) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  /// Apply (or clear) native macOS window vibrancy.
  ///
  /// When enabled, installs an `NSVisualEffectView` behind the (transparent)
  /// webview so the frosted-glass look is composited by the OS instead of the
  /// CSS `backdrop-filter: blur`, which was measured to continuously pin the
  /// WebKit render/GPU processes on macOS (see #1718). No-op elsewhere.
  #[cfg(target_os = "macos")]
  pub fn apply_window_vibrancy(app: &tauri::AppHandle, glass_blur: u8) {
    // window-vibrancy's apply/clear touch AppKit and MUST run on the main
    // thread, but async Tauri commands run on a worker thread, so dispatch to
    // the main thread (otherwise it fails with NotMainThread). Tauri's own
    // `set_effects(None)` also does not reliably remove the NSVisualEffectView,
    // so we use window-vibrancy's explicit apply/clear instead.
    //
    // glass_blur doubles as a frost-intensity level on macOS:
    //   0 = off (clear), 1..=10 = low, 11..=20 = medium, 21.. = high.
    let app_handle = app.clone();
    let _ = app.run_on_main_thread(move || {
      use window_vibrancy::{NSVisualEffectMaterial, apply_vibrancy, clear_vibrancy};

      let Some(window) = app_handle.get_webview_window("main") else {
        return;
      };

      // apply_vibrancy ADDS an NSVisualEffectView (it does not replace an
      // existing one), so switching materials just stacks views and the change
      // is not visible. Remove any existing effect views first.
      while clear_vibrancy(&window).unwrap_or(false) {}

      // Levels go weak -> strong as glass_blur increases. Materials are chosen
      // by appearance (HudWindow reads lightest / most see-through, Sheet the
      // most frosted); swap these if the feel is off.
      let result = match glass_blur {
        0 => Ok(()),
        1..=10 => apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None),
        11..=20 => apply_vibrancy(&window, NSVisualEffectMaterial::Menu, None, None),
        _ => apply_vibrancy(
          &window,
          NSVisualEffectMaterial::UnderWindowBackground,
          None,
          None,
        ),
      };

      if let Err(e) = result {
        log_error!(
          "Failed to update window vibrancy",
          "settings::apply_window_vibrancy",
          Some(format!("{e:?}"))
        );
      }
    });
  }

  /// No-op on non-macOS platforms (native vibrancy is a macOS feature).
  #[cfg(not(target_os = "macos"))]
  pub fn apply_window_vibrancy(_app: &tauri::AppHandle, _glass_blur: u8) {}

  #[tauri::command]
  #[specta::specta]
  pub async fn set_transparent_ui(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let frost_level;
    {
      let mut settings = state.settings.lock().unwrap();

      if let Err(e) = settings.set_transparent_ui(new_value) {
        emit_error(&window)?;
        return Err(e);
      }
      // Frost intensity follows glass_blur, but only while transparency is on.
      frost_level = if new_value { settings.glass_blur } else { 0 };
    }

    apply_window_vibrancy(window.app_handle(), frost_level);
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_window_opacity(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: u8,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_window_opacity(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_glass_blur(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: u8,
  ) -> Result<(), String> {
    let frost_level;
    {
      let mut settings = state.settings.lock().unwrap();

      if let Err(e) = settings.set_glass_blur(new_value) {
        emit_error(&window)?;
        return Err(e);
      }
      // Frost intensity follows glass_blur, but only while transparency is on.
      frost_level = if settings.transparent_ui {
        new_value
      } else {
        0
      };
    }

    apply_window_vibrancy(window.app_handle(), frost_level);
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_temperature_unit(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_unit: enums::settings::TemperatureUnit,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_temperature_unit(new_unit) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  /// Apply a mutation to `CoreSettings` and persist via the Core
  /// storage helper (which merges into the shared `settings.json` so
  /// App-owned keys are preserved).
  ///
  /// The mutation is applied to a clone first; the in-memory state is
  /// only swapped in after the on-disk write succeeds. This keeps the
  /// process from advertising a value the disk hasn't accepted (e.g.
  /// when the existing file is corrupted and `save_to_path` refuses to
  /// overwrite it).
  fn update_core_settings<F>(
    state: &tauri::State<'_, AppState>,
    mutate: F,
  ) -> Result<(), String>
  where
    F: FnOnce(&mut CoreSettings),
  {
    let mut core_settings = state.core_settings.lock().unwrap();
    let mut next = core_settings.clone();
    mutate(&mut next);
    let path =
      utils::file::get_app_data_dir(services::settings_service::SETTINGS_FILENAME);
    next.save_to_path(&path)?;
    *core_settings = next;
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_hardware_archive_enabled(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    if let Err(e) =
      update_core_settings(&state, |s| s.hardware_archive.enabled = new_value)
    {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_hardware_archive_retention_days(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_retention_days: u32,
  ) -> Result<(), String> {
    // Reject 0: zero retention would mean "delete every record
    // immediately" via the scheduled cleanup path.
    if new_retention_days == 0 {
      return Err("hardware archive retention days must be greater than 0".to_string());
    }
    if let Err(e) = update_core_settings(&state, |s| {
      s.hardware_archive.retention_days = new_retention_days
    }) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_hardware_archive_scheduled_data_deletion(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    if let Err(e) = update_core_settings(&state, |s| {
      s.hardware_archive.scheduled_data_deletion = new_value
    }) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_storage_health_retention_days(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_retention_days: u32,
  ) -> Result<(), String> {
    if new_retention_days == 0 {
      return Err("storage health retention days must be greater than 0".to_string());
    }
    if let Err(e) = update_core_settings(&state, |s| {
      s.storage_health.retention_days = new_retention_days
    }) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_burn_in_shift(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_burn_in_shift(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_burn_in_shift_mode(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: enums::settings::BurnInShiftMode,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_burn_in_shift_mode(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_burn_in_shift_preset(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: enums::settings::BurnInShiftPreset,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_burn_in_shift_preset(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_burn_in_shift_idle_only(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_burn_in_shift_idle_only(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_burn_in_shift_options(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: Option<models::settings::BurnInShiftOptions>,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_burn_in_shift_options(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_text_selectable(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_text_selectable(new_value) {
      emit_error(&window)?;
      return Err(e);
    }
    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_tray_widget_settings(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: TrayWidgetSettings,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_tray_widget_settings(new_value) {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_close_to_tray_preference(
    window: Window,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.set_close_to_tray_preference(new_value) {
      emit_error(&window)?;
      return Err(e);
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn acknowledge_external_component_guidance_key(
    window: Window,
    state: tauri::State<'_, AppState>,
    guidance_state: tauri::State<'_, Arc<ExternalComponentGuidanceState>>,
    key: String,
  ) -> Result<(), String> {
    let key = normalize_external_component_guidance_key(&key)?;
    let mut settings = state.settings.lock().unwrap();

    if let Err(e) = settings.acknowledge_external_component_guidance_key(key.clone()) {
      emit_error(&window)?;
      return Err(e);
    }

    guidance_state.remove_key(&key);

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn set_elevated_startup_mode(
    window: Window,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    new_value: bool,
  ) -> Result<(), String> {
    {
      let mut settings = state.settings.lock().unwrap();

      if let Err(e) = settings.set_elevated_startup_mode(new_value) {
        emit_error(&window)?;
        return Err(e);
      }
    }

    if new_value {
      let is_elevated = match crate::services::system_service::is_process_elevated() {
        Ok(is_elevated) => is_elevated,
        Err(e) => {
          let mut settings = state.settings.lock().unwrap();
          if let Err(save_err) = settings.set_elevated_startup_mode(false) {
            emit_error(&window)?;
            return Err(format!(
              "Failed to check administrator status: {e}; additionally failed to restore setting: {save_err}"
            ));
          }
          return Err(format!("Failed to check administrator status: {e}"));
        }
      };

      if !is_elevated {
        match crate::services::system_service::restart_app_elevated(&app_handle).await {
          Ok(()) => {}
          Err(e) => {
            let mut settings = state.settings.lock().unwrap();
            if let Err(save_err) = settings.set_elevated_startup_mode(false) {
              emit_error(&window)?;
              return Err(format!(
                "{e}; additionally failed to restore setting: {save_err}"
              ));
            }
            return Err(e);
          }
        }
      }
    }

    Ok(())
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn read_license_file(app: tauri::AppHandle) -> Result<String, String> {
    let resource_path = app
      .path()
      .resource_dir()
      .map_err(|e| format!("Failed to get resource directory: {}", e))?
      .join("LICENSE");

    std::fs::read_to_string(&resource_path)
      .map_err(|e| format!("Failed to read LICENSE file: {}", e))
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn read_third_party_notices_file(
    app: tauri::AppHandle,
  ) -> Result<String, String> {
    let resource_path = app
      .path()
      .resource_dir()
      .map_err(|e| format!("Failed to get resource directory: {}", e))?
      .join("THIRD_PARTY_NOTICES.md");

    std::fs::read_to_string(&resource_path)
      .map_err(|e| format!("Failed to read THIRD_PARTY_NOTICES.md file: {}", e))
  }

  #[tauri::command]
  #[specta::specta]
  pub async fn open_license_file_path(app: tauri::AppHandle) -> Result<(), String> {
    let resource_path = app
      .path()
      .resource_dir()
      .map_err(|e| format!("Failed to get resource directory: {}", e));

    let path_str = resource_path?
      .to_str()
      .ok_or_else(|| "Failed to convert path to string".to_string())?
      .to_string();

    app
      .opener()
      .open_path(path_str, None::<&str>)
      .map_err(|e| format!("Failed to open LICENSE file path: {}", e))
  }
}
