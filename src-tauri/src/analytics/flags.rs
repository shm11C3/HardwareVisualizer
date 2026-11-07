//! State Flag sampling: classifies settled facts into one complete flag set,
//! or none.
//!
//! A flag set is complete or absent. While any source is undetermined the
//! sample is `None`, so the day is missing from the Flag Census instead of
//! being counted as "off".

use hardviz_core::models::{OptionalSensorProviderOutcomes, ProviderAttemptOutcome};

use super::vocabulary::{
  CpuVendor, Feature, GpuVendor, Navigation, OsFamily, PerformanceView, Provider,
  StateFlag,
};
use crate::enums::settings::NavigationLayout;

/// Application Preference facts read from the settings service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreferenceFacts {
  pub(crate) features_on: Vec<Feature>,
  pub(crate) navigation: Navigation,
  pub(crate) performance_view: PerformanceView,
}

impl PreferenceFacts {
  pub(crate) fn from_settings(
    settings: &crate::models::settings::Settings,
    core_settings: &hardviz_core::settings::CoreSettings,
    performance_view: PerformanceView,
  ) -> Self {
    let features_on = [
      (
        Feature::HardwareArchive,
        core_settings.hardware_archive.enabled,
      ),
      (Feature::StorageHealth, core_settings.storage_health.enabled),
      (Feature::TrayWidget, settings.tray_widget.enabled),
      (Feature::CloseToTray, settings.close_to_tray),
      (Feature::ElevatedStartupMode, settings.elevated_startup_mode),
      (
        Feature::AmbientSensor,
        core_settings.environmental_sensors.switchbot_meter_enabled,
      ),
      (Feature::BurnInShift, settings.burn_in_shift),
      (
        Feature::BackgroundImage,
        settings.selected_background_img.is_some(),
      ),
    ]
    .into_iter()
    .filter_map(|(feature, on)| on.then_some(feature))
    .collect();

    Self {
      features_on,
      navigation: match settings.navigation_layout {
        NavigationLayout::Grouped => Navigation::Grouped,
        NavigationLayout::Classic => Navigation::Classic,
      },
      performance_view,
    }
  }
}

impl PerformanceView {
  /// Classifies the stored Performance view the way the frontend's
  /// `normalizePerformanceView` resolves it: the retired `detailed` and
  /// `custom` presets are Panels, and anything absent or unknown falls back
  /// to the default, Panels.
  pub(crate) fn from_stored(value: Option<&serde_json::Value>) -> Self {
    match value.and_then(serde_json::Value::as_str) {
      Some("compact") => Self::Compact,
      Some("monitor") => Self::Monitor,
      _ => Self::Panels,
    }
  }
}

/// Every source a flag set needs. `None` means not determined yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlagSources {
  pub(crate) preferences: PreferenceFacts,
  pub(crate) cpu_vendor: Option<CpuVendor>,
  /// The vendor of each enumerated GPU; empty when none was enumerated.
  pub(crate) gpu_vendors: Option<Vec<GpuVendor>>,
  /// Outcomes from the latest `MetricsSnapshot`; `None` before the first.
  pub(crate) providers: Option<OptionalSensorProviderOutcomes>,
}

/// Samples the day's complete flag set, or `None` while a source is
/// undetermined.
///
/// Provider flags exist only on Windows, the one OS family where the v1
/// providers do; elsewhere their outcomes are neither waited for nor read.
pub(crate) fn sample(
  sources: &FlagSources,
  os_family: OsFamily,
) -> Option<Vec<StateFlag>> {
  let cpu_vendor = sources.cpu_vendor?;
  let gpu_vendors = sources.gpu_vendors.as_ref()?;
  let providers = match os_family {
    OsFamily::Windows => available_providers(sources.providers.as_ref()?)?,
    OsFamily::Macos | OsFamily::Linux => Vec::new(),
  };

  let mut flags: Vec<StateFlag> = sources
    .preferences
    .features_on
    .iter()
    .copied()
    .map(StateFlag::FeatureOn)
    .collect();
  flags.push(StateFlag::Navigation(sources.preferences.navigation));
  flags.push(StateFlag::PerformanceView(
    sources.preferences.performance_view,
  ));
  flags.push(StateFlag::CpuVendor(cpu_vendor));
  flags.extend(gpu_vendors.iter().copied().map(StateFlag::GpuVendor));
  flags.extend(providers.into_iter().map(StateFlag::ProviderAvailable));
  flags.sort_unstable();
  flags.dedup();
  Some(flags)
}

/// The providers whose latest attempt answered, or `None` while one is
/// still pending. A provider the platform lacks (`None`) is never pending.
fn available_providers(
  outcomes: &OptionalSensorProviderOutcomes,
) -> Option<Vec<Provider>> {
  let mut available = Vec::new();
  for (provider, outcome) in [
    (Provider::PawnIoCpu, outcomes.cpu_package_temperature),
    (Provider::SuperIo, outcomes.motherboard_sensors),
  ] {
    match outcome {
      Some(ProviderAttemptOutcome::Pending) => return None,
      Some(ProviderAttemptOutcome::Answered) => available.push(provider),
      Some(ProviderAttemptOutcome::NotAnswered) | None => {}
    }
  }
  Some(available)
}

