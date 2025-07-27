# アーキテクチャ移行計画

## 概要

HardwareVisualizer プロジェクトにおいて、macOS 対応を見据えたプラットフォーム抽象化アーキテクチャへの移行計画を記述します。

## 現在のアーキテクチャ

```
src/
├── commands/          # Tauri commands
├── services/          # Business logic + Platform implementations
│   ├── platform/      # Platform-specific code (mixed)
│   ├── nvidia_gpu_service.rs
│   ├── wmi_service.rs
│   └── ...
├── structs/
└── utils/
```

### 問題点

- Platform 層と Services 層の循環依存
- OS 固有コードとビジネスロジックの混在
- macOS 対応時の影響範囲が不明確
- テスト時のプラットフォーム依存性

## 目標アーキテクチャ

```
src/
├── commands/          # UI interface layer
├── services/          # Business logic layer
├── platform/          # Platform abstraction layer
│   ├── traits.rs      # Common interfaces
│   ├── factory.rs     # Platform selection
│   ├── common/        # Shared utilities
│   ├── windows/       # Windows implementations
│   ├── linux/         # Linux implementations
│   └── macos/         # macOS implementations (future)
├── structs/
└── utils/
```

### 依存関係

```
Commands → Services → Platform → OS APIs
```

- **単方向依存**: 上位層は下位層のみに依存
- **レイヤー分離**: 各層の責任を明確に分離
- **インターフェース統一**: Platform 層は統一されたインターフェースを提供

## 段階的移行計画

### Phase 1: Platform 基盤構築 🏗️

**目的**: Platform 層の基礎となるインターフェースと Factory Pattern を構築

**作業内容**:

- [ ] `src/platform/` ディレクトリ作成
- [ ] `platform/traits.rs` 作成 - 共通インターフェース定義
- [ ] `platform/factory.rs` 作成 - Factory Pattern 実装
- [ ] `platform/mod.rs` 作成と lib.rs でのモジュール宣言
- [ ] 基本的なトレイトとファクトリーのテスト作成

**成果物**:

```rust
// platform/traits.rs
pub trait MemoryService: Send + Sync {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String>;
}

pub trait GpuService: Send + Sync {
    async fn get_gpu_usage(&self) -> Result<f32, String>;
}

pub trait NetworkService: Send + Sync {
    async fn get_network_info(&self) -> Result<Vec<NetworkInfo>, String>;
}

pub trait PlatformServices: Send + Sync {
    fn memory_service(&self) -> Box<dyn MemoryService>;
    fn gpu_service(&self) -> Box<dyn GpuService>;
    fn network_service(&self) -> Box<dyn NetworkService>;
}
```

**影響範囲**: 新規ファイルのみ、既存コードに影響なし  
**リスク**: 低  
**PR**: Architecture-Phase1-Platform-Foundation

---

### Phase 2-1: Windows Platform 移行 🪟

**目的**: Windows 固有の実装を platform 層に移動

**作業内容**:

- [ ] `services/wmi_service.rs` → `platform/windows/wmi_service.rs`
- [ ] `services/directx_gpu_service.rs` → `platform/windows/directx_gpu_service.rs`
- [ ] `platform/windows/mod.rs` 作成
- [ ] Windows Platform 実装クラス作成
- [ ] import パス修正
- [ ] Windows 固有テストの移動と更新

**成果物**:

```rust
// platform/windows/mod.rs
pub struct WindowsPlatform;

impl PlatformServices for WindowsPlatform {
    fn memory_service(&self) -> Box<dyn MemoryService> {
        Box::new(WindowsMemoryService)
    }
    // ...
}
```

**影響範囲**: Windows platform 実装のみ  
**リスク**: 中  
**PR**: Architecture-Phase2-Windows-Migration

---

### Phase 2-2: Linux Platform 移行 🐧

**目的**: Linux 固有の実装を platform 層に移動

**作業内容**:

