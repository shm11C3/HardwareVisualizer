# Super I/O Sensor Work — Session Handoff (#1635)

This folder contains ready-to-paste prompts for continuing the Windows
native Super I/O sensor work (#1635). Each session is intentionally
small and scoped so clean-room boundaries and PR scope stay clean.

## Status snapshot (updated 2026-06-27)

- Draft PR: [#1732](https://github.com/shm11C3/HardwareVisualizer/pull/1732)
  — `feat(sensors): Super I/O chip-id diagnostic via PawnIO LpcIO`
- Branch: `feat/1635-superio-chip-id-diagnostics`
- Base: `develop`
- Current scope: read Super I/O chip-id registers (`0x20` / `0x21`)
  through PawnIO `LpcIO`, nothing more.
- Spec gate: resolved by
  [#1734](https://github.com/shm11C3/HardwareVisualizer/pull/1734)
  / commit `a8c167b1`. `docs/specs/sensors/superio-access.md` is now
  `Implementation-ready (rev 3)` for the Phase 2 raw chip-id diagnostic
  scope only.
- #1732 still needs PR hygiene before it can leave Draft:
  - re-pin provenance to `pawnio-interface.md` rev 4 and
    `superio-access.md` rev 3,
  - complete the implementer attestation box for ready specs,
  - file and link the Phase 2 child issue under #1635,
  - decide `LpcIO.bin` distribution/bundling and LGPL third-party notice
    handling.
- Next execution session: [02-hardware-validation.md](02-hardware-validation.md).

## Sessions

| File | Session | Role | Current state |
| --- | --- | --- | --- |
| [01-spec-gate.md](01-spec-gate.md) | Resolve the `superio-access.md` draft gate | spec author | Done via #1734 / `a8c167b1` |
| [02-hardware-validation.md](02-hardware-validation.md) | Validate the chip-id diagnostic on real Windows hardware | tester | Next |
| [03-chip-id-mapping.md](03-chip-id-mapping.md) | chip-id -> model mapping + hardware-monitor base discovery | clean-room implementer | Not started; needs new ready specs for chip tables/base discovery |
| [04-nuvoton-ite-decode.md](04-nuvoton-ite-decode.md) | Nuvoton OR ITE temperature + fan RPM decode (one per PR) | clean-room implementer | Not started; blocked on per-family ready specs and dumps |
| [05-metrics-ui.md](05-metrics-ui.md) | Wire motherboard temps/fans into metrics stream + dashboard | implementer | Not started; blocked until low-level provider returns motherboard temps/fans |
| [06-external-component-guidance.md](06-external-component-guidance.md) | PawnIO LpcIO guidance (separate from CPU-temp guidance) | implementer | Not started; can be split once motherboard-sensor failure states are defined |

Recommended order from here: #1732 PR hygiene -> 02 -> 03 -> 04 -> 05 -> 06.
Sessions 04, 05, and 06 each become their own PR. Keep 03 separate unless it is
only a small diagnostic extension.

## Common preamble (paste at the start of every session)

```md
対象リポジトリ: /Users/shm11c3/Develop/HardwareVisualizer

現在の関連Draft PR:
- #1732: feat(sensors): Super I/O chip-id diagnostic via PawnIO LpcIO
- Branch: feat/1635-superio-chip-id-diagnostics
- Base: develop

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
- chip-model mapping / hardware-monitor base discovery / temperature / fan RPM には、別途 ready spec が必要。
- Draft PR #1732 は、PR本文の provenance re-pin / attestation / Phase 2 child issue link / `LpcIO.bin` distribution方針が未更新。
```

## #1732 operating policy

```md
# #1732 運用方針

#1732 は、PR本文の provenance re-pin / attestation / Phase 2 child issue
link / `LpcIO.bin` distribution方針が更新されるまでは Draft として維持する。

このPRでやるのは:
- LpcIOでchip IDを読む最初のdiagnostic
- clean-room gate / next steps / 実機検証計画の明文化
- gate解消済みspecへのprovenance re-pin

このPRでやらない:
- 温度取得
- ファンRPM取得
- hardware-monitor base decode
- UI表示
- External Component Guidance本実装

CIが赤くなった場合だけ、このPR内で最小修正する。
TODO本体は別セッション・別PRで進める。
```
