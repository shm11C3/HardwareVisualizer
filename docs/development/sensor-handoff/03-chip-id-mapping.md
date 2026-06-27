# Session 3: Chip-ID Mapping and Hardware-Monitor Base Discovery

## Goal

After chip ID bytes are readable, add chip model classification and
hardware-monitor base discovery.

## Paste this prompt into the next session

```md
# 目的

マージ済み Phase 2 chip-id diagnostic の次の実装段階として、Super I/O chip ID diagnostic の結果から chip model を識別し、hardware-monitor base address discovery に進めたい。

ただし、このセッションは clean-room implementation セッションなので、実装前に必ず spec readiness を確認すること。

# 事前確認

最初に以下を確認する。

1. `docs/specs/sensors/superio-access.md` が、この実装範囲について `Implementation-ready` になっているか。
2. chip ID table / hardware-monitor base discovery に必要なspecが存在するか。
3. Nuvotonなら `superio-nuvoton-*`、ITEなら `superio-ite-*` の per-family spec が `Implementation-ready` か確認する。
4. まだ Draft / 未作成なら、実装せずに止めて、必要なspec gapを列挙する。

# やりたい実装範囲

可能なら以下を実装する。

- Nuvoton / ITE の chip ID → chip model mapping
- recognized / unknown / absent の分類
- hardware-monitor logical device select
  - Nuvoton: logical device `0x0B`
  - ITE: logical device `0x04`
- base address registers `0x60 / 0x61` のread
- base address absent rule
  - `0x0000`
  - `0xFFFF`
- `ioctl_find_bars`
- diagnostic outputに以下を追加
  - chipModel
  - recognized
  - hardwareMonitorBase
  - findBars result
  - detectionStage errors

# 制約

- temperature / fan / voltage はまだ読まない。
- fan control / limit / alarm register には触らない。
- mutex `Global\Access_ISABUS.HTP.Method` をtransaction全体で保持する。
- re-detection cachingは設計だけでよい。実装する場合は別commitでもよい。
- prohibited sources は参照しない。

# テスト

- pure helper は `core/src/utils/**` に置き、macOS/Linux CIでもテストされるようにする。
- Windows-only providerも `x86_64-pc-windows-gnu` などでcompile確認する。
- `RUSTFLAGS="-D warnings"` でも確認する。

# 最終回答に含めること

- spec readinessの判定
- 実装した範囲
- まだ実装しなかった範囲
- validation結果
- 次PRに切るべき作業
```
