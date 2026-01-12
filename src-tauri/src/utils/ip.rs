use std::net::IpAddr;

///
/// ## Determine if IP is a unicast link-local address
///
#[allow(dead_code)]
pub fn is_unicast_link_local<T>(ip: &T) -> bool
where
  T: Into<IpAddr> + Clone,
{
  match ip.clone().into() {
    IpAddr::V6(v6) => v6.segments()[0] & 0xffc0 == 0xfe80,
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

  #[test]
  fn test_link_local_ipv6() {
    let ip = IpAddr::V6("fe80::1".parse::<Ipv6Addr>().unwrap());
    assert!(is_unicast_link_local(&ip));

    let ip = IpAddr::V6("2400:4051::1".parse::<Ipv6Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));

    let ip = IpAddr::V6("ff02::1".parse::<Ipv6Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));

    let ip = IpAddr::V6("::".parse::<Ipv6Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));
  }

  #[test]
  fn test_ipv4_not_link_local() {
    let ip = IpAddr::V4("192.168.1.1".parse::<Ipv4Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));

    let ip = IpAddr::V4("127.0.0.1".parse::<Ipv4Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));
  }

  #[test]
  fn test_with_direct_ipv6() {
    let ip = "fe80::1".parse::<Ipv6Addr>().unwrap();
    assert!(is_unicast_link_local(&ip));

    let ip = "2400:4051::1".parse::<Ipv6Addr>().unwrap();
    assert!(!is_unicast_link_local(&ip));
  }

  #[test]
  fn test_with_invalid_format() {
    let ip = IpAddr::V4("255.255.255.255".parse::<Ipv4Addr>().unwrap());
    assert!(!is_unicast_link_local(&ip));
  }
}
