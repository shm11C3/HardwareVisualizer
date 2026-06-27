# Session 2: Validate the Chip-ID Diagnostic and Capture Register Dumps

## Goal

Run the merged Phase 2 Super I/O chip-id diagnostic on real Windows
self-built PC hardware, finish the open validation cases, and capture
raw register dumps that Phase 3 / Phase 4 spec authoring needs.

## State

- ✅ Nuvoton-class board: non-elevated access-denied + elevated chip-id captured.
- ⬜ ITE board, PawnIO-absent host, concurrent-monitor (HWiNFO / LHM / FanControl) behavior.
- ⬜ Raw hardware-monitor register dumps for the detected chip(s) — the
  primary input for clean-room Phase 3/4 specs.

## Paste this prompt into the next session

```md
# 目的

#1635 Phase 2 (merged) の Super I/O chip-id diagnostic を Windows 自作PC実機で
検証し、まだ取れていないケースと、Phase 3/4 spec authoring に必要な register dump を集めたい。

このセッションではコード追加よりも、実機からの証拠収集を優先する。

# 前提

- #1732 は develop にマージ済み（merge commit d8bb4bb8）。develop で作業する。
- Backend Tauri command: `get_super_io_chip_id_diagnostics`
- TypeScript binding: `commands.getSuperIoChipIdDiagnostics()`
- 既に Nuvoton系で 非昇格access-denied / 昇格chip-id 取得済み。残りを埋める。

# やること

1. develop を checkout する。
2. Windows環境でビルド/起動する。
3. PawnIO runtime / `LpcIO.bin` or `LpcIO.amx` の配置状況を確認する。
4. HardwareVisualizer を通常権限で起動して command を実行する。
5. HardwareVisualizer を管理者権限で起動して command を実行する。
6. 結果JSONを保存する。
7. 可能なら ITE機 / PawnIO非導入機 / 他モニタ併用 でも実行する。

# 収集したい情報

- motherboard model
- CPU
- OS version
- PawnIO runtime installedか
- `PawnIOLib.dll` path
- `LpcIO.bin` / `LpcIO.amx` path
- 通常権限での結果
- 管理者権限での結果
- slot 0 / slot 1 の結果
- Nuvoton attempt
  - idHigh
  - idLow
  - chipId
  - absent
  - error
  - exitError
- ITE attempt
  - idHigh
  - idLow
  - chipId
  - absent
  - error
  - exitError

# 期待する確認ポイント

- PawnIO absent / access denied / openable が区別できるか
- `0x80070005` が権限不足として見えるか
- Nuvoton / ITE のどちらかで meaningful な chip ID が取れるか
- `0x00/0x00` または `0xFF/0xFF` が absent として返るか
- HWiNFO / LibreHardwareMonitor / FanControl 等を起動中でも mutex timeout / access issue がどう出るか

# Phase 3/4 spec authoring 用に追加で残したい dump

- 検出された chip ID と確定した chip model
- hardware-monitor base address（取得できれば）
- temperature / fan tachometer register 周辺の raw bytes（independently collected dump として）
- これらは clean-room spec の normative source になり得るので、取得手順と環境を併記する

# 実装変更の扱い

- 原則、実装変更はしない。
- ただし diagnostic output が足りない場合は、最小限の追加だけ提案する。
- 変更が必要なら、その場で大きく直さず、別PR/別セッションに切る。

# 最終回答に含めること

- 実行環境
- 通常権限の結果
- 管理者権限の結果
- raw JSON
- Phase 3/4 spec authoring に渡せる register dump
- 次に必要な修正があるか
```
