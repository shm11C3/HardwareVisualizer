# Session 1: Resolve the Super I/O Spec Gate

> Status: completed by
> [#1734](https://github.com/shm11C3/HardwareVisualizer/pull/1734)
> / commit `a8c167b1`.
>
> The chosen path was Option B: scope-flip only the Phase 2 raw chip-id
> diagnostic surface to `Implementation-ready (rev 3)`. Keep this prompt as
> historical context. Re-run it only if the ready scope expands beyond raw
> chip-id diagnostics.

## Original goal

Resolve the `docs/specs/sensors/superio-access.md` draft gate that blocks
Draft PR #1732 from becoming a merge-ready clean-room implementation PR.

## Follow-up for #1732

Update the PR body to pin:

- `docs/specs/sensors/pawnio-interface.md` revision 4.
- `docs/specs/sensors/superio-access.md` revision 3, commit `a8c167b1`.

Then complete the implementer attestation item that every pinned spec is
`Implementation-ready` with no unresolved `TODO(provenance)` markers.

## Historical prompt

```md
# 目的

#1635 / Draft PR #1732 のブロッカーである `docs/specs/sensors/superio-access.md` の Draft gate を解消したい。

現在 #1732 は `LpcIO` で Super I/O chip ID registers (`0x20` / `0x21`) を読む diagnostic 実装だが、参照している `superio-access.md` が `Draft — not implementation-ready` のため clean-room implementation PR としてはmerge-readyではない。

このセッションでは実装コードには触らず、spec側を整理する。

# やること

1. `docs/specs/sensors/README.md`
2. `.github/instructions/clean-room-sensors.instructions.md`
3. `docs/specs/sensors/superio-access.md`
4. `docs/specs/sensors/pawnio-interface.md`

を読み、clean-room gateを確認する。

そのうえで、次のどちらが良いか判断してほしい。

## Option A

`superio-access.md` 全体を `Implementation-ready` にする。

## Option B

まず #1732 の範囲だけ、つまり:

- LpcIO slot 0 / slot 1
- config port pairs `0x2E/0x2F`, `0x4E/0x4F`
- Nuvoton enter / exit sequence
- ITE enter / exit sequence
- chip ID registers `0x20` / `0x21`
- absent id rule `0x00/0x00` or `0xFF/0xFF`
- `Global\Access_ISABUS.HTP.Method` mutex

に限定して implementation-ready にする。

# 成果物

- `docs/specs/sensors/superio-access.md` の改訂案
- 必要なら別specに分割する案
- revision bump
- status update
- unresolved open questions の整理
- #1732 のPR本文に反映すべき provenance text

# 制約

- 実装コードは変更しない。
- spec author ロールとして作業する。
- vendor datasheet / public hardware spec / independently collected dump は参照してよい。
- ただし、LibreHardwareMonitor / OpenHardwareMonitor / Linux kernel / lm-sensors / decompiled tool のコードや構造をspecに持ち込まない。
- copyleft実装はnormative sourceにしない。
- 不確かな点は fact table に入れず、Open questions に残す。

# 最終回答に含めること

- Option A / B の推奨
- 変更したファイル
- まだ残るblocker
- #1732 をready化するために必要なPR本文更新案
```
