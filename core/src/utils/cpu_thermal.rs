//! Pure decode / detection helpers for native CPU package-temperature
//! sensors read through PawnIO (issue #1635, Phase 1).
//!
//! Implemented clean-room from these spec documents only:
//!
//! - `docs/specs/sensors/cpu-intel-dts-msr.md` (rev 2) — Intel digital
//!   thermal sensor MSR layout, decode, and plausibility gates
//! - `docs/specs/sensors/cpu-amd-zen-smn.md` (rev 3) — AMD Zen THM
//!   register layout, decode, Tctl offsets, per-family enablement
//! - `docs/specs/sensors/pawnio-interface.md` (rev 2) — module names
//!   referenced in doc comments
//!
//! Everything here is a platform-neutral pure function so the decode
//! logic stays unit-testable on every OS. The Windows-only provider
//! (`infrastructure::providers::windows::pawnio_cpu_temp`) feeds it raw
//! CPUID / MSR / SMN values and publishes the result.

/// CPUID leaf 0 vendor string of Intel CPUs.
pub const INTEL_VENDOR: &str = "GenuineIntel";
/// CPUID leaf 0 vendor string of AMD CPUs.
pub const AMD_VENDOR: &str = "AuthenticAMD";

/// `MSR_TEMPERATURE_TARGET` — TCC activation temperature ("TjMax").
/// Read once per session; bits 23:16 carry the Temperature Target.
pub const MSR_TEMPERATURE_TARGET: u64 = 0x1A2;

/// `IA32_PACKAGE_THERM_STATUS` — package-scope digital readout.
/// Reads identically from any logical CPU of the package and has no
/// Reading Valid bit.
pub const MSR_IA32_PACKAGE_THERM_STATUS: u64 = 0x1B1;

/// SMN address of `SMU::THM::THM_TCON_CUR_TMP` (SMUTHM aperture
/// `0x0005_9800`, register offset 0). Inside the PawnIO `RyzenSMU`
/// module's allowed window `0x56000`–`0x5AFFF`.
pub const AMD_SMN_THM_TCON_CUR_TMP: u64 = 0x0005_9800;

/// Intel plausibility bounds for the Temperature Target in °C
/// (project policy from the spec: "t_target within 50–120 °C").
const INTEL_T_TARGET_MIN: u32 = 50;
const INTEL_T_TARGET_MAX: u32 = 120;

/// AMD plausibility bounds for Tdie in °C (project policy from the
/// spec: "accept only −40 ≤ Tdie ≤ 120 °C").
const AMD_TDIE_MIN: f32 = -40.0;
const AMD_TDIE_MAX: f32 = 120.0;

// ── CPUID register decoding ──

/// Assemble the CPU vendor string from CPUID leaf 0.
///
/// The 12 ASCII bytes are carried in EBX, EDX, ECX — in that register
/// order — as little-endian byte groups (standard x86 CPUID convention).
pub fn cpuid_vendor_string(ebx: u32, ecx: u32, edx: u32) -> String {
  let mut bytes = Vec::with_capacity(12);
  bytes.extend_from_slice(&ebx.to_le_bytes());
  bytes.extend_from_slice(&edx.to_le_bytes());
  bytes.extend_from_slice(&ecx.to_le_bytes());
  String::from_utf8_lossy(&bytes).into_owned()
}

