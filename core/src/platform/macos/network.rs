use std::{
  collections::HashMap,
  net::{Ipv4Addr, Ipv6Addr},
};

use crate::{
  enums::error::BackendError, infrastructure, models::hardware::NetworkInfo, utils::ip,
};

type GatewayV4ByIfIndex = HashMap<u32, Vec<Ipv4Addr>>;
type GatewayV6ByIfIndex = HashMap<u32, Vec<Ipv6Addr>>;

/// Returns network interfaces for macOS as `NetworkInfo`.
///
/// This function is the aggregation/policy layer:
/// - pulls raw interface facts (name, flags, MAC, IPs) from the provider
/// - pulls default gateways (v4/v6) from PF_ROUTE via the provider
/// - filters to active, non-loopback interfaces and formats fields for the UI
pub fn get_network_info() -> Result<Vec<NetworkInfo>, BackendError> {
  let raw = infrastructure::providers::net_sys::get_raw_interfaces()
    .map_err(|_| BackendError::UnexpectedError)?;
  let (gw_v4_by_index, gw_v6_by_index) =
    infrastructure::providers::net_sys::get_default_gateways_by_ifindex()
      .map_err(|_| BackendError::UnexpectedError)?;

  Ok(create_network_info(raw, &gw_v4_by_index, &gw_v6_by_index))
}

fn create_network_info(
  raw: Vec<infrastructure::providers::net_sys::RawInterface>,
  gw_v4_by_index: &GatewayV4ByIfIndex,
  gw_v6_by_index: &GatewayV6ByIfIndex,
) -> Vec<NetworkInfo> {
  let mut result: Vec<NetworkInfo> = raw
    .into_iter()
    .filter(|iface| is_interface_active(iface.flags))
    .filter_map(|iface| {
      let (ipv4, ip_subnet) = collect_ipv4(&iface);
      let (ipv6, link_local_ipv6) = collect_ipv6(&iface);

      let has_ipv4 = !ipv4.is_empty();
      let has_global_ipv6 = !ipv6.is_empty();
      if !(has_ipv4 || has_global_ipv6) {
        return None;
      }

      let (default_ipv4_gateway, default_ipv6_gateway) = match iface.if_index {
        Some(idx) => (
          gw_v4_by_index
            .get(&idx)
            .map(|gws| gws.iter().map(|ip| ip.to_string()).collect())
            .unwrap_or_default(),
          gw_v6_by_index
            .get(&idx)
            .map(|gws| gws.iter().map(|ip| ip.to_string()).collect())
            .unwrap_or_default(),
        ),
        None => (vec![], vec![]),
      };

      Some(NetworkInfo {
        description: Some(iface.name),
        mac_address: iface.mac_address,
        ipv4,
        ipv6,
        link_local_ipv6,
        ip_subnet,
        default_ipv4_gateway,
        default_ipv6_gateway,
      })
    })
    .collect();

  result.sort_by(|a, b| a.description.cmp(&b.description));
  result
}

/// Returns whether an interface should be considered active.
///
/// We treat an interface as active when it is UP + RUNNING and not LOOPBACK.
fn is_interface_active(flags: u32) -> bool {
  const IFF_UP: u32 = libc::IFF_UP as u32;
  const IFF_RUNNING: u32 = libc::IFF_RUNNING as u32;
  const IFF_LOOPBACK: u32 = libc::IFF_LOOPBACK as u32;

  (flags & IFF_UP != 0) && (flags & IFF_RUNNING != 0) && (flags & IFF_LOOPBACK == 0)
}

/// Collects display-ready IPv4 addresses and subnets for a single interface.
///
/// - Filters out loopback, link-local, and unspecified addresses.
/// - Formats subnets as `{ip}/{prefix}` (CIDR prefix length from netmask).
fn collect_ipv4(
  iface: &infrastructure::providers::net_sys::RawInterface,
) -> (Vec<String>, Vec<String>) {
  let mut ips = Vec::new();
  let mut subnets = Vec::new();

  for (ip, prefix) in &iface.ipv4 {
    if ip.is_loopback() || ip.is_link_local() || *ip == Ipv4Addr::UNSPECIFIED {
      continue;
    }
    ips.push(ip.to_string());
    subnets.push(format!("{ip}/{prefix}"));
  }

  (ips, subnets)
}

