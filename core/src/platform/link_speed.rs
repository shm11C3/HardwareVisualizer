//! Attributes OS-reported negotiated link rates to `NetworkInfo` entries.
//!
//! The rate source (`sysinfo`) and `NetworkInfo` do not share one identity on
//! every OS, so the caller picks the key that both sides really have. A rate is
//! attached only when the key identifies exactly one interface on each side;
//! otherwise it is left `None` rather than guessed (DP-02).

use std::collections::HashMap;

use crate::models::hardware::NetworkInfo;

/// Identity shared by `NetworkInfo` and the rate records on a given OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkSpeedMatchKey {
  /// `NetworkInfo::description` equals the kernel interface name (Linux, macOS).
  Name,
  /// `NetworkInfo::mac_address` equals the interface MAC (Windows, where the
  /// description is the adapter description, not the interface alias).
  Mac,
}

/// One interface's rates as read from the OS, before attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceLinkRate {
  pub name: String,
  pub mac: Option<String>,
  pub receive_bps: Option<u64>,
  pub transmit_bps: Option<u64>,
}

/// Lower-cases and strips separators; an empty or all-zero MAC is not an identity.
fn normalize_mac(mac: &str) -> Option<String> {
  let normalized: String = mac
    .chars()
    .filter(char::is_ascii_hexdigit)
    .map(|c| c.to_ascii_lowercase())
    .collect();
  if normalized.is_empty() || normalized.chars().all(|c| c == '0') {
    None
  } else {
    Some(normalized)
  }
}

fn info_key(info: &NetworkInfo, key: LinkSpeedMatchKey) -> Option<String> {
  match key {
    LinkSpeedMatchKey::Name => info.description.clone().filter(|d| !d.is_empty()),
    LinkSpeedMatchKey::Mac => info.mac_address.as_deref().and_then(normalize_mac),
  }
}

fn rate_key(rate: &InterfaceLinkRate, key: LinkSpeedMatchKey) -> Option<String> {
  match key {
    LinkSpeedMatchKey::Name => Some(rate.name.clone()).filter(|n| !n.is_empty()),
    LinkSpeedMatchKey::Mac => rate.mac.as_deref().and_then(normalize_mac),
  }
}

/// A reported rate of 0 means "not reported", never "0 bit/s".
fn nonzero(rate: Option<u64>) -> Option<u64> {
  rate.filter(|r| *r > 0)
}

