//! Contract v1 vocabulary: the only names Product Analytics can send.
//!
//! Every wire name is spelled out in a `match` instead of being derived from
//! the variant name, so the strings a reader finds here are exactly the ones
//! listed in `docs/user/product-analytics.md`, and no code path turns a
//! runtime string into a wire name.

use serde::{Deserialize, Serialize, Serializer};
use specta::Type;

/// One enumerated screen view or capability use. It carries no payload
/// beyond its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ProductEvent {
  ScreenView(Screen),
  Capability(Capability),
}

/// Mirrors the frontend `SelectedDisplayType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Screen {
  Dashboard,
  Performance,
  SystemSpecifications,
  Usage,
  CpuDetail,
  Insights,
  Settings,
}

/// A user-started action, counted when it starts rather than when it
/// succeeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Capability {
  HardwareReportExported,
  TrayWidgetOpened,
  ExternalComponentSetupStarted,
  InsightOpened(Insight),
}

/// Mirrors the frontend `InsightType`; its `main` tab is `Hardware`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Insight {
  Hardware,
  Cooling,
  Gpu,
  Process,
  Snapshot,
}

impl ProductEvent {
  pub(crate) fn wire_name(self) -> &'static str {
    match self {
      Self::ScreenView(screen) => match screen {
        Screen::Dashboard => "screen_view.dashboard",
        Screen::Performance => "screen_view.performance",
        Screen::SystemSpecifications => "screen_view.system_specifications",
        Screen::Usage => "screen_view.usage",
        Screen::CpuDetail => "screen_view.cpu_detail",
        Screen::Insights => "screen_view.insights",
        Screen::Settings => "screen_view.settings",
      },
      Self::Capability(capability) => match capability {
        Capability::HardwareReportExported => "capability.hardware_report_exported",
        Capability::TrayWidgetOpened => "capability.tray_widget_opened",
        Capability::ExternalComponentSetupStarted => {
          "capability.external_component_setup_started"
        }
        Capability::InsightOpened(insight) => match insight {
          Insight::Hardware => "capability.insight_opened.hardware",
          Insight::Cooling => "capability.insight_opened.cooling",
          Insight::Gpu => "capability.insight_opened.gpu",
          Insight::Process => "capability.insight_opened.process",
          Insight::Snapshot => "capability.insight_opened.snapshot",
        },
      },
    }
  }
}

/// An enumerated true fact about the installation on one UTC day. App
/// internal: the frontend can neither name nor send one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum StateFlag {
  FeatureOn(Feature),
  Navigation(Navigation),
  PerformanceView(PerformanceView),
  CpuVendor(CpuVendor),
  GpuVendor(GpuVendor),
  ProviderAvailable(Provider),
}

/// An Application Preference a user switches on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Feature {
  HardwareArchive,
  StorageHealth,
  TrayWidget,
  CloseToTray,
  ElevatedStartupMode,
  AmbientSensor,
  BurnInShift,
  BackgroundImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Navigation {
  Grouped,
  Classic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum PerformanceView {
  Panels,
  Compact,
  Monitor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum CpuVendor {
  Intel,
  Amd,
  Apple,
  Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum GpuVendor {
  Nvidia,
  Amd,
  Intel,
  Apple,
  Other,
}

/// An optional native sensor provider; both exist only on Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Provider {
  PawnIoCpu,
  SuperIo,
}

impl StateFlag {
  pub(crate) fn wire_name(self) -> &'static str {
    match self {
      Self::FeatureOn(feature) => match feature {
        Feature::HardwareArchive => "feature_on.hardware_archive",
        Feature::StorageHealth => "feature_on.storage_health",
        Feature::TrayWidget => "feature_on.tray_widget",
        Feature::CloseToTray => "feature_on.close_to_tray",
        Feature::ElevatedStartupMode => "feature_on.elevated_startup_mode",
        Feature::AmbientSensor => "feature_on.ambient_sensor",
        Feature::BurnInShift => "feature_on.burn_in_shift",
        Feature::BackgroundImage => "feature_on.background_image",
      },
      Self::Navigation(layout) => match layout {
        Navigation::Grouped => "navigation.grouped",
        Navigation::Classic => "navigation.classic",
      },
      Self::PerformanceView(view) => match view {
        PerformanceView::Panels => "view.performance.panels",
        PerformanceView::Compact => "view.performance.compact",
        PerformanceView::Monitor => "view.performance.monitor",
      },
      Self::CpuVendor(vendor) => match vendor {
        CpuVendor::Intel => "hardware.cpu_vendor.intel",
        CpuVendor::Amd => "hardware.cpu_vendor.amd",
        CpuVendor::Apple => "hardware.cpu_vendor.apple",
        CpuVendor::Other => "hardware.cpu_vendor.other",
      },
      Self::GpuVendor(vendor) => match vendor {
        GpuVendor::Nvidia => "hardware.gpu_vendor.nvidia",
        GpuVendor::Amd => "hardware.gpu_vendor.amd",
        GpuVendor::Intel => "hardware.gpu_vendor.intel",
        GpuVendor::Apple => "hardware.gpu_vendor.apple",
        GpuVendor::Other => "hardware.gpu_vendor.other",
      },
      Self::ProviderAvailable(provider) => match provider {
        Provider::PawnIoCpu => "provider_available.pawnio_cpu",
        Provider::SuperIo => "provider_available.superio",
      },
    }
  }
}

impl Serialize for StateFlag {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(self.wire_name())
  }
}

