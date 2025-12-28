use std::process::Command;

pub fn get_network_interfaces() -> Result<Vec<String>, String> {
  let output = Command::new("networksetup")
    .arg("-listallhardwareports")
    .output()
    .map_err(|e| format!("Failed to execute networksetup: {e}"))?;

  if !output.status.success() {
    return Err("networksetup command failed".to_string());
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  let mut interfaces = Vec::new();
  for line in output_str.lines() {
    let trimmed_line = line.trim();
    if let Some((key, device)) = trimmed_line.split_once(':')
      && key.trim() == "Device"
    {
      let device_name = device.trim().to_string();
      if !device_name.is_empty() {
        interfaces.push(device_name);
      }
    }
  }

  Ok(interfaces)
}

pub fn get_interface_info(
  interface: &str,
  default_ipv4_gateway: &[String],
  default_ipv6_gateway: &[String],
) -> Result<crate::models::hardware::NetworkInfo, String> {
  let output = Command::new("ifconfig")
    .arg(interface)
    .output()
    .map_err(|e| format!("Failed to execute ifconfig: {e}"))?;

  if !output.status.success() {
    return Err(format!("ifconfig command failed for {interface}"));
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  let mut mac_address: Option<String> = None;
  let mut ipv4_addresses = Vec::new();
  let mut ipv6_addresses = Vec::new();
  let mut link_local_ipv6 = Vec::new();
  let mut ip_subnet = Vec::new();

  for line in output_str.lines() {
    let trimmed = line.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();

    match parts.first() {
      Some(&"ether") => {
        mac_address = parts.get(1).map(|s| s.to_string());
      }
      Some(&"inet") => {
        if let Some(&ip) = parts.get(1) {
          ipv4_addresses.push(ip.to_string());

          // Parse subnet information inline
          if parts.len() >= 4
            && let (Some(&"netmask"), Some(&netmask)) = (parts.get(2), parts.get(3))
            && let Some(subnet) = calculate_subnet(ip, netmask)
          {
            ip_subnet.push(subnet);
          }
        }
      }
      Some(&"inet6") => {
        if let Some(&ip) = parts.get(1) {
          let ip_str = ip.to_string();
          if ip_str.starts_with("fe80:") {
            link_local_ipv6.push(ip_str);
          } else {
            ipv6_addresses.push(ip_str);
          }
        }
      }
      _ => {}
    }
  }

  Ok(crate::models::hardware::NetworkInfo {
    description: Some(
      get_interface_description(interface).unwrap_or_else(|_| interface.to_string()),
    ),
    mac_address,
    ipv4: ipv4_addresses,
    ipv6: ipv6_addresses,
    link_local_ipv6,
    ip_subnet,
    default_ipv4_gateway: default_ipv4_gateway.to_vec(),
    default_ipv6_gateway: default_ipv6_gateway.to_vec(),
  })
}

fn get_interface_description(interface: &str) -> Result<String, String> {
  let output = Command::new("networksetup")
    .arg("-listallhardwareports")
    .output()
    .map_err(|e| format!("Failed to execute networksetup: {e}"))?;

  if !output.status.success() {
    return Err("networksetup command failed".to_string());
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  let mut current_port: Option<String> = None;
  for line in output_str.lines() {
    if line.starts_with("Hardware Port:") {
      current_port = line.split(':').nth(1).map(|s| s.trim().to_string());
    } else if line.starts_with("Device:") {
      let Some(device) = line.split(':').nth(1) else {
        continue;
      };
      if device.trim() != interface {
        continue;
      }

      if let Some(port) = current_port {
        return Ok(port);
      }
    }
  }

  Ok(interface.to_string())
}

pub fn get_default_gateways() -> Result<(Vec<String>, Vec<String>), String> {
  let output = Command::new("netstat")
    .arg("-nr")
    .output()
    .map_err(|e| format!("Failed to execute netstat: {e}"))?;

  if !output.status.success() {
    return Err("netstat command failed".to_string());
  }

  let output_str = String::from_utf8(output.stdout)
    .map_err(|e| format!("Failed to parse output: {e}"))?;

  let mut ipv4_gateways = Vec::new();
  let mut ipv6_gateways = Vec::new();

  for line in output_str.lines() {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() >= 2 && parts[0] == "default" {
      let gateway = parts[1].to_string();
      if gateway.contains(':') {
        ipv6_gateways.push(gateway);
      } else if gateway.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        ipv4_gateways.push(gateway);
      }
    }
  }

  Ok((ipv4_gateways, ipv6_gateways))
}

fn calculate_subnet(ip: &str, netmask: &str) -> Option<String> {
  // Convert netmask from hexadecimal to CIDR notation
  if !netmask.starts_with("0x") {
    return None;
  }

  let mask_hex = &netmask[2..];
  let mask_value = u32::from_str_radix(mask_hex, 16).ok()?;
  let prefix_len = mask_value.count_ones();

  // Parse IP address
  let ip_parts: Vec<u8> = ip.split('.').filter_map(|s| s.parse().ok()).collect();

  if ip_parts.len() != 4 {
    return None;
  }

  // Calculate network address
  let ip_value = ((ip_parts[0] as u32) << 24)
    | ((ip_parts[1] as u32) << 16)
    | ((ip_parts[2] as u32) << 8)
    | (ip_parts[3] as u32);

  let network_value = ip_value & mask_value;

  let network_ip = format!(
    "{}.{}.{}.{}/{}",
    (network_value >> 24) & 0xFF,
    (network_value >> 16) & 0xFF,
    (network_value >> 8) & 0xFF,
    network_value & 0xFF,
    prefix_len
  );

  Some(network_ip)
}
