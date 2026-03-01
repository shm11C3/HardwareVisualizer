use sys_locale;

// List of languages currently supported by the app
pub const SUPPORTED_LANGUAGES: [&str; 2] = ["en", "ja"];

///
/// Get default language setting
///
pub fn get_default_language() -> String {
  let os_language = get_os_language();
  resolve_language(os_language.as_deref())
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
}