/// Classifies a CPU vendor string from the App's CPU info. Every string the
/// mapping does not name is `Other`, so a raw vendor never reaches the wire.
pub(crate) fn cpu_vendor_class(vendor: &str) -> CpuVendor {
  match vendor.trim().to_ascii_lowercase().as_str() {
    "intel" | "genuineintel" => CpuVendor::Intel,
    "amd" | "authenticamd" => CpuVendor::Amd,
    "apple" => CpuVendor::Apple,
    _ => CpuVendor::Other,
  }
}

/// Classifies a GPU vendor name from the App's GPU info, like
/// [`cpu_vendor_class`].
pub(crate) fn gpu_vendor_class(vendor_name: &str) -> GpuVendor {
  match vendor_name.trim().to_ascii_lowercase().as_str() {
    "nvidia" => GpuVendor::Nvidia,
    "amd" => GpuVendor::Amd,
    "intel" => GpuVendor::Intel,
    "apple" => GpuVendor::Apple,
    _ => GpuVendor::Other,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use ProviderAttemptOutcome::{Answered, NotAnswered, Pending};

  fn preferences() -> PreferenceFacts {
    PreferenceFacts {
      features_on: vec![Feature::TrayWidget, Feature::HardwareArchive],
      navigation: Navigation::Grouped,
      performance_view: PerformanceView::Panels,
    }
  }

  fn settled(
    cpu: Option<ProviderAttemptOutcome>,
    motherboard: Option<ProviderAttemptOutcome>,
  ) -> FlagSources {
    FlagSources {
      preferences: preferences(),
      cpu_vendor: Some(CpuVendor::Amd),
      gpu_vendors: Some(vec![GpuVendor::Nvidia, GpuVendor::Amd]),
      providers: Some(OptionalSensorProviderOutcomes {
        cpu_package_temperature: cpu,
        motherboard_sensors: motherboard,
      }),
    }
  }

  fn names(flags: &[StateFlag]) -> Vec<&'static str> {
    flags.iter().map(|flag| flag.wire_name()).collect()
  }

  fn exclusive_group_counts(flags: &[StateFlag]) -> [usize; 3] {
    [
      flags
        .iter()
        .filter(|flag| matches!(flag, StateFlag::Navigation(_)))
        .count(),
      flags
        .iter()
        .filter(|flag| matches!(flag, StateFlag::PerformanceView(_)))
        .count(),
      flags
        .iter()
        .filter(|flag| matches!(flag, StateFlag::CpuVendor(_)))
        .count(),
    ]
  }

  #[test]
  fn a_settled_windows_set_has_one_variant_per_exclusive_group() {
    let flags = sample(
      &settled(Some(Answered), Some(NotAnswered)),
      OsFamily::Windows,
    )
    .unwrap();
    assert_eq!(
      names(&flags),
      [
        "feature_on.hardware_archive",
        "feature_on.tray_widget",
        "navigation.grouped",
        "view.performance.panels",
        "hardware.cpu_vendor.amd",
        "hardware.gpu_vendor.nvidia",
        "hardware.gpu_vendor.amd",
        "provider_available.pawnio_cpu",
      ]
    );
    assert_eq!(exclusive_group_counts(&flags), [1, 1, 1]);
  }

  #[test]
  fn a_pending_windows_provider_leaves_the_set_absent() {
    for outcomes in [
      (Some(Pending), Some(Answered)),
      (Some(Answered), Some(Pending)),
      (Some(Pending), Some(Pending)),
    ] {
      assert_eq!(
        sample(&settled(outcomes.0, outcomes.1), OsFamily::Windows),
        None
      );
    }
    let mut no_snapshot = settled(Some(Answered), Some(Answered));
    no_snapshot.providers = None;
    assert_eq!(sample(&no_snapshot, OsFamily::Windows), None);
  }

  #[test]
  fn provider_flags_follow_answered_outcomes_only() {
    let both =
      sample(&settled(Some(Answered), Some(Answered)), OsFamily::Windows).unwrap();
    assert!(both.contains(&StateFlag::ProviderAvailable(Provider::PawnIoCpu)));
    assert!(both.contains(&StateFlag::ProviderAvailable(Provider::SuperIo)));

    let neither = sample(
      &settled(Some(NotAnswered), Some(NotAnswered)),
      OsFamily::Windows,
    )
    .unwrap();
    assert!(
      !neither
        .iter()
        .any(|flag| matches!(flag, StateFlag::ProviderAvailable(_)))
    );
    assert_eq!(exclusive_group_counts(&neither), [1, 1, 1]);
  }

  #[test]
  fn macos_and_linux_never_sample_provider_flags() {
    for os_family in [OsFamily::Macos, OsFamily::Linux] {
      for providers in [
        None,
        Some(OptionalSensorProviderOutcomes::default()),
        settled(Some(Pending), Some(Pending)).providers,
        settled(Some(Answered), Some(Answered)).providers,
      ] {
        let sources = FlagSources {
          providers,
          ..settled(None, None)
        };
        let flags = sample(&sources, os_family).unwrap();
        assert!(
          !flags
            .iter()
            .any(|flag| matches!(flag, StateFlag::ProviderAvailable(_)))
        );
        assert_eq!(exclusive_group_counts(&flags), [1, 1, 1]);
      }
    }
  }

  #[test]
  fn missing_cpu_or_gpu_facts_leave_the_set_absent() {
    let mut no_cpu = settled(Some(Answered), Some(Answered));
    no_cpu.cpu_vendor = None;
    assert_eq!(sample(&no_cpu, OsFamily::Windows), None);

    let mut no_gpu = settled(Some(Answered), Some(Answered));
    no_gpu.gpu_vendors = None;
    assert_eq!(sample(&no_gpu, OsFamily::Windows), None);
  }

  #[test]
  fn no_enumerated_gpu_reports_no_gpu_flag_and_duplicates_collapse() {
    let mut no_gpu = settled(Some(NotAnswered), Some(NotAnswered));
    no_gpu.gpu_vendors = Some(Vec::new());
    let flags = sample(&no_gpu, OsFamily::Windows).unwrap();
    assert!(
      !flags
        .iter()
        .any(|flag| matches!(flag, StateFlag::GpuVendor(_)))
    );

    let mut two_nvidia = no_gpu.clone();
    two_nvidia.gpu_vendors = Some(vec![GpuVendor::Nvidia, GpuVendor::Nvidia]);
    let flags = sample(&two_nvidia, OsFamily::Windows).unwrap();
    assert_eq!(
      flags
        .iter()
        .filter(|flag| matches!(flag, StateFlag::GpuVendor(_)))
        .count(),
      1
    );
  }

  /// Vendor strings the Core providers produce, plus raw and unknown ones.
  #[test]
  fn every_cpu_vendor_fixture_maps_to_a_named_class_or_other() {
    for (vendor, class) in [
      ("Intel", CpuVendor::Intel),
      ("GenuineIntel", CpuVendor::Intel),
      ("AMD", CpuVendor::Amd),
      ("AuthenticAMD", CpuVendor::Amd),
      ("Apple", CpuVendor::Apple),
      ("HygonGenuine", CpuVendor::Other),
      ("CentaurHauls", CpuVendor::Other),
      ("Qualcomm Technologies Inc", CpuVendor::Other),
      ("", CpuVendor::Other),
    ] {
      assert_eq!(cpu_vendor_class(vendor), class, "{vendor:?}");
    }
  }

  #[test]
  fn every_gpu_vendor_fixture_maps_to_a_named_class_or_other() {
    for (vendor, class) in [
      ("NVIDIA", GpuVendor::Nvidia),
      ("AMD", GpuVendor::Amd),
      ("Intel", GpuVendor::Intel),
      ("Apple", GpuVendor::Apple),
      ("Unknown", GpuVendor::Other),
      ("Microsoft Basic Render Driver", GpuVendor::Other),
      ("Qualcomm", GpuVendor::Other),
      ("", GpuVendor::Other),
    ] {
      assert_eq!(gpu_vendor_class(vendor), class, "{vendor:?}");
    }
  }

  #[test]
  fn stored_performance_views_resolve_like_the_frontend() {
    use serde_json::json;
    for (stored, view) in [
      (Some(json!("panels")), PerformanceView::Panels),
      (Some(json!("compact")), PerformanceView::Compact),
      (Some(json!("monitor")), PerformanceView::Monitor),
      (Some(json!("detailed")), PerformanceView::Panels),
      (Some(json!("custom")), PerformanceView::Panels),
      (Some(json!("unknown")), PerformanceView::Panels),
      (Some(json!(3)), PerformanceView::Panels),
      (None, PerformanceView::Panels),
    ] {
      assert_eq!(
        PerformanceView::from_stored(stored.as_ref()),
        view,
        "{stored:?}"
      );
    }
  }

  #[test]
  fn preference_facts_read_app_and_core_settings() {
    let mut settings = crate::models::settings::Settings {
      close_to_tray: true,
      burn_in_shift: true,
      selected_background_img: Some("image.png".to_string()),
      navigation_layout: NavigationLayout::Classic,
      elevated_startup_mode: false,
      ..crate::models::settings::Settings::default()
    };
    settings.tray_widget.enabled = false;
    let mut core_settings = hardviz_core::settings::CoreSettings::default();
    core_settings.hardware_archive.enabled = false;
    core_settings.storage_health.enabled = true;
    core_settings.environmental_sensors.switchbot_meter_enabled = true;

    let facts =
      PreferenceFacts::from_settings(&settings, &core_settings, PerformanceView::Monitor);

    assert_eq!(
      facts,
      PreferenceFacts {
        features_on: vec![
          Feature::StorageHealth,
          Feature::CloseToTray,
          Feature::AmbientSensor,
          Feature::BurnInShift,
          Feature::BackgroundImage,
        ],
        navigation: Navigation::Classic,
        performance_view: PerformanceView::Monitor,
      }
    );
  }
}