/// Assemble the CPU brand string from CPUID leaves
/// `0x80000002..=0x80000004` (standard x86 convention): each leaf
/// contributes EAX, EBX, ECX, EDX as little-endian byte groups, 48
/// bytes total, NUL-padded. Trailing NULs and surrounding whitespace
/// are stripped.
pub fn cpuid_brand_string(regs: &[u32; 12]) -> String {
  let mut bytes = Vec::with_capacity(48);
  for reg in regs {
    bytes.extend_from_slice(&reg.to_le_bytes());
  }
  let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
  String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

// ── Intel DTS (cpu-intel-dts-msr.md) ──

/// `CPUID.06H:EAX[0]` — digital thermal sensor (DTS) supported.
pub fn intel_dts_supported(leaf06_eax: u32) -> bool {
  leaf06_eax & 0x1 != 0
}

/// `CPUID.06H:EAX[6]` — package thermal management (PTM) supported;
/// `IA32_PACKAGE_THERM_STATUS` exists only when this is set.
pub fn intel_ptm_supported(leaf06_eax: u32) -> bool {
  leaf06_eax & (1 << 6) != 0
}

/// Extract the Temperature Target (TCC activation temperature, °C)
/// from `MSR_TEMPERATURE_TARGET` (`0x1A2`) bits 23:16.
///
/// Returns `None` when the field is 0 — the spec's "not supported"
/// marker (pre-Nehalem parts without a usable target). A faulted MSR
/// read is the other "unsupported" condition and is handled by the
/// caller. The TCC Activation Offset (bits 27:24, 29:24 on newer
/// products) never enters the decode: bits 23:16 stay the fixed
/// reference.
pub fn intel_temperature_target(msr_1a2: u64) -> Option<u32> {
  let t_target = ((msr_1a2 >> 16) & 0xFF) as u32;
  (t_target != 0).then_some(t_target)
}

/// Publishing bound on the Temperature Target: 50–120 °C inclusive.
pub fn intel_t_target_plausible(t_target: u32) -> bool {
  (INTEL_T_TARGET_MIN..=INTEL_T_TARGET_MAX).contains(&t_target)
}

/// Decode the package temperature from `IA32_PACKAGE_THERM_STATUS`
/// (`0x1B1`).
///
/// Hardware semantics: bits 22:16 carry the Package Digital Readout —
/// degrees **below** the package TCC activation temperature — so
/// `temperature = t_target − readout` in °C. All other status bits of
/// the register are ignored (the package register has no Reading Valid
/// bit).
///
/// Plausibility gate before publishing (project policy from the spec):
/// accept only `0 < temperature ≤ t_target` — inclusive, so a readout
/// of 0 decodes to `temperature == t_target` ("at TjMax") and passes —
/// and `t_target` within 50–120 °C. Returns `None` for dropped samples.
pub fn intel_package_temperature(t_target: u32, msr_1b1: u64) -> Option<f32> {
  if !intel_t_target_plausible(t_target) {
    return None;
  }
  let readout = ((msr_1b1 >> 16) & 0x7F) as i64;
  let temperature = t_target as i64 - readout;
  (temperature > 0 && temperature <= t_target as i64).then_some(temperature as f32)
}

// ── AMD Zen THM over SMN (cpu-amd-zen-smn.md) ──

/// Effective AMD family from CPUID leaf 1 EAX: base family (bits 11:8)
/// plus extended family (bits 27:20), the latter added only when the
/// base family is `0xF` (AMD CPUID convention per the spec).
pub fn amd_effective_family(leaf1_eax: u32) -> u32 {
  let base = (leaf1_eax >> 8) & 0xF;
  if base == 0xF {
    base + ((leaf1_eax >> 20) & 0xFF)
  } else {
    base
  }
}

/// Per-family enablement (the spec's scoped-enablement table).
///
/// "Recognized by the PawnIO module" and "enabled by this project" are
/// separate decisions: a family is enabled only once its THM register
/// facts are verified against a primary source in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmdFamilySupport {
  /// THM register facts verified by the spec — enabled (`0x17`, `0x19`).
  Enabled,
  /// Recognized by the PawnIO `RyzenSMU` module but not yet verified by
  /// the spec — disabled by default (`0x1A`, Zen 5).
  RecognizedDisabled,
  /// Not a supported Zen SMN target.
  Unsupported,
}

/// Look up a family in the spec's enablement table.
pub fn amd_family_support(family: u32) -> AmdFamilySupport {
  match family {
    0x17 | 0x19 => AmdFamilySupport::Enabled,
    0x1A => AmdFamilySupport::RecognizedDisabled,
    _ => AmdFamilySupport::Unsupported,
  }
}

/// Decode Tctl from the 32-bit `THM_TCON_CUR_TMP` register value.
///
/// Hardware semantics:
/// - An all-zero register read indicates a failed SMN transaction
///   rather than 0 °C → `None` (invalid sample).
/// - `CUR_TEMP` (bits 31:21, unsigned 11-bit) scales at 0.125 °C/LSB.
/// - `CUR_TEMP_RANGE_SEL` (bit 19) = 1 selects the −49…206 °C range:
///   subtract 49 °C. The bit must always be honored — the −49 °C range
///   is the normal case on many desktop parts.
/// - Bits 18:0 other than `RANGE_SEL` are Reserved on the CPU THM and
///   are ignored.
pub fn amd_tctl_celsius(thm_tcon_cur_tmp: u32) -> Option<f32> {
  if thm_tcon_cur_tmp == 0 {
    return None;
  }
  let cur_temp = (thm_tcon_cur_tmp >> 21) & 0x7FF;
  let mut tctl = cur_temp as f32 * 0.125;
  if thm_tcon_cur_tmp & (1 << 19) != 0 {
    tctl -= 49.0;
  }
  Some(tctl)
}