/// Sets `receive_link_speed_bps` / `transmit_link_speed_bps` on every entry whose
/// key matches exactly one rate record, and only when no other entry shares that
/// key. Entries are never added, removed, or reordered.
pub fn attach_link_speeds(
  infos: &mut [NetworkInfo],
  rates: &[InterfaceLinkRate],
  key: LinkSpeedMatchKey,
) {
  let mut rates_by_key: HashMap<String, Vec<&InterfaceLinkRate>> = HashMap::new();
  for rate in rates {
    if let Some(k) = rate_key(rate, key) {
      rates_by_key.entry(k).or_default().push(rate);
    }
  }

  let mut info_counts: HashMap<String, usize> = HashMap::new();
  for info in infos.iter() {
    if let Some(k) = info_key(info, key) {
      *info_counts.entry(k).or_default() += 1;
    }
  }

  for info in infos.iter_mut() {
    let Some(k) = info_key(info, key) else {
      continue;
    };
    if info_counts.get(&k) != Some(&1) {
      continue;
    }
    if let Some([rate]) = rates_by_key.get(&k).map(Vec::as_slice) {
      info.receive_link_speed_bps = nonzero(rate.receive_bps);
      info.transmit_link_speed_bps = nonzero(rate.transmit_bps);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn info(description: &str, mac: Option<&str>) -> NetworkInfo {
    NetworkInfo {
      description: Some(description.to_string()),
      mac_address: mac.map(str::to_string),
      ipv4: vec![],
      ipv6: vec![],
      link_local_ipv6: vec![],
      ip_subnet: vec![],
      default_ipv4_gateway: vec![],
      default_ipv6_gateway: vec![],
      receive_link_speed_bps: None,
      transmit_link_speed_bps: None,
    }
  }

  fn rate(
    name: &str,
    mac: Option<&str>,
    rx: Option<u64>,
    tx: Option<u64>,
  ) -> InterfaceLinkRate {
    InterfaceLinkRate {
      name: name.to_string(),
      mac: mac.map(str::to_string),
      receive_bps: rx,
      transmit_bps: tx,
    }
  }

  #[test]
  fn name_key_attaches_rate_to_the_named_interface_only() {
    let mut infos = vec![info("eth0", None), info("wlan0", None)];
    let rates = [
      rate("eth0", None, Some(1_000_000_000), Some(1_000_000_000)),
      rate("wlan0", None, Some(866_000_000), Some(866_000_000)),
    ];
    attach_link_speeds(&mut infos, &rates, LinkSpeedMatchKey::Name);
    assert_eq!(infos[0].receive_link_speed_bps, Some(1_000_000_000));
    assert_eq!(infos[1].receive_link_speed_bps, Some(866_000_000));
  }

  #[test]
  fn mac_key_matches_across_separator_and_case_differences() {
    // WMI reports "AA:BB:..." while sysinfo prints lower-case colon form.
    let mut infos = vec![info("Intel(R) Ethernet", Some("AA:BB:CC:DD:EE:FF"))];
    let rates = [rate(
      "Ethernet",
      Some("aa-bb-cc-dd-ee-ff"),
      Some(2_500_000_000),
      Some(2_500_000_000),
    )];
    attach_link_speeds(&mut infos, &rates, LinkSpeedMatchKey::Mac);
    assert_eq!(infos[0].receive_link_speed_bps, Some(2_500_000_000));
    assert_eq!(infos[0].transmit_link_speed_bps, Some(2_500_000_000));
  }

  #[test]
  fn duplicate_mac_on_info_side_attaches_nothing_to_those_but_keeps_others() {
    let mut infos = vec![
      info("A", Some("aa:aa:aa:aa:aa:aa")),
      info("B", Some("AA-AA-AA-AA-AA-AA")),
      info("C", Some("cc:cc:cc:cc:cc:cc")),
    ];
    let rates = [
      rate("a", Some("aa:aa:aa:aa:aa:aa"), Some(1), Some(1)),
      rate("c", Some("cc:cc:cc:cc:cc:cc"), Some(5), Some(6)),
    ];
    attach_link_speeds(&mut infos, &rates, LinkSpeedMatchKey::Mac);
    assert_eq!(infos[0].receive_link_speed_bps, None);
    assert_eq!(infos[1].receive_link_speed_bps, None);
    assert_eq!(infos[2].receive_link_speed_bps, Some(5));
    assert_eq!(infos[2].transmit_link_speed_bps, Some(6));
  }

  #[test]
  fn duplicate_mac_on_rate_side_attaches_nothing_to_those_but_keeps_others() {
    let mut infos = vec![
      info("A", Some("aa:aa:aa:aa:aa:aa")),
      info("C", Some("cc:cc:cc:cc:cc:cc")),
    ];
    let rates = [
      rate("a1", Some("aa:aa:aa:aa:aa:aa"), Some(1), Some(1)),
      rate("a2", Some("aa:aa:aa:aa:aa:aa"), Some(2), Some(2)),
      rate("c", Some("cc:cc:cc:cc:cc:cc"), Some(5), Some(5)),
    ];
    attach_link_speeds(&mut infos, &rates, LinkSpeedMatchKey::Mac);
    assert_eq!(infos[0].receive_link_speed_bps, None);
    assert_eq!(infos[1].receive_link_speed_bps, Some(5));
  }

  #[test]
  fn all_zero_or_missing_mac_is_not_an_identity() {
    let mut infos = vec![info("A", Some("00:00:00:00:00:00")), info("B", None)];
    let rates = [
      rate("a", Some("00-00-00-00-00-00"), Some(9), Some(9)),
      rate("b", None, Some(9), Some(9)),
    ];
    attach_link_speeds(&mut infos, &rates, LinkSpeedMatchKey::Mac);
    assert_eq!(infos[0].receive_link_speed_bps, None);
    assert_eq!(infos[1].receive_link_speed_bps, None);
  }

  #[test]
  fn zero_rate_becomes_none() {
    let mut infos = vec![info("eth0", None)];
    let rates = [rate("eth0", None, Some(0), Some(0))];
    attach_link_speeds(&mut infos, &rates, LinkSpeedMatchKey::Name);
    assert_eq!(infos[0].receive_link_speed_bps, None);
    assert_eq!(infos[0].transmit_link_speed_bps, None);
  }

  #[test]
  fn asymmetric_rates_are_preserved_per_direction() {
    let mut infos = vec![info("Ethernet", Some("aa:bb:cc:dd:ee:01"))];
    let rates = [rate(
      "Ethernet",
      Some("aa:bb:cc:dd:ee:01"),
      Some(1_000_000_000),
      Some(100_000_000),
    )];
    attach_link_speeds(&mut infos, &rates, LinkSpeedMatchKey::Mac);
    assert_eq!(infos[0].receive_link_speed_bps, Some(1_000_000_000));
    assert_eq!(infos[0].transmit_link_speed_bps, Some(100_000_000));
  }

  #[test]
  fn unmatched_interfaces_are_kept_unchanged() {
    let mut infos = vec![info("eth0", None), info("tun0", None)];
    let before = infos.clone();
    attach_link_speeds(
      &mut infos,
      &[rate("eth1", None, Some(1), Some(1))],
      LinkSpeedMatchKey::Name,
    );
    assert_eq!(infos, before);
    attach_link_speeds(&mut infos, &[], LinkSpeedMatchKey::Name);
    assert_eq!(infos, before);
  }
}
