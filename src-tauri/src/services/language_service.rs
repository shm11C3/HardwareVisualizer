use sys_locale;

// List of languages currently supported by the app
pub const SUPPORTED_LANGUAGES: [&str; 2] = ["en", "ja"];

///
/// Get default language setting
///
pub fn get_default_language() -> String {
  let os_language = get_os_language();

  // Return the language if it can be obtained and is supported
  if let Some(language) = os_language
    && SUPPORTED_LANGUAGES.contains(&language.as_str())
  {
    return language;
  }

  // Return English (default) if no match
  "en".to_string()
}

///
/// Get system locale (language setting)
///
fn get_os_language() -> Option<String> {
  sys_locale::get_locale()
    .map(|locale| locale.split('-').next().unwrap_or(&locale).to_string())
}
