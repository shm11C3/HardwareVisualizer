// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Re-export the logging macros from `hardviz_core` so existing
// `use crate::{log_internal, log_warn};` sites keep compiling. The macros
// themselves expand to `tracing::*` calls and live in `hardviz_core::utils::logger`.
pub use hardviz_core::{log_debug, log_error, log_info, log_internal, log_warn};

mod adapters;
mod app;
pub mod cli;
mod commands;
mod enums;
mod infrastructure;
mod lifecycle;
mod models;
mod services;
mod tray;
mod utils;
mod webview_memory;
mod workers;

#[cfg(test)]
mod _tests;

use commands::ambient_sensor;
use commands::background_image;
use commands::cooling_insight;
use commands::external_component_guidance;
use commands::external_component_setup;
use commands::hardware;
use commands::settings;
use commands::system;
use commands::ui;
use commands::updater::app_updates;
use hardviz_core::collector::HistoryStore;
use services::external_component_guidance_service::ExternalComponentGuidanceState;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri::Wry;
use tauri_plugin_autostart::MacosLauncher;
use tauri_specta::{Builder, collect_commands, collect_events};

#[cfg(debug_assertions)]
use specta_typescript::Typescript;

#[cfg(debug_assertions)]
const TYPED_ERROR_IMPL: &str = r#"async function typedError<T, E>(result: Promise<T>): Promise<{ status: "ok"; data: T } | { status: "error"; error: E }> {
    void _assertTypedErrorFollowsContract;
    try {
        return { status: "ok", data: await result };
    } catch (e) {
        return { status: "error", error: e as E };
    }
}
// @ts-expect-error tauri-specta's generated contract assertion leaves E unused under noUnusedLocals.
"#;

/// Apply pending schema migrations against Core's pool, synchronously.
///
/// Runs on a short-lived current-thread runtime because this executes
/// during `run()` setup, before the Tauri (and its Tokio) runtime starts —
/// the same pattern the DB preflight uses. [`db::init`] must have been
/// called first so Core can resolve the database file.
fn apply_pending_migrations() -> Result<(), String> {
  let migrations = infrastructure::database::migration::get_migrations();
  let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .map_err(|e| format!("Failed to build runtime for migrations: {e}"))?;
  runtime.block_on(hardviz_core::infrastructure::database::migrate::run(
    migrations,
  ))
}

/// Tell Core's database dispatch boundary where the two databases and the
/// selection marker live, then make it observe whatever durable selection is
/// already on disk.
///
/// This is App's half of the seam #2134 documents for #2135: App owns path
/// resolution (Core cannot resolve the bundle identifier), so it is the only
/// side that can name [`AuthorityPaths`]. The `reobserve_authority` call
/// covers the ordinary restart case - a database selected in an earlier
/// session must be adopted again this session, or dispatch would silently
/// keep answering from SQLite forever - and is safe to call here because no
/// [`hardviz_core::infrastructure::database::native_database::NativeDatabase`]
/// owner exists yet: this is the first call into the dispatch boundary during
/// App startup, mirroring `db::init` immediately above it.
///
/// Returns `Err` for every durable state where SQLite is not genuinely
/// authoritative and the boundary could not become servable either - a
/// failed native open, or an [`AuthorityState::Inconsistent`] finding this
/// function cannot repair itself. The caller must treat that as a DB-startup
/// failure (`is_db_ok = false`), the same gate an incompatible SQLite schema
/// already uses to keep every database-backed worker from starting: once the
/// durable record says native is or may be selected, silently continuing on
/// SQLite would answer from a backend that is no longer authoritative.
///
/// The one repair attempted here is the single self-healing case
/// [`AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata`] names: the
/// selection committed into the native database but the marker never landed.
/// Rewriting the marker from the database it describes cannot lose history,
/// so it is safe to do without a person deciding. Every other inconsistency
/// is reported rather than guessed at - #2135's `NativeLifecycleOwner` owns
/// the startup authority decision from here; this function only has to
/// surface it, not build a second decision tree.
#[cfg(feature = "duckdb-archive")]
fn initialize_native_database_dispatch(db_path: &std::path::Path) -> Result<(), String> {
  use hardviz_core::infrastructure::database::dispatch;
  use hardviz_core::infrastructure::database::native_database::{
    AuthorityPaths, AuthorityRecovery, AuthorityState, repair_authority_marker,
  };

  let directory = db_path.parent().ok_or_else(|| {
    "native database directory could not be resolved from the SQLite path".to_owned()
  })?;
  let source_file_name = db_path
    .file_name()
    .and_then(|name| name.to_str())
    .ok_or_else(|| "SQLite database path has no file name".to_owned())?;
  let paths =
    AuthorityPaths::in_directory(directory, source_file_name, "hv-database.duckdb");
  // First and only caller during App startup, matching `db::init`'s contract
  // above: we don't care about the return value here.
  let _ = dispatch::init(
    paths.clone(),
    infrastructure::database::native_schema::NATIVE_SCHEMA_VERSION,
  );

  let runtime = tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .map_err(|e| {
      format!("failed to build a runtime to observe native database authority: {e}")
    })?;

  let state = runtime
    .block_on(dispatch::reobserve_authority())
    .map_err(|e| format!("failed to observe native database authority: {e}"))?;
  match state {
    AuthorityState::SqliteAuthoritative
    | AuthorityState::ConversionInProgress { .. }
    | AuthorityState::FinalizedUnselected
    | AuthorityState::NativeSelected => Ok(()),
    AuthorityState::Inconsistent {
      recovery: AuthorityRecovery::RepairSelectionMarkerFromNativeMetadata,
      ..
    } => {
      repair_authority_marker(&paths)
        .map_err(|e| format!("failed to repair the native selection marker: {e}"))?;
      let repaired = runtime
        .block_on(dispatch::reobserve_authority())
        .map_err(|e| format!("failed to re-observe native database authority: {e}"))?;
      if matches!(repaired, AuthorityState::NativeSelected) {
        Ok(())
      } else {
        Err(format!(
          "native database authority remained inconsistent after repairing the marker: {repaired:?}"
        ))
      }
    }
    AuthorityState::Inconsistent { reason, recovery } => Err(format!(
      "native database authority is inconsistent ({reason:?}, recovery: {recovery:?})"
    )),
  }
}

