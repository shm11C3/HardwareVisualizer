#[cfg(test)]
mod tests {
  use crate::utils::ip::*;
  use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

  #[test]
  fn test_link_local_ipv6() {
    // リンクローカルアドレス
    let ip = IpAddr::V6("fe80::1".parse::<Ipv6Addr>().unwrap());
    assert_eq!(is_unicast_link_local(&ip), true);

    // グローバルIPv6アドレス
    let ip = IpAddr::V6("2400:4051::1".parse::<Ipv6Addr>().unwrap());
    assert_eq!(is_unicast_link_local(&ip), false);

    // IPv6マルチキャスト
    let ip = IpAddr::V6("ff02::1".parse::<Ipv6Addr>().unwrap());
    assert_eq!(is_unicast_link_local(&ip), false);

    // 未指定アドレス
    let ip = IpAddr::V6("::".parse::<Ipv6Addr>().unwrap());
    assert_eq!(is_unicast_link_local(&ip), false);
  }

  #[test]
  fn test_ipv4_not_link_local() {
    // IPv4アドレス（リンクローカルではない）
    let ip = IpAddr::V4("192.168.1.1".parse::<Ipv4Addr>().unwrap());
    assert_eq!(is_unicast_link_local(&ip), false);

    // IPv4アドレス（リンクローカルではない）
    let ip = IpAddr::V4("127.0.0.1".parse::<Ipv4Addr>().unwrap());
    assert_eq!(is_unicast_link_local(&ip), false);
  }

  #[test]
  fn test_with_direct_ipv6() {
    // 直接Ipv6Addr型でテスト
    let ip = "fe80::1".parse::<Ipv6Addr>().unwrap();
    assert_eq!(is_unicast_link_local(&ip), true);

    let ip = "2400:4051::1".parse::<Ipv6Addr>().unwrap();
    assert_eq!(is_unicast_link_local(&ip), false);
  }

  #[test]
  fn test_with_invalid_format() {
    // 無効なアドレス文字列はパースエラーを引き起こすためここではテストしないが、
    // 型制約に沿ってテストする例
    let ip = IpAddr::V4("255.255.255.255".parse::<Ipv4Addr>().unwrap());
    assert_eq!(is_unicast_link_local(&ip), false);
  }
}
