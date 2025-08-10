# 依存関係分析レポート

## 📊 メトリクス概要

- **総モジュール数**: 120+ （TypeScript/TSX ファイル）
- **主要依存関係**: 22 本体 + 15 開発依存関係
- **アーキテクチャパターン**: Feature-Based + Atomic Design
- **状態管理**: Jotai (Atomic approach)
- **循環依存**: **2 件検出** ⚠️

## ⚠️ 発見された問題

### 🔴 **高優先度** - 循環依存

#### 1. Settings ⟷ Hardware 循環依存

```
src/features/settings/hooks/useSettingsAtom.ts:3-4
├─ import defaultColorRGB from "@/features/hardware/consts/chart"
└─ import ChartDataType from "@/features/hardware/types/hardwareDataType"

多数のハードウェアコンポーネント
└─ import useSettingsAtom from "@/features/settings/hooks/useSettingsAtom"
```

**影響**: バンドルサイズ増加、コード分割の妨げ

#### 2. Dialog Hook 間接循環依存

```
hooks/useBgImage.ts → features/settings/hooks/useSettingsAtom.ts
features/hardware/hooks/useHardwareInfoAtom.ts → hooks/useTauriDialog.ts
```

### 🟡 **中優先度** - アーキテクチャ違反

#### Feature 間の直接依存

- Settings が Hardware の内部実装に依存
- 共通型定義の散在
- Constants の重複定義

## ✅ 良好な設計パターン

### 1. **lib/utils.ts の実装**

- 純粋なユーティリティ関数
- 外部依存なし（clsx、tailwind-merge のみ）
- 一方向依存のみ

### 2. **Feature-Based アーキテクチャ**

```
features/
├─ hardware/     # ハードウェア監視機能
├─ menu/         # メニュー・ナビゲーション
└─ settings/     # 設定管理
```

### 3. **適切な状態管理**

- Jotai による軽量なアトミック状態管理
- Hook ベースの状態ロジック抽象化

## 🔧 推奨する修正アクション

### **即座の対応**

1. **共通 Constants の分離**

   ```typescript
   // 新規: src/constants/chart.ts
   export const defaultColorRGB = {
     /* ... */
   };

   // src/features/settings/hooks/useSettingsAtom.ts
   import { defaultColorRGB } from "@/constants/chart";
   ```

2. **共通型定義の統合**
   ```typescript
   // 新規: src/types/chart.ts
   export type ChartDataType = {
     /* ... */
   };
   ```

### **中期的改善**

3. **Settings-Hardware 間のインターフェース定義**
   - 設定項目の中央集約
   - Feature 間の通信プロトコル明確化

### **長期的構造改善**

4. **Clean Architecture の採用**
   ```
   src/
   ├─ domain/       # ビジネスロジック
   ├─ application/  # ユースケース
   ├─ infrastructure/ # 外部依存
   └─ presentation/   # UI コンポーネント
   ```

## 📈 依存関係マトリックス

| モジュール           | ファンイン | ファンアウト | 結合度 |
| -------------------- | ---------- | ------------ | ------ |
| lib/utils            | 高 (20+)   | 低 (2)       | 良好   |
| hooks/useTauriDialog | 高 (15+)   | 中 (5)       | 注意   |
| features/settings    | 中 (8)     | 高 (12)      | 改善要 |
| features/hardware    | 低 (3)     | 高 (15)      | 改善要 |

## 🎯 次のステップ

1. **緊急**: Settings-Hardware 循環依存の解消
2. **重要**: 共通型定義の中央集約
3. **推奨**: Feature 間インターフェースの明確化
4. **将来**: Clean Architecture への段階的移行

## 📋 詳細な循環依存チェーン

### チェーン 1: Settings ↔ Hardware

1. `features/settings/hooks/useSettingsAtom.ts` → `features/hardware/consts/chart.ts`
2. `features/hardware/consts/chart.ts` → `features/hardware/types/hardwareDataType.ts`
3. 多数のハードウェアコンポーネント → `features/settings/hooks/useSettingsAtom.ts`

### チェーン 2: Background Image Management

1. `hooks/useBgImage.ts` → `features/settings/hooks/useSettingsAtom.ts`
2. `features/settings/components/SelectBackgroundImage.tsx` → `hooks/useBgImage.ts`
3. `features/settings/Settings.tsx` → `features/settings/components/SelectBackgroundImage.tsx`

### チェーン 3: Dialog Utilities

1. 多数のフィーチャーフック → `hooks/useTauriDialog.ts`
2. `hooks/useTauriDialog.ts` → グローバルに使用される
3. フィーチャーコンポーネント → フィーチャーフック

## 🔍 lib/utils との双方向依存

**検出された双方向依存：なし**

`lib/utils.ts` は純粋なユーティリティ関数として実装されており、以下の特徴があります：

- 外部依存なし（clsx と tailwind-merge のみ）
- プロジェクト内の他のモジュールを import していない
- 一方向的な依存関係のみ（他のファイルから import されるのみ）

## 🎯 影響度評価

### **高リスク循環依存**

- **Settings ⟷ Hardware**: コード分割とバンドルサイズに影響
- **Background Image Management**: 設定管理の複雑化

### **中リスク循環依存**

- **Hardware フィーチャー内**: パフォーマンスに軽微な影響
- **Dialog Utilities**: 一般的なパターンで問題は少ない

### **低リスク**

- **lib/utils**: 循環依存なし、適切に実装済み

## 🛠️ 具体的な修正手順

### ステップ 1: 共通 Constants の分離

```bash
# 新しいディレクトリを作成
mkdir src/constants

# chart 関連の constants を移動
touch src/constants/chart.ts
```

### ステップ 2: Import 文の更新

```typescript
// Before: src/features/settings/hooks/useSettingsAtom.ts
import { defaultColorRGB } from "@/features/hardware/consts/chart";

// After: src/features/settings/hooks/useSettingsAtom.ts
import { defaultColorRGB } from "@/constants/chart";
```

### ステップ 3: 型定義の統合

```bash
# 共通型定義ディレクトリの拡張
touch src/types/chart.ts
touch src/types/hardware.ts
```

## 📝 監視とメンテナンス

### 定期的なチェック項目

- [ ] 新しい循環依存の検出
- [ ] Feature 間の依存関係の監視
- [ ] バンドルサイズの追跡
- [ ] パフォーマンスメトリクスの確認

### 推奨ツール

- **dependency-cruiser**: 循環依存の自動検出
- **madge**: 依存関係の可視化
- **webpack-bundle-analyzer**: バンドルサイズの分析

---

**生成日時**: 2025-07-27  
**分析対象**: hardware-monitor プロジェクト  
**分析ツール**: 手動分析 + grep パターンマッチング