/// If `state` reports the native database as authoritative, synchronously
/// open it before startup decides whether the database is usable at all.
///
/// Runs on a short-lived current-thread runtime, the same pattern
/// [`apply_pending_migrations`] uses: this executes during `run()` setup,
/// before the Tauri (and its Tokio) runtime starts. A failed open must be
/// part of that startup decision, not a detached task's to log afterward
/// while `is_db_ok` and the owner's state have already been decided —
/// ADR 0022 rejects a split topology, so silently continuing to answer
/// from SQLite once a native database has been selected is never an
/// acceptable fallback for an open failure any more than it is for the
/// disagreements [`app::native_lifecycle::inspect_startup_authority`]
/// itself refuses to guess at.
///
/// `native_database_path` and `expected_schema_version` are taken as
/// parameters rather than resolved from
/// `infrastructure::database::native_paths` here, so this function never
/// depends on the real app-data directory and a test can point it at a
/// temporary one.
#[cfg(feature = "duckdb-archive")]
fn open_selected_native_database(
  state: app::native_lifecycle::DatabaseLifecycleState,
  native_database_path: &std::path::Path,
  expected_schema_version: u32,
) -> (
  app::native_lifecycle::DatabaseLifecycleState,
  Option<hardviz_core::infrastructure::database::native_database::NativeDatabase>,
) {
  use app::native_lifecycle::{DatabaseLifecycleState, LifecycleIssue};
  use hardviz_core::infrastructure::database::native_database::{
    NativeDatabase, NativeDatabaseOptions,
  };

  if !matches!(state, DatabaseLifecycleState::NativeAuthoritative) {
    return (state, None);
  }

  let runtime = match tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
  {
    Ok(runtime) => runtime,
    Err(e) => {
      log_error!(
        "failed to build a runtime to open the native database",
        "lib::open_selected_native_database",
        Some(e.to_string())
      );
      return (
        DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed {
          message: e.to_string(),
        }),
        None,
      );
    }
  };

  match runtime.block_on(NativeDatabase::open(
    native_database_path,
    NativeDatabaseOptions::new(expected_schema_version),
  )) {
    Ok(database) => (state, Some(database)),
    Err(e) => {
      log_error!(
        "failed to open the selected native database at startup",
        "lib::open_selected_native_database",
        Some(e.to_string())
      );
      (
        DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed {
          message: e.to_string(),
        }),
        None,
      )
    }
  }
}

#[cfg(all(test, feature = "duckdb-archive"))]
mod open_selected_native_database_tests {
  use app::native_lifecycle::{DatabaseLifecycleState, LifecycleIssue};

  use super::*;

  #[test]
  fn every_state_but_native_authoritative_is_returned_unchanged_without_opening_anything()
  {
    for state in [
      DatabaseLifecycleState::SqliteAuthoritative,
      DatabaseLifecycleState::ConversionRecoverable { resumable: true },
      DatabaseLifecycleState::ConversionRecoverable { resumable: false },
    ] {
      let (result_state, database) = open_selected_native_database(
        state.clone(),
        std::path::Path::new("unused - not NativeAuthoritative"),
        1,
      );
      assert_eq!(result_state, state);
      assert!(database.is_none());
    }
  }

  #[test]
  fn native_authoritative_with_no_file_becomes_action_required_with_nothing_opened() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("hv-database.duckdb");

    let (state, database) = open_selected_native_database(
      DatabaseLifecycleState::NativeAuthoritative,
      &missing,
      1,
    );

