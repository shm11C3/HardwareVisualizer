/*
This file includes code adapted from Tauri documentation:

- Repository: tauri-apps/tauri-docs (MIT)
- Source: tauri-docs/src/content/docs/plugin/updater.mdx (6b2a2ecc631b69fedaa4187f895a5d632f6b4ed0)

Copyright (c) 2020-2023 Tauri Programme within the Commons Conservancy
Licensed under the MIT License. See THIRD_PARTY_NOTICES.md for the full text.

*/

pub mod app_updates {
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
  #[serde(tag = "event", content = "data")]
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

  #[derive(Serialize, specta::Type)]
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
    let update = app.updater()?.check().await?;

    let meta = update.as_ref().map(|u| UpdateMetadata {
      version: u.version.clone(),
      current_version: u.current_version.clone(),
      notes: u.body.clone(),
      pub_date: u.date.map(|d| d.to_string()),
    });

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
    let Some(update) = pending_update.0.lock().unwrap().take() else {
      return Err(UpdaterError::NoPendingUpdate);
    };

    let mut started = false;

    update
      .download_and_install(
        |chunk_length, content_length| {
          if !started {
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
          let _ = on_event.send(DownloadEvent::Finished);
        },
      )
      .await?;

    Ok(())
  }
}
