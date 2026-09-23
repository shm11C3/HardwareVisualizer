/*
This file includes code adapted from Tauri documentation:

- Repository: tauri-apps/tauri-docs (MIT)
- Source: tauri-docs/src/content/docs/plugin/updater.mdx (6b2a2ecc631b69fedaa4187f895a5d632f6b4ed0)

Copyright (c) 2020-2023 Tauri Programme within the Commons Conservancy
Licensed under the MIT License. See THIRD_PARTY_NOTICES.md for the full text.

*/

pub mod app_updates {
  use crate::{log_debug, log_error, log_info};
  use serde::Serialize;
  use specta;
  use std::sync::Mutex;
  use tauri;
  use tauri_plugin_updater::{Update, UpdaterExt};

  #[derive(Debug, Serialize, specta::Type)]
  pub enum UpdaterError {
    NoPendingUpdate,
    Updater(String),
  }

  impl std::fmt::Display for UpdaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        UpdaterError::NoPendingUpdate => write!(f, "there is no pending update"),
        UpdaterError::Updater(msg) => write!(f, "{msg}"),
      }
    }
  }

  impl std::error::Error for UpdaterError {}

  impl From<tauri_plugin_updater::Error> for UpdaterError {
    fn from(e: tauri_plugin_updater::Error) -> Self {
      UpdaterError::Updater(e.to_string())
    }
  }
  #[derive(Clone, Serialize, specta::Type)]
  #[serde(tag = "event", content = "data", rename_all = "camelCase")]
  pub enum DownloadEvent {
    #[serde(rename_all = "camelCase")]
    Started {
      content_length: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    Progress {
      chunk_length: String,
    },
    Finished,
  }

  #[derive(Debug, Serialize, specta::Type)]
  #[serde(rename_all = "camelCase")]
  pub struct UpdateMetadata {
    version: String,
    current_version: String,
    notes: Option<String>,
    pub_date: Option<String>,
  }

  pub struct PendingUpdate(pub Mutex<Option<Update>>);

  ///
  /// Fetch available update metadata
  ///
  #[tauri::command]
  #[specta::specta]
  pub async fn fetch_update(
    app: tauri::AppHandle,
    pending: tauri::State<'_, PendingUpdate>,
  ) -> Result<Option<UpdateMetadata>, UpdaterError> {
    // Avoid updater popups/noise during development.
    // In debug builds we skip the updater check entirely.
    if cfg!(debug_assertions) {
      *pending.0.lock().unwrap() = None;
      return Ok(None);
    }

    let update = app.updater()?.check().await?;

    let meta = update.as_ref().map(|u| UpdateMetadata {
      version: u.version.clone(),
      current_version: u.current_version.clone(),
      notes: u.body.clone(),
      pub_date: u.date.map(|d| d.to_string()),
    });

    log_debug!(
      "get metadata",
      "fetch_update",
      meta.as_ref().map(|m| format!("{:?}", m)).as_deref()
    );

    *pending.0.lock().unwrap() = update;
    Ok(meta)
  }

  ///
  /// Install the pending update
  ///
  #[tauri::command]
  #[specta::specta]
  pub async fn install_update(
    pending_update: tauri::State<'_, PendingUpdate>,
    on_event: tauri::ipc::Channel<DownloadEvent>,
  ) -> Result<(), UpdaterError> {
    log_info!("start", "install_update", None::<&str>);

    let Some(update) = pending_update.0.lock().unwrap().take() else {
      log_error!("no_pending_update", "install_update", None::<&str>);
      return Err(UpdaterError::NoPendingUpdate);
    };

    let mut started = false;

    update
      .download_and_install(
        |chunk_length, content_length| {
          if !started {
            log_info!(
              "started",
              "install_update",
              content_length.map(|c| c.to_string())
            );

            let _ = on_event.send(DownloadEvent::Started {
              content_length: content_length.map(|c| c.to_string()), // Specta does not support u64
            });
            started = true;
          }

          let _ = on_event.send(DownloadEvent::Progress {
            chunk_length: chunk_length.to_string(),
          });
        },
        || {
          log_info!("finished", "install_update", None::<&str>);
          let _ = on_event.send(DownloadEvent::Finished);
        },
      )
      .await?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  /// Keys `tauri-plugin-updater` reads. The plugin ignores unknown keys, so a
  /// misspelled or Tauri v1 key silently falls back to the plugin default
  /// (#2212). Update these lists when a plugin upgrade adds a key we use.
  const UPDATER_KEYS: &[&str] = &[
    "allowDowngrades",
    "dangerousAcceptInvalidCerts",
    "dangerousAcceptInvalidHostnames",
    "dangerousInsecureTransportProtocol",
    "endpoints",
    "pubkey",
    "requireSignedVersion",
    "windows",
  ];
  const UPDATER_WINDOWS_KEYS: &[&str] = &["installMode", "installerArgs"];

  fn updater_config() -> serde_json::Value {
    let config: serde_json::Value =
      serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    config["plugins"]["updater"].clone()
  }

  #[test]
  fn updater_config_only_uses_keys_the_plugin_reads() {
    let updater = updater_config();
    for key in updater.as_object().expect("plugins.updater").keys() {
      assert!(
        UPDATER_KEYS.contains(&key.as_str()),
        "plugins.updater.{key} is not read by tauri-plugin-updater"
      );
    }
    for key in updater["windows"]
      .as_object()
      .expect("plugins.updater.windows")
      .keys()
    {
      assert!(
        UPDATER_WINDOWS_KEYS.contains(&key.as_str()),
        "plugins.updater.windows.{key} is not read by tauri-plugin-updater"
      );
    }
  }

  /// Updates stay passive: the MSI runs without its UI sequence, so an update
  /// never offers or runs External Component Setup (ADR 0024).
  #[test]
  fn updater_installs_windows_updates_in_passive_mode() {
    let config: tauri_plugin_updater::Config =
      serde_json::from_value(updater_config()).expect("plugin parses the config");
    let windows = config.windows.expect("plugins.updater.windows");
    assert_eq!(format!("{:?}", windows.install_mode), "Passive");
    assert_eq!(updater_config()["windows"]["installMode"], "passive");
  }
}