    assert!(database.is_none());
    assert!(matches!(
      state,
      DatabaseLifecycleState::ActionRequired(LifecycleIssue::NativeOpenFailed { .. })
    ));
  }

  // Plain `#[test]`, not `#[tokio::test]`: `open_selected_native_database`
  // builds and blocks on its own short-lived runtime — matching what it
  // does during real startup, before the Tauri runtime exists — and tokio
  // panics if that runs on a thread that already has a runtime entered.
  // The async setup below therefore runs its own runtime to completion
  // and drops it before calling the function under test.
  #[test]
  fn a_real_selected_database_opens_and_stays_native_authoritative() {
    // A real conversion end to end, matching the fixture philosophy
    // `app::native_conversion`'s own tests already established: nothing
    // here hand-writes a native file.
    use hardviz_core::infrastructure::database::candidate_database::build_candidate_database;
    use hardviz_core::infrastructure::database::migrate;
    use hardviz_core::infrastructure::database::native_database::{
      AUTHORITY_MARKER_FILE_NAME, AuthorityPaths, finalize_candidate_database,
      reconcile_native_database, select_native_database,
    };
    use sqlx::ConnectOptions;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("hv-database.db");
    let candidate = directory.path().join("candidate.duckdb");
    let native = directory.path().join("hv-database.duckdb");

    tokio::runtime::Runtime::new().unwrap().block_on(async {
      let options = SqliteConnectOptions::new()
        .filename(&source)
        .create_if_missing(true)
        .disable_statement_logging();
      let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
      migrate::run_on_pool(&pool, infrastructure::database::migration::get_migrations())
        .await
        .unwrap();
      pool.close().await;

      build_candidate_database(
        &source,
        &candidate,
        infrastructure::database::migration::get_migrations(),
      )
      .await
      .unwrap();
      finalize_candidate_database(
        &candidate,
        &native,
        infrastructure::database::native_schema::get_native_schema(),
      )
      .await
      .unwrap();
      let (_report, verified) = reconcile_native_database(
        &source,
        &native,
        infrastructure::database::migration::get_migrations(),
        infrastructure::database::native_schema::get_native_schema(),
      )
      .await
      .unwrap();
      select_native_database(
        AuthorityPaths {
          source_database: source.clone(),
          native_database: native.clone(),
          marker: directory.path().join(AUTHORITY_MARKER_FILE_NAME),
        },
        verified,
      )
      .await
      .unwrap();
    });

    let (state, database) = open_selected_native_database(
      DatabaseLifecycleState::NativeAuthoritative,
      &native,
      infrastructure::database::native_schema::NATIVE_SCHEMA_VERSION,
    );

    assert_eq!(state, DatabaseLifecycleState::NativeAuthoritative);
    let database = database.expect("a real selected database must open");
    tokio::runtime::Runtime::new()
      .unwrap()
      .block_on(database.close())
      .unwrap();
  }
}

fn build_specta_builder() -> Builder<Wry> {
  Builder::<Wry>::new()
    .events(collect_events![models::hardware::HardwareMonitorUpdate,])
    .commands(collect_commands![
      app_updates::fetch_update,
      app_updates::install_update,
      hardware::get_process_list,
      hardware::get_cpu_usage,
      hardware::get_hardware_info,
      hardware::get_memory_info_detail,
      hardware::get_memory_usage,
      hardware::get_gpu_usage,
      hardware::get_processors_usage,
      hardware::get_gpu_temperature,
      hardware::get_cpu_usage_history,
      hardware::get_memory_usage_history,
      hardware::get_gpu_usage_history,
      hardware::get_network_info,
      hardware::get_super_io_chip_id_diagnostics,
      hardware::get_gpu_memory_usage,
      hardware::get_storage_health_latest_records,
      hardware::get_live_storage_health,
      hardware::refresh_storage_devices,
      external_component_guidance::get_external_component_guidance_candidates,
      external_component_guidance::defer_external_component_guidance_for_session,
      external_component_setup::get_external_component_setup_components,
      external_component_setup::get_external_component_setup_status,
      external_component_setup::run_external_component_setup,
      hardware::get_data_archive_series,
      hardware::get_gpu_archive_series,
      hardware::get_fan_archive_series,
      hardware::get_ambient_archive_series,
      hardware::get_process_stats,
      hardware::get_process_stats_in_period,
      hardware::get_gpu_archive_names,
      cooling_insight::get_cooling_trend,
      cooling_insight::get_cooling_fan_trend,
      cooling_insight::get_cooling_band_comparison,
      cooling_insight::get_cooling_baseline_delta,
      cooling_insight::get_cooling_load_temperature_explorer,
      cooling_insight::get_cooling_covariate_comparison,
      settings::commands::get_settings,
      settings::commands::set_language,
      settings::commands::set_theme,
      settings::commands::set_navigation_layout,
      settings::commands::acknowledge_navigation_restructure_announcement,
      settings::commands::set_display_targets,
      settings::commands::set_power_display_targets,
      settings::commands::set_graph_size,
      settings::commands::set_graph_fit_to_window,
      settings::commands::set_graph_margin_px,
      settings::commands::set_line_graph_type,
      settings::commands::set_line_graph_border,
      settings::commands::set_line_graph_fill,
      settings::commands::set_line_graph_color,
      settings::commands::set_line_graph_mix,
      settings::commands::set_line_graph_show_legend,
      settings::commands::set_line_graph_show_scale,
      settings::commands::set_line_graph_show_tooltip,
      settings::commands::set_background_img_opacity,
      settings::commands::set_selected_background_img,
      settings::commands::set_transparent_ui,
      settings::commands::set_window_opacity,
      settings::commands::set_glass_blur,
      settings::commands::set_temperature_unit,
      settings::commands::set_hardware_archive_enabled,
      settings::commands::set_switchbot_meter_enabled,
      settings::commands::set_switchbot_meter_device,
      ambient_sensor::get_ambient_sensor_candidates,
      settings::commands::set_hardware_archive_retention_days,
      settings::commands::set_hardware_archive_scheduled_data_deletion,
      settings::commands::set_storage_health_retention_days,
      settings::commands::set_burn_in_shift,
      settings::commands::set_burn_in_shift_mode,
      settings::commands::set_burn_in_shift_preset,
      settings::commands::set_burn_in_shift_idle_only,
      settings::commands::set_burn_in_shift_options,
      settings::commands::set_text_selectable,
      settings::commands::set_tray_widget_settings,
      settings::commands::set_close_to_tray_preference,
      settings::commands::acknowledge_external_component_guidance_key,
      settings::commands::set_elevated_startup_mode,
      settings::commands::read_license_file,
      settings::commands::read_third_party_notices_file,
      settings::commands::open_license_file_path,
      background_image::get_background_image,
      background_image::get_background_images,
      background_image::save_background_image,
      background_image::delete_background_image,
      ui::set_decoration,
      system::restart_app,
      system::is_process_elevated,
      system::quit_app,
      system::is_close_to_tray_available,
      system::mark_close_to_tray_listener_ready,
      system::hide_main_window_to_tray,
    ])
}

