use crate::enums::error::PlatformError;
use crate::infrastructure;
use crate::models;

pub async fn get_gpu_usage() -> Result<(f32, String), PlatformError> {
  let cards = infrastructure::providers::drm_sys::get_card_ids()
    .await
    .map_err(PlatformError::fault)?;

  for card in cards {
    // TODO Also handle Vendor ID detection in infrastructure layer
    match card.vendor_id.as_str() {
      "0x1002" => {
        if let Ok(usage) =
          infrastructure::providers::drm_sys::get_amd_gpu_usage(card.id).await
        {
          return Ok(((usage * 100.0) as f32, "DRM (AMD)".to_string()));
        }
      }
      "0x8086" => {
        if let Ok(usage) = infrastructure::providers::drm_sys::get_intel_gpu_usage().await
        {
          return Ok(((usage * 100.0) as f32, "DRM (Intel)".to_string()));
        }
      }
      _ => {}
    }
  }

  Err(PlatformError::unavailable(
    "Failed to get GPU usage on Linux (non-NVIDIA fallback)",
  ))
}

/// Build the GPU inventory from every DRM card, whatever its vendor.
///
/// ADR 0016 defines the System Specifications sheet as an inventory of
/// every detected adapter. A vendor with no Linux reading provider
/// (NVIDIA, unrecognized vendors) still gets a named row; ending the
/// vendor match with a wildcard `None` is how those cards used to vanish
/// from the sheet entirely.
pub async fn get_gpu_info() -> Result<Vec<models::hardware::GraphicInfo>, PlatformError> {
  use crate::infrastructure::providers::drm_sys::GpuVendor;
  use tokio::task::JoinSet;

  let card_ids = infrastructure::providers::drm_sys::get_all_card_ids();
  let mut join_set = JoinSet::new();

  for card_id in card_ids {
    join_set.spawn(async move {
      let vendor = infrastructure::providers::drm_sys::detect_gpu_vendor(card_id);
      match vendor {
        GpuVendor::Amd => get_amd_graphic_info(card_id).await.ok(),
        GpuVendor::Intel => get_intel_graphic_info(card_id).await.ok(),
        // A named row with unsupported values, not a missing row.
        GpuVendor::Nvidia | GpuVendor::Unknown => {
          Some(get_generic_graphic_info(card_id, vendor))
        }
      }
      .map(|info| (card_id, info)) // Attach card_id for sorting
    });
  }

  let mut infos: Vec<(u8, models::hardware::GraphicInfo)> = Vec::new();
  while let Some(res) = join_set.join_next().await {
    if let Ok(Some((card_id, info))) = res {
      infos.push((card_id, info));
    }
  }

  // Sort by original card_id in ascending order
  infos.sort_by_key(|(id, _)| *id);

  Ok(infos.into_iter().map(|(_, info)| info).collect())
}

async fn get_amd_graphic_info(
  card_id: u8,
) -> Result<models::hardware::GraphicInfo, PlatformError> {
  let lspci_name =
    infrastructure::providers::drm_sys::get_card_bdf(card_id).and_then(|bdf| {
      infrastructure::providers::lspci::get_gpu_name_from_lspci_by_bdf(&bdf)
    });
  let identity = resolve_card_identity(
    infrastructure::providers::drm_sys::GpuVendor::Amd,
    card_id,
    lspci_name.as_deref(),
  );

  let clock = infrastructure::providers::kernel::read_pm_info_sclk(card_id).unwrap_or(0);
  let memory_total =
    infrastructure::providers::drm_sys::read_vram_total_bytes(card_id).unwrap_or(0);

  Ok(models::hardware::GraphicInfo {
    id: format!("card{card_id}"),
    name: identity.name,
    vendor_name: identity.vendor_label.into(),
    clock,
    memory_size: crate::utils::formatter::format_size(memory_total, 1),
    memory_size_dedicated: crate::utils::formatter::format_size(memory_total, 1),
    core_count: None,
  })
}

