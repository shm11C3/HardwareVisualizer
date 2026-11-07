//! The runtime's view of Analytics Consent.

use chrono::NaiveDate;

use super::period::{UtcMonth, parse_day};
use super::tokens::Seed;
use crate::models::settings::{ProductAnalyticsConsent, ProductAnalyticsSettings};
use crate::services::settings_service::CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION;

/// A grant at the current contract version, with everything a batch needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GrantedConsent {
  pub(crate) consent_version: u32,
  pub(crate) seed: Seed,
  pub(crate) cohort_month: UtcMonth,
  pub(crate) reported_day: Option<NaiveDate>,
  pub(crate) reported_month: Option<UtcMonth>,
}

impl GrantedConsent {
  /// `None` unless consent is effective: granted at the current contract
  /// version with a usable seed and cohort. A grant at an older version is
  /// treated as not granted until it is renewed.
  pub(crate) fn from_settings(section: &ProductAnalyticsSettings) -> Option<Self> {
    if section.consent != ProductAnalyticsConsent::Granted
      || section.consent_version < CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION
    {
      return None;
    }
    Some(Self {
      consent_version: section.consent_version,
      seed: Seed::from_hex(section.seed.as_deref()?)?,
      cohort_month: UtcMonth::parse(section.cohort_month.as_deref()?)?,
      reported_day: match section.reported_day.as_deref() {
        Some(day) => Some(parse_day(day)?),
        None => None,
      },
      reported_month: match section.reported_month.as_deref() {
        Some(month) => Some(UtcMonth::parse(month)?),
        None => None,
      },
    })
  }
}

#[cfg(test)]
pub(crate) mod test_support {
  use super::*;

  pub(crate) const SEED: &str =
    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

  pub(crate) fn granted_section() -> ProductAnalyticsSettings {
    ProductAnalyticsSettings {
      consent: ProductAnalyticsConsent::Granted,
      consent_version: CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION,
      seed: Some(SEED.to_string()),
      cohort_month: Some("2026-11".to_string()),
      reported_day: None,
      reported_month: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::test_support::granted_section;
  use super::*;

  #[test]
  fn a_current_grant_is_effective() {
    let mut section = granted_section();
    section.reported_day = Some("2026-11-03".to_string());
    section.reported_month = Some("2026-11".to_string());
    let consent = GrantedConsent::from_settings(&section).unwrap();
    assert_eq!(consent.cohort_month.to_string(), "2026-11");
    assert_eq!(consent.reported_day, parse_day("2026-11-03"));
    assert_eq!(consent.reported_month, UtcMonth::parse("2026-11"));
  }

  #[test]
  fn undecided_declined_and_outdated_grants_are_not_effective() {
    for consent in [
      ProductAnalyticsConsent::Undecided,
      ProductAnalyticsConsent::Declined,
    ] {
      let section = ProductAnalyticsSettings {
        consent,
        ..granted_section()
      };
      assert_eq!(GrantedConsent::from_settings(&section), None);
    }

    let outdated = ProductAnalyticsSettings {
      consent_version: CURRENT_PRODUCT_ANALYTICS_CONSENT_VERSION - 1,
      ..granted_section()
    };
    assert_eq!(GrantedConsent::from_settings(&outdated), None);
  }
}