/// AMD-published Tctl − Tdie offsets, keyed by product name (OPN).
/// Products not listed have offset 0 (no positive offset is documented
/// in the primary sources for them, including Zen 2 and newer).
const AMD_TCTL_OFFSETS: &[(&str, f32)] = &[
  ("Ryzen 7 1700X", 20.0),
  ("Ryzen 7 1800X", 20.0),
  ("Ryzen Threadripper 1900X", 27.0),
  ("Ryzen Threadripper 1920X", 27.0),
  ("Ryzen Threadripper 1950X", 27.0),
  ("Ryzen 7 2700X", 10.0),
  ("Ryzen Threadripper 2920X", 27.0),
  ("Ryzen Threadripper 2950X", 27.0),
  ("Ryzen Threadripper 2970WX", 27.0),
  ("Ryzen Threadripper 2990WX", 27.0),
];

/// Tctl − Tdie offset in °C for a CPU brand string.
///
/// Matching is by product name (OPN), not family/model numbers: the
/// brand string is whitespace-normalized and matched case-insensitively
/// against the documented product names. Unlisted products — including
/// otherwise similar SKUs like the Ryzen 5 1600X — get the default 0.
pub fn amd_tctl_offset(brand: &str) -> f32 {
  let normalized = brand
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(" ")
    .to_ascii_uppercase();
  for (product, offset) in AMD_TCTL_OFFSETS {
    if normalized.contains(&product.to_ascii_uppercase()) {
      return *offset;
    }
  }
  0.0
}

/// Decode Tdie (the published CPU temperature) from a
/// `THM_TCON_CUR_TMP` register value and the model's Tctl offset.
///
/// `Tdie = Tctl − offset`. Plausibility gate before publishing (project
/// policy from the spec): accept only −40 ≤ Tdie ≤ 120 °C; all-zero
/// register reads are already rejected by [`amd_tctl_celsius`].
pub fn amd_tdie_celsius(thm_tcon_cur_tmp: u32, tctl_offset: f32) -> Option<f32> {
  let tdie = amd_tctl_celsius(thm_tcon_cur_tmp)? - tctl_offset;
  (AMD_TDIE_MIN..=AMD_TDIE_MAX)
    .contains(&tdie)
    .then_some(tdie)
}

// ── Detection plan ──

/// Which native CPU package-temperature source the host CPU supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuSensorPlan {
  /// `GenuineIntel` with DTS and PTM: read the package DTS MSRs through
  /// the PawnIO `IntelMSR` module.
  IntelDts,
  /// `AuthenticAMD` on a spec-enabled Zen family: read `THM_TCON_CUR_TMP`
  /// through the PawnIO `RyzenSMU` module.
  AmdZenSmn { family: u32 },
  /// `AuthenticAMD` on a family the PawnIO module recognizes but the
  /// spec has not verified yet (`0x1A`) — disabled by default.
  AmdFamilyDisabled { family: u32 },
  /// Everything else: vendor, family, or feature bits rule it out.
  Unsupported,
}