pub async fn get_intel_graphic_info(
  card_id: u8,
) -> Result<models::hardware::GraphicInfo, PlatformError> {
  Ok(models::hardware::GraphicInfo {
    id: format!("card{card_id}"),
    name: "Intel Integrated Graphics".into(),
    vendor_name: "Intel".into(),
    clock: 0, // Difficult to obtain. Set to 0 as unsupported
    memory_size: "N/A".into(),
    memory_size_dedicated: "N/A".into(),
    core_count: None,
  })
}

/// Inventory entry for a card no Linux reading provider covers (NVIDIA and
/// unrecognized vendors): named through `lspci` like the AMD path, with
/// clock and memory reported as unsupported rather than invented.
fn get_generic_graphic_info(
  card_id: u8,
  vendor: infrastructure::providers::drm_sys::GpuVendor,
) -> models::hardware::GraphicInfo {
  let lspci_name =
    infrastructure::providers::drm_sys::get_card_bdf(card_id).and_then(|bdf| {
      infrastructure::providers::lspci::get_gpu_name_from_lspci_by_bdf(&bdf)
    });
  let identity = resolve_card_identity(vendor, card_id, lspci_name.as_deref());

  models::hardware::GraphicInfo {
    id: format!("card{card_id}"),
    name: identity.name,
    vendor_name: identity.vendor_label.into(),
    clock: 0, // Not readable without a vendor provider. Set to 0 as unsupported
    memory_size: "N/A".into(),
    memory_size_dedicated: "N/A".into(),
    core_count: None,
  }
}

/// Always returns raw degrees Celsius. Presentation conversion lives at
/// the App-side boundary.
pub async fn get_gpu_temperature()
-> Result<Vec<models::hardware::NameValue>, PlatformError> {
  let cards = infrastructure::providers::drm_sys::get_all_card_ids();
  let mut all_temps: Vec<models::hardware::NameValue> = Vec::new();

  for card_id in cards {
    // Only read hwmon for AMD GPUs (vendor 0x1002)
    if infrastructure::providers::drm_sys::detect_gpu_vendor(card_id)
      != infrastructure::providers::drm_sys::GpuVendor::Amd
    {
      continue;
    }

    if let Ok(temps) = infrastructure::providers::hwmon::read_hwmon_temperatures(card_id)
    {
      for temp in temps {
        all_temps.push(models::hardware::NameValue {
          name: temp.name,
          value: temp.value,
        });
      }
    }
  }

  if all_temps.is_empty() {
    Err(PlatformError::unavailable(
      "No GPU temperature sensors found on Linux",
    ))
  } else {
    Ok(all_temps)
  }
}