- [ ] Linux 関連サービスファイルを `platform/linux/` に移動
  - `services/gpu_linux.rs` → `platform/linux/gpu_linux.rs`
  - `services/amd_gpu_linux.rs` → `platform/linux/amd_gpu_linux.rs`
  - `services/intel_gpu_linux.rs` → `platform/linux/intel_gpu_linux.rs`
  - `services/ip_linux.rs` → `platform/linux/ip_linux.rs`
- [ ] `platform/linux/mod.rs` 作成
- [ ] Linux Platform 実装クラス作成
- [ ] import パス修正
- [ ] Linux 固有テストの移動と更新

**影響範囲**: Linux platform 実装のみ  
**リスク**: 中  
**PR**: Architecture-Phase2-Linux-Migration

---

### Phase 2-3: 共通ユーティリティ移行 📦

**目的**: プラットフォーム間で共有されるコンポーネントを common 層に移動

**作業内容**:

- [ ] `services/nvidia_gpu_service.rs` → `platform/common/nvidia_gpu_service.rs`
- [ ] `services/memory/` → `platform/common/memory/`
- [ ] `platform/common/mod.rs` 作成
- [ ] 相互参照の修正
- [ ] 共通ユーティリティのテスト更新

**影響範囲**: 共通ユーティリティのみ  
**リスク**: 中  
**PR**: Architecture-Phase2-Common-Migration

---

### Phase 2-4: macOS Platform 準備 🍎

**目的**: macOS 対応の基盤を準備（スタブ実装）

**作業内容**:

- [ ] `platform/macos/` ディレクトリ作成
- [ ] `platform/macos/mod.rs` 作成
- [ ] macOS 用スタブ実装作成
- [ ] Factory Pattern で macOS 選択ロジック追加
- [ ] macOS 用テスト作成（エラー処理確認）

**成果物**:

```rust
// platform/macos/mod.rs
pub struct MacOSPlatform;

impl PlatformServices for MacOSPlatform {
    fn memory_service(&self) -> Box<dyn MemoryService> {
        Box::new(MacOSMemoryService) // スタブ実装
    }
    // ...
}
```

**影響範囲**: 新規 macOS 実装のみ  
**リスク**: 低  
**PR**: Architecture-Phase2-MacOS-Preparation

---

### Phase 3: Services 層統合 🔄

**目的**: Services 層に Platform 抽象化レイヤーを作成

**作業内容**:

- [ ] `services/hardware_service.rs` 作成
- [ ] Platform 層への統一アクセスポイント実装
- [ ] NVIDIA 関連関数の統合
- [ ] NameValue 構造体の移行
- [ ] Services 層のクリーンアップ
- [ ] `services/mod.rs` 更新

**成果物**:

```rust
// services/hardware_service.rs
pub struct HardwareService;

impl HardwareService {
    pub async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
        let platform = PlatformFactory::create();
        platform.memory_service().get_memory_info().await
    }

    pub async fn get_gpu_usage(&self) -> Result<f32, String> {
        let platform = PlatformFactory::create();
        platform.gpu_service().get_gpu_usage().await
    }
    // ...
}
```

**影響範囲**: Services 層の統合  
**リスク**: 中  
**PR**: Architecture-Phase3-Services-Integration

---

### Phase 4: Commands 層リファクタリング 🎛️

**目的**: Commands 層から Platform 直接参照を削除し、Services 経由に変更

**作業内容**:

- [ ] `commands/hardware.rs` の PlatformFactory 直接呼び出し削除
- [ ] HardwareService 経由でのアクセスに変更
- [ ] nvidia_gpu_service 参照を services 経由に変更
- [ ] 戻り値型の統一（NameValue 等）
- [ ] エラーハンドリングの統一

**変更例**:

```rust
// Before
let platform = PlatformFactory::create();
let gpu_service = platform.gpu_service();
match gpu_service.get_gpu_usage().await {

// After
let hardware_service = HardwareService::new();
match hardware_service.get_gpu_usage().await {
```