#[cfg(debug_assertions)]
fn bindings_path() -> std::path::PathBuf {
  std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/rspc/bindings.ts")
}

#[cfg(debug_assertions)]
fn export_typescript_bindings(builder: &Builder<Wry>) {
  builder
    .export(Typescript::default(), bindings_path())
    .expect("Failed to export typescript bindings");
}

/// Build the ambient sensor registry the archive worker reads (#2044).
///
/// The registry is built once and read-only afterwards (#2043), so this
/// is the single point where a user's ambient hardware becomes part of
/// the archive. Returning an empty registry is the normal result: the
/// SwitchBot scan is off by default, and an empty registry makes the
/// archive behave exactly as it did before ambient data existed.
///
/// Never fails and never blocks. Whether a radio exists is discovered
/// asynchronously inside the scan, and a machine without one is not an
/// error worth a dialog - it simply produces no readings, which #2043
/// already reports as an unavailable source.
#[cfg(target_os = "windows")]
fn setup_environmental_sensors(
  app: &tauri::AppHandle,
  core_settings: &hardviz_core::settings::CoreSettings,
  runtime: &tokio::runtime::Handle,
) -> Arc<
  hardviz_core::infrastructure::providers::environmental::EnvironmentalSensorRegistry,
> {
  use hardviz_core::infrastructure::providers::environmental::EnvironmentalSensorRegistry;
  use hardviz_core::infrastructure::providers::switchbot_meter::{
    SwitchBotMeterProvider, SwitchBotScanController,
  };

  let mut registry = EnvironmentalSensorRegistry::new();

  if core_settings.environmental_sensors.switchbot_meter_enabled {
    // Hand the provider the device this machine was told to use. Until
    // the user picks one the provider reports nothing: several sensors
    // in one room can read degrees apart, so choosing for them would be
    // guessing at the number every Thermal Delta is measured against.
    let provider = Arc::new(SwitchBotMeterProvider::new(
      core_settings
        .environmental_sensors
        .chosen_device()
        .map(str::to_string),
    ));

    let scan = SwitchBotScanController::setup(runtime.clone(), Arc::clone(&provider));
    app
      .state::<workers::WorkersState>()
      .switchbot_scan
      .lock()
      .unwrap()
      .replace(scan);
    // Kept beside the registry so the settings screen can list what the
    // radio is hearing right now, which is the only way a user can tell
    // their sensors apart.
    app
      .state::<workers::WorkersState>()
      .switchbot_provider
      .lock()
      .unwrap()
      .replace(Arc::clone(&provider));
    registry.register(provider);
  }

  Arc::new(registry)
}

/// No ambient transport is implemented outside Windows yet (#2044), so
/// the registry is always empty and the archive writes no ambient rows.
///
/// The provider abstraction, the decode, and the cache are all portable;
/// only the radio layer is missing, so adding a platform means adding a
/// scan rather than reworking this.
#[cfg(not(target_os = "windows"))]
fn setup_environmental_sensors(
  _app: &tauri::AppHandle,
  _core_settings: &hardviz_core::settings::CoreSettings,
  _runtime: &tokio::runtime::Handle,
) -> Arc<
  hardviz_core::infrastructure::providers::environmental::EnvironmentalSensorRegistry,
> {
  Arc::default()
}

#[cfg(debug_assertions)]
pub fn export_bindings() {
  let builder = build_specta_builder().typed_error_impl(TYPED_ERROR_IMPL);
  export_typescript_bindings(&builder);
}

/// Run a command-line mode when the process was started with one.
///
/// Returns the exit code to terminate with, or `None` for a normal launch.
/// Called before any Tauri runtime is created so the elevated setup child
/// never competes with the running app for the single-instance lock.
pub fn run_cli_mode_if_requested() -> Option<i32> {
  match cli::parse_cli_mode(std::env::args()) {
    Ok(Some(mode)) => Some(cli::run_cli_mode(mode)),
    Ok(None) => None,
    Err(error) => {
      eprintln!("invalid command line: {error:?}");
      Some(2)
    }
  }
}

