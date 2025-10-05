use crate::models::libre_hardware_monitor_model::LibreHardwareMonitorNode;
use crate::models::settings::LibreHardwareMonitorImportSettings;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct LibreHardwareMonitorProvider {
  client: Client,
  base_url: String,
  auth: Option<(String, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum LibreHardwareMonitorError {
  #[error("Connection failed: {0}")]
  ConnectionFailed(String),

  #[error("HTTP request failed: {0}")]
  RequestFailed(String),

  #[error("Authentication failed")]
  AuthenticationFailed,

  #[error("Invalid response: {0}")]
  InvalidResponse(String),

  #[error("Not enabled")]
  NotEnabled,
}

impl LibreHardwareMonitorProvider {
  /// Create a new provider from settings
  pub fn from_settings(
    settings: &LibreHardwareMonitorImportSettings,
  ) -> Result<Self, LibreHardwareMonitorError> {
    if !settings.enabled {
      return Err(LibreHardwareMonitorError::NotEnabled);
    }

    let protocol = if settings.use_https { "https" } else { "http" };
    let base_url = format!("{}://{}:{}", protocol, settings.host, settings.port);

    let client = Client::builder()
      .build()
      .map_err(|e| LibreHardwareMonitorError::ConnectionFailed(e.to_string()))?;

    let auth = match (&settings.basic_auth_username, &settings.basic_auth_password) {
      (Some(username), Some(password)) => Some((username.clone(), password.clone())),
      _ => None,
    };

    Ok(Self {
      client,
      base_url,
      auth,
    })
  }

  /// Test connection to LibreHardwareMonitor API
  pub async fn test_connection(&self) -> Result<(), LibreHardwareMonitorError> {
    let url = format!("{}/data.json", self.base_url);

    let mut request = self.client.get(&url);

    if let Some((username, password)) = &self.auth {
      request = request.basic_auth(username, Some(password));
    }

    let response = request
      .send()
      .await
      .map_err(|e| LibreHardwareMonitorError::RequestFailed(e.to_string()))?;

    if response.status().is_client_error() {
      if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(LibreHardwareMonitorError::AuthenticationFailed);
      }
      return Err(LibreHardwareMonitorError::RequestFailed(format!(
        "HTTP {}",
        response.status()
      )));
    }

    if !response.status().is_success() {
      return Err(LibreHardwareMonitorError::RequestFailed(format!(
        "HTTP {}",
        response.status()
      )));
    }

    // Verify that the response is valid JSON
    let _: LibreHardwareMonitorNode = response
      .json()
      .await
      .map_err(|e| LibreHardwareMonitorError::InvalidResponse(e.to_string()))?;

    Ok(())
  }

  #[allow(dead_code)]
  /// Fetch hardware data from LibreHardwareMonitor
  pub async fn fetch_data(
    &self,
  ) -> Result<LibreHardwareMonitorNode, LibreHardwareMonitorError> {
    let url = format!("{}/data.json", self.base_url);

    let mut request = self.client.get(&url);

    if let Some((username, password)) = &self.auth {
      request = request.basic_auth(username, Some(password));
    }

    let response = request
      .send()
      .await
      .map_err(|e| LibreHardwareMonitorError::RequestFailed(e.to_string()))?;

    if response.status().is_client_error() {
      if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(LibreHardwareMonitorError::AuthenticationFailed);
      }
      return Err(LibreHardwareMonitorError::RequestFailed(format!(
        "HTTP {}",
        response.status()
      )));
    }

    if !response.status().is_success() {
      return Err(LibreHardwareMonitorError::RequestFailed(format!(
        "HTTP {}",
        response.status()
      )));
    }

    let data: LibreHardwareMonitorNode = response
      .json()
      .await
      .map_err(|e| LibreHardwareMonitorError::InvalidResponse(e.to_string()))?;

    Ok(data)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn create_test_settings(enabled: bool) -> LibreHardwareMonitorImportSettings {
    LibreHardwareMonitorImportSettings {
      enabled,
      host: "127.0.0.1".to_string(),
      port: 8085,
      use_https: false,
      basic_auth_username: None,
      basic_auth_password: None,
    }
  }

  #[test]
  fn test_provider_creation() {
    let settings = create_test_settings(true);
    let provider = LibreHardwareMonitorProvider::from_settings(&settings);
    assert!(provider.is_ok());
  }

  #[test]
  fn test_provider_creation_with_https() {
    let mut settings = create_test_settings(true);
    settings.use_https = true;

    let provider = LibreHardwareMonitorProvider::from_settings(&settings);
    assert!(provider.is_ok());
  }

  #[test]
  fn test_provider_creation_with_auth() {
    let mut settings = create_test_settings(true);
    settings.basic_auth_username = Some("admin".to_string());
    settings.basic_auth_password = Some("password".to_string());

    let provider = LibreHardwareMonitorProvider::from_settings(&settings);
    assert!(provider.is_ok());
  }

  #[test]
  fn test_provider_creation_disabled() {
    let settings = create_test_settings(false);
    let provider = LibreHardwareMonitorProvider::from_settings(&settings);
    assert!(provider.is_err());

    match provider.unwrap_err() {
      LibreHardwareMonitorError::NotEnabled => {}
      _ => panic!("Expected NotEnabled error"),
    }
  }
}
