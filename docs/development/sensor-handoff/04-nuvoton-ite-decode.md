# Session 4: Nuvoton or ITE Temperature/Fan RPM Decode

## Goal

Implement real motherboard temperature and fan RPM decoding for **one**
Super I/O family per session/PR.

## Paste this prompt into the next session

````md
# 目的

Super I/O chip detection / hardware-monitor base discovery の次段階として、マザーボード温度とファンRPMを読みたい。

このセッションでは、まず Nuvoton または ITE のどちらか一方だけを対象にする。
両方を同時にやらない。

# 最初に決めること

対象を一つ選ぶ。

- Nuvoton NCT67xx / NCT679x
- ITE IT86xx / IT87xx

選んだ対象について、必要なspecが `Implementation-ready` になっているか確認する。
Draftなら実装せず、spec gapを整理して終了する。

想定spec:

- Nuvoton: `docs/specs/sensors/superio-nuvoton-nct67xx.md`
- ITE: `docs/specs/sensors/superio-ite-it86xx-it87xx.md`

# 実装範囲

対象chip familyについて:

- temperature sensor registers
- fan tachometer registers
- fan RPM conversion
- disconnected / stopped fan handling
- invalid range filter
- sensor name labeling
- diagnostic source metadata

# やらないこと

- fan control
- PWM write
- threshold write
- alarm clear
- voltage read
- UI実装
- persistence
- unrelated CPU/GPU sensor変更

# モデル設計

必要ならCore側に以下を追加する。

```rust
pub struct MotherboardTemperature {
  pub name: String,
  pub temperature_celsius: f32,
  pub source: String,
}

pub struct MotherboardFanSpeed {
  pub name: String,
  pub rpm: Option<u32>,
  pub status: FanSpeedStatus,
  pub source: String,
}

pub enum FanSpeedStatus {
  Active,
  Stopped,
  Disconnected,
  Invalid,
}
```

ただし、最終的な `MetricsSnapshot` 接続は別セッションでもよい。
decode flowでは tachometer raw value から `rpm` と `status` を分けて返す。
実測0 RPM、停止、未接続、範囲外を同じ `0` に潰さない。

# テスト方針

- decode関数はpure helperにする。
- 実機dump / synthetic register bytes をfixture化する。
- macOS/Linux CIでもdecode testが動くようにする。
- Windows providerはcross compile確認する。
- 実機検証結果があれば、それに基づくfixtureを追加する。

# clean-room制約

- 実装コードは `docs/specs/sensors/**` とこのrepoだけから書く。
- LibreHardwareMonitor / OpenHardwareMonitor / Linux kernel / lm-sensors / decompiled tools は参照禁止。
- 足りない情報があれば、実装せずspec authorセッションへ戻す。

# 最終回答に含めること

- 対象chip family
- 参照spec revision
- 実装したregister decode
- 実装しなかったregister
- 実機検証の有無
- 次のPRでやるべきこと
````
