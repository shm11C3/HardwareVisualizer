#[cfg(test)]
mod tests {
  use crate::utils::ip::*;
  use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

  #[test]
  fn test_link_local_ipv6() {
    // Link-local address
    let ip = IpAddr::V6("fe80::1".parse::<Ipv6Addr>().unwrap());
    assert!(is_unicast_link_local(&ip));

    // Global IPv6 address
    let ip = IpAddr::V6("2400:4051::1".parse::<Ipv6Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));

    // IPv6 multicast
    let ip = IpAddr::V6("ff02::1".parse::<Ipv6Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));

    // Unspecified address
    let ip = IpAddr::V6("::".parse::<Ipv6Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));
  }

  #[test]
  fn test_ipv4_not_link_local() {
    // IPv4 address (not link-local)
    let ip = IpAddr::V4("192.168.1.1".parse::<Ipv4Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));

    // IPv4 address (not link-local)
    let ip = IpAddr::V4("127.0.0.1".parse::<Ipv4Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));
  }

  #[test]
  fn test_with_direct_ipv6() {
    // Test directly with Ipv6Addr type
    let ip = "fe80::1".parse::<Ipv6Addr>().unwrap();
    assert!(is_unicast_link_local(&ip));

    let ip = "2400:4051::1".parse::<Ipv6Addr>().unwrap();
    assert!(!is_unicast_link_local(&ip));
  }

  #[test]
  fn test_with_invalid_format() {
    // Invalid address strings cause parse errors so we don't test them here,
    // but here's an example that follows type constraints
    let ip = IpAddr::V4("255.255.255.255".parse::<Ipv4Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));
  }
}