/// The `osFamily` batch field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OsFamily {
  Windows,
  Macos,
  Linux,
}

impl OsFamily {
  /// Evaluated at compile time, so a build for an OS the contract does not
  /// list fails instead of sending a new string.
  pub(crate) const CURRENT: Self = match std::env::consts::OS.as_bytes() {
    b"windows" => Self::Windows,
    b"macos" => Self::Macos,
    b"linux" => Self::Linux,
    _ => panic!("Product Analytics contract v1 has no osFamily for this target OS"),
  };
}

/// The `cpuArch` batch field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum CpuArch {
  #[serde(rename = "x86_64")]
  X86_64,
  #[serde(rename = "aarch64")]
  Aarch64,
}

impl CpuArch {
  /// Evaluated at compile time, like [`OsFamily::CURRENT`].
  pub(crate) const CURRENT: Self = match std::env::consts::ARCH.as_bytes() {
    b"x86_64" => Self::X86_64,
    b"aarch64" => Self::Aarch64,
    _ => {
      panic!("Product Analytics contract v1 has no cpuArch for this target architecture")
    }
  };
}

#[cfg(test)]
mod tests {
  use super::*;

  const SCREENS: [Screen; 7] = [
    Screen::Dashboard,
    Screen::Performance,
    Screen::SystemSpecifications,
    Screen::Usage,
    Screen::CpuDetail,
    Screen::Insights,
    Screen::Settings,
  ];
  const INSIGHTS: [Insight; 5] = [
    Insight::Hardware,
    Insight::Cooling,
    Insight::Gpu,
    Insight::Process,
    Insight::Snapshot,
  ];

  fn all_events() -> Vec<ProductEvent> {
    let mut events: Vec<_> = SCREENS.into_iter().map(ProductEvent::ScreenView).collect();
    events.extend(
      [
        Capability::HardwareReportExported,
        Capability::TrayWidgetOpened,
        Capability::ExternalComponentSetupStarted,
      ]
      .into_iter()
      .chain(INSIGHTS.into_iter().map(Capability::InsightOpened))
      .map(ProductEvent::Capability),
    );
    events
  }

  fn all_flags() -> Vec<StateFlag> {
    use Feature::*;
    let mut flags: Vec<_> = [
      HardwareArchive,
      StorageHealth,
      TrayWidget,
      CloseToTray,
      ElevatedStartupMode,
      AmbientSensor,
      BurnInShift,
      BackgroundImage,
    ]
    .into_iter()
    .map(StateFlag::FeatureOn)
    .collect();
    flags.extend([Navigation::Grouped, Navigation::Classic].map(StateFlag::Navigation));
    flags.extend(
      [
        PerformanceView::Panels,
        PerformanceView::Compact,
        PerformanceView::Monitor,
      ]
      .map(StateFlag::PerformanceView),
    );
    flags.extend(
      [
        CpuVendor::Intel,
        CpuVendor::Amd,
        CpuVendor::Apple,
        CpuVendor::Other,
      ]
      .map(StateFlag::CpuVendor),
    );
    flags.extend(
      [
        GpuVendor::Nvidia,
        GpuVendor::Amd,
        GpuVendor::Intel,
        GpuVendor::Apple,
        GpuVendor::Other,
      ]
      .map(StateFlag::GpuVendor),
    );
    flags
      .extend([Provider::PawnIoCpu, Provider::SuperIo].map(StateFlag::ProviderAvailable));
    flags
  }

