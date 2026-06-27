# Session 5: Metrics Stream and Motherboard UI Wiring

## Goal

Expose motherboard temperatures and fan RPMs through the live metrics
pipeline and the Dashboard Motherboard card.

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
  pub rpm: u32,
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
  - `motherboardTemperatures`
  - `fanSpeeds`

# render performance注意

過去に live metrics / thermal sensor rows の再レンダリング問題があったため:

- unchanged arraysでatomを更新しない
- row renderingをmemo化する
- 既存の render-fanout test に近い形で回帰テストを追加する

# 表示ルール

- センサーがない場合はセクションを出さない
- 温度単位は既存 `settings.temperatureUnit` に従う
- fan RPMは単位変換しない
- CPU thermal zonesとは別セクションにする
- Motherboard cardだけに表示する

# テスト

- `useHardwareEventListener` test
- atom更新test
- Motherboard card rendering test
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
