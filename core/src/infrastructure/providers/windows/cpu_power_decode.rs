//! Pure decoding and state transition logic for CPU package power counters.
//!
//! The hardware-facing sampler supplies a low-level counter value (or a read
//! failure) and a monotonic timestamp. Keeping the wrap, gap, and re-baseline
//! rules here makes them testable without PawnIO or Windows.

const POWER_GATE_WATTS: f64 = 1000.0;
const COUNTER_MODULUS: f64 = 4_294_967_296.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PowerUnitDecodeError {
  ZeroUnitRegister,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Baseline {
  counter: u32,
  timestamp_seconds: f64,
}

/// Stateful low-32-bit RAPL power decoder shared by Intel and AMD paths.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PowerDecoder {
  energy_status_units: u8,
  energy_unit_joules: f64,
  baseline: Option<Baseline>,
}

pub(crate) fn extract_energy_status_units(unit_register: u64) -> u8 {
  ((unit_register >> 8) & 0x1f) as u8
}

pub(crate) fn energy_unit_from_register(
  unit_register: u64,
) -> Result<f64, PowerUnitDecodeError> {
  if unit_register == 0 {
    return Err(PowerUnitDecodeError::ZeroUnitRegister);
  }

  let energy_status_units = extract_energy_status_units(unit_register);
  Ok(2_f64.powi(-(energy_status_units as i32)))
}

pub(crate) fn maximum_gap_seconds(energy_status_units: u8) -> f64 {
  COUNTER_MODULUS * 2_f64.powi(-(energy_status_units as i32)) / POWER_GATE_WATTS
}

impl PowerDecoder {
  pub(crate) fn from_unit_register(
    unit_register: u64,
  ) -> Result<Self, PowerUnitDecodeError> {
    let energy_status_units = extract_energy_status_units(unit_register);
    Ok(Self {
      energy_status_units,
      energy_unit_joules: energy_unit_from_register(unit_register)?,
      baseline: None,
    })
  }

  pub(crate) fn from_unit_register_with_baseline(
    unit_register: u64,
    counter: u64,
    timestamp_seconds: f64,
  ) -> Result<Self, PowerUnitDecodeError> {
    let mut decoder = Self::from_unit_register(unit_register)?;
    decoder.baseline = Some(Baseline {
      counter: counter as u32,
      timestamp_seconds,
    });
    Ok(decoder)
  }