pub fn run() {
  let builder = build_specta_builder();

  #[cfg(debug_assertions)]
  let builder = builder.typed_error_impl(TYPED_ERROR_IMPL);

  // TS bindings
  #[cfg(debug_assertions)]
  export_typescript_bindings(&builder);

  let app_state = settings::AppState::new();
  let elevated_startup_mode = app_state.settings.lock().unwrap().elevated_startup_mode;
  let transparent_ui = app_state.settings.lock().unwrap().transparent_ui;
  let glass_blur = app_state.settings.lock().unwrap().glass_blur;

  // Core-owned shared sensor history. App-side commands and the collector
  // loop read/write through this store. Persistence no longer shares it; the
  // archive worker subscribes to the EventBus instead (#1407).
  let history_store = Arc::new(HistoryStore::new());
  let external_component_guidance_state =
    Arc::new(ExternalComponentGuidanceState::default());

  let core_settings = app_state.core_settings.lock().unwrap().clone();

  let db_path = utils::file::get_app_data_dir("hv-database.db");
  // Initialize Core's DB pool location once at process start. Core
  // can't resolve the bundle identifier on its own, so App owns path
  // resolution and hands the file path to
  // `hardviz_core::infrastructure::database::db`. We don't care about
  // the return value here: this is the first and only caller during
  // App startup. This alone performs no I/O — it only records the path —
  // so it is safe before authority inspection below.
  let _ = hardviz_core::infrastructure::database::db::init(db_path.clone());

  // App resolves the native database, marker and spill paths beside the
  // SQLite file (Core cannot see the app-data directory) and is the single
  // reader of the authority decision those paths imply. See
  // `app::native_lifecycle` for the small state vocabulary this produces and
  // `AGENTS.md`/#2135 for why the decision must come from exactly one place.
  //
  // This must run before anything below that can create or modify
  // `hv-database.db` — `apply_pending_migrations` opens the file with
  // `create_if_missing`. If the source were recreated as an empty file
  // first, `observe_authority` could no longer see a missing source beside
  // surviving native files or conversion debris (`SourceDatabaseMissing`);
  // it would instead read the fresh empty file as compatible, and a later
  // conversion could reconcile real native history against it.
  #[cfg(feature = "duckdb-archive")]
  let native_lifecycle_state = app::native_lifecycle::inspect_startup_authority(
    &infrastructure::database::native_paths::authority_paths(),
    infrastructure::database::native_schema::NATIVE_SCHEMA_VERSION,
  );
  // When authority already selected the native database, open it now, as
  // part of startup's own decision — not from a detached task that can
  // only log a failure after `is_db_ok` and the owner's state have already
  // been decided. See `open_selected_native_database`.
  #[cfg(feature = "duckdb-archive")]
  let (native_lifecycle_state, opened_native_database) = open_selected_native_database(
    native_lifecycle_state,
    &infrastructure::database::native_paths::authority_paths().native_database,
    infrastructure::database::native_schema::NATIVE_SCHEMA_VERSION,
  );
  #[cfg(not(feature = "duckdb-archive"))]
  let _opened_native_database: Option<()> = None;

  // An unresolved native authority disagreement — including a selected
  // database that failed to open — blocks every SQLite operation below
  // exactly like an incompatible SQLite schema: `inspect_authority` (or the
  // open above) already refused to guess or already failed, so touching
  // `hv-database.db` here would be a second, App-side guess about the same
  // files, and once selection is durable ADR 0022 rejects silently running
  // on SQLite instead.
  #[cfg(feature = "duckdb-archive")]
  let native_authority_blocks_sqlite = matches!(
    native_lifecycle_state,
    app::native_lifecycle::DatabaseLifecycleState::ActionRequired(_)
  );
  #[cfg(not(feature = "duckdb-archive"))]
  let native_authority_blocks_sqlite = false;

  // Whether `hv-database.db` may be created, migrated or otherwise touched
  // this boot. Narrower than `!native_authority_blocks_sqlite`: once native
  // authority is *already* selected (`NativeAuthoritative`), a previous
  // boot's `retire_sqlite_source` may already have renamed the source away,
  // and `apply_pending_migrations` opens SQLite with `create_if_missing` —
  // running it here would recreate an empty `hv-database.db`, which this
  // same boot's later `retire_sqlite_source` call would then rename over
  // the *real* retired copy, destroying it. SQLite is therefore left alone
  // in every state but the two where it is still genuinely the live
  // source.
  #[cfg(feature = "duckdb-archive")]
  let sqlite_source_is_authoritative =
    app::native_lifecycle::sqlite_source_is_authoritative(&native_lifecycle_state);
  #[cfg(not(feature = "duckdb-archive"))]
  let sqlite_source_is_authoritative = true;

  let app_max_version = infrastructure::database::migration::get_max_migration_version();
  let mut db_error = hardviz_core::persistence::preflight::check_db_compatibility(
    &db_path,
    app_max_version,
  );

  // Route database consumers to whichever backend is durably selected
  // (#2134). A no-op build detail with the feature disabled: dispatch's
  // SQLite path needs no configuration. Skipped once SQLite itself is
  // already known incompatible - `is_db_ok` below already keeps every
  // database-backed worker from starting in that case. A failure here
  // (the durable record says native is or may be selected, but this
  // boundary cannot safely serve it) is surfaced through the same gate:
  // continuing on SQLite would silently answer from a backend that is no
  // longer authoritative.
  #[cfg(feature = "duckdb-archive")]
  if db_error.is_none()
    && let Err(e) = initialize_native_database_dispatch(&db_path)
  {
    log_error!(
      "Native database authority is not safely servable; database-backed workers will not start",
      "lib::run",
      Some(e.clone())
    );
    db_error = Some(hardviz_core::persistence::preflight::DbStartupError::Other(
      e,
    ));
  }

  // Core owns the database pool, so it also applies the schema migrations —
  // synchronously here, before any persistence worker writes. These were
  // previously registered with `tauri-plugin-sql` but never ran, because
  // the DB is never loaded through the plugin (no `preload`, no frontend
  // `Database.load`), leaving newer tables such as `storage_devices`
  // missing. A migration failure is surfaced as a DB-incompatible startup
  // so the existing recovery dialog handles it.
  if db_error.is_none()
    && sqlite_source_is_authoritative
    && let Err(e) = apply_pending_migrations()
  {
    log_error!(
      "Failed to apply database migrations",
      "lib::run",
      Some(e.clone())
    );
    db_error = Some(hardviz_core::persistence::preflight::DbStartupError::Other(
      e,
    ));
  }

  let is_db_ok = db_error.is_none() && !native_authority_blocks_sqlite;

  let store_for_setup = Arc::clone(&history_store);
  let guidance_for_setup = Arc::clone(&external_component_guidance_state);

  let tauri_builder = tauri::Builder::<Wry>::default()
    .invoke_handler(builder.invoke_handler())
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      lifecycle::on_second_instance(app);
    }))
    .setup(move |app| {
      let path_resolver = app.path();

      // Initialize logger
      utils::logger::init(path_resolver.app_log_dir().unwrap());

      if elevated_startup_mode {
        match services::system_service::relaunch_for_elevated_startup_if_needed(app.handle())
        {
          Ok(true) => return Ok(()),
          Ok(false) => {}
          Err(e) => {
            log_warn!(
              &format!("Elevated Startup Mode could not restart as administrator: {e}"),
              "lib::setup",
              None::<&str>
            );
          }
        }
      }

      // Initialize UI and real-time monitoring (independent of DB)
      commands::ui::init(app);
      builder.mount_events(app);

      // Apply native macOS vibrancy up front when transparent UI is on and the
      // background-frost toggle (glass_blur) is non-zero, so the frosted glass is
      // composited by the OS instead of the costly CSS backdrop-filter blur
      // (see #1718). No-op on other platforms.
      settings::commands::apply_window_vibrancy(
        app.handle(),
        if transparent_ui { glass_blur } else { 0 },
      );

      // Real-time pipeline: collector publishes MetricsSnapshot to the
      // EventBus, WindowAdapter subscribes and emits HardwareMonitorUpdate.
      let bus = hardviz_core::event_bus::EventBus::new();
      let window_adapter =
        adapters::window::WindowAdapter::setup(app.handle().clone(), bus.subscribe());

      // Run the Core collector on Tauri's tokio runtime. Core has no
      // `tauri` dep, so it can't reach Tauri's static runtime directly —
      // we hand it the inner `tokio::runtime::Handle`.
      let runtime_handle = tauri::async_runtime::handle().inner().clone();
      let monitor = hardviz_core::collector::SystemMonitorController::setup(
        Arc::clone(&store_for_setup),
        bus.clone(),
        runtime_handle.clone(),
      );
      {
        let ws = app.state::<workers::WorkersState>();
        ws.monitor.lock().unwrap().replace(monitor);
        ws.window_adapter.lock().unwrap().replace(window_adapter);
      }

      // #1422: register the tray unconditionally. The visibility
      // policy (always-on vs persisted user setting) is a UX call owned
      // by #1423 and lives outside the adapter.
      match adapters::tray::TrayAdapter::setup(app, bus.subscribe()) {
        Ok(tray) => {
          let ws = app.state::<workers::WorkersState>();
          ws.tray.lock().unwrap().replace(tray);
        }
        Err(e) => {
          app
            .state::<lifecycle::CloseToTrayRuntimeState>()
            .disable_for_session();
          // Linux desktops without an indicator implementation, or any
          // other tray failure: log and continue. Close-to-tray is
          // disabled for this session so the close button still quits.
          log_warn!(
            &format!("tray icon setup failed; continuing without tray: {e}"),
            "lib::setup",
            None::<&str>
          );
        }
      }

      // Record the startup authority decision on the App's single lifecycle
      // owner before anything else reads it — including the `is_db_ok`
      // branch below, so `ActionRequired` is recorded even when `db_error`
      // is also set and the branch that used to record it is skipped.
      #[cfg(feature = "duckdb-archive")]
      app
        .state::<app::native_lifecycle::NativeLifecycleOwner>()
        .set_state(native_lifecycle_state.clone());

      if is_db_ok {
        // When a previous conversion already selected the native database,
        // `opened_native_database` (opened synchronously above, as part of
        // startup's own decision — see `open_selected_native_database`) is
        // handed to the owner here, and the SQLite source is retired (rename
        // in place; decided 2026-09-13, Design Doc). Retiring only happens
        // on a startup that finds the database *already* selected from a
        // previous run — never on the run that just selected it — because
        // a successful open is the "later verified startup" the Design Doc
        // requires before the ordinary SQLite recovery path is taken away.
        // Nothing routes reads or writes through the opened database yet;
        // it is only made reachable through the `// #2134 seam:` on
        // `NativeLifecycleOwner` for the dispatch boundary #2134 adds.
        #[cfg(feature = "duckdb-archive")]
        let start_sqlite_backed_producers = {
          let native_authoritative = matches!(
            native_lifecycle_state,
            app::native_lifecycle::DatabaseLifecycleState::NativeAuthoritative
          );
          if let Some(database) = opened_native_database {
            app
              .state::<app::native_lifecycle::NativeLifecycleOwner>()
              .set_selected_database(database);
            let source_database_path =
              infrastructure::database::native_paths::authority_paths().source_database;
            app::native_maintenance::retire_sqlite_source(&source_database_path);
          }
          // Rerouting these producers to write through the selected native
          // database instead is #2134's dispatch boundary, which had not
          // landed when this was written. Until it does, starting them
          // against a SQLite source this same startup may have just retired
          // would be worse than not collecting: not only would the writes
          // go nowhere useful, a rename out from under an open connection is
          // unreliable on some platforms. So collection is idle in this one
          // state rather than unsafe. `is_db_ok` already guarantees
          // `opened_native_database` is `Some` whenever `native_authoritative`
          // is true — otherwise `open_selected_native_database` would have
          // downgraded the state to `ActionRequired` and `is_db_ok` would be
          // false — so this never idles collection over an open that
          // silently failed.
          !native_authoritative
        };
        #[cfg(not(feature = "duckdb-archive"))]
        let start_sqlite_backed_producers = true;

        // Start DB-dependent archive services. Persistence subscribes to
        // the EventBus so a slow DB write can't back-pressure the
        // collector cadence (#1407).
        if start_sqlite_backed_producers && core_settings.hardware_archive.enabled {
          // Ambient sources (#2043) ride the archive's one-minute tick,
          // so they are built here and only here: with the archive off
          // there is nowhere for an ambient reading to go, and starting
          // a radio scan to feed a worker that isn't running would be
          // collection cost with no visible value.
          let environmental_sensors = setup_environmental_sensors(
            app.handle(),
            &core_settings,
            &runtime_handle,
          );

          let hw_archive =
            hardviz_core::persistence::ArchiveController::setup_with_environmental_sensors(
              &bus,
              runtime_handle.clone(),
              environmental_sensors,
            );
          {
            let ws = app.state::<workers::WorkersState>();
            ws.hw_archive.lock().unwrap().replace(hw_archive);
          }
        }

        // The cooling daily rollup derives its summary from whatever
        // Hardware Archive rows already exist in the database, so it
        // starts independently of `hardware_archive.enabled`: even when
        // live archive collection is currently turned off, already
        // archived days it hasn't caught up on yet still get rolled up.
        // Still gated on `start_sqlite_backed_producers`: it is a SQLite
        // reader/writer like the others above.
        let cooling_rollup_first_catch_up = if start_sqlite_backed_producers {
          let (cooling_rollup, first_catch_up) =
            hardviz_core::persistence::CoolingRollupController::setup(runtime_handle.clone());
          let ws = app.state::<workers::WorkersState>();
          ws.cooling_rollup.lock().unwrap().replace(cooling_rollup);
          Some(first_catch_up)
        } else {
          None
        };

        if start_sqlite_backed_producers && core_settings.storage_health.enabled {
          match core_settings.storage_health_identity.hash_key_bytes() {
            Ok(identity_hash_key) => {
              let storage_guidance_state = Arc::clone(&guidance_for_setup);
              let storage_guidance_sink:
                hardviz_core::persistence::ExternalComponentGuidanceSink =
                Arc::new(move |candidates| {
                  storage_guidance_state.record_candidates(candidates);
                });
              let storage_health =
                hardviz_core::persistence::StorageHealthController::setup_with_guidance_sink(
                  runtime_handle.clone(),
                  core_settings.storage_health.retention_days,
                  identity_hash_key,
                  Some(storage_guidance_sink),
                );
              // Live Storage Health (ADR 0006): enumerate devices once at
              // startup so on-demand reads never enumerate. The WMI query
              // is blocking, so it runs off the main thread.
              let live_storage_health = Arc::new(
                hardviz_core::collector::LiveStorageHealthCollector::new(
                  identity_hash_key,
                ),
              );
              {
                let ws = app.state::<workers::WorkersState>();
                ws.storage_health.lock().unwrap().replace(storage_health);
                ws.live_storage_health
                  .lock()
                  .unwrap()
                  .replace(Arc::clone(&live_storage_health));
              }
              tauri::async_runtime::spawn_blocking(move || {
                live_storage_health.enumerate_devices()
              });
            }
            Err(e) => {
              log_error!(
                "Storage Health worker was not started because the identity key is invalid",
                "lib::run",
                Some(e)
              );
            }
          }
        }

        // Retention cleanup runs once per process boot — the pre-Phase-4
        // `batch_delete_old_data` wrapper had the same one-shot semantics.
        // The `scheduled_data_deletion` flag still means startup cleanup,
        // not a recurring background schedule.
        // See `hardviz_core::persistence::cleanup_old_data` doc comment.
        //
        // It waits for the cooling rollup's first catch-up pass: archive
        // rows older than the retention cutoff can still be present at
        // startup, and the backfill must read them before they are
        // deleted or those days would be lost from the rollup forever.
        // Cleanup runs only when that pass actually succeeded — after a
        // failed pass (or a dead worker, which closes the channel) this
        // boot's cleanup is skipped so a transient DB error cannot let
        // deletion outrun the rollup; the next boot retries both.
        if start_sqlite_backed_producers && core_settings.hardware_archive.scheduled_data_deletion
        {
          let retention_days = core_settings.hardware_archive.retention_days;
          // `start_sqlite_backed_producers` gated the branch above that
          // produces this, so it is always `Some` here.
          let cooling_rollup_first_catch_up = cooling_rollup_first_catch_up
            .expect("cooling rollup runs whenever SQLite-backed producers do");
          // Spawned on the raw `runtime_handle` rather than
          // `tauri::async_runtime::spawn` so the handle is a plain
          // `tokio::task::JoinHandle` — the type `WorkersState` already
          // stores worker handles as — and can be awaited by
          // `WorkersState::terminate_all` and the #2135 conversion driver's
          // pause/drain, neither of which previously had a way to wait for
          // this one-shot pass to finish.
          let cleanup = runtime_handle.spawn(async move {
            if cooling_rollup_first_catch_up.await == Ok(true) {
              hardviz_core::persistence::cleanup_old_data(retention_days).await;
            } else {
              log_warn!(
                "Skipping this boot's retention cleanup: the cooling rollup's first catch-up did not succeed",
                "lib::run",
                None::<&str>
              );
            }
          });
          app
            .state::<workers::WorkersState>()
            .scheduled_cleanup
            .lock()
            .unwrap()
            .replace(cleanup);
        }
      } else {
        // The database is not usable yet — either an incompatible SQLite
        // schema or (below, feature-gated) a native authority disagreement
        // `inspect_authority` refused to guess at. Hide the window while the
        // matching dialog is shown, then restore based on the user's choice.
        if let Some(window) = app.get_webview_window("main") {
          let _ = window.hide();
        }

        if let Some(db_err) = db_error.clone() {
          let handle = app.handle().clone();
          std::thread::spawn(move || {
            use app::startup::{self, StartupErrorAction};
            match startup::prompt_startup_error(&handle, db_err) {
              StartupErrorAction::ResetAndRestart => {
                startup::reset_database_and_restart(&handle);
              }
              StartupErrorAction::ContinueAnyway => {
                // Show the main window — app runs without DB-backed features
                if let Some(window) = handle.get_webview_window("main") {
                  let _ = window.show();
                }
              }
              StartupErrorAction::Exit => handle.exit(1),
            }
          });
        }

        // `is_db_ok` is false and `db_error` is `None` only when the native
        // authority decision was `ActionRequired` — see how `is_db_ok` is
        // computed above. The owner's state was already recorded before the
        // `is_db_ok` branch, above; nothing further to set here.
        #[cfg(feature = "duckdb-archive")]
        if db_error.is_none()
          && let app::native_lifecycle::DatabaseLifecycleState::ActionRequired(issue) =
            native_lifecycle_state.clone()
        {
          let handle = app.handle().clone();
          std::thread::spawn(move || {
            use app::startup::{self, NativeAuthorityAction};
            match startup::prompt_native_authority_issue(&handle, &issue) {
              NativeAuthorityAction::ContinueAnyway => {
                if let Some(window) = handle.get_webview_window("main") {
                  let _ = window.show();
                }
              }
              NativeAuthorityAction::Exit => handle.exit(1),
            }
          });
        }
      }

      Ok(())
    })
    .on_window_event(|win, ev| {
      if win.label() == tray::TRAY_WIDGET_FLYOUT_LABEL {
        match ev {
          tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if win.hide().is_ok() {
              webview_memory::suspend_for_window(win);
            }
          }
          // An explicit flyout hide also emits Focused(false). Only the
          // focus-loss path for a still-visible flyout owns another hide and
          // suspend transition.
          tauri::WindowEvent::Focused(false)
            if win.is_visible().unwrap_or(false) && win.hide().is_ok() =>
          {
            webview_memory::suspend_for_window(win);
          }
          _ => {}
        }
        return;
      }

      if win.label() == "main"
        && let tauri::WindowEvent::CloseRequested { api, .. } = ev
      {
        // Always prevent the default close so lifecycle picks the
        // outcome (hide vs. quit) deterministically — see
        // `lifecycle::handle_close_request` for the policy.
        api.prevent_close();
        lifecycle::on_close_requested(win);
      }
    })
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_autostart::init(
      MacosLauncher::LaunchAgent,
      Some(vec![]),
    ))
    .plugin(tauri_plugin_clipboard_manager::init())
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_opener::init())
    .manage(history_store)
    .manage(app_state)
    .manage(external_component_guidance_state)
    .manage(lifecycle::CloseToTrayRuntimeState::default())
    .manage(workers::WorkersState::default())
    .manage(app_updates::PendingUpdate(Mutex::new(None)));

  #[cfg(feature = "duckdb-archive")]
  let tauri_builder =
    tauri_builder.manage(app::native_lifecycle::NativeLifecycleOwner::new());

  let mut context = tauri::generate_context!();
  utils::tauri::apply_runtime_config(context.config_mut());

  tauri_builder
    .build(context)
    .expect("error while building tauri application")
    .run(lifecycle::on_run_event);
}
