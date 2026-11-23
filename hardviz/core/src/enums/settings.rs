/// POJO mirror of `src-tauri/src/enums/settings.rs::TemperatureUnit`.
///
/// The wire variant (with `specta::Type` and explicit Serialize / Deserialize
/// impls) lives in `src-tauri`. This Core enum is what platform sensor
/// functions accept and what the App-side converts to before calling.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TemperatureUnit {
  Celsius,
  Fahrenheit,
}