**影響範囲**: UI ↔ Backend 接続部分  
**リスク**: 高（UI 機能に直結）  
**PR**: Architecture-Phase4-Commands-Refactoring

---

### Phase 5-1: テスト移行と更新 🧪

**目的**: テストファイルの適切な配置と参照更新

**作業内容**:

- [ ] `_tests/services/platform_*.rs` → `_tests/platform/` に移動
- [ ] `_tests/platform/mod.rs` 作成
- [ ] `_tests/mod.rs` 更新
- [ ] テスト内の import パス修正
- [ ] プラットフォーム固有テストの整理

**影響範囲**: テストファイルのみ  
**リスク**: 低  
**PR**: Architecture-Phase5-Test-Migration

---

### Phase 5-2: 残件参照修正 🧹

**目的**: その他ファイルでの参照修正

**作業内容**:

- [ ] `backgrounds/system_monitor.rs` の参照修正
- [ ] その他の残存参照の特定と修正
- [ ] 未使用 import の削除
- [ ] dead code の削除

**影響範囲**: マイナーな参照修正  
**リスク**: 低  
**PR**: Architecture-Phase5-Reference-Cleanup

---

### Phase 5-3: 依存関係最適化 📦

**目的**: 依存関係の最適化と軽量化

**作業内容**:

- [ ] image クレートの機能制限（PNG のみ）
- [ ] 未使用依存関係の削除確認
- [ ] Cargo.toml の最適化

**変更例**:

```toml
# Before
image = "0.25.6"

# After
image = { version = "0.25.6", default-features = false, features = ["png"] }
```

**影響範囲**: 依存関係のみ  
**リスク**: 低  
**PR**: Architecture-Phase5-Dependencies-Optimization

---

### Phase 6: 最終検証と文書化 📚

**目的**: 移行完了の確認と文書化

**作業内容**:

- [ ] 全テストの実行と確認（83 テスト）
- [ ] lint/format の実行
- [ ] `CLAUDE.md` のアーキテクチャ説明更新
- [ ] 移行ガイドの作成
- [ ] パフォーマンス影響の確認

**成果物**:

- 更新されたアーキテクチャドキュメント
- 移行完了レポート
- macOS 対応準備完了の確認

**影響範囲**: ドキュメントのみ  
**リスク**: 最低  
**PR**: Architecture-Phase6-Documentation-Finalization

---

## 各 Phase の前提条件と成功基準

### 前提条件

- 各 Phase は前の Phase の完了が前提
- 各 PR 作成前に該当箇所のテスト実行
- コンパイルエラーがないことを確認

### 成功基準

- **Phase 1**: Platform 基盤のテストが通過
- **Phase 2**: 各プラットフォーム実装のテストが通過
- **Phase 3**: Services 統合のテストが通過
- **Phase 4**: Commands 層のテストが通過、UI 機能が正常動作
- **Phase 5**: 全 83 テストが通過、lint/format エラーなし
- **Phase 6**: 文書化完了、macOS 対応準備完了

## リスク管理

### 高リスク: Phase 4 (Commands 層)

- **対策**: 段階的実装、機能ごとのテスト
- **ロールバック**: Services 経由と Platform 直接を並行実装

### 中リスク: Phase 2, 3 (Platform 移行、Services 統合)

- **対策**: プラットフォームごとの分離実装
- **ロールバック**: 旧参照を一時的に維持

### 低リスク: Phase 1, 5, 6

- **対策**: 通常のレビュープロセス

## 期待される効果

1. **macOS 対応の簡素化**: 新しいプラットフォーム実装のみで対応可能
2. **保守性向上**: 責任の明確な分離
3. **テスタビリティ向上**: モック実装の容易性
4. **拡張性確保**: 新機能追加時の影響範囲限定

## 完了後のアーキテクチャ

```
Commands → Services → Platform → OS APIs
    ↓         ↓         ↓
  UI層    ビジネス    OS抽象化
        ロジック層      層
```

これにより、macOS 対応時は `platform/macos/` の実装のみで完結する設計が実現されます。
