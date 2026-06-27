# Session 2: Validate the Chip-ID Diagnostic on Real Hardware

## Goal

Run Draft PR #1732's Super I/O chip-id diagnostic on real Windows
self-built PC hardware and capture evidence.

## Paste this prompt into the next session

```md
# 目的

Draft PR #1732 の Super I/O chip-id diagnostic を Windows 自作PC実機で検証したい。

このセッションではコード追加よりも、実機からの証拠収集を優先する。

# 前提

PR #1732:
- Branch: feat/1635-superio-chip-id-diagnostics
- Session target: `getSuperIoChipIdDiagnostics()` diagnostic flow
- Backend Tauri command: `get_super_io_chip_id_diagnostics`
- TypeScript binding: `commands.getSuperIoChipIdDiagnostics()`

# やること

1. #1732 のブランチを checkout する。
2. Windows環境でビルド/起動する。
3. PawnIO runtime / `LpcIO.bin` or `LpcIO.amx` の配置状況を確認する。
4. HardwareVisualizer を通常権限で起動して command を実行する。
5. HardwareVisualizer を管理者権限で起動して command を実行する。
6. 結果JSONを保存する。

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

# 実装変更の扱い

- 原則、実装変更はしない。
- ただし diagnostic output が足りない場合は、最小限の追加だけ提案する。
- 変更が必要なら、その場で大きく直さず、別PR/別セッションに切る。

# 最終回答に含めること

- 実行環境
- 通常権限の結果
- 管理者権限の結果
- raw JSON
- 判断: #1732 のdiagnosticは実機検証に使えるか
- 次に必要な修正があるか
```