/// Collect GPU metrics on Linux for every DRM card.
///
/// AMD (sysfs/hwmon) and Intel (intel_gpu_top) have reading providers;
/// NVIDIA and unrecognized vendors have none here, so their samples carry
/// a name and no values. They are emitted anyway because a card absent
/// from the sample stream has no name, no readings, and no entry in the
/// GPU switcher: it reads as nonexistent rather than unavailable. DRM
/// detection decides presence; what a provider can read only decides
/// which fields are filled.
pub async fn sample_gpus() -> Vec<models::GpuSample> {
  use crate::infrastructure::providers::drm_sys::GpuVendor;

  let mut metrics: Vec<models::GpuSample> = Vec::new();
  let card_ids = infrastructure::providers::drm_sys::get_all_card_ids();
  let card_vendors = card_ids
    .into_iter()
    .map(|card_id| {
      (
        card_id,
        infrastructure::providers::drm_sys::detect_gpu_vendor(card_id),
      )
    })
    .collect::<Vec<_>>();

  // lspci is the name source for every vendor except Intel, whose live
  // name is fixed — an Intel-only machine never touches the cache.
  let lspci_output = if card_vendors
    .iter()
    .any(|(_, vendor)| *vendor != GpuVendor::Intel)
  {
    lspci_output_cached().await
  } else {
    None
  };

  for (card_id, vendor) in card_vendors {
    let bdf = infrastructure::providers::drm_sys::get_card_bdf(card_id);
    let lspci_name = bdf.as_deref().and_then(|bdf| {
      lspci_output
        .and_then(|out| infrastructure::providers::lspci::parse_gpu_name_by_bdf(out, bdf))
    });
    let identity = resolve_card_identity(vendor, card_id, lspci_name.as_deref());

    let (usage, temperature) = match vendor {
      GpuVendor::Amd => {
        let usage = infrastructure::providers::drm_sys::get_amd_gpu_usage(card_id as u32)
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        let temperature =
          infrastructure::providers::hwmon::read_hwmon_temperatures(card_id)
            .ok()
            .and_then(|temps| temps.first().map(|t| t.value as f32));
        (usage, temperature)
      }
      GpuVendor::Intel => {
        let usage = infrastructure::providers::drm_sys::get_intel_gpu_usage()
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        (usage, None)
      }
      // No reading provider — the sample still names the card so the
      // switcher can list it in its "no live readings" state.
      GpuVendor::Nvidia | GpuVendor::Unknown => (None, None),
    };

    let gpu_id = bdf
      .map(|bdf| format!("pci:{bdf}"))
      .unwrap_or_else(|| format!("drm:card{card_id}"));

    metrics.push(models::GpuSample {
      gpu_id,
      name: identity.name,
      usage,
      temperature,
      dedicated_memory_kb: None,
      cooler_level: None,
      source: identity.live_source.to_string(),
    });
  }

  metrics
}

/// The `lspci -nn` output, fetched once and cached for the process
/// lifetime.
///
/// `sample_gpus` runs at the monitor cadence, and forking `lspci` every
/// second to re-read a static adapter name would make the monitor its own
/// workload. Only a successful run is cached; a failure returns `None` for
/// that tick and the next tick retries, which costs no more than the
/// uncached behavior did. A card hot-plugged after the first success reads
/// a stale listing and falls back to its vendor placeholder name.
#[cfg(target_os = "linux")]
async fn lspci_output_cached() -> Option<&'static str> {
  static OUTPUT: tokio::sync::OnceCell<String> = tokio::sync::OnceCell::const_new();

  OUTPUT
    .get_or_try_init(|| async {
      tokio::task::spawn_blocking(infrastructure::providers::lspci::get_lspci_nn_output)
        .await
        .ok()
        .flatten()
        .ok_or(())
    })
    .await
    .ok()
    .map(String::as_str)
}

/// How one DRM card presents itself, resolved from the vendor alone so a
/// card is named even when no provider can read values for it.
#[cfg(any(target_os = "linux", test))]
struct CardIdentity {
  /// Display name: the `lspci` line for the card's PCI slot when
  /// available, otherwise a vendor placeholder carrying the card index.
  name: String,
  /// Vendor label for the inventory (`GraphicInfo.vendor_name`).
  vendor_label: &'static str,
  /// `GpuSample.source` label for the live stream.
  live_source: &'static str,
}

