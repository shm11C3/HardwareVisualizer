use std::collections::HashMap;
use std::net::IpAddr;
use std::process::Command;

use crate::structs::hardware::NetworkInfo;

///
/// ## ネットワーク情報を取得
///
pub fn get_network_info() -> Result<Vec<NetworkInfo>, String> {
  let mut interfaces = collect_interfaces_with_mac()?;
  populate_ip_addresses(&mut interfaces)?;
  let (ipv4_gw, ipv6_gw) = get_default_gateways()?;

  for net in interfaces.values_mut() {
    net.default_ipv4_gateway = ipv4_gw.clone();
    net.default_ipv6_gateway = ipv6_gw.clone();
  }

  Ok(interfaces.into_values().collect())
}

///
/// ## MACアドレスとinterface名を収集
///
fn collect_interfaces_with_mac() -> Result<HashMap<String, NetworkInfo>, String> {
  let mut map = HashMap::new();

  for entry in std::fs::read_dir("/sys/class/net").map_err(|e| e.to_string())? {
    let entry = entry.map_err(|e| e.to_string())?;
    let iface = entry.file_name().into_string().unwrap_or_default();

    let mac_path = format!("/sys/class/net/{iface}/address");
    let mac_address = std::fs::read_to_string(mac_path)
      .ok()
      .map(|s| s.trim().to_string());

    map.insert(
      iface.clone(),
      NetworkInfo {
        description: Some(iface),
        mac_address,
        ipv4: vec![],
        ipv6: vec![],
        link_local_ipv6: vec![],
        ip_subnet: vec![],
        default_ipv4_gateway: vec![],
        default_ipv6_gateway: vec![],
      },
    );
  }

  Ok(map)
}

///
/// ## IPアドレスとサブネット、リンクローカルIPv6を収集
///
fn populate_ip_addresses(map: &mut HashMap<String, NetworkInfo>) -> Result<(), String> {
  use regex::Regex;

  let iface_line_re = Regex::new(r"^\d+:\s+([\w\-\.@]+):").unwrap();

  let output = Command::new("ip")
    .arg("addr")
    .output()
    .map_err(|e| format!("Failed to run ip addr: {e}"))?;

  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut current_iface = None;

  for line in stdout.lines() {
    if let Some(caps) = iface_line_re.captures(line) {
      let iface = caps[1].to_string();
      current_iface = if map.contains_key(&iface) {
        Some(iface)
      } else {
        None
      };
    } else if let Some(iface) = &current_iface {
      if line.trim().starts_with("inet ") {
        if let Some(ip_str) = line.split_whitespace().nth(1) {
          if let Ok(addr) = ip_str.parse::<ipnet::IpNet>() {
            let iface_entry = map.get_mut(iface).unwrap();
            match addr.addr() {
              IpAddr::V4(v4) => {
                iface_entry.ipv4.push(v4.to_string());
                iface_entry.ip_subnet.push(ip_str.to_string());
              }
              IpAddr::V6(v6) => {
                if v6.is_unicast_link_local() {
                  iface_entry.link_local_ipv6.push(v6.to_string());
                } else {
                  iface_entry.ipv6.push(v6.to_string());
                }
              }
            }
          }
        }
      }
    }
  }

  Ok(())
}

///
/// ## デフォルトゲートウェイの取得（IPv4 & IPv6）
///
fn get_default_gateways() -> Result<(Vec<String>, Vec<String>), String> {
  let ipv4_gw = get_default_gateway("ip route", 2)?;
  let ipv6_gw = get_default_gateway("ip -6 route", 2)?;
  Ok((ipv4_gw, ipv6_gw))
}

fn get_default_gateway(cmd: &str, ip_pos: usize) -> Result<Vec<String>, String> {
  let parts: Vec<&str> = cmd.split_whitespace().collect();
  let output = Command::new(parts[0])
    .args(&parts[1..])
    .output()
    .map_err(|e| e.to_string())?;

  let text = String::from_utf8_lossy(&output.stdout);
  Ok(
    text
      .lines()
      .filter(|l| l.starts_with("default via "))
      .filter_map(|line| line.split_whitespace().nth(ip_pos))
      .map(|s| s.to_string())
      .collect(),
  )
}
