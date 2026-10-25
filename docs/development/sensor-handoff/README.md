# Super I/O Sensor Work — Session Handoff (#1635)

This folder contains ready-to-paste prompts for continuing the Windows
native Super I/O sensor work (#1635). Each session is intentionally
small and scoped so clean-room boundaries and PR scope stay clean.

## Status snapshot (updated 2026-10-25)

- ✅ **Phase 1** — CPU package temperature (PawnIO `IntelMSR` / `RyzenSMU`). Shipped.
- ✅ **Phase 2** — Super I/O chip-id diagnostic via PawnIO `LpcIO`. **Merged.**
  - PR [#1732](https://github.com/shm11C3/HardwareVisualizer/pull/1732) merged into `develop` (commit `d8bb4bb8`).
  - Spec gate resolved by [#1734](https://github.com/shm11C3/HardwareVisualizer/pull/1734) (`a8c167b1`); `docs/specs/sensors/superio-access.md` is `Implementation-ready (rev 4)`; the rev 3 ready scope remains the **Phase 2 raw chip-id diagnostic scope**, with the slot-1 ITE exit-withhold rule added in rev 4.
  - Code: `core/src/infrastructure/providers/windows/super_io_diagnostics.rs` (chip-id `0x20`/`0x21` only), routed through the `SuperIoPlatform` trait + `PlatformFactory` (post-review refactor). Pure helpers in `core/src/utils/super_io.rs`.
  - Command: `get_super_io_chip_id_diagnostics` / TS `commands.getSuperIoChipIdDiagnostics()`.
  - Hardware validation follow-ups remain useful for ITE graduation,
    PawnIO-absent behavior, and concurrent-monitor behavior, but do not
    block the current exact ready scopes.
- ✅ **Phase 3** — exact `0xD802` / NCT6799D normal-HM temperatures and
  direct fan RPM are **Implementation-ready (rev 5)** and implemented in
  the current motherboard-sensor provider and presentation pipeline.
- ✅ **Phase 4 spec** — exact `0x8728` / IT8728F/EX generic
  `TMPIN1`-`TMPIN3` is **Implementation-ready (rev 2)** as an
  Experimental read-only scope. Its implementation is a separate
  clean-room task that follows the rev 4 exit-withhold work. ITE fans
  and every other IT86xx/IT87xx ID remain disabled or Unsupported.
- ⚠️ **Phase 2 safety amendment** — `superio-access.md` rev 4 adds the
  "Slot-1 ITE exit withhold rule": on slot 1 (`0x4E`/`0x4F`) the ITE
  configuration-mode exit write must not be issued for raw IDs `0x8733`,
  `0x8790`, `0x8695`, `0x8883`, and the diagnostic reports the exit as
  intentionally skipped. The current diagnostic and motherboard provider
  still exit on every slot; applying the rule is a separate clean-room
  implementer task.

### What unblocks the next code

The `superio-access.md` rev 4 exit withhold is unblocked for a separate
clean-room implementation session covering
`core/src/infrastructure/providers/windows/super_io_diagnostics.rs` and
the ITE slot handling in `super_io_motherboard.rs`, plus the skipped-exit
state in the diagnostic model. It should land before further ITE chip
scopes widen the number of Gigabyte two-chip boards that run the slot-1
ITE probe.

The exact `0x8728` / IT8728F/EX Experimental temperature scope is
unblocked for a separate clean-room implementation session. That work
may implement exact-ID mapping, LDN `0x04` activation/base discovery,
PawnIO BAR authorization, and generic `TMPIN1`-`TMPIN3` reads strictly
from the ready Phase 2/4 specifications. It should reuse the existing
motherboard-sensor collection, UI, and External Component Guidance
pipeline.

ITE fan RPM and every other IT86xx/IT87xx ID remain blocked until a later
implementation-ready spec revision defines and enables those exact
scopes.

## Sessions

| File | Session | Role | Current state |
| --- | --- | --- | --- |
| [01-spec-gate.md](01-spec-gate.md) | Resolve the `superio-access.md` draft gate | spec author | ✅ Done (#1734 / `a8c167b1`) |
| [02-hardware-validation.md](02-hardware-validation.md) | Validate chip-id + capture register dumps on real Windows hardware | tester | ✅ Nuvoton ready evidence completed; ITE manual feedback and environment/concurrency cases remain follow-up |
| [07-phase3-nuvoton-spec.md](07-phase3-nuvoton-spec.md) | Author the Nuvoton register-map spec | spec author | ✅ Done: exact `0xD802` / NCT6799D normal-HM scope ready in rev 5 |
| [08-phase4-ite-spec.md](08-phase4-ite-spec.md) | Author the ITE register-map spec | spec author | ✅ Done: exact `0x8728` Experimental TMPIN1-3 scope ready in rev 2; ITE fan and broader-family scopes remain disabled |
| [03-chip-id-mapping.md](03-chip-id-mapping.md) | chip-id -> model mapping + hardware-monitor base discovery | clean-room implementer | ✅ Nuvoton done; exact `0x8728` ITE mapping/base implementation is ready for a separate clean-room session |
| [04-nuvoton-ite-decode.md](04-nuvoton-ite-decode.md) | Nuvoton or ITE decode, one family per change | clean-room implementer | ✅ Nuvoton done; ITE Experimental TMPIN1-3 implementation pending and unblocked; ITE fan remains blocked |
| [05-metrics-ui.md](05-metrics-ui.md) | Wire motherboard temps/fans into metrics stream + dashboard | implementer | ✅ Shipped; ITE temperatures reuse the existing pipeline without a successful-reading Experimental badge |
| [06-external-component-guidance.md](06-external-component-guidance.md) | PawnIO LpcIO guidance (separate from CPU-temp guidance) | implementer | ✅ Shipped; ITE reuses the existing failure/guidance policy rather than adding a new guidance path |

Recommended order from here:

1. In a separate clean-room implementer session, apply the rev 4 slot-1
   ITE exit-withhold rule to the diagnostic, motherboard provider, and
   diagnostic model state.
2. In a separate clean-room implementer session, add exact `0x8728`
   mapping, LDN/base authorization, and Experimental `TMPIN1`-`TMPIN3`
   sampling to the existing motherboard-sensor provider.
3. Reuse the shipped collection, presentation, and guidance pipeline;
   successful Experimental readings receive no badge or label suffix.
4. Collect manual user feedback/hardware dumps and, when accepted,
   revise the spec to graduate the exact scope from Experimental to
   Verified.
5. Do not implement ITE fan RPM or additional IT86xx/IT87xx IDs until a
   later implementation-ready spec revision enables them.

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
- `superio-access.md` は `Implementation-ready (rev 4)`。rev 3 からの ready 範囲は Phase 2 raw chip-id diagnostic のみで、rev 4 では slot-1 ITE exit-withhold rule が追加されている。
- Phase 3 Nuvoton は exact `0xD802` / NCT6799D normal-HM scope が
  `superio-nuvoton-nct67xx.md` rev 5 で ready・実装済み。
- Phase 4 ITE は exact `0x8728` / IT8728F/EX TMPIN1-3 のみ
  `superio-ite-it86xx-it87xx.md` rev 2 で Experimental ready。
- ITE fan とその他 ID は disabled/Unsupported のため実装しない。
```

## Post-#1732 operating policy

```md
# #1732 後の運用方針

#1732 は develop にマージ済み。
今後 #1732 のブランチには追加実装しない。

次の作業は別ブランチ・別PRで進める。

現在のrepoでは Nuvoton mapping/base/decode、motherboard metrics/UI、
External Component Guidance は実装済み。

次の clean-room implementation は別ブランチ・別PRとし、まず
`superio-access.md` rev 4 の slot-1 ITE exit-withhold rule を診断・
motherboard provider・診断 model に適用する。その後、ready rev 2 の
exact `0x8728` TMPIN1-3 だけを既存 pipeline へ追加する。ITE fan RPM、
その他 IT86xx/IT87xx ID、成功値への Experimental badge、新しい
telemetry/guidance path は追加しない。
```
