# Session 6: External Component Guidance for PawnIO LpcIO

## Goal

Add user-facing guidance for the motherboard-sensor PawnIO LpcIO path,
separate from the existing CPU package temperature PawnIO guidance.

## Paste this prompt into the next session

````md
# 目的

PawnIO LpcIO / Super I/O motherboard sensor 用の External Component Guidance を設計・実装したい。

既存の PawnIO CPU package temperature guidance とは別扱いにする。

# 背景

既存:

```text
pawnio:cpu-package-temperature:v1
```

これは CPU package temperature 用。

新しく必要:

```text
pawnio:motherboard-sensors:v1
```

またはより具体的に:

```text
pawnio-lpcio:motherboard-sensors:v1
```

# やること

- key名を決める
- `ExternalComponentUsage` に MotherboardSensors を追加するか検討
- missing signals:
  - motherboard-temperature
  - motherboard-fan-speed
- reason分類:
  - PawnIO runtime missing
  - LpcIO module missing
  - access denied / elevation required
  - unsupported / unknown Super I/O chip
- docs/user/external-components.md / .ja.md に案内追加
- frontend dialog copy追加
- view filtering:
  - Dashboard / Motherboard card でだけ出すか
  - SettingsのExternal Componentsから見えるようにするか

# 注意

- すぐにdialogを出しすぎない。
- chip ID diagnosticが未実行なだけでguidanceを出さない。
- motherboard sensor displayが有効で、かつ必要な信号が取れない場合に限定する。
- CPU温度のPawnIO guidanceと混同しない。

# 成果物

- guidance key設計
- core model更新
- tauri wire更新
- frontend copy / i18n
- docs/user更新
- tests

# 最終回答に含めること

- 追加したguidance key
- 表示条件
- CPU temperature guidanceとの差分
- 残タスク
````