/// Decide how one DRM card is presented, from its vendor and the optional
/// `lspci` line for its PCI slot.
///
/// The match is exhaustive on purpose: adding a `GpuVendor` variant must
/// be a compile error here instead of a silently dropped card. Ending the
/// vendor match with a wildcard `continue`/`None` is exactly the bug that
/// hid NVIDIA and unknown-vendor cards from the switcher and the
/// inventory.
#[cfg(any(target_os = "linux", test))]
fn resolve_card_identity(
  vendor: infrastructure::providers::drm_sys::GpuVendor,
  card_id: u8,
  lspci_name: Option<&str>,
) -> CardIdentity {
  use crate::infrastructure::providers::drm_sys::GpuVendor;

  match vendor {
    GpuVendor::Amd => CardIdentity {
      name: lspci_name
        .map(str::to_string)
        .unwrap_or_else(|| format!("AMD GPU (card{card_id})")),
      vendor_label: "AMD",
      live_source: "DRM (AMD)",
    },
    GpuVendor::Intel => CardIdentity {
      // Intel keeps its fixed live name rather than the lspci line: the
      // lspci prefetch is skipped on Intel-only machines, so an
      // lspci-based name would change with which other vendors happen to
      // be installed.
      name: format!("Intel GPU (card{card_id})"),
      vendor_label: "Intel",
      live_source: "DRM (Intel)",
    },
    GpuVendor::Nvidia => CardIdentity {
      name: lspci_name
        .map(str::to_string)
        .unwrap_or_else(|| format!("NVIDIA GPU (card{card_id})")),
      vendor_label: "NVIDIA",
      live_source: "DRM",
    },
    GpuVendor::Unknown => CardIdentity {
      name: lspci_name
        .map(str::to_string)
        .unwrap_or_else(|| format!("GPU (card{card_id})")),
      vendor_label: "Unknown",
      live_source: "DRM",
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::infrastructure::providers::drm_sys::GpuVendor;

  // ── resolve_card_identity ──

  const AMD_LSPCI_LINE: &str = "06:00.0 VGA compatible controller [0300]: Advanced Micro Devices, Inc. [AMD/ATI] Navi 31 [Radeon RX 7900 XTX] [1002:744c]";
  const NVIDIA_LSPCI_LINE: &str = "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation AD107 [GeForce RTX 4060] [10de:2882]";

  #[test]
  fn amd_card_is_named_from_lspci() {
    let identity = resolve_card_identity(GpuVendor::Amd, 0, Some(AMD_LSPCI_LINE));
    assert_eq!(identity.name, AMD_LSPCI_LINE);
    assert_eq!(identity.vendor_label, "AMD");
    assert_eq!(identity.live_source, "DRM (AMD)");
  }

  #[test]
  fn nvidia_card_is_named_from_lspci() {
    let identity = resolve_card_identity(GpuVendor::Nvidia, 0, Some(NVIDIA_LSPCI_LINE));
    assert_eq!(identity.name, NVIDIA_LSPCI_LINE);
    assert_eq!(identity.vendor_label, "NVIDIA");
    assert_eq!(identity.live_source, "DRM");
  }

  #[test]
  fn nvidia_card_without_lspci_falls_back_to_a_placeholder_name() {
    let identity = resolve_card_identity(GpuVendor::Nvidia, 1, None);
    assert_eq!(identity.name, "NVIDIA GPU (card1)");
  }

  #[test]
  fn unknown_vendor_card_without_lspci_falls_back_to_a_placeholder_name() {
    let identity = resolve_card_identity(GpuVendor::Unknown, 2, None);
    assert_eq!(identity.name, "GPU (card2)");
    assert_eq!(identity.vendor_label, "Unknown");
    assert_eq!(identity.live_source, "DRM");
  }

  #[test]
  fn intel_card_keeps_its_fixed_live_name() {
    let identity = resolve_card_identity(GpuVendor::Intel, 0, Some(AMD_LSPCI_LINE));
    assert_eq!(identity.name, "Intel GPU (card0)");
    assert_eq!(identity.live_source, "DRM (Intel)");
  }

  #[test]
  fn every_vendor_resolves_to_an_emitted_identity() {
    // The regression this module carried: a wildcard vendor arm dropped
    // the card instead of emitting it. Every vendor must resolve to a
    // named identity, with or without an lspci line.
    for vendor in [
      GpuVendor::Nvidia,
      GpuVendor::Amd,
      GpuVendor::Intel,
      GpuVendor::Unknown,
    ] {
      for lspci_name in [None, Some(NVIDIA_LSPCI_LINE)] {
        let identity = resolve_card_identity(vendor, 3, lspci_name);
        assert!(!identity.name.is_empty(), "{vendor:?} must be named");
        assert!(!identity.vendor_label.is_empty());
        assert!(identity.live_source.starts_with("DRM"));
      }
    }
  }
}