  /// Consume one sampling result.
  ///
  /// `None` represents a failed counter read and clears the baseline. A
  /// successful reading always becomes the next baseline, including when the
  /// current sample is rejected by timestamp, gap, or plausibility checks.
  pub(crate) fn sample(
    &mut self,
    counter: Option<u64>,
    timestamp_seconds: f64,
  ) -> Option<f64> {
    let Some(counter) = counter else {
      self.baseline = None;
      return None;
    };

    let current_counter = counter as u32;
    let Some(previous) = self.baseline else {
      self.baseline = Some(Baseline {
        counter: current_counter,
        timestamp_seconds,
      });
      return None;
    };

    let elapsed_seconds = timestamp_seconds - previous.timestamp_seconds;
    let maximum_gap = maximum_gap_seconds(self.energy_status_units);
    let delta = current_counter.wrapping_sub(previous.counter);

    // A successful reading is the baseline even when the interval is not
    // usable. This prevents an invalid sample from being reused as stale
    // power on a later tick.
    self.baseline = Some(Baseline {
      counter: current_counter,
      timestamp_seconds,
    });

    if !elapsed_seconds.is_finite()
      || elapsed_seconds <= 0.0
      || elapsed_seconds >= maximum_gap
    {
      return None;
    }

    let energy_joules = delta as f64 * self.energy_unit_joules;
    let power_watts = energy_joules / elapsed_seconds;
    if !power_watts.is_finite() || !(0.0..=POWER_GATE_WATTS).contains(&power_watts) {
      return None;
    }

    Some(power_watts)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const UNIT_REGISTER_ESU_16: u64 = 16 << 8;

  fn decoder() -> PowerDecoder {
    PowerDecoder::from_unit_register(UNIT_REGISTER_ESU_16).unwrap()
  }

  #[test]
  fn extracts_esu_from_bits_12_to_8() {
    assert_eq!(extract_energy_status_units(0x1f00), 0x1f);
    assert_eq!(extract_energy_status_units(0x1010), 0x10);
  }

  #[test]
  fn rejects_an_all_zero_unit_register() {
    assert_eq!(
      PowerDecoder::from_unit_register(0),
      Err(PowerUnitDecodeError::ZeroUnitRegister)
    );
  }

  #[test]
  fn accepts_esu_zero_when_the_unit_register_is_not_all_zero() {
    assert_eq!(energy_unit_from_register(0x3), Ok(1.0));
  }

  #[test]
  fn decodes_an_ordinary_counter_delta() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(100), 0.0), None);
    assert_eq!(decoder.sample(Some(16_100), 1.0), Some(16_000.0 / 65_536.0));
  }

  #[test]
  fn decodes_a_wrapped_counter_delta() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(u32::MAX as u64 - 10), 0.0), None);
    assert_eq!(decoder.sample(Some(9), 1.0), Some(20.0 / 65_536.0));
  }

  #[test]
  fn ignores_the_high_32_counter_bits() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(0xaaaa_0000_0000_0100), 0.0), None);
    assert_eq!(
      decoder.sample(Some(0xbbbb_0000_0000_0200), 1.0),
      Some(256.0 / 65_536.0)
    );
  }

  #[test]
  fn first_sample_only_establishes_a_baseline() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(123), 10.0), None);
  }

  #[test]
  fn accepts_zero_watts() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(123), 0.0), None);
    assert_eq!(decoder.sample(Some(123), 1.0), Some(0.0));
  }

  #[test]
  fn accepts_exactly_1000_watts() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(0), 0.0), None);
    assert_eq!(decoder.sample(Some(65_536_000), 1.0), Some(1000.0));
  }

  #[test]
  fn rejects_power_over_1000_watts_and_rebaselines() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(0), 0.0), None);
    assert_eq!(decoder.sample(Some(65_536_001), 1.0), None);
    assert_eq!(decoder.sample(Some(65_536_017), 2.0), Some(16.0 / 65_536.0));
  }

  #[test]
  fn rejects_zero_elapsed_and_rebaselines() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(100), 1.0), None);
    assert_eq!(decoder.sample(Some(200), 1.0), None);
    assert_eq!(decoder.sample(Some(300), 2.0), Some(100.0 / 65_536.0));
  }

  #[test]
  fn rejects_non_finite_elapsed_and_rebaselines() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(100), 0.0), None);
    assert_eq!(decoder.sample(Some(200), f64::NAN), None);
    assert_eq!(decoder.sample(Some(300), 1.0), None);
    assert_eq!(decoder.sample(Some(400), 2.0), Some(100.0 / 65_536.0));
  }

  #[test]
  fn read_failure_clears_the_baseline() {
    let mut decoder = decoder();
    assert_eq!(decoder.sample(Some(100), 0.0), None);
    assert_eq!(decoder.sample(None, 1.0), None);
    assert_eq!(decoder.sample(Some(200), 2.0), None);
    assert_eq!(decoder.sample(Some(300), 3.0), Some(100.0 / 65_536.0));
  }

  #[test]
  fn accepts_a_gap_below_t_max() {
    let mut decoder = decoder();
    let t_max = maximum_gap_seconds(16);
    assert_eq!(decoder.sample(Some(0), 0.0), None);
    assert!(decoder.sample(Some(1), t_max - 0.001).is_some());
  }

  #[test]
  fn rejects_a_gap_equal_to_t_max() {
    let mut decoder = decoder();
    let t_max = maximum_gap_seconds(16);
    assert_eq!(decoder.sample(Some(0), 0.0), None);
    assert_eq!(decoder.sample(Some(1), t_max), None);
    assert_eq!(decoder.sample(Some(2), t_max + 1.0), Some(1.0 / 65_536.0));
  }

  #[test]
  fn rejects_a_gap_above_t_max_and_rebaselines() {
    let mut decoder = decoder();
    let t_max = maximum_gap_seconds(16);
    assert_eq!(decoder.sample(Some(0), 0.0), None);
    assert_eq!(decoder.sample(Some(1), t_max + 0.001), None);
    assert_eq!(decoder.sample(Some(2), t_max + 1.001), Some(1.0 / 65_536.0));
  }
}
