# Session 8: Phase 4 ITE Register-Map Spec

## Goal

Author an implementation-ready clean-room spec for ITE IT86xx/IT87xx
motherboard temperature and fan RPM reads.

## Paste this prompt into the next session

```md
# 目的

#1635 Phase 4 として、ITE IT86xx / IT87xx 系 Super I/O の temperature / fan RPM 読み取りに必要な clean-room spec を作成したい。

完了済み:
- Phase 2 chip-id diagnostic は #1732 で develop にマージ済み
- `docs/specs/sensors/superio-access.md` は Implementation-ready (rev 3)
- ただし ready 範囲は raw chip-id diagnostic まで

このセッションは **spec author** ロール。実装コードは触らない。

# やること

1. `docs/specs/sensors/README.md` と `.agents/rules/clean-room-sensors.md` を読む。
2. `docs/specs/sensors/superio-access.md` rev 3 を読む。
3. ITE IT86xx / IT87xx 系に必要な vendor datasheet / public hardware spec / independently collected dump を使って、fact-only specを作成する。

# 作るspec候補

`docs/specs/sensors/superio-ite-it86xx-it87xx.md`

# specに含めたい内容

- Scope / Revision / Status
- Sources table
- 対象chip family / supported chip IDs
- chip ID high/low registerの解釈（Phase 2 specとの接続）
- EC / hardware-monitor logical device
- EC base address discovery
- temperature sensor registers
- fan tachometer registers
- fan RPM conversion
- divisor / counter handling
- stopped / disconnected fan handling
- invalid range / plausibility rules
- read transaction order
- required writes for reads only
- safety policy:
  - fan control / PWM / threshold / alarm clear は対象外
  - no sensor-setting writes
- open questions
- implementation-ready transition checklist

# 重要制約

- LibreHardwareMonitor / OpenHardwareMonitor / Linux kernel / lm-sensors / decompiled tools のコード・構造・識別子をspecに持ち込まない。
- copyleft実装はnormative sourceにしない。
- 不明点はOpen questionsへ。
- TODO(provenance) が残るなら Draft のままにする。
- Implementation-readyにするなら、READMEのstatus transition条件を満たすこと。

# 成果物

- 新規specファイル、または既存specへの追加
- `docs/specs/sensors/README.md` の Current documents 更新
- rev history
- 次実装PRに書くべき provenance text

# 最終回答に含めること

- 作成/変更したspec
- Status（DraftかImplementation-readyか）
- 対象chip IDs
- 実装可能になった範囲
- まだ残るOpen questions
- 次の clean-room implementer セッションへの指示
```
