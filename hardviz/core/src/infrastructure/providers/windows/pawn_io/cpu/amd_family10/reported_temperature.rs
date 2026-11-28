//! The pure Reported Temperature Control decode shared by every
//! `AMDFamily10` Tctl path.

/// `CurTmp` counts eighths of a degree; 120 is the plausibility ceiling.
const MAX_PLAUSIBLE_CUR_TMP: u16 = 120 * 8;

/// How a family treats bits 17:16 of the register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectFieldCheck {
  /// Bits 17:16 select the source of `CurTmp`; only `00b` is the
  /// hardware-updated Tctl.
  RequireHardwareTctl,
  /// Bits 17:16 are Reserved and are not examined.
  NotExamined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TctlDecodeError {
  #[error("AMD Tctl register read 0x{0:08X}, a failed transaction")]
  FailedTransaction(u32),
  #[error("AMD Tctl select field is {0:02b}, not the hardware-updated Tctl")]
  NotHardwareTctl(u8),
  #[error("AMD Tctl {cur_tmp}/8 degrees is outside 0 to 120")]
  Implausible { cur_tmp: u16 },
}

/// Decode a Reported Temperature Control value. Every map shares the
/// `CurTmp` (31:21) field and the 0.125 scale; `select_field` states how the
/// family treats bits 17:16.
pub(crate) fn decode_tctl(
  raw: u32,
  select_field: SelectFieldCheck,
) -> Result<f32, TctlDecodeError> {
  if raw == 0 || raw == u32::MAX {
    return Err(TctlDecodeError::FailedTransaction(raw));
  }
  match select_field {
    SelectFieldCheck::RequireHardwareTctl => {
      // 01b and 10b are reserved; 11b holds a software-specified value.
      let select = ((raw >> 16) & 0b11) as u8;
      if select != 0 {
        return Err(TctlDecodeError::NotHardwareTctl(select));
      }
    }
    SelectFieldCheck::NotExamined => {}
  }
  let cur_tmp = (raw >> 21) as u16;
  if cur_tmp > MAX_PLAUSIBLE_CUR_TMP {
    return Err(TctlDecodeError::Implausible { cur_tmp });
  }
  Ok(f32::from(cur_tmp) * 0.125)
}

/// Synthetic register value built from the specification's field layout
/// (rev 2, Register map): `CurTmp` at 31:21, select at 17:16, plus
/// arbitrary slew-control bits 12:0 that the decode must ignore.
#[cfg(test)]
pub(super) fn reported_temperature(cur_tmp: u32, select: u32, slew: u32) -> u32 {
  (cur_tmp << 21) | (select << 16) | (slew & 0x1FFF)
}

#[cfg(test)]
mod tests {
  use super::*;

  const CHECKED: SelectFieldCheck = SelectFieldCheck::RequireHardwareTctl;

  #[test]
  fn decode_scales_cur_tmp_by_one_eighth_and_ignores_slew_bits() {
    // Synthetic: CurTmp 0x1A4 (420) = 52.5, slew controls all set.
    assert_eq!(
      decode_tctl(reported_temperature(420, 0b00, 0x1FFF), CHECKED),
      Ok(52.5)
    );
    // Synthetic: a low idle value on the Tctl scale is not an error.
    assert_eq!(
      decode_tctl(reported_temperature(1, 0b00, 0x0A5), CHECKED),
      Ok(0.125)
    );
  }

  #[test]
  fn decode_rejects_every_select_field_other_than_hardware_tctl() {
    for select in [0b01, 0b10, 0b11] {
      assert_eq!(
        decode_tctl(reported_temperature(400, select, 0), CHECKED),
        Err(TctlDecodeError::NotHardwareTctl(select as u8))
      );
    }
  }

  #[test]
  fn decode_rejects_all_zero_and_all_ones_as_failed_transactions() {
    assert_eq!(
      decode_tctl(0, CHECKED),
      Err(TctlDecodeError::FailedTransaction(0))
    );
    assert_eq!(
      decode_tctl(u32::MAX, CHECKED),
      Err(TctlDecodeError::FailedTransaction(u32::MAX))
    );
  }

  #[test]
  fn decode_publishes_only_zero_to_120_degrees() {
    // CurTmp 0 with a nonzero slew field is a real 0 on the Tctl scale.
    assert_eq!(
      decode_tctl(reported_temperature(0, 0b00, 0x10), CHECKED),
      Ok(0.0)
    );
    assert_eq!(
      decode_tctl(reported_temperature(960, 0b00, 0), CHECKED),
      Ok(120.0)
    );
    assert_eq!(
      decode_tctl(reported_temperature(961, 0b00, 0), CHECKED),
      Err(TctlDecodeError::Implausible { cur_tmp: 961 })
    );
    assert_eq!(
      decode_tctl(reported_temperature(0x7FF, 0b00, 0), CHECKED),
      Err(TctlDecodeError::Implausible { cur_tmp: 0x7FF })
    );
  }
}
