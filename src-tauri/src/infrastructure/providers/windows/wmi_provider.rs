use crate::models::hardware::{MemoryInfo, MotherboardInfo, NetworkInfo};
use crate::utils;
use crate::utils::formatter;
use crate::{log_debug, log_error, log_internal};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::net::IpAddr;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use wmi::WMIConnection;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32PhysicalMemory {
  capacity: u64,
  speed: u32,
  memory_type: Option<u16>,
  smbios_memory_type: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32PhysicalMemoryArray {
  memory_devices: Option<u32>,
}

///
/// ## Get memory information
///
pub async fn query_memory_info() -> Result<MemoryInfo, String> {
  let physical_memory: Vec<Win32PhysicalMemory> = wmi_query_in_thread(
    "SELECT Capacity, Speed, MemoryType, SMBIOSMemoryType FROM Win32_PhysicalMemory"
      .to_string(),
  )?;

  let physical_memory_array: Vec<Win32PhysicalMemoryArray> =
    tokio::task::spawn_blocking(|| {
      wmi_query_in_thread(
        "SELECT MemoryDevices FROM Win32_PhysicalMemoryArray".to_string(),
      )
    })
    .await
    .map_err(|e| format!("Join error: {e}"))??;

  log_debug!(
    &format!("mem info: {physical_memory:?}"),
    "get_memory_info",
    None::<&str>
  );

  let memory_info = MemoryInfo {
    size: formatter::format_size(physical_memory.iter().map(|mem| mem.capacity).sum(), 1),
    clock: physical_memory[0].speed as u32,
    clock_unit: "MHz".to_string(),
    memory_count: physical_memory.len() as u32,
    total_slots: physical_memory_array[0].memory_devices.unwrap_or(0),
    memory_type: describe_memory_type_with_fallback(
      physical_memory[0].memory_type,
      physical_memory[0].smbios_memory_type,
    ),
    is_detailed: true,
  };

  Ok(memory_info)
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct NetworkAdapterConfiguration {
  description: Option<String>,
  #[serde(rename = "MACAddress")]
  mac_address: Option<String>,
  #[serde(rename = "IPAddress")]
  ip_address: Option<Vec<String>>,
  #[serde(rename = "IPSubnet")]
  ip_subnet: Option<Vec<String>>,
  #[serde(rename = "DefaultIPGateway")]
  default_ip_gateway: Option<Vec<String>>,
}

pub fn query_network_info() -> Result<Vec<NetworkInfo>, String> {
  let results: Vec<NetworkAdapterConfiguration> = wmi_query_in_thread(
    "SELECT Description, MACAddress, IPAddress, IPSubnet, DefaultIPGateway FROM Win32_NetworkAdapterConfiguration WHERE IPEnabled = TRUE".to_string(),
  )?;

  log_debug!(
    &format!("Network adapter configuration data: {results:?}"),
    "query_network_info",
    None::<&str>
  );

  let network_info: Result<Vec<NetworkInfo>, String> = results
    .into_iter()
    .map(|adapter| -> Result<NetworkInfo, String> {
      // Separate IPv4 and IPv6
      let (ipv4, ipv6): (Vec<_>, Vec<_>) = adapter
        .ip_address
        .unwrap_or_default()
        .into_iter()
        .filter_map(|ip| ip.parse::<IpAddr>().ok())
        .partition(|ip| matches!(ip, IpAddr::V4(_))); // Separate IPv4 and IPv6

      // Split IPv6 addresses
      let (link_local_ipv6, normal_ipv6): (Vec<_>, Vec<_>) =
        ipv6.into_iter().partition(|ip| match ip {
          IpAddr::V6(v6) if utils::ip::is_unicast_link_local(v6) => true, // Link-local
          _ => false,
        });

      // Get IPv4 subnet
      let ipv4_subnet: Vec<String> = adapter
        .ip_subnet
        .unwrap_or_default()
        .into_iter()
        .filter(|subnet| subnet.contains('.')) // Verify IPv4 format
        .collect();

      // Split default gateways into IPv4 and IPv6
      let (default_ipv4_gateway, default_ipv6_gateway): (Vec<_>, Vec<_>) = adapter
        .default_ip_gateway
        .unwrap_or_default()
        .into_iter()
        .filter_map(|ip| ip.parse::<IpAddr>().ok())
        .partition(|ip| matches!(ip, IpAddr::V4(_)));

      Ok(NetworkInfo {
        description: Some(adapter.description.unwrap_or_default()),
        mac_address: Some(adapter.mac_address.unwrap_or_default()),
        ipv4: ipv4.into_iter().map(|ip| ip.to_string()).collect(),
        ipv6: normal_ipv6.into_iter().map(|ip| ip.to_string()).collect(),
        link_local_ipv6: link_local_ipv6
          .into_iter()
          .map(|ip| ip.to_string())
          .collect(),
        ip_subnet: ipv4_subnet,
        default_ipv4_gateway: default_ipv4_gateway
          .into_iter()
          .map(|ip| ip.to_string())
          .collect(),
        default_ipv6_gateway: default_ipv6_gateway
          .into_iter()
          .map(|ip| ip.to_string())
          .collect(),
      })
    })
    .collect();

  network_info
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32BaseBoard {
  manufacturer: Option<String>,
  product: Option<String>,
  version: Option<String>,
  serial_number: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Win32Bios {
  manufacturer: Option<String>,
  #[serde(rename = "SMBIOSBIOSVersion")]
  smbios_bios_version: Option<String>,
  release_date: Option<String>,
}

///
/// ## Get motherboard and BIOS information
///
pub fn query_motherboard_info() -> Result<MotherboardInfo, String> {
  let baseboards: Vec<Win32BaseBoard> = wmi_query_in_thread(
    "SELECT Manufacturer, Product, Version, SerialNumber FROM Win32_BaseBoard"
      .to_string(),
  )?;

  let bios_list: Vec<Win32Bios> = wmi_query_in_thread(
    "SELECT Manufacturer, SMBIOSBIOSVersion, ReleaseDate FROM Win32_BIOS".to_string(),
  )?;

  let board = baseboards
    .into_iter()
    .next()
    .ok_or("No baseboard info found")?;
  let bios = bios_list.into_iter().next().ok_or("No BIOS info found")?;

  let bios_release_date = bios
    .release_date
    .as_deref()
    .and_then(format_wmi_datetime)
    .unwrap_or_default();

  Ok(MotherboardInfo {
    manufacturer: board.manufacturer.unwrap_or_default(),
    product: board.product.unwrap_or_default(),
    version: board.version.unwrap_or_default(),
    serial_number: board.serial_number.unwrap_or_default(),
    bios_vendor: bios.manufacturer.unwrap_or_default(),
    bios_version: bios.smbios_bios_version.unwrap_or_default(),
    bios_release_date,
  })
}

/// Format WMI datetime string (e.g. "20230101000000.000000+000") to "YYYY-MM-DD"
fn format_wmi_datetime(s: &str) -> Option<String> {
  if s.len() >= 8 {
    Some(format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8]))
  } else {
    None
  }
}

///
/// ## Execute WMI query in separate thread
///
fn wmi_query_in_thread<T>(query: String) -> Result<Vec<T>, String>
where
  T: DeserializeOwned + std::fmt::Debug + Send + 'static,
{
  type ResultChannel<T> = Result<Vec<T>, String>;
  type SenderChannel<T> = Sender<ResultChannel<T>>;
  type ReceiverChannel<T> = Receiver<ResultChannel<T>>;

  let (tx, rx): (SenderChannel<T>, ReceiverChannel<T>) = channel();

  // Spawn separate thread to execute WMI query
  thread::spawn(move || {
    let result = (|| {
      let wmi_con = WMIConnection::new()
        .map_err(|e| format!("Failed to create WMI connection: {e:?}"))?;

      // Execute WMI query to get memory information
      let results: Vec<T> = wmi_con
        .raw_query(query)
        .map_err(|e| format!("Failed to execute query: {e:?}"))?;

      Ok(results)
    })();

    // Send result to main thread
    if let Err(err) = tx.send(result) {
      log_error!(
        "Failed to send data from thread",
        "get_wmi_data_in_thread",
        Some(err.to_string())
      );
    }
  });

  // Receive result in main thread
  rx.recv()
    .map_err(|_| "Failed to receive data from thread".to_string())?
}

///
/// ## Return memory type as string for MemoryType value
///
fn describe_memory_type(memory_type: Option<u16>) -> String {
  log_debug!(
    &format!("mem type: {memory_type:?}"),
    "get_memory_type_description",
    None::<&str>
  );

  match memory_type {
    Some(0) => "Unknown or Unsupported".to_string(),
    Some(1) => "Other".to_string(),
    Some(2) => "DRAM".to_string(),
    Some(3) => "Synchronous DRAM".to_string(),
    Some(4) => "Cache DRAM".to_string(),
    Some(5) => "EDO".to_string(),
    Some(6) => "EDRAM".to_string(),
    Some(7) => "VRAM".to_string(),
    Some(8) => "SRAM".to_string(),
    Some(9) => "RAM".to_string(),
    Some(10) => "ROM".to_string(),
    Some(11) => "Flash".to_string(),
    Some(12) => "EEPROM".to_string(),
    Some(13) => "FEPROM".to_string(),
    Some(14) => "EPROM".to_string(),
    Some(15) => "CDRAM".to_string(),
    Some(16) => "3DRAM".to_string(),
    Some(17) => "SDRAM".to_string(),
    Some(18) => "SGRAM".to_string(),
    Some(19) => "RDRAM".to_string(),
    Some(20) => "DDR".to_string(),
    Some(21) => "DDR2".to_string(),
    Some(22) => "DDR2 FB-DIMM".to_string(),
    Some(24) => "DDR3".to_string(),
    Some(25) => "FBD2".to_string(),
    Some(26) => "DDR4".to_string(),
    Some(mt) => format!("Other or Unknown Memory Type ({mt})"),
    None => "Unknown".to_string(),
  }
}

///
/// ## Get memory type from MemoryType or SMBIOSMemoryType
///
fn describe_memory_type_with_fallback(
  memory_type: Option<u16>,
  smbios_memory_type: Option<u16>,
) -> String {
  match memory_type {
    Some(0) => match smbios_memory_type {
      Some(20) => "DDR".to_string(),
      Some(21) => "DDR2".to_string(),
      Some(24) => "DDR3".to_string(),
      Some(26) => "DDR4".to_string(),
      Some(34) => "DDR5".to_string(),
      Some(mt) => format!("Other SMBIOS Memory Type ({mt})"),
      None => "Unknown".to_string(),
    },
    Some(mt) => describe_memory_type(Some(mt)),
    None => "Unknown".to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── format_wmi_datetime ──

  #[test]
  fn format_wmi_datetime_typical() {
    let result = format_wmi_datetime("20230101000000.000000+000");
    assert_eq!(result, Some("2023-01-01".to_string()));
  }

  #[test]
  fn format_wmi_datetime_exact_8_chars() {
    let result = format_wmi_datetime("20240315");
    assert_eq!(result, Some("2024-03-15".to_string()));
  }

  #[test]
  fn format_wmi_datetime_too_short() {
    let result = format_wmi_datetime("2023");
    assert!(result.is_none());
  }

  #[test]
  fn format_wmi_datetime_empty() {
    let result = format_wmi_datetime("");
    assert!(result.is_none());
  }

  // ── describe_memory_type ──

  #[test]
  fn describe_memory_type_ddr4() {
    assert_eq!(describe_memory_type(Some(26)), "DDR4");
  }

  #[test]
  fn describe_memory_type_ddr3() {
    assert_eq!(describe_memory_type(Some(24)), "DDR3");
  }

  #[test]
  fn describe_memory_type_ddr2() {
    assert_eq!(describe_memory_type(Some(21)), "DDR2");
  }

  #[test]
  fn describe_memory_type_ddr() {
    assert_eq!(describe_memory_type(Some(20)), "DDR");
  }

  #[test]
  fn describe_memory_type_sdram() {
    assert_eq!(describe_memory_type(Some(17)), "SDRAM");
  }

  #[test]
  fn describe_memory_type_unknown_zero() {
    assert_eq!(describe_memory_type(Some(0)), "Unknown or Unsupported");
  }

  #[test]
  fn describe_memory_type_none() {
    assert_eq!(describe_memory_type(None), "Unknown");
  }

  #[test]
  fn describe_memory_type_unrecognized_value() {
    assert_eq!(
      describe_memory_type(Some(99)),
      "Other or Unknown Memory Type (99)"
    );
  }

  // ── describe_memory_type_with_fallback ──

  #[test]
  fn fallback_memory_type_none_returns_unknown() {
    assert_eq!(describe_memory_type_with_fallback(None, None), "Unknown");
  }

  #[test]
  fn fallback_memory_type_zero_with_smbios_ddr5() {
    assert_eq!(
      describe_memory_type_with_fallback(Some(0), Some(34)),
      "DDR5"
    );
  }

  #[test]
  fn fallback_memory_type_zero_with_smbios_ddr4() {
    assert_eq!(
      describe_memory_type_with_fallback(Some(0), Some(26)),
      "DDR4"
    );
  }

  #[test]
  fn fallback_memory_type_zero_with_smbios_ddr3() {
    assert_eq!(
      describe_memory_type_with_fallback(Some(0), Some(24)),
      "DDR3"
    );
  }

  #[test]
  fn fallback_memory_type_zero_with_smbios_ddr2() {
    assert_eq!(
      describe_memory_type_with_fallback(Some(0), Some(21)),
      "DDR2"
    );
  }

  #[test]
  fn fallback_memory_type_zero_with_smbios_ddr() {
    assert_eq!(describe_memory_type_with_fallback(Some(0), Some(20)), "DDR");
  }

  #[test]
  fn fallback_memory_type_zero_with_unknown_smbios() {
    assert_eq!(
      describe_memory_type_with_fallback(Some(0), Some(50)),
      "Other SMBIOS Memory Type (50)"
    );
  }

  #[test]
  fn fallback_memory_type_zero_with_no_smbios() {
    assert_eq!(describe_memory_type_with_fallback(Some(0), None), "Unknown");
  }

  #[test]
  fn fallback_memory_type_nonzero_uses_primary() {
    // When memory_type is non-zero, it should use describe_memory_type directly
    assert_eq!(
      describe_memory_type_with_fallback(Some(26), Some(34)),
      "DDR4"
    );
  }

  #[test]
  fn fallback_memory_type_nonzero_ignores_smbios() {
    // memory_type=24 (DDR3) should be used, not smbios_memory_type=34 (DDR5)
    assert_eq!(
      describe_memory_type_with_fallback(Some(24), Some(34)),
      "DDR3"
    );
  }
}
