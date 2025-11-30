# Copilot Instructions for HardwareVisualizer

## Overview

HardwareVisualizer は、リアルタイムでコンピュータのハードウェアパフォーマンスを監視するツールです。このプロジェクトは、フロントエンド（TypeScript/React）とバックエンド（Rust/Tauri）で構成されています。以下のガイドラインは、AI コーディングエージェントがこのコードベースで効率的に作業するための指針を提供します。

## プロジェクト構造

- **`src/`**: フロントエンドの主要なコードが含まれています。
  - `components/`: 再利用可能な UI コンポーネント。
  - `features/`: 機能ごとのモジュール（例: hardware, menu, settings）。
  - `hooks/`: React カスタムフック。
  - `lib/`: ユーティリティ関数。
  - `store/`: 状態管理関連のコード。
  - `types/`: TypeScript 型定義。
- **`src-tauri/`**: バックエンドの Rust コード。
  - `src/`: Rust の主要なコード。
  - `commands/`: フロントエンドと通信するための Tauri コマンド。
  - `database/`: データベース関連のロジック。
  - `services/`: サービス層のロジック。
  - `utils/`: ユーティリティ関数。
- **`test/`**: 単体テスト。
- **`coverage/`**: テストカバレッジレポート。

## 開発フロー

### 必要なツール

- Node.js v22
- Rust 1.85

### コマンド

- **依存関係のインストール**:
  ```bash
  npm ci
  ```
- **開発モードでの起動**:
  ```bash
  npm run tauri dev
  ```
- **本番ビルド**:
  ```bash
  npm run tauri build
  ```
- **コードのリント**:
  ```bash
  npm run lint
  cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
  ```
- **コードのフォーマット**:
  ```bash
  npm run format
  cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
  ```
- **テストの実行**:

  ```bash
  npm test
  cargo test --manifest-path src-tauri/Cargo.toml -- --test-threads=1 --nocapture
  ```

## プロジェクト固有のパターン

- **Tauri コマンド**: フロントエンドとバックエンド間の通信は、`src-tauri/src/commands/` 内のコマンドを使用して行われます。フロントエンドからは、自動生成された `rspc/bindings.ts` を通じて Tauri コマンドを呼び出します。
- **ユーティリティ関数**: 再利用可能なロジックは `lib/` または `src-tauri/src/utils/` に配置されています。

## 参考ファイル

- `README.md`: プロジェクトの全体像とセットアップ手順。
- `vite.config.ts`: フロントエンドのビルド設定。
- `tauri.conf.json`: Tauri アプリケーションの設定。

このガイドラインに基づいて作業を進めてください。不明点があれば、README.md やコードベースを参照してください。
