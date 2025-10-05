use serde::{Deserialize, Serialize};

/// Response from /data.json API endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorApiResponse {
  pub result: SensorApiResult,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub value: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub min: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub format: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SensorApiResult {
  #[serde(rename = "ok")]
  Ok,
  #[serde(rename = "fail")]
  Fail,
}

/// Request parameters for /Sensor API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorApiRequest {
  pub action: SensorApiAction,
  pub id: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub value: Option<SensorApiValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SensorApiAction {
  #[serde(rename = "Get")]
  Get,
  #[serde(rename = "Set")]
  Set,
  #[serde(rename = "ResetMinMax")]
  ResetMinMax,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SensorApiValue {
  Number(f64),
  Null,
}

/// Configuration for the HTTP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpServerConfig {
  #[serde(rename = "listenerIp")]
  pub listener_ip: String,
  #[serde(rename = "listenerPort")]
  pub listener_port: u16,
  #[serde(rename = "authenticationEnabled")]
  pub authentication_enabled: bool,
  #[serde(
    rename = "authenticationUserName",
    skip_serializing_if = "Option::is_none"
  )]
  pub authentication_user_name: Option<String>,
  #[serde(
    rename = "authenticationPassword",
    skip_serializing_if = "Option::is_none"
  )]
  pub authentication_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibreHardwareMonitorNode {
  pub id: u32,
  #[serde(rename = "Text")]
  pub text: String,
  #[serde(rename = "Min")]
  pub min: String,
  #[serde(rename = "Value")]
  pub value: String,
  #[serde(rename = "Max")]
  pub max: String,
  #[serde(rename = "ImageURL")]
  pub image_url: String,
  #[serde(rename = "SensorId", skip_serializing_if = "Option::is_none")]
  pub sensor_id: Option<String>,
  #[serde(rename = "Type", skip_serializing_if = "Option::is_none")]
  pub sensor_type: Option<SensorType>,
  #[serde(rename = "Children")]
  pub children: Vec<LibreHardwareMonitorNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SensorType {
  #[serde(rename = "Voltage")]
  Voltage,
  #[serde(rename = "Current")]
  Current,
  #[serde(rename = "Clock")]
  Clock,
  #[serde(rename = "Load")]
  Load,
  #[serde(rename = "Temperature")]
  Temperature,
  #[serde(rename = "Fan")]
  Fan,
  #[serde(rename = "Flow")]
  Flow,
  #[serde(rename = "Control")]
  Control,
  #[serde(rename = "Level")]
  Level,
  #[serde(rename = "Power")]
  Power,
  #[serde(rename = "Noise")]
  Noise,
  #[serde(rename = "Conductivity")]
  Conductivity,
  #[serde(rename = "Throughput")]
  Throughput,
  #[serde(rename = "Humidity")]
  Humidity,
  // Legacy types for backward compatibility
  #[serde(rename = "Factor")]
  Factor,
  #[serde(rename = "Data")]
  Data,
  #[serde(rename = "SmallData")]
  SmallData,
}

#[allow(dead_code)]
impl LibreHardwareMonitorNode {
  /// Parse JSON string and create root node
  pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
    serde_json::from_str(json_str)
  }

  /// Find node by sensor ID
  pub fn find_by_sensor_id(&self, sensor_id: &str) -> Option<&LibreHardwareMonitorNode> {
    if let Some(ref id) = self.sensor_id
      && id == sensor_id
    {
      return Some(self);
    }

    for child in &self.children {
      if let Some(node) = child.find_by_sensor_id(sensor_id) {
        return Some(node);
      }
    }

    None
  }

  /// Find all nodes by sensor type
  pub fn find_by_sensor_type(
    &self,
    sensor_type: &SensorType,
  ) -> Vec<&LibreHardwareMonitorNode> {
    let mut result = Vec::new();

    if let Some(ref node_type) = self.sensor_type
      && node_type == sensor_type
    {
      result.push(self);
    }

    for child in &self.children {
      result.extend(child.find_by_sensor_type(sensor_type));
    }

    result
  }

  /// Find nodes that contain specified text
  pub fn find_by_text_contains(&self, text: &str) -> Vec<&LibreHardwareMonitorNode> {
    let mut result = Vec::new();

    if self.text.contains(text) {
      result.push(self);
    }

    for child in &self.children {
      result.extend(child.find_by_text_contains(text));
    }

    result
  }

  /// Get sensor leaf nodes (nodes with sensor values)
  pub fn get_sensor_leaves(&self) -> Vec<&LibreHardwareMonitorNode> {
    let mut result = Vec::new();

    if self.sensor_id.is_some() && self.children.is_empty() {
      result.push(self);
    }

    for child in &self.children {
      result.extend(child.get_sensor_leaves());
    }

    result
  }

  /// Extract numeric value from sensor value string
  pub fn get_numeric_value(&self) -> Option<f64> {
    // Extract numeric part from strings like "1.136 V" or "33.333"
    let value_str = self.value.trim();
    if value_str.is_empty() {
      return None;
    }

    // Extract only the numeric part
    let numeric_part = value_str.split_whitespace().next()?.replace(',', ""); // Remove commas

    numeric_part.parse::<f64>().ok()
  }

  /// Extract unit from sensor value string
  pub fn get_unit(&self) -> Option<&str> {
    let value_str = self.value.trim();
    if value_str.is_empty() {
      return None;
    }

    let parts: Vec<&str> = value_str.split_whitespace().collect();
    if parts.len() >= 2 {
      Some(parts[1])
    } else {
      None
    }
  }

  /// Convert hierarchy to string representation (for debugging)
  pub fn to_tree_string(&self, indent: usize) -> String {
    let mut result = String::new();
    let indent_str = "  ".repeat(indent);

    result.push_str(&format!("{}{}", indent_str, self.text));

    if let Some(ref sensor_id) = self.sensor_id {
      result.push_str(&format!(" [ID: {}]", sensor_id));
    }

    if let Some(ref sensor_type) = self.sensor_type {
      result.push_str(&format!(" [{:?}]", sensor_type));
    }

    if !self.value.is_empty() && self.value != "Value" {
      result.push_str(&format!(" = {}", self.value));
    }

    result.push('\n');

    for child in &self.children {
      result.push_str(&child.to_tree_string(indent + 1));
    }

    result
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_sensor_type_serialization() {
    let voltage = SensorType::Voltage;
    let json = serde_json::to_string(&voltage).unwrap();
    assert_eq!(json, "\"Voltage\"");

    let deserialized: SensorType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, SensorType::Voltage);
  }

  #[test]
  fn test_numeric_value_extraction() {
    let mut node = LibreHardwareMonitorNode {
      id: 1,
      text: "Test".to_string(),
      min: "".to_string(),
      value: "1.136 V".to_string(),
      max: "".to_string(),
      image_url: "".to_string(),
      sensor_id: Some("/test/voltage/0".to_string()),
      sensor_type: Some(SensorType::Voltage),
      children: Vec::new(),
    };

    assert_eq!(node.get_numeric_value(), Some(1.136));
    assert_eq!(node.get_unit(), Some("V"));

    node.value = "33.333".to_string();
    assert_eq!(node.get_numeric_value(), Some(33.333));
    assert_eq!(node.get_unit(), None);

    node.value = "".to_string();
    assert_eq!(node.get_numeric_value(), None);
    assert_eq!(node.get_unit(), None);
  }

  #[test]
  fn test_find_by_sensor_id() {
    let child = LibreHardwareMonitorNode {
      id: 2,
      text: "Child".to_string(),
      min: "".to_string(),
      value: "1.0 V".to_string(),
      max: "".to_string(),
      image_url: "".to_string(),
      sensor_id: Some("/test/voltage/0".to_string()),
      sensor_type: Some(SensorType::Voltage),
      children: Vec::new(),
    };

    let root = LibreHardwareMonitorNode {
      id: 1,
      text: "Root".to_string(),
      min: "".to_string(),
      value: "".to_string(),
      max: "".to_string(),
      image_url: "".to_string(),
      sensor_id: None,
      sensor_type: None,
      children: vec![child],
    };

    let found = root.find_by_sensor_id("/test/voltage/0");
    assert!(found.is_some());
    assert_eq!(found.unwrap().text, "Child");

    let not_found = root.find_by_sensor_id("/nonexistent");
    assert!(not_found.is_none());
  }

  #[test]
  fn test_find_by_sensor_type() {
    let voltage_child = LibreHardwareMonitorNode {
      id: 2,
      text: "Voltage Child".to_string(),
      min: "".to_string(),
      value: "1.0 V".to_string(),
      max: "".to_string(),
      image_url: "".to_string(),
      sensor_id: Some("/test/voltage/0".to_string()),
      sensor_type: Some(SensorType::Voltage),
      children: Vec::new(),
    };

    let temp_child = LibreHardwareMonitorNode {
      id: 3,
      text: "Temperature Child".to_string(),
      min: "".to_string(),
      value: "30.0 °C".to_string(),
      max: "".to_string(),
      image_url: "".to_string(),
      sensor_id: Some("/test/temperature/0".to_string()),
      sensor_type: Some(SensorType::Temperature),
      children: Vec::new(),
    };

    let root = LibreHardwareMonitorNode {
      id: 1,
      text: "Root".to_string(),
      min: "".to_string(),
      value: "".to_string(),
      max: "".to_string(),
      image_url: "".to_string(),
      sensor_id: None,
      sensor_type: None,
      children: vec![voltage_child, temp_child],
    };

    let voltage_nodes = root.find_by_sensor_type(&SensorType::Voltage);
    assert_eq!(voltage_nodes.len(), 1);
    assert_eq!(voltage_nodes[0].text, "Voltage Child");

    let temp_nodes = root.find_by_sensor_type(&SensorType::Temperature);
    assert_eq!(temp_nodes.len(), 1);
    assert_eq!(temp_nodes[0].text, "Temperature Child");
  }

  #[test]
  fn test_sensor_api_response_serialization() {
    let response = SensorApiResponse {
      result: SensorApiResult::Ok,
      value: Some(25.5),
      min: Some(20.0),
      max: Some(30.0),
      format: Some("{0:F1} °C".to_string()),
      message: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: SensorApiResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.result, SensorApiResult::Ok);
    assert_eq!(deserialized.value, Some(25.5));
    assert_eq!(deserialized.format, Some("{0:F1} °C".to_string()));
  }

  #[test]
  fn test_sensor_api_request_serialization() {
    let request = SensorApiRequest {
      action: SensorApiAction::Get,
      id: "/cpu/0/temperature/0".to_string(),
      value: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: SensorApiRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.action, SensorApiAction::Get);
    assert_eq!(deserialized.id, "/cpu/0/temperature/0");
    assert_eq!(deserialized.value, None);
  }

  #[test]
  fn test_http_server_config_serialization() {
    let config = HttpServerConfig {
      listener_ip: "127.0.0.1".to_string(),
      listener_port: 8085,
      authentication_enabled: false,
      authentication_user_name: None,
      authentication_password: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: HttpServerConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.listener_ip, "127.0.0.1");
    assert_eq!(deserialized.listener_port, 8085);
    assert!(!deserialized.authentication_enabled);
  }

  #[test]
  fn test_new_sensor_types() {
    let sensor_types = vec![
      SensorType::Current,
      SensorType::Flow,
      SensorType::Noise,
      SensorType::Conductivity,
      SensorType::Humidity,
    ];

    for sensor_type in sensor_types {
      let json = serde_json::to_string(&sensor_type).unwrap();
      let deserialized: SensorType = serde_json::from_str(&json).unwrap();
      assert_eq!(deserialized, sensor_type);
    }
  }
}
