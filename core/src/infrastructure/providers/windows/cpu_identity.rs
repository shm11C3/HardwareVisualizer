#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
  Intel,
  Amd,
  Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuIdentity {
  pub vendor: CpuVendor,
  pub vendor_id: String,
  pub brand: String,
  pub family: u32,
  pub model: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CpuidLeaf {
  pub(crate) eax: u32,
  pub(crate) ebx: u32,
  pub(crate) ecx: u32,
  pub(crate) edx: u32,
}

pub(crate) fn detect_cpu_identity() -> CpuIdentity {
  detect_cpu_identity_x86().unwrap_or_else(CpuIdentity::unknown)
}

pub(crate) fn detect_amd_rapl_support() -> Option<bool> {
  cpuid_leaf(0x8000_0007).map(amd_rapl_supported_from_leaf)
}

pub(crate) fn amd_rapl_supported_from_leaf(leaf: CpuidLeaf) -> bool {
  (leaf.edx & (1 << 14)) != 0
}

impl CpuIdentity {
  pub(crate) fn unknown() -> Self {
    Self {
      vendor: CpuVendor::Other,
      vendor_id: "unknown".to_string(),
      brand: String::new(),
      family: 0,
      model: 0,
    }
  }
}

fn detect_cpu_identity_x86() -> Option<CpuIdentity> {
  let leaf0 = cpuid_leaf(0)?;
  let vendor_id = vendor_id_from_leaf0(leaf0);
  let leaf1 = cpuid_leaf(1)?;
  let (family, model) = effective_family_model(leaf1.eax);
  let brand = cpu_brand_string();
  let vendor = match vendor_id.as_str() {
    "GenuineIntel" => CpuVendor::Intel,
    "AuthenticAMD" => CpuVendor::Amd,
    _ => CpuVendor::Other,
  };

  Some(CpuIdentity {
    vendor,
    vendor_id,
    brand,
    family,
    model,
  })
}

fn vendor_id_from_leaf0(leaf: CpuidLeaf) -> String {
  let mut bytes = Vec::with_capacity(12);
  bytes.extend_from_slice(&leaf.ebx.to_le_bytes());
  bytes.extend_from_slice(&leaf.edx.to_le_bytes());
  bytes.extend_from_slice(&leaf.ecx.to_le_bytes());
  String::from_utf8_lossy(&bytes).trim().to_string()
}

fn cpu_brand_string() -> String {
  let Some(extended) = cpuid_leaf(0x8000_0000) else {
    return String::new();
  };
  if extended.eax < 0x8000_0004 {
    return String::new();
  }

  let mut bytes = Vec::with_capacity(48);
  for leaf in 0x8000_0002..=0x8000_0004 {
    let result = cpuid_leaf_unchecked(leaf);
    bytes.extend_from_slice(&result.eax.to_le_bytes());
    bytes.extend_from_slice(&result.ebx.to_le_bytes());
    bytes.extend_from_slice(&result.ecx.to_le_bytes());
    bytes.extend_from_slice(&result.edx.to_le_bytes());
  }

  String::from_utf8_lossy(&bytes)
    .trim_matches(char::from(0))
    .trim()
    .to_string()
}

pub(crate) fn effective_family_model(eax: u32) -> (u32, u32) {
  let base_family = (eax >> 8) & 0x0f;
  let base_model = (eax >> 4) & 0x0f;
  let extended_family = (eax >> 20) & 0xff;
  let extended_model = (eax >> 16) & 0x0f;

  let family = if base_family == 0x0f {
    base_family + extended_family
  } else {
    base_family
  };
  let model = if base_family == 0x06 || base_family == 0x0f {
    base_model + (extended_model << 4)
  } else {
    base_model
  };

  (family, model)
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn cpuid_leaf(leaf: u32) -> Option<CpuidLeaf> {
  use core::arch::x86_64::__cpuid;
  let max_leaf = if leaf >= 0x8000_0000 {
    __cpuid(0x8000_0000).eax
  } else {
    __cpuid(0).eax
  };
  (leaf <= max_leaf).then(|| {
    let result = __cpuid(leaf);
    CpuidLeaf {
      eax: result.eax,
      ebx: result.ebx,
      ecx: result.ecx,
      edx: result.edx,
    }
  })
}

#[cfg(target_arch = "x86")]
pub(crate) fn cpuid_leaf(leaf: u32) -> Option<CpuidLeaf> {
  use core::arch::x86::__cpuid;
  let max_leaf = if leaf >= 0x8000_0000 {
    __cpuid(0x8000_0000).eax
  } else {
    __cpuid(0).eax
  };
  (leaf <= max_leaf).then(|| {
    let result = __cpuid(leaf);
    CpuidLeaf {
      eax: result.eax,
      ebx: result.ebx,
      ecx: result.ecx,
      edx: result.edx,
    }
  })
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub(crate) fn cpuid_leaf(_leaf: u32) -> Option<CpuidLeaf> {
  None
}

#[cfg(target_arch = "x86_64")]
pub(crate) fn cpuid_leaf_unchecked(leaf: u32) -> CpuidLeaf {
  use core::arch::x86_64::__cpuid;
  let result = __cpuid(leaf);
  CpuidLeaf {
    eax: result.eax,
    ebx: result.ebx,
    ecx: result.ecx,
    edx: result.edx,
  }
}

#[cfg(target_arch = "x86")]
pub(crate) fn cpuid_leaf_unchecked(leaf: u32) -> CpuidLeaf {
  use core::arch::x86::__cpuid;
  let result = __cpuid(leaf);
  CpuidLeaf {
    eax: result.eax,
    ebx: result.ebx,
    ecx: result.ecx,
    edx: result.edx,
  }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
pub(crate) fn cpuid_leaf_unchecked(_leaf: u32) -> CpuidLeaf {
  CpuidLeaf {
    eax: 0,
    ebx: 0,
    ecx: 0,
    edx: 0,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn effective_family_model_adds_extended_family_for_amd_zen() {
    let eax = (0x8 << 20) | (0x1 << 16) | (0xf << 8) | (0x1 << 4);

    assert_eq!(effective_family_model(eax), (0x17, 0x11));
  }

  #[test]
  fn effective_family_model_adds_extended_model_for_intel_family_6() {
    let eax = (0xa << 16) | (0x6 << 8) | (0x5 << 4);

    assert_eq!(effective_family_model(eax), (0x6, 0xa5));
  }

  #[test]
  fn amd_rapl_capability_uses_cpuid_bit_14() {
    let leaf = CpuidLeaf {
      eax: 0,
      ebx: 0,
      ecx: 0,
      edx: 1 << 14,
    };
    assert!(amd_rapl_supported_from_leaf(leaf));
    assert!(!amd_rapl_supported_from_leaf(CpuidLeaf { edx: 0, ..leaf }));
  }
}
