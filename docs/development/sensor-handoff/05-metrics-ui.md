# Session 5: Metrics Stream and Motherboard UI Wiring

## Goal

Expose motherboard temperatures and fan RPMs through the live metrics
pipeline and the Dashboard Motherboard card.

## Resolved design decisions

- Implement one PR that includes the scoped Nuvoton read provider, live
  metrics stream wiring, and Dashboard Motherboard card display.
- Keep ITE support, External Component Guidance, persistence, and new
  settings out of this PR.
- Enable only the implementation-ready `0xD802` / `NCT6799D` normal HM
  read path from `docs/specs/sensors/superio-nuvoton-nct67xx.md` rev 5.
- Cache Super I/O detection for the process lifetime only; do not write
  detection results to `settings.json` or Tauri Store.
- During each live sample, reuse the cached slot/chip/HM-base detection
  and read only the enabled bank 4 temperature and direct RPM registers.
- Show all returned motherboard temperature and fan-speed readings in the
  Dashboard Motherboard card.
- Keep motherboard temperatures separate from CPU thermal zones and GPU
  sensors.
- Show a compact source label near the motherboard sensor sections,
  similar to the existing GPU usage source display.
- Use `FanSpeedStatus::{Active, Inactive, Invalid}`. Treat 0 RPM as
  `Inactive`, not as disconnection or failure, because fans may stop
  normally at low temperature.
- Hide motherboard sensor sections when no readings are available. Do not
  show PawnIO / privilege / unsupported-chip reasons on the Dashboard in
  this first UI slice; leave user guidance to a later External Component
  Guidance session.
- Do not create an ADR for these decisions; this handoff document is the
  decision record for the first dashboard slice.

## Paste this prompt into the next session

````md
# 目的

Core側で取得できるようになった motherboard temperature / fan RPM を、HardwareVisualizer の metrics stream と Dashboard の Motherboard card に接続する。

このセッションはUI/metrics接続が主目的。低レベルSuper I/O register実装はしない。

# 前提

低レベルprovider側で、少なくとも以下が返せる状態になっていること。

- motherboard temperatures
- motherboard fan speeds
- source / sensor name

もし低レベルproviderがまだない場合、このセッションでは mock / fixture だけでパイプラインを作るか、作業を止める。

# 実装方針

CPUの `sensor_temperatures` には混ぜない。
マザーボード用に分離する。

Core:

```rust
pub struct FanSpeed {
  pub name: String,
  pub rpm: Option<u32>,
  pub status: FanSpeedStatus,
}

pub enum FanSpeedStatus {
  Active,
  Inactive,
  Invalid,
}

pub struct MetricsSnapshot {
  // existing fields...
  pub motherboard_temperatures: Vec<SensorTemperature>,
  pub motherboard_fan_speeds: Vec<FanSpeed>,
}
```

Tauri wire:

```rust
pub struct HardwareMonitorUpdate {
  // existing fields...
  pub motherboard_temperatures: Vec<NameValue>,
  pub motherboard_fan_speeds: Vec<FanSpeedValue>,
}
```

Frontend:

- `motherboardTempsAtom`
- `motherboardFanSpeedsAtom`
- `useHardwareEventListener` に追加
- `MotherboardDataInfo` に表示
- i18n:
  - `motherboardSensors.title`
  - `motherboardSensors.temperatures`
  - `motherboardSensors.fanSpeeds`
  - `motherboardSensors.status.active`
  - `motherboardSensors.status.inactive`
  - `motherboardSensors.status.invalid`

# render performance注意

過去に live metrics / thermal sensor rows の再レンダリング問題があったため:

- 既存の live metrics atom 更新パターンに寄せる
- motherboard sensor rows は Motherboard card 内だけに閉じる
- 実測で問題が出た場合に render-fanout test と memoization を追加する

# 表示ルール

- センサーがない場合はセクションを出さない
- 温度単位は既存 `settings.temperatureUnit` に従う
- fan RPMは単位変換しない
- fan status は Active / Inactive / Invalid を区別する
- 0 RPM は Inactive として表示し、切断や故障とは表現しない
- CPU thermal zonesとは別セクションにする
- Motherboard cardだけに表示する

# テスト

- `useHardwareEventListener` test
- atom更新test
- Motherboard card rendering / e2e fixture確認
- generated bindings更新確認
- `npm run build`
- 必要なら `DashboardItems.renderFanout.test.tsx` 系の回帰テスト

# 最終回答に含めること

- Core → Tauri → frontend の接続範囲
- UI表示仕様
- render-fanout対策
- validation結果
- 実機センサーなし環境での挙動
````