/// Classify the host CPU from raw CPUID values.
///
/// - `vendor` — CPUID leaf 0 vendor string.
/// - `leaf1_eax` — CPUID leaf 1 EAX (family fields); pass 0 when the
///   leaf is unavailable.
/// - `leaf06_eax` — CPUID leaf 6 EAX (thermal feature bits); pass 0
///   when the leaf is unavailable (treated as "no DTS/PTM").
///
/// Intel requires every detection fact of the spec's table: the vendor
/// string plus both `EAX[0]` (DTS) and `EAX[6]` (PTM — the package
/// register Phase 1 reads exists only with this bit). The remaining
/// Intel condition (a usable Temperature Target) needs an MSR read and
/// is checked by the provider at session establishment.
pub fn plan_cpu_sensor(vendor: &str, leaf1_eax: u32, leaf06_eax: u32) -> CpuSensorPlan {
  if vendor == INTEL_VENDOR {
    if intel_dts_supported(leaf06_eax) && intel_ptm_supported(leaf06_eax) {
      return CpuSensorPlan::IntelDts;
    }
    return CpuSensorPlan::Unsupported;
  }
  if vendor == AMD_VENDOR {
    let family = amd_effective_family(leaf1_eax);
    return match amd_family_support(family) {
      AmdFamilySupport::Enabled => CpuSensorPlan::AmdZenSmn { family },
      AmdFamilySupport::RecognizedDisabled => CpuSensorPlan::AmdFamilyDisabled { family },
      AmdFamilySupport::Unsupported => CpuSensorPlan::Unsupported,
    };
  }
  CpuSensorPlan::Unsupported
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── cpuid_vendor_string ──

  #[test]
  fn vendor_string_genuine_intel() {
    // CPUID leaf 0 fixture: EBX="Genu", EDX="ineI", ECX="ntel".
    assert_eq!(
      cpuid_vendor_string(0x756E_6547, 0x6C65_746E, 0x4965_6E69),
      "GenuineIntel"
    );
  }

  #[test]
  fn vendor_string_authentic_amd() {
    // CPUID leaf 0 fixture: EBX="Auth", EDX="enti", ECX="cAMD".
    assert_eq!(
      cpuid_vendor_string(0x6874_7541, 0x444D_4163, 0x6974_6E65),
      "AuthenticAMD"
    );
  }

  // ── cpuid_brand_string ──

  fn brand_regs(brand: &str) -> [u32; 12] {
    let mut bytes = [0u8; 48];
    bytes[..brand.len()].copy_from_slice(brand.as_bytes());
    let mut regs = [0u32; 12];
    for (i, chunk) in bytes.chunks_exact(4).enumerate() {
      regs[i] = u32::from_le_bytes(chunk.try_into().unwrap());
    }
    regs
  }

  #[test]
  fn brand_string_assembles_and_trims_nul_padding() {
    let regs = brand_regs("AMD Ryzen 7 1800X Eight-Core Processor");
    assert_eq!(
      cpuid_brand_string(&regs),
      "AMD Ryzen 7 1800X Eight-Core Processor"
    );
  }

  #[test]
  fn brand_string_trims_surrounding_whitespace() {
    let regs = brand_regs("  Intel(R) Core(TM) i7 CPU   ");
    assert_eq!(cpuid_brand_string(&regs), "Intel(R) Core(TM) i7 CPU");
  }

  #[test]
  fn brand_string_all_zero_regs_is_empty() {
    assert_eq!(cpuid_brand_string(&[0u32; 12]), "");
  }

  // ── intel leaf 6 feature bits ──

  #[test]
  fn intel_leaf06_bits_decode_independently() {
    assert!(!intel_dts_supported(0));
    assert!(!intel_ptm_supported(0));
    assert!(intel_dts_supported(0x1));
    assert!(!intel_ptm_supported(0x1));
    assert!(!intel_dts_supported(1 << 6));
    assert!(intel_ptm_supported(1 << 6));
    assert!(intel_dts_supported(0x41));
    assert!(intel_ptm_supported(0x41));
  }

  // ── intel_temperature_target ──

  #[test]
  fn t_target_extracted_from_bits_23_16() {
    // Fixture: Temperature Target 100 °C, all other bits clear.
    assert_eq!(intel_temperature_target(0x0064_0000), Some(100));
  }

  #[test]
  fn t_target_zero_field_means_unsupported() {
    assert_eq!(intel_temperature_target(0), None);
    // Bits outside 23:16 set, field itself 0 → still unsupported.
    assert_eq!(intel_temperature_target(0xFF00_FFFF), None);
  }

  #[test]
  fn t_target_ignores_tcc_activation_offset_bits() {
    // Fixture: target 100 °C with a 6-bit TCC Activation Offset of 10
    // programmed at bits 29:24 — the offset never enters the decode.
    let msr = (10u64 << 24) | (100u64 << 16);
    assert_eq!(intel_temperature_target(msr), Some(100));
  }

  #[test]
  fn t_target_ignores_unrelated_low_and_high_bits() {
    let msr = 0xFFFF_FFFF_0064_FFFF;
    assert_eq!(intel_temperature_target(msr), Some(0x64));
  }

  // ── intel_t_target_plausible ──

  #[test]
  fn t_target_bounds_are_inclusive() {
    assert!(intel_t_target_plausible(50));
    assert!(intel_t_target_plausible(100));
    assert!(intel_t_target_plausible(120));
    assert!(!intel_t_target_plausible(49));
    assert!(!intel_t_target_plausible(121));
    assert!(!intel_t_target_plausible(0));
    assert!(!intel_t_target_plausible(255));
  }

  // ── intel_package_temperature ──

  #[test]
  fn package_temperature_subtracts_readout_from_t_target() {
    // Fixture: readout 38 °C below target, plus unrelated status flag
    // bits the decode must ignore.
    let msr_1b1 = (38u64 << 16) | 0x0000_0001;
    assert_eq!(intel_package_temperature(100, msr_1b1), Some(62.0));
  }

  #[test]
  fn package_readout_masks_bits_22_16_only() {
    // Neighbor bits 15 and 23 set; readout field carries 30.
    let msr_1b1 = (30u64 << 16) | (1u64 << 15) | (1u64 << 23);
    assert_eq!(intel_package_temperature(100, msr_1b1), Some(70.0));
  }

  #[test]
  fn package_readout_zero_decodes_to_t_target_and_passes() {
    // Saturated readout: 0 below target = "at TjMax" — the gate's
    // inclusive upper bound accepts it.
    assert_eq!(intel_package_temperature(100, 0), Some(100.0));
  }

  #[test]
  fn package_temperature_zero_or_below_is_dropped() {
    // readout == t_target → temperature 0 → fails the strict lower bound.
    assert_eq!(intel_package_temperature(100, 100u64 << 16), None);
    // readout > t_target → negative temperature → dropped.
    assert_eq!(intel_package_temperature(100, 110u64 << 16), None);
  }

  #[test]
  fn package_temperature_gates_t_target_bounds() {
    let msr_1b1 = 30u64 << 16;
    // Boundary targets pass…
    assert_eq!(intel_package_temperature(50, msr_1b1), Some(20.0));
    assert_eq!(intel_package_temperature(120, msr_1b1), Some(90.0));
    // …out-of-bounds targets drop the sample regardless of readout.
    assert_eq!(intel_package_temperature(49, msr_1b1), None);
    assert_eq!(intel_package_temperature(121, msr_1b1), None);
  }

  // ── amd_effective_family ──

  #[test]
  fn effective_family_adds_extended_family_when_base_is_0xf() {
    // Fixture: family 17h (Zen) leaf 1 EAX — base 0xF, extended 0x8.
    assert_eq!(amd_effective_family(0x0080_0F11), 0x17);
    // Fixture: family 19h (Zen 3) — base 0xF, extended 0xA.
    assert_eq!(amd_effective_family(0x00A2_0F10), 0x19);
    // Fixture: family 1Ah (Zen 5) — base 0xF, extended 0xB.
    assert_eq!(amd_effective_family(0x00B4_0F00), 0x1A);
  }

  #[test]
  fn effective_family_ignores_extended_family_when_base_is_not_0xf() {
    // Base family 6 with a nonzero extended-family field: not added.
    assert_eq!(amd_effective_family(0x0090_06EA), 0x6);
    assert_eq!(amd_effective_family(0x0000_0F00), 0xF);
  }

  // ── amd_family_support ──

  #[test]
  fn family_0x17_and_0x19_are_enabled() {
    assert_eq!(amd_family_support(0x17), AmdFamilySupport::Enabled);
    assert_eq!(amd_family_support(0x19), AmdFamilySupport::Enabled);
  }

  #[test]
  fn family_0x1a_is_recognized_but_disabled() {
    assert_eq!(
      amd_family_support(0x1A),
      AmdFamilySupport::RecognizedDisabled
    );
  }

  #[test]
  fn other_families_are_unsupported() {
    assert_eq!(amd_family_support(0x15), AmdFamilySupport::Unsupported);
    assert_eq!(amd_family_support(0x16), AmdFamilySupport::Unsupported);
    assert_eq!(amd_family_support(0x6), AmdFamilySupport::Unsupported);
    assert_eq!(amd_family_support(0x1B), AmdFamilySupport::Unsupported);
  }

  // ── amd_tctl_celsius ──

  #[test]
  fn tctl_scales_cur_temp_at_one_eighth_degree() {
    // Fixture: CUR_TEMP = 760 → 95.0 °C, RANGE_SEL clear.
    assert_eq!(amd_tctl_celsius(760 << 21), Some(95.0));
    // Fixture: CUR_TEMP = 1 → smallest nonzero step.
    assert_eq!(amd_tctl_celsius(1 << 21), Some(0.125));
  }

  #[test]
  fn tctl_range_sel_subtracts_49() {
    // Fixture: CUR_TEMP = 1152 (144.0 °C raw) with RANGE_SEL set →
    // −49 °C rebase → 95.0 °C.
    let reg = (1152u32 << 21) | (1 << 19);
    assert_eq!(amd_tctl_celsius(reg), Some(95.0));
  }

  #[test]
  fn tctl_all_zero_read_is_rejected() {
    // All-zero register = failed SMN transaction, not 0 °C.
    assert_eq!(amd_tctl_celsius(0), None);
  }

  #[test]
  fn tctl_ignores_reserved_bits_18_0() {
    // All reserved bits 18:0 set (bit 19 is RANGE_SEL = 0x0008_0000 and
    // stays clear here; 18:16 are named TJ_SEL only in the GPU-IP
    // header) — they must not affect the decode.
    let reg = (760u32 << 21) | 0x0007_FFFF;
    assert_eq!(amd_tctl_celsius(reg), Some(95.0));
  }

  #[test]
  fn tctl_max_field_value_decodes() {
    // CUR_TEMP = 2047 → 255.875 °C (the SB-TSI-corroborated full range);
    // the publishing gate, not this decode, rejects it later.
    assert_eq!(amd_tctl_celsius(0x7FFu32 << 21), Some(255.875));
  }

  // ── amd_tctl_offset ──

  #[test]
  fn tctl_offset_ryzen_7_1700x_and_1800x_is_20() {
    assert_eq!(
      amd_tctl_offset("AMD Ryzen 7 1700X Eight-Core Processor"),
      20.0
    );
    assert_eq!(
      amd_tctl_offset("AMD Ryzen 7 1800X Eight-Core Processor"),
      20.0
    );
  }

  #[test]
  fn tctl_offset_first_gen_threadripper_is_27() {
    assert_eq!(
      amd_tctl_offset("AMD Ryzen Threadripper 1900X 8-Core Processor"),
      27.0
    );
    assert_eq!(
      amd_tctl_offset("AMD Ryzen Threadripper 1920X 12-Core Processor"),
      27.0
    );
    assert_eq!(
      amd_tctl_offset("AMD Ryzen Threadripper 1950X 16-Core Processor"),
      27.0
    );
  }

  #[test]
  fn tctl_offset_ryzen_7_2700x_is_10() {
    assert_eq!(
      amd_tctl_offset("AMD Ryzen 7 2700X Eight-Core Processor"),
      10.0
    );
  }

  #[test]
  fn tctl_offset_second_gen_threadripper_is_27() {
    assert_eq!(
      amd_tctl_offset("AMD Ryzen Threadripper 2920X 12-Core Processor"),
      27.0
    );
    assert_eq!(
      amd_tctl_offset("AMD Ryzen Threadripper 2950X 16-Core Processor"),
      27.0
    );
    assert_eq!(
      amd_tctl_offset("AMD Ryzen Threadripper 2970WX 24-Core Processor"),
      27.0
    );
    assert_eq!(
      amd_tctl_offset("AMD Ryzen Threadripper 2990WX 32-Core Processor"),
      27.0
    );
  }

  #[test]
  fn tctl_offset_defaults_to_zero_for_unlisted_products() {
    // Zen 2 and newer have no documented positive offset…
    assert_eq!(amd_tctl_offset("AMD Ryzen 7 3700X 8-Core Processor"), 0.0);
    assert_eq!(amd_tctl_offset("AMD Ryzen 9 7950X 16-Core Processor"), 0.0);
    // …and neither do similar-but-unlisted first-gen SKUs or other lines.
    assert_eq!(amd_tctl_offset("AMD Ryzen 5 1600X Six-Core Processor"), 0.0);
    assert_eq!(amd_tctl_offset("AMD EPYC 7551P 32-Core Processor"), 0.0);
    assert_eq!(amd_tctl_offset(""), 0.0);
  }

  #[test]
  fn tctl_offset_matching_normalizes_whitespace_and_case() {
    assert_eq!(amd_tctl_offset("AMD  Ryzen  7  1800X Eight-Core"), 20.0);
    assert_eq!(amd_tctl_offset("amd ryzen threadripper 2990wx"), 27.0);
  }

  // ── amd_tdie_celsius ──

  #[test]
  fn tdie_subtracts_model_offset() {
    // Fixture: Tctl 95.0 °C (RANGE_SEL path) on a 27 °C-offset part.
    let reg = (1152u32 << 21) | (1 << 19);
    assert_eq!(amd_tdie_celsius(reg, 27.0), Some(68.0));
    // Zero offset publishes Tctl as-is.
    assert_eq!(amd_tdie_celsius(760 << 21, 0.0), Some(95.0));
  }

  #[test]
  fn tdie_gate_bounds_are_inclusive() {
    // CUR_TEMP = 72 → 9.0 °C raw − 49 → exactly −40.0 °C: passes.
    let reg_low = (72u32 << 21) | (1 << 19);
    assert_eq!(amd_tdie_celsius(reg_low, 0.0), Some(-40.0));
    // One LSB lower → −40.125 °C: dropped.
    let reg_below = (71u32 << 21) | (1 << 19);
    assert_eq!(amd_tdie_celsius(reg_below, 0.0), None);
    // CUR_TEMP = 960 → exactly 120.0 °C: passes.
    assert_eq!(amd_tdie_celsius(960 << 21, 0.0), Some(120.0));
    // One LSB higher → 120.125 °C: dropped.
    assert_eq!(amd_tdie_celsius(961 << 21, 0.0), None);
  }

  #[test]
  fn tdie_rejects_all_zero_register() {
    assert_eq!(amd_tdie_celsius(0, 0.0), None);
    assert_eq!(amd_tdie_celsius(0, 20.0), None);
  }

  // ── plan_cpu_sensor ──

  #[test]
  fn plan_intel_requires_dts_and_ptm_bits() {
    assert_eq!(
      plan_cpu_sensor(INTEL_VENDOR, 0, 0x41),
      CpuSensorPlan::IntelDts
    );
    // Missing PTM (no package register) → unsupported.
    assert_eq!(
      plan_cpu_sensor(INTEL_VENDOR, 0, 0x1),
      CpuSensorPlan::Unsupported
    );
    // Missing DTS → unsupported.
    assert_eq!(
      plan_cpu_sensor(INTEL_VENDOR, 0, 1 << 6),
      CpuSensorPlan::Unsupported
    );
    // Leaf unavailable (passed as 0) → unsupported.
    assert_eq!(
      plan_cpu_sensor(INTEL_VENDOR, 0, 0),
      CpuSensorPlan::Unsupported
    );
  }

  #[test]
  fn plan_amd_enabled_families_use_smn() {
    assert_eq!(
      plan_cpu_sensor(AMD_VENDOR, 0x0080_0F11, 0),
      CpuSensorPlan::AmdZenSmn { family: 0x17 }
    );
    assert_eq!(
      plan_cpu_sensor(AMD_VENDOR, 0x00A2_0F10, 0),
      CpuSensorPlan::AmdZenSmn { family: 0x19 }
    );
  }

  #[test]
  fn plan_amd_family_0x1a_is_recognized_but_disabled() {
    assert_eq!(
      plan_cpu_sensor(AMD_VENDOR, 0x00B4_0F00, 0),
      CpuSensorPlan::AmdFamilyDisabled { family: 0x1A }
    );
  }

  #[test]
  fn plan_amd_pre_zen_family_is_unsupported() {
    // Family 15h fixture: base 0xF, extended 0x6.
    assert_eq!(
      plan_cpu_sensor(AMD_VENDOR, 0x0060_0F12, 0),
      CpuSensorPlan::Unsupported
    );
  }

  #[test]
  fn plan_unknown_vendor_is_unsupported() {
    assert_eq!(
      plan_cpu_sensor("CentaurHauls", 0, 0x41),
      CpuSensorPlan::Unsupported
    );
    assert_eq!(
      plan_cpu_sensor("", 0x0080_0F11, 0x41),
      CpuSensorPlan::Unsupported
    );
    // Vendor comparison is exact — no partial matches.
    assert_eq!(
      plan_cpu_sensor("GenuineIntel ", 0, 0x41),
      CpuSensorPlan::Unsupported
    );
  }
}
