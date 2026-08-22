use crate::enums;
use crate::models::hardware::NetworkInfo;
use hardviz_core::enums::error::PlatformError;
use hardviz_core::platform::factory::PlatformFactory;

///
/// Get network interface information.
///
/// Core reports failures as [`PlatformError`]; this service translates
/// them into the frontend-stable wire variants.
///
pub fn fetch_network_info() -> Result<Vec<NetworkInfo>, enums::error::BackendError> {
  let platform =
    PlatformFactory::create().map_err(|_| enums::error::BackendError::UnexpectedError)?;
  let core_list = platform.get_network_info().map_err(wire_network_error)?;
  Ok(core_list.into_iter().map(NetworkInfo::from).collect())
}

/// Translate a Core platform error into the network wire variant.
///
/// Absence (unsupported/unavailable) maps to `NetworkInfoNotAvailable`;
/// faults and initialization failures stay `UnexpectedError`, matching
/// the pre-typed wire behavior.
fn wire_network_error(error: PlatformError) -> enums::error::BackendError {
  match error {
    PlatformError::Unsupported { .. } | PlatformError::Unavailable { .. } => {
      enums::error::BackendError::NetworkInfoNotAvailable
    }
    PlatformError::Fault { .. } | PlatformError::InitializationFailed { .. } => {
      enums::error::BackendError::UnexpectedError
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn absence_maps_to_network_info_not_available() {
    assert_eq!(
      wire_network_error(PlatformError::unsupported("stub")),
      enums::error::BackendError::NetworkInfoNotAvailable
    );
    assert_eq!(
      wire_network_error(PlatformError::unavailable("no interface")),
      enums::error::BackendError::NetworkInfoNotAvailable
    );
  }

  #[test]
  fn faults_stay_unexpected_error() {
    assert_eq!(
      wire_network_error(PlatformError::fault("provider failed")),
      enums::error::BackendError::UnexpectedError
    );
    assert_eq!(
      wire_network_error(PlatformError::initialization_failed("boom")),
      enums::error::BackendError::UnexpectedError
    );
  }
}
