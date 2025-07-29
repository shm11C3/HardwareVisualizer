# プラットフォーム抽象化アーキテクチャ

## 概要

このプロジェクトでは、Strategy パターン、Adapter パターン、Factory パターンを組み合わせて OS 固有の処理を抽象化し、新しい OS への対応を簡単にできるアーキテクチャを実装しています。

## アーキテクチャ図

```
┌─────────────────────┐
│   Hardware Commands │  ← フロントエンド API
└─────────┬───────────┘
          │
          v
┌─────────────────────┐
│  PlatformFactory    │  ← Factory Pattern
└─────────┬───────────┘
          │
          v
┌─────────────────────┐
│  PlatformServices   │  ← Strategy Pattern Interface
└─────────┬───────────┘
          │
          v
┌─────────┬───────────┬─────────┐
│ Windows │   Linux   │  macOS  │  ← Adapter Pattern
│Platform │ Platform  │Platform │
└─────────┴───────────┴─────────┘
```

## ディレクトリ構造

```
src/services/platform/
├── mod.rs                     # モジュール定義
├── traits.rs                  # 共通インターフェース定義
├── factory.rs                 # Factory パターン実装
├── windows/                   # Windows 固有実装
│   ├── mod.rs
│   ├── memory_service.rs
│   ├── gpu_service.rs
│   └── network_service.rs
├── linux/                     # Linux 固有実装
│   ├── mod.rs
│   ├── memory_service.rs
│   ├── gpu_service.rs
│   └── network_service.rs
└── macos/                     # macOS 固有実装
    ├── mod.rs
    ├── memory_service.rs
    ├── gpu_service.rs
    └── network_service.rs
```

## 主要コンポーネント

### 1. traits.rs - 共通インターフェース

- `MemoryService`: メモリ情報取得の抽象化
- `GpuService`: GPU 情報取得の抽象化
- `NetworkService`: ネットワーク情報取得の抽象化
- `PlatformServices`: 各サービスを統合するトレイト

### 2. factory.rs - ファクトリーパターン

- `PlatformFactory::create()`: 実行時 OS に基づいて適切なプラットフォーム実装を返す
- コンパイル時の条件分岐により、対象 OS のコードのみがビルドされる

### 3. OS 固有実装

各 OS ディレクトリには以下が含まれます：

- **mod.rs**: プラットフォーム統合
- **memory_service.rs**: OS 固有のメモリ情報取得
- **gpu_service.rs**: OS 固有の GPU 情報取得  
- **network_service.rs**: OS 固有のネットワーク情報取得

## 新しい OS への対応手順

### 1. 新しいプラットフォームディレクトリを作成

```bash
mkdir src/services/platform/new_os/
```

### 2. 必要なファイルを作成

```rust
// src/services/platform/new_os/mod.rs
pub mod memory_service;
pub mod gpu_service;
pub mod network_service;

use super::traits::{GpuService, MemoryService, NetworkService, PlatformServices};

pub struct NewOSPlatform;

impl NewOSPlatform {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformServices for NewOSPlatform {
    fn memory_service(&self) -> Box<dyn MemoryService> {
        Box::new(memory_service::NewOSMemoryService)
    }

    fn gpu_service(&self) -> Box<dyn GpuService> {
        Box::new(gpu_service::NewOSGpuService)
    }

    fn network_service(&self) -> Box<dyn NetworkService> {
        Box::new(network_service::NewOSNetworkService)
    }
}
```

### 3. 各サービスを実装

```rust
// src/services/platform/new_os/memory_service.rs
use crate::services::platform::traits::MemoryService;
use crate::structs::hardware::MemoryInfo;
use async_trait::async_trait;

pub struct NewOSMemoryService;

#[async_trait]
impl MemoryService for NewOSMemoryService {
    async fn get_memory_info(&self) -> Result<MemoryInfo, String> {
        // 新しい OS 固有の実装
        todo!("Implement for new OS")
    }

    async fn get_detailed_memory_info(&self) -> Result<Vec<MemoryInfo>, String> {
        // 新しい OS 固有の実装
        todo!("Implement for new OS")
    }
}
```

### 4. ファクトリーを更新

```rust
// src/services/platform/factory.rs に追加
#[cfg(target_os = "new_os")]
use super::new_os::NewOSPlatform;

impl PlatformFactory {
    pub fn create() -> Box<dyn PlatformServices> {
        // 既存の OS 判定に追加
        #[cfg(target_os = "new_os")]
        return Box::new(NewOSPlatform::new());
        
        // ... 既存コード
    }
}
```

### 5. モジュール宣言を追加

```rust
// src/services/platform/mod.rs に追加
pub mod new_os;
```

## テスト戦略

- **単体テスト**: 各 OS 固有サービスのテスト
- **統合テスト**: プラットフォームファクトリーのテスト
- **条件付きテスト**: OS 固有のテストは `#[cfg(target_os = "...")]` で分離

## 利点

1. **拡張性**: 新しい OS への対応が容易
2. **保守性**: OS 固有コードが明確に分離
3. **テスト性**: モック可能な抽象化
4. **型安全性**: コンパイル時の型チェック
5. **パフォーマンス**: 実行時オーバーヘッドなし

## macOS 対応の準備

現在の macOS 実装はスタブですが、以下の手順で実装できます：

1. **Core Foundation / IOKit の利用**: システム情報取得
2. **Metal Framework**: GPU 情報取得
3. **System Configuration Framework**: ネットワーク情報取得

macOS 固有の依存関係を `Cargo.toml` に追加：

```toml
[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "0.9"
system-configuration = "0.5"
metal = "0.24"
```

この設計により、将来的な macOS 対応や他の OS 対応が効率的に実現できます。