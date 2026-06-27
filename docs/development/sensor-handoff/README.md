# Super I/O Sensor Work — Session Handoff (#1635)

This folder contains ready-to-paste prompts for continuing the Windows
native Super I/O sensor work (#1635). Each session is intentionally
small and scoped so clean-room boundaries and PR scope stay clean.

## Status snapshot (updated 2026-06-27)

- ✅ **Phase 1** — CPU package temperature (PawnIO `IntelMSR` / `RyzenSMU`). Shipped.
- ✅ **Phase 2** — Super I/O chip-id diagnostic via PawnIO `LpcIO`. **Merged.**
  - PR [#1732](https://github.com/shm11C3/HardwareVisualizer/pull/1732) merged into `develop` (commit `d8bb4bb8`).
  - Spec gate resolved by [#1734](https://github.com/shm11C3/HardwareVisualizer/pull/1734) (`a8c167b1`); `docs/specs/sensors/superio-access.md` is `Implementation-ready (rev 3)` for the **Phase 2 raw chip-id diagnostic scope only**.
  - Code: `core/src/infrastructure/providers/windows/super_io_diagnostics.rs` (chip-id `0x20`/`0x21` only), routed through the `SuperIoPlatform` trait + `PlatformFactory` (post-review refactor). Pure helpers in `core/src/utils/super_io.rs`.
  - Command: `get_super_io_chip_id_diagnostics` / TS `commands.getSuperIoChipIdDiagnostics()`.
  - Hardware validation: Nuvoton-class board captured (non-elevated access-denied + elevated chip-id). **Still open:** ITE board, PawnIO-absent host, concurrent-monitor (HWiNFO / LHM / FanControl) behavior — fold these into the Phase 3 hardware-dump session below.
- ⬜ **Phase 3** — Nuvoton NCT67xx/NCT679x register map (temps + fan RPM). **Spec not written.** ← current focus.
- ⬜ **Phase 4** — ITE IT86xx/87xx register map (temps + fan RPM). **Spec not written.**

### What unblocks the next code

No further *reading* implementation can start until the per-chip
register-map specs (Phase 3 / Phase 4) and hardware-monitor base
discovery are authored and flipped to `Implementation-ready`. The
`superio-access.md` ready scope stops at the raw chip-id diagnostic.
So **spec authoring comes first** ([07](07-phase3-nuvoton-spec.md) /
[08](08-phase4-ite-spec.md)), ideally informed by real chip-id +
register dumps collected via [02](02-hardware-validation.md).

## Sessions

| File | Session | Role | Current state |
| --- | --- | --- | --- |
| [01-spec-gate.md](01-spec-gate.md) | Resolve the `superio-access.md` draft gate | spec author | ✅ Done (#1734 / `a8c167b1`) |
| [02-hardware-validation.md](02-hardware-validation.md) | Validate chip-id + capture register dumps on real Windows hardware | tester | 🔶 Partial (Nuvoton-class captured; ITE / PawnIO-absent / concurrent pending) |
| [07-phase3-nuvoton-spec.md](07-phase3-nuvoton-spec.md) | Author the Nuvoton register-map spec | spec author | ⬜ **Next** |
| [08-phase4-ite-spec.md](08-phase4-ite-spec.md) | Author the ITE register-map spec | spec author | ⬜ Not started |
| [03-chip-id-mapping.md](03-chip-id-mapping.md) | chip-id -> model mapping + hardware-monitor base discovery | clean-room implementer | ⬜ Blocked on ready Phase 3/4 specs |
| [04-nuvoton-ite-decode.md](04-nuvoton-ite-decode.md) | Nuvoton OR ITE temperature + fan RPM decode (one per PR) | clean-room implementer | ⬜ Blocked on ready per-family specs + dumps |
| [05-metrics-ui.md](05-metrics-ui.md) | Wire motherboard temps/fans into metrics stream + dashboard | implementer | ⬜ Blocked until the provider returns motherboard temps/fans |
| [06-external-component-guidance.md](06-external-component-guidance.md) | PawnIO LpcIO guidance (separate from CPU-temp guidance) | implementer | ⬜ Blocked until motherboard-sensor failure states are defined |

Recommended order from here:
`02 dumps` → `07 (and/or 08) spec authoring` → `03 mapping + base discovery`
→ `04 decode` → `05 metrics/UI` → `06 guidance`.
Sessions 03/04/05/06 each become their own PR; split 04 per chip family
(Nuvoton PR, then ITE PR).

## Common preamble (paste at the start of every session)

```md
対象リポジトリ: /Users/shm11c3/Develop/HardwareVisualizer

#1732 は develop にマージ済み:
- #1732: feat(sensors): Super I/O chip-id diagnostic via PawnIO LpcIO
- Merge commit: d8bb4bb8
- Command: get_super_io_chip_id_diagnostics / commands.getSuperIoChipIdDiagnostics()

この作業は #1635 の Windows native sensor / Super I/O / PawnIO 関連作業です。

重要:
- clean-room sensor rules を守ること。
- 実装側セッションでは LibreHardwareMonitor / OpenHardwareMonitor / Linux kernel / lm-sensors / decompiled monitoring tool を参照しないこと。
- 参照してよいのは、このrepo内の `docs/specs/sensors/**` とこのrepo自身。
- spec author セッションだけは、vendor datasheet / public hardware spec / independently collected dump を扱ってよいが、実装コードやcopyleft実装からコード構造を持ち込まないこと。
- PR作成時は `CONTRIBUTING.md` に従い、branch prefix / commit prefix / PR title / PR type checkbox を揃えること。
- HardwareVisualizer のPR baseは基本 `develop`。
- `src/rspc/bindings.ts` は生成物。必要なら `npm run tauri:dev` など既存の生成経路で更新し、手編集しない。
- `superio-access.md` は `Implementation-ready (rev 3)` だが、ready範囲は Phase 2 raw chip-id diagnostic のみ。
- chip-model mapping / hardware-monitor base discovery / temperature / fan RPM には、別途 Phase 3/4 の ready spec が必要。
- Phase 3 の前に、Nuvoton/ITE実機dumpやvendor datasheetに基づくspec authoringを行うこと。
```

## Post-#1732 operating policy

```md
# #1732 後の運用方針

#1732 は develop にマージ済み。
今後 #1732 のブランチには追加実装しない。

次の作業は別ブランチ・別PRで進める。

次PRでまだやらない:
- 温度取得
- ファンRPM取得
- hardware-monitor base decode
- UI表示
- External Component Guidance本実装

Phase 3/4のspecが ready になるまで、低レベル読み取り実装に進まない。
```
