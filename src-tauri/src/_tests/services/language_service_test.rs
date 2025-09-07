#[cfg(test)]
mod tests {
  use crate::services::language_service::*;

  #[test]
  fn test_supported_languages_constant() {
    assert_eq!(SUPPORTED_LANGUAGES, ["en", "ja"]);
    assert_eq!(SUPPORTED_LANGUAGES.len(), 2);
  }

  #[test]
  fn test_get_default_language_returns_supported() {
    let default_language = get_default_language();

    // デフォルト言語はサポートされている言語のいずれかである
    assert!(
      SUPPORTED_LANGUAGES.contains(&default_language.as_str()),
      "Default language '{}' should be in supported languages",
      default_language
    );
  }

  #[test]
  fn test_get_default_language_not_empty() {
    let default_language = get_default_language();
    assert!(
      !default_language.is_empty(),
      "Default language should not be empty"
    );
  }

  #[test]
  fn test_get_default_language_fallback() {
    let default_language = get_default_language();

    // システムの言語設定に関わらず、サポートされている言語が返されるべき
    assert!(
      default_language == "en" || default_language == "ja",
      "Default language should be either 'en' or 'ja', got '{}'",
      default_language
    );
  }

  #[test]
  fn test_supported_languages_contains_en() {
    assert!(SUPPORTED_LANGUAGES.contains(&"en"));
  }

  #[test]
  fn test_supported_languages_contains_ja() {
    assert!(SUPPORTED_LANGUAGES.contains(&"ja"));
  }
}