  /// The Product Event list of `docs/user/product-analytics.md`.
  #[test]
  fn event_wire_names_match_the_public_contract() {
    let names: Vec<_> = all_events()
      .into_iter()
      .map(ProductEvent::wire_name)
      .collect();
    assert_eq!(
      names,
      [
        "screen_view.dashboard",
        "screen_view.performance",
        "screen_view.system_specifications",
        "screen_view.usage",
        "screen_view.cpu_detail",
        "screen_view.insights",
        "screen_view.settings",
        "capability.hardware_report_exported",
        "capability.tray_widget_opened",
        "capability.external_component_setup_started",
        "capability.insight_opened.hardware",
        "capability.insight_opened.cooling",
        "capability.insight_opened.gpu",
        "capability.insight_opened.process",
        "capability.insight_opened.snapshot",
      ]
    );
  }

  /// The State Flag list of `docs/user/product-analytics.md`.
  #[test]
  fn flag_wire_names_match_the_public_contract() {
    let names: Vec<_> = all_flags().into_iter().map(StateFlag::wire_name).collect();
    assert_eq!(
      names,
      [
        "feature_on.hardware_archive",
        "feature_on.storage_health",
        "feature_on.tray_widget",
        "feature_on.close_to_tray",
        "feature_on.elevated_startup_mode",
        "feature_on.ambient_sensor",
        "feature_on.burn_in_shift",
        "feature_on.background_image",
        "navigation.grouped",
        "navigation.classic",
        "view.performance.panels",
        "view.performance.compact",
        "view.performance.monitor",
        "hardware.cpu_vendor.intel",
        "hardware.cpu_vendor.amd",
        "hardware.cpu_vendor.apple",
        "hardware.cpu_vendor.other",
        "hardware.gpu_vendor.nvidia",
        "hardware.gpu_vendor.amd",
        "hardware.gpu_vendor.intel",
        "hardware.gpu_vendor.apple",
        "hardware.gpu_vendor.other",
        "provider_available.pawnio_cpu",
        "provider_available.superio",
      ]
    );
  }

  #[test]
  fn a_flag_serializes_as_its_wire_name() {
    let json =
      serde_json::to_string(&StateFlag::ProviderAvailable(Provider::SuperIo)).unwrap();
    assert_eq!(json, "\"provider_available.superio\"");
  }

  #[test]
  fn ipc_events_round_trip_through_their_enum_shape() {
    for event in all_events() {
      let json = serde_json::to_value(event).unwrap();
      assert_eq!(serde_json::from_value::<ProductEvent>(json).unwrap(), event);
    }
    let json = serde_json::json!({ "capability": { "insightOpened": "cooling" } });
    assert_eq!(
      serde_json::from_value::<ProductEvent>(json).unwrap(),
      ProductEvent::Capability(Capability::InsightOpened(Insight::Cooling))
    );
  }

  #[test]
  fn an_unlisted_event_name_is_rejected_at_the_ipc_boundary() {
    let json = serde_json::json!({ "screenView": "secretScreen" });
    assert!(serde_json::from_value::<ProductEvent>(json).is_err());
    assert!(
      serde_json::from_value::<ProductEvent>(serde_json::json!("screen_view.usage"))
        .is_err()
    );
  }

  #[test]
  fn platform_fields_serialize_as_contract_values() {
    assert_eq!(
      serde_json::to_string(&OsFamily::CURRENT).unwrap(),
      format!("\"{}\"", std::env::consts::OS)
    );
    assert_eq!(
      serde_json::to_string(&CpuArch::CURRENT).unwrap(),
      format!("\"{}\"", std::env::consts::ARCH)
    );
    assert_eq!(
      serde_json::to_string(&OsFamily::Macos).unwrap(),
      "\"macos\""
    );
    assert_eq!(
      serde_json::to_string(&CpuArch::X86_64).unwrap(),
      "\"x86_64\""
    );
  }
}