/// Collects display-ready IPv6 addresses for a single interface.
///
/// - Splits global IPv6 vs link-local IPv6 into separate fields.
/// - Filters out loopback.
fn collect_ipv6(
  iface: &infrastructure::providers::net_sys::RawInterface,
) -> (Vec<String>, Vec<String>) {
  let mut global = Vec::new();
  let mut link_local = Vec::new();

  for ip6 in &iface.ipv6 {
    if ip6.is_loopback() {
      continue;
    }
    if ip::is_unicast_link_local(ip6) {
      link_local.push(ip6.to_string());
    } else {
      global.push(ip6.to_string());
    }
  }

  (global, link_local)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
  use super::*;

  fn raw_iface(
    name: &str,
    flags: u32,
    if_index: Option<u32>,
    ipv4: Vec<(Ipv4Addr, u32)>,
    ipv6: Vec<Ipv6Addr>,
  ) -> infrastructure::providers::net_sys::RawInterface {
    infrastructure::providers::net_sys::RawInterface {
      name: name.to_string(),
      flags,
      if_index,
      mac_address: None,
      ipv4,
      ipv6,
    }
  }

  #[test]
  fn active_filter_requires_up_and_running() {
    let flags = libc::IFF_UP as u32;
    assert!(!is_interface_active(flags));

    let flags = (libc::IFF_UP as u32) | (libc::IFF_RUNNING as u32);
    assert!(is_interface_active(flags));

    let flags =
      (libc::IFF_UP as u32) | (libc::IFF_RUNNING as u32) | (libc::IFF_LOOPBACK as u32);
    assert!(!is_interface_active(flags));
  }

  #[test]
  fn collect_ipv4_filters_loopback_link_local_and_unspecified() {
    let iface = raw_iface(
      "en0",
      (libc::IFF_UP as u32) | (libc::IFF_RUNNING as u32),
      Some(1),
      vec![
        (Ipv4Addr::new(127, 0, 0, 1), 8),
        (Ipv4Addr::new(169, 254, 10, 20), 16),
        (Ipv4Addr::UNSPECIFIED, 0),
        (Ipv4Addr::new(192, 168, 0, 2), 24),
      ],
      vec![],
    );

    let (ipv4, ip_subnet) = collect_ipv4(&iface);
    assert_eq!(ipv4, vec!["192.168.0.2".to_string()]);
    assert_eq!(ip_subnet, vec!["192.168.0.2/24".to_string()]);
  }

  #[test]
  fn collect_ipv6_splits_global_and_link_local_and_filters_loopback() {
    let iface = raw_iface(
      "en0",
      (libc::IFF_UP as u32) | (libc::IFF_RUNNING as u32),
      Some(1),
      vec![],
      vec![
        Ipv6Addr::LOCALHOST,
        Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1),
        Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
      ],
    );

    let (ipv6, link_local_ipv6) = collect_ipv6(&iface);
    assert_eq!(ipv6, vec!["2001:db8::1".to_string()]);
    assert_eq!(link_local_ipv6, vec!["fe80::1".to_string()]);
  }

  #[test]
  fn create_network_info_excludes_interface_with_only_link_local_ipv6() {
    let raw = vec![raw_iface(
      "en0",
      (libc::IFF_UP as u32) | (libc::IFF_RUNNING as u32),
      Some(1),
      vec![],
      vec![Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)],
    )];

    let gw_v4 = GatewayV4ByIfIndex::new();
    let gw_v6 = GatewayV6ByIfIndex::new();
    let result = create_network_info(raw, &gw_v4, &gw_v6);
    assert!(result.is_empty());
  }

  #[test]
  fn create_network_info_attaches_gateways_and_sorts_by_description() {
    let raw = vec![
      raw_iface(
        "en1",
        (libc::IFF_UP as u32) | (libc::IFF_RUNNING as u32),
        Some(5),
        vec![(Ipv4Addr::new(192, 168, 1, 10), 24)],
        vec![],
      ),
      raw_iface(
        "en0",
        (libc::IFF_UP as u32) | (libc::IFF_RUNNING as u32),
        None,
        vec![(Ipv4Addr::new(10, 0, 0, 2), 24)],
        vec![],
      ),
    ];

    let mut gw_v4 = GatewayV4ByIfIndex::new();
    gw_v4.insert(5, vec![Ipv4Addr::new(192, 168, 1, 1)]);

    let mut gw_v6 = GatewayV6ByIfIndex::new();
    gw_v6.insert(5, vec![Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)]);

    let result = create_network_info(raw, &gw_v4, &gw_v6);
    assert_eq!(result.len(), 2);

    // Sorted by description (en0, en1)
    assert_eq!(result[0].description.as_deref(), Some("en0"));
    assert_eq!(result[0].default_ipv4_gateway, Vec::<String>::new());
    assert_eq!(result[0].default_ipv6_gateway, Vec::<String>::new());

    assert_eq!(result[1].description.as_deref(), Some("en1"));
    assert_eq!(
      result[1].default_ipv4_gateway,
      vec!["192.168.1.1".to_string()]
    );
    assert_eq!(result[1].default_ipv6_gateway, vec!["fe80::1".to_string()]);
  }
}
