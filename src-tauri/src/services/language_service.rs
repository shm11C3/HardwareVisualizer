use sys_locale;

// List of languages currently supported by the app
pub const SUPPORTED_LANGUAGES: [&str; 3] = ["en", "ja", "ru"];
const E2E_ENV: &str = "HARDVIZ_E2E";
const E2E_DEFAULT_LANGUAGE_ENV: &str = "HARDVIZ_E2E_DEFAULT_LANGUAGE";

///
/// Get default language setting
///
pub fn get_default_language() -> String {
  let os_language = get_os_language();
  let e2e_language = std::env::var(E2E_DEFAULT_LANGUAGE_ENV).ok();

  default_language_for(
    os_language.as_deref(),
    is_env_enabled(E2E_ENV),
    e2e_language.as_deref(),
  )
}

fn is_env_enabled(key: &str) -> bool {
  std::env::var(key).as_deref() == Ok("1")
}

fn default_language_for(
  os_language: Option<&str>,
  e2e_enabled: bool,
  e2e_language: Option<&str>,
) -> String {
  if e2e_enabled && let Some(language) = e2e_language {
    return resolve_language(Some(language));
  }

  resolve_language(os_language)
}

/// Resolve a language code to a supported language, falling back to "en".
fn resolve_language(language: Option<&str>) -> String {
  match language {
    Some(lang) if SUPPORTED_LANGUAGES.contains(&lang) => lang.to_string(),
    _ => "en".to_string(),
  }
}

/// Extract language code from a locale string (e.g. "ja-JP" -> "ja").
fn extract_language_code(locale: &str) -> &str {
  locale.split('-').next().unwrap_or(locale)
}

///
/// Get system locale (language setting)
///
fn get_os_language() -> Option<String> {
  sys_locale::get_locale().map(|locale| extract_language_code(&locale).to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  // ── extract_language_code ──

  #[test]
  fn extract_language_code_with_region() {
    assert_eq!(extract_language_code("ja-JP"), "ja");
  }

  #[test]
  fn extract_language_code_without_region() {
    assert_eq!(extract_language_code("en"), "en");
  }

  #[test]
  fn extract_language_code_multiple_parts() {
    assert_eq!(extract_language_code("zh-Hant-TW"), "zh");
  }

  #[test]
  fn extract_language_code_empty_string() {
    assert_eq!(extract_language_code(""), "");
  }

  // ── resolve_language ──

  #[test]
  fn resolve_language_supported_ja() {
    assert_eq!(resolve_language(Some("ja")), "ja");
  }

  #[test]
  fn resolve_language_supported_en() {
    assert_eq!(resolve_language(Some("en")), "en");
  }

  #[test]
  fn resolve_language_unsupported_falls_back_to_en() {
    assert_eq!(resolve_language(Some("fr")), "en");
  }

  #[test]
  fn resolve_language_none_falls_back_to_en() {
    assert_eq!(resolve_language(None), "en");
  }

  #[test]
  fn resolve_language_empty_string_falls_back_to_en() {
    assert_eq!(resolve_language(Some("")), "en");
  }

  // ── get_default_language ──

  #[test]
  fn default_language_uses_e2e_language_when_e2e_is_enabled() {
    assert_eq!(default_language_for(Some("ja"), true, Some("en")), "en");
    assert_eq!(default_language_for(Some("en"), true, Some("ja")), "ja");
  }

  #[test]
  fn default_language_ignores_e2e_language_when_e2e_is_disabled() {
    assert_eq!(default_language_for(Some("ja"), false, Some("en")), "ja");
  }

  #[test]
  fn default_language_uses_os_language_when_e2e_language_is_missing() {
    assert_eq!(default_language_for(Some("ja"), true, None), "ja");
  }

  #[test]
  fn default_language_falls_back_to_en_for_unsupported_e2e_language() {
    assert_eq!(default_language_for(Some("ja"), true, Some("fr")), "en");
  }

  #[test]
  fn get_default_language_returns_supported_language() {
    let result = get_default_language();
    assert!(
      SUPPORTED_LANGUAGES.contains(&result.as_str()),
      "Expected a supported language, got: {result}"
    );
  }

  // ── SUPPORTED_LANGUAGES ──

  #[test]
  fn supported_languages_contains_en() {
    assert!(SUPPORTED_LANGUAGES.contains(&"en"));
  }

  #[test]
  fn supported_languages_contains_ja() {
    assert!(SUPPORTED_LANGUAGES.contains(&"ja"));
  }

  #[test]
  fn supported_languages_contains_ru() {
    assert!(SUPPORTED_LANGUAGES.contains(&"ru"));
  }
}
