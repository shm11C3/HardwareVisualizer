use crate::structs::hardware::NetworkInfo;
use crate::utils;
use crate::{log_debug, log_error, log_internal};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::net::IpAddr;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use wmi::{COMLibrary, WMIConnection};

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

pub fn get_network_info() -> Result<Vec<NetworkInfo>, String> {
  let results: Vec<NetworkAdapterConfiguration> = wmi_query_in_thread(
    "SELECT Description, MACAddress, IPAddress, IPSubnet, DefaultIPGateway FROM Win32_NetworkAdapterConfiguration WHERE IPEnabled = TRUE".to_string(),
  )?;

  log_debug!(
    &format!("Network adapter configuration data: {results:?}"),
    "get_network_info",
    None::<&str>
  );

  let network_info: Result<Vec<NetworkInfo>, String> = results
    .into_iter()
    .map(|adapter| -> Result<NetworkInfo, String> {
      // IPv4とIPv6を分ける
      let (ipv4, ipv6): (Vec<_>, Vec<_>) = adapter
        .ip_address
        .unwrap_or_default()
        .into_iter()
        .filter_map(|ip| ip.parse::<IpAddr>().ok())
        .partition(|ip| matches!(ip, IpAddr::V4(_))); // IPv4とIPv6に分ける

      // IPv6を分割する
      let (link_local_ipv6, normal_ipv6): (Vec<_>, Vec<_>) =
        ipv6.into_iter().partition(|ip| match ip {
          IpAddr::V6(v6) if utils::ip::is_unicast_link_local(v6) => true, // リンクローカル
          _ => false,
        });

      // IPv4サブネットを取得
      let ipv4_subnet: Vec<String> = adapter
        .ip_subnet
        .unwrap_or_default()
        .into_iter()
        .filter(|subnet| subnet.contains('.')) // IPv4形式を確認
        .collect();

      // IPv4とIPv6のデフォルトゲートウェイを分割する
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

///
/// ## 別スレッドでWMIクエリ実行する
///
fn wmi_query_in_thread<T>(query: String) -> Result<Vec<T>, String>
where
  T: DeserializeOwned + std::fmt::Debug + Send + 'static,
{
  type ResultChannel<T> = Result<Vec<T>, String>;
  type SenderChannel<T> = Sender<ResultChannel<T>>;
  type ReceiverChannel<T> = Receiver<ResultChannel<T>>;

  let (tx, rx): (SenderChannel<T>, ReceiverChannel<T>) = channel();

  // 別スレッドを起動してWMIクエリを実行
  thread::spawn(move || {
    let result = (|| {
      let com_con = COMLibrary::new()
        .map_err(|e| format!("Failed to initialize COM Library: {e:?}"))?;
      let wmi_con = WMIConnection::new(com_con)
        .map_err(|e| format!("Failed to create WMI connection: {e:?}"))?;

      // WMIクエリを実行してメモリ情報を取得
      let results: Vec<T> = wmi_con
        .raw_query(query)
        .map_err(|e| format!("Failed to execute query: {e:?}"))?;

      Ok(results)
    })();

    // メインスレッドに結果を送信
    if let Err(err) = tx.send(result) {
      log_error!(
        "Failed to send data from thread",
        "get_wmi_data_in_thread",
        Some(err.to_string())
      );
    }
  });

  // メインスレッドで結果を受信
  rx.recv()
    .map_err(|_| "Failed to receive data from thread".to_string())?
}
