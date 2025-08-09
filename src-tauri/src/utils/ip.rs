use std::net::IpAddr;

///
/// ## ipがユニキャストアドレスかどうかを判定
///
pub fn is_unicast_link_local<T>(ip: &T) -> bool
where
  T: Into<IpAddr> + Clone,
{
  match ip.clone().into() {
    IpAddr::V6(v6) => v6.segments()[0] & 0xffc0 == 0xfe80,
    _ => false,
  }
}
