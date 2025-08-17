use crate::enums;
use crate::platform::factory::PlatformFactory;
use crate::structs::hardware::NetworkInfo;

///
/// ネットワークインターフェイス情報を取得
/// Platform が未対応 / 失敗時は `BackendError::UnexpectedError`
///
pub fn fetch_network_info() -> Result<Vec<NetworkInfo>, enums::error::BackendError> {
  let platform =
    PlatformFactory::create().map_err(|_| enums::error::BackendError::UnexpectedError)?;
  platform
    .get_network_info()
    .map_err(|_| enums::error::BackendError::UnexpectedError)
}
