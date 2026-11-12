use crate::enums::error::PlatformError;
use crate::infrastructure;
use crate::infrastructure::providers::drm_sys::GpuVendor;
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

/// Build the GPU inventory from DRM and NVIDIA display functions without DRM.
///
/// ADR 0016 defines the System Specifications sheet as an inventory of
/// every detected adapter. A device still gets a named row when readings
/// are unavailable.
pub async fn get_gpu_info() -> Result<Vec<models::hardware::GraphicInfo>, PlatformError> {
  use infrastructure::providers::nvml_provider;
  use tokio::task::JoinSet;

  let cards = drm_cards();
  let pci_only =
    pci_only_nvidia_addresses(&cards, nvml_provider::pci_display_addresses());
  let mut join_set = JoinSet::new();

  for card in cards {
    join_set.spawn(async move {
      match card.vendor {
        GpuVendor::Amd => get_amd_graphic_info(&card).await.ok(),
        GpuVendor::Intel => get_intel_graphic_info(card.id).await.ok(),
        // A named row with unsupported values, not a missing row.
        GpuVendor::Nvidia | GpuVendor::Unknown => Some(get_generic_graphic_info(&card)),
      }
      .map(|info| (card.id, info)) // Attach card_id for sorting
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

  let mut inventory: Vec<_> = infos.into_iter().map(|(_, info)| info).collect();
  inventory.extend(
    pci_only
      .iter()
      .map(|address| models::hardware::GraphicInfo {
        id: format!("pci-device:{address}"),
        name: pci_only_nvidia_name(address),
        vendor_name: "NVIDIA".into(),
        clock: 0,
        memory_size: "N/A".into(),
        memory_size_dedicated: "N/A".into(),
        core_count: None,
      }),
  );
  Ok(inventory)
}

async fn get_amd_graphic_info(
  card: &DrmCard,
) -> Result<models::hardware::GraphicInfo, PlatformError> {
  let card_id = card.id;
  let lspci_name = card
    .pci_address
    .as_deref()
    .and_then(infrastructure::providers::lspci::get_gpu_name_from_lspci_by_bdf);
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

/// Inventory entry for a generic DRM card, named through `lspci` like the AMD path, with
/// clock and memory reported as unsupported rather than invented.
fn get_generic_graphic_info(card: &DrmCard) -> models::hardware::GraphicInfo {
  let card_id = card.id;
  let lspci_name = card
    .pci_address
    .as_deref()
    .and_then(infrastructure::providers::lspci::get_gpu_name_from_lspci_by_bdf);
  let identity = resolve_card_identity(card.vendor, card_id, lspci_name.as_deref());

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

/// Collect GPU metrics for DRM cards and PCI-only NVIDIA display functions.
pub async fn sample_gpus() -> Vec<models::GpuSample> {
  use infrastructure::providers::{drm_sys, hwmon, lspci, nvml_provider};

  let mut metrics: Vec<models::GpuSample> = Vec::new();
  let cards = drm_cards();
  let nvml_addresses = nvml_pci_addresses(&cards, drm_sys::pci_runtime_suspended);
  let pci_only =
    pci_only_nvidia_addresses(&cards, nvml_provider::pci_display_addresses());

  // Keep lspci on the existing DRM paths. PCI-only inventory is based on
  // cached kernel attributes and does not need a PCI configuration scan.
  let lspci_output = if cards.iter().any(|card| card.vendor != GpuVendor::Intel) {
    lspci_output_cached().await
  } else {
    None
  };
  let mut nvidia_samples = nvml_provider::sample(nvml_addresses, &pci_only).await;

  for card in &cards {
    let card_id = card.id;
    let vendor = card.vendor;
    let pci_address = unique_pci_address(card, &cards);
    let lspci_name = pci_address.and_then(|address| {
      lspci_output.and_then(|out| lspci::parse_gpu_name_by_bdf(out, address))
    });
    let identity = resolve_card_identity(vendor, card_id, lspci_name.as_deref());
    let mut source = identity.live_source;
    let mut dedicated_memory_kb = None;

    let (usage, temperature) = match vendor {
      GpuVendor::Amd => {
        let usage = drm_sys::get_amd_gpu_usage(u32::from(card_id))
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        let temperature = hwmon::read_hwmon_temperatures(card_id)
          .ok()
          .and_then(|temps| temps.first().map(|t| t.value as f32));
        (usage, temperature)
      }
      GpuVendor::Intel => {
        let usage = drm_sys::get_intel_gpu_usage()
          .await
          .map(|u| (u * 100.0) as f32)
          .ok();
        (usage, None)
      }
      GpuVendor::Nvidia => {
        if let Some(readings) =
          pci_address.and_then(|address| nvidia_samples.remove(address))
        {
          source = "NVML";
          dedicated_memory_kb = readings.dedicated_memory_kb;
          (readings.usage, readings.temperature)
        } else {
          (None, None)
        }
      }
      GpuVendor::Unknown => (None, None),
    };

    metrics.push(models::GpuSample {
      gpu_id: linux_gpu_id(card_id, pci_address),
      name: identity.name,
      usage,
      temperature,
      dedicated_memory_kb,
      cooler_level: None,
      source: source.to_string(),
    });
  }
  metrics.extend(pci_only.into_iter().map(|address| {
    let readings = nvidia_samples.remove(&address);
    pci_only_nvidia_sample(&address, readings)
  }));

  metrics
}

struct DrmCard {
  id: u8,
  vendor: GpuVendor,
  pci_address: Option<String>,
}

fn drm_cards() -> Vec<DrmCard> {
  use infrastructure::providers::drm_sys;

  drm_sys::get_all_card_ids()
    .into_iter()
    .map(|id| DrmCard {
      id,
      vendor: drm_sys::detect_gpu_vendor(id),
      pci_address: drm_sys::get_card_pci_address(id),
    })
    .collect()
}

// An exact duplicate PCI address cannot identify a single DRM adapter.
fn unique_pci_address<'a>(card: &'a DrmCard, cards: &[DrmCard]) -> Option<&'a str> {
  let address = card.pci_address.as_deref()?;
  (!cards.iter().any(|sibling| {
    sibling.id != card.id && sibling.pci_address.as_deref() == Some(address)
  }))
  .then_some(address)
}

fn linux_gpu_id(card_id: u8, address: Option<&str>) -> String {
  address.map_or_else(|| format!("drm:card{card_id}"), pci_gpu_id)
}

fn pci_gpu_id(address: &str) -> String {
  format!("pci:{address}")
}

fn pci_only_nvidia_addresses(
  cards: &[DrmCard],
  pci_addresses: Vec<String>,
) -> Vec<String> {
  if cards
    .iter()
    .any(|card| card.vendor == GpuVendor::Nvidia && card.pci_address.is_none())
  {
    // The PCI-only functions cannot be distinguished from this DRM row.
    return Vec::new();
  }
  // A DRM row owns its address even if another DRM row makes it ambiguous.
  pci_addresses
    .into_iter()
    .filter(|address| {
      !cards
        .iter()
        .any(|card| card.pci_address.as_ref() == Some(address))
    })
    .collect()
}

fn pci_only_nvidia_name(address: &str) -> String {
  format!("NVIDIA GPU ({address})")
}

fn pci_only_nvidia_sample(
  address: &str,
  readings: Option<infrastructure::providers::nvml_provider::NvidiaReadings>,
) -> models::GpuSample {
  let source = if readings.is_some() { "NVML" } else { "PCI" };
  let readings = readings.unwrap_or_default();
  models::GpuSample {
    gpu_id: pci_gpu_id(address),
    name: pci_only_nvidia_name(address),
    usage: readings.usage,
    temperature: readings.temperature,
    dedicated_memory_kb: readings.dedicated_memory_kb,
    cooler_level: None,
    source: source.into(),
  }
}

/// Select unambiguous NVIDIA DRM cards without querying a suspended device.
fn nvml_pci_addresses(
  cards: &[DrmCard],
  is_suspended: impl Fn(&str) -> bool,
) -> Vec<String> {
  cards
    .iter()
    .filter(|card| card.vendor == GpuVendor::Nvidia)
    .filter_map(|card| unique_pci_address(card, cards))
    // A device handle lookup can wake a GPU from runtime suspension.
    .filter(|address| !is_suspended(address))
    .map(str::to_owned)
    .collect()
}

/// The `lspci -D -nn` output, fetched once and cached for the process
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
  fn full_pci_identity_preserves_nvidia_domains_and_declines_duplicate_addresses() {
    let card = |id, vendor, address: Option<&str>| DrmCard {
      id,
      vendor,
      pci_address: address.map(str::to_string),
    };
    let cards = [
      card(0, GpuVendor::Nvidia, Some("0000:01:00.0")),
      card(1, GpuVendor::Nvidia, Some("0003:04:00.1")),
      card(2, GpuVendor::Nvidia, Some("0001:01:00.0")),
      card(3, GpuVendor::Nvidia, None),
      card(4, GpuVendor::Intel, Some("0000:00:02.0")),
      card(5, GpuVendor::Nvidia, Some("0000:02:00.0")),
      card(6, GpuVendor::Nvidia, Some("0000:03:00.0")),
      card(7, GpuVendor::Nvidia, Some("0000:05:00.0")),
      card(8, GpuVendor::Amd, Some("0000:05:00.0")),
    ];

    assert_eq!(
      nvml_pci_addresses(&cards, |address| address == "0000:03:00.0"),
      [
        "0000:01:00.0",
        "0003:04:00.1",
        "0001:01:00.0",
        "0000:02:00.0"
      ]
    );
    let live_id = |index: usize| {
      let card = &cards[index];
      linux_gpu_id(card.id, unique_pci_address(card, &cards))
    };
    assert_eq!(live_id(0), "pci:0000:01:00.0");
    assert_eq!(live_id(2), "pci:0001:01:00.0");
    assert_eq!(live_id(3), "drm:card3");
    assert_eq!(live_id(6), "pci:0000:03:00.0");
    assert_eq!(live_id(7), "drm:card7");
    assert_eq!(live_id(8), "drm:card8");
    assert_eq!(
      linux_gpu_id(cards[2].id, unique_pci_address(&cards[2], &cards[1..])),
      live_id(2)
    );
  }

  #[test]
  fn pci_only_nvidia_preserves_full_identity_and_partial_or_missing_readings() {
    use crate::infrastructure::providers::nvml_provider::NvidiaReadings;

    let card = |id, address: &str| DrmCard {
      id,
      vendor: GpuVendor::Nvidia,
      pci_address: Some(address.into()),
    };
    let cards = [
      card(0, "0000:01:00.0"),
      card(1, "0000:01:00.0"),
      card(2, "0000:03:00.0"),
    ];
    let discovered = ["0000:01:00.0", "0001:01:00.0", "0002:01:00.0"];
    let pci_only =
      pci_only_nvidia_addresses(&cards, discovered.map(str::to_owned).to_vec());
    assert_eq!(pci_only, ["0001:01:00.0", "0002:01:00.0"]);

    let unavailable = pci_only_nvidia_sample(&pci_only[0], None);
    assert_eq!(unavailable.gpu_id, "pci:0001:01:00.0");
    assert_eq!(unavailable.name, "NVIDIA GPU (0001:01:00.0)");
    assert_eq!(unavailable.source, "PCI");
    assert_eq!(unavailable.usage, None);
    assert_eq!(unavailable.temperature, None);
    assert_eq!(unavailable.dedicated_memory_kb, None);

    let partial = pci_only_nvidia_sample(
      &pci_only[1],
      Some(NvidiaReadings {
        temperature: Some(42.0),
        ..NvidiaReadings::default()
      }),
    );
    assert_eq!(partial.gpu_id, linux_gpu_id(7, Some("0002:01:00.0")));
    assert_eq!(partial.source, "NVML");
    assert_eq!(partial.usage, None);
    assert_eq!(partial.temperature, Some(42.0));
    assert_eq!(partial.dedicated_memory_kb, None);

    let nvidia_without_address = DrmCard {
      id: 4,
      vendor: GpuVendor::Nvidia,
      pci_address: None,
    };
    assert!(pci_only_nvidia_addresses(&[nvidia_without_address], pci_only).is_empty());
  }

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
