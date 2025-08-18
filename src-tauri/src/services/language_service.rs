use sys_locale;

// 現在アプリがサポートしている言語のリスト
pub const SUPPORTED_LANGUAGES: [&str; 2] = ["en", "ja"];

///
/// デフォルトの言語設定を取得
///
pub fn get_default_language() -> String {
  let os_language = get_os_language();

  // 言語設定が取得でき、サポートされている場合はその言語を返す
  if let Some(language) = os_language
    && SUPPORTED_LANGUAGES.contains(&language.as_str())
  {
    return language;
  }

  // 一致しない場合は英語（デフォルト）を返す
  "en".to_string()
}

///
/// システムのロケール（言語設定）を取得
///
fn get_os_language() -> Option<String> {
  sys_locale::get_locale()
    .map(|locale| locale.split('-').next().unwrap_or(&locale).to_string())
}
