# HardwareVisualizer

[English](README.md) | [日本語](README.ja.md)

[![Release](https://img.shields.io/github/v/release/shm11C3/HardwareVisualizer?&display_name=release)](https://github.com/shm11C3/HardwareVisualizer/releases)
[![CI develop](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/shm11C3/HardwareVisualizer/actions/workflows/ci.yml)
![Platforms](https://img.shields.io/badge/platform-Windows%20|%20Linux%20|%20macOS-blue)
![Downloads](https://img.shields.io/github/downloads/shm11C3/HardwareVisualizer/total)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Fshm11C3%2FHardwareVisualizer?ref=badge_shield)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/shm11C3/HardwareVisualizer)

![image](https://github.com/user-attachments/assets/c474a132-5768-4046-9703-766e74ee3e66)

HardwareVisualizer は、コンピュータのハードウェアパフォーマンスをリアルタイムで監視するためのツールです。直感的なダッシュボード、詳細な使用率グラフ、カスタマイズ可能な設定を備えており、システムの重要な統計情報を把握するのに役立ちます。

Web サイト: <https://hardviz.com/>

> [!NOTE]
>
> ## 公式ダウンロードとセキュリティに関する注意
>
> HardwareVisualizer は、以下のチャネルを通じて**のみ**公式に配布されています。
>
> - GitHub Releases: https://github.com/shm11C3/HardwareVisualizer/releases
> - 公式ウェブサイト: https://hardviz.com/
>
> その他の配布元（例：サードパーティのミラーサイトや SourceForge などのダウンロードサイトの掲載）は、本プロジェクトとは**一切関係ありません**。
>
> 特に、SourceForge 上の `Hardware Visualizer` (`https://sourceforge.net/projects/hardware-visualizer/`) というプロジェクトは、開発者の関与なしに作成されたものです。そこで公開されている ZIP アーカイブの真正性や安全性については確認が取れていません。利用される場合は自己責任でお願いいたします。

## 目次

- [HardwareVisualizer](#hardwarevisualizer)
  - [目次](#目次)
  - [インストールガイド](#インストールガイド)
    - [ダウンロード](#ダウンロード)
    - [Windows へのインストール](#windows-へのインストール)
      - [インストーラを使用する](#インストーラを使用する)
      - [Winget コマンドを使用する](#winget-コマンドを使用する)
    - [Linux へのインストール](#linux-へのインストール)
    - [初期設定](#初期設定)
  - [機能一覧](#機能一覧)
  - [サポート OS](#サポート-os)
  - [スクリーンショット](#スクリーンショット)
    - [ダッシュボード](#ダッシュボード)
    - [使用率グラフ](#使用率グラフ)
    - [インサイト](#インサイト)
    - [カスタムグラフ](#カスタムグラフ)
    - [背景画像](#背景画像)
  - [権限とセキュリティについて](#権限とセキュリティについて)
  - [ロードマップ](#ロードマップ)
  - [コントリビューション](#コントリビューション)
  - [コード署名ポリシー（英語版のみ）](#コード署名ポリシー英語版のみ)
  - [ライセンス](#ライセンス)

## インストールガイド

### ダウンロード

お使いのプラットフォームに合わせて、最新のインストーラーをダウンロードしてください。

- **公式ウェブサイト**: [hardviz.com/#download](https://hardviz.com/#download)
- **GitHub Releases**: [最新リリース](https://github.com/shm11C3/HardwareVisualizer/releases/latest) > Assets セクション

チェックサムと provenance の確認方法は、
[ダウンロード検証ガイド](docs/download-verification.ja.md) を参照してください。

### Windows へのインストール

#### インストーラを使用する

1. ダウンロードページから `HardwareVisualizer_x.x.x_x64-setup_windows.exe` または `HardwareVisualizer_x.x.x_x64_en-US_windows.msi` をダウンロードします。
2. インストーラー（`.exe` または `.msi` ファイル）を実行します。
3. インストールウィザードの指示に従います。
4. スタートメニューまたはデスクトップのショートカットから **HardwareVisualizer** を起動します。

#### Winget コマンドを使用する

Windows の場合、Windows Package Manager（Winget）を使用してインストールすることもできます。　　
PowerShell またはコマンドプロンプトで以下のコマンドを実行してください。

```powershell
winget install shm11C3.HardwareVisualizer
```

> [!NOTE]
> Windows では追加の権限は必要ありません。

### Linux へのインストール

1. ダウンロードページから `hardware-visualizer_x.x.x_amd64.deb` をダウンロードします。
2. パッケージマネージャー経由でインストールします。

   ```bash
   sudo dpkg -i hardware-visualizer_*.deb
   sudo apt-get install -f  # 必要に応じて依存関係をインストール
   ```

3. アプリケーションメニューまたはターミナルから起動します。

   ```bash
   hardware-visualizer
   ```

> [!TIP]
>
> ### ハードウェアデータが表示されない場合
>
> 一部のメトリクスには管理者権限が必要です。すべてのハードウェア情報にアクセスするには、sudo で再起動してください。
>
> ```bash
> sudo hardware-visualizer
> ```

### 初期設定

アプリ起動後の手順：

1. **設定**（サイドバーの ⚙️ アイコン）へ移動します。
2. お好みの**テーマ**と**言語**を選択します。
3. （任意）カスタムの**背景画像**を設定します。

## 機能一覧

| カテゴリ                     | ステータス | 備考                                      |
| ---------------------------- | ---------- | ----------------------------------------- |
| CPU / RAM 使用率             | ✅         | リアルタイム + 履歴                       |
| GPU 使用率                   | ✅         | NVIDIA は完全対応 / その他は一部対応      |
| GPU 温度                     | ✅         | NVIDIA は完全対応 / その他は一部対応      |
| ファン監視                   | ⏳         | 計画中                                    |
| ストレージ監視               | ✅         | デバイスの概要                            |
| ネットワーク監視             | ✅         | 基本的なインターフェース / 使用量は計画中 |
| カスタムグラフテーマ         | ✅         | 設定保存可能                              |
| ダッシュボードのカスタマイズ | ✅         | レイアウト編集は一部対応                  |
| 背景画像                     | ✅         | ローカル画像を使用可能                    |
| 履歴インサイト               | ✅         | デフォルトで最大 30 日間                  |
| GPU インサイト               | ✅         | NVIDIA は完全対応 / その他は一部対応      |
| 言語サポート                 | ✅         | 英語、日本語、ロシア語                    |

## サポート OS

| OS      | ステータス  | ダウンロード                                  |
| ------- | ----------- | --------------------------------------------- |
| Windows | ✅ 対応済み | [ダウンロード](https://hardviz.com/#download) |
| Linux   | ✅ 対応済み | [ダウンロード](https://hardviz.com/#download) |
| macOS   | ✅ 対応済み | [ダウンロード](https://hardviz.com/#download) |

## スクリーンショット

### ダッシュボード

ハードウェアの現在の状態を一目で確認できます。

![image](https://github.com/user-attachments/assets/a578909a-5b85-4d3a-98cb-a885dc10eaec)

### 使用率グラフ

直近 1 分間のリソース使用状況を確認できます。

![image](https://github.com/user-attachments/assets/ef3e1630-e567-47a1-a437-f9a3981dd587)

![image](https://github.com/user-attachments/assets/7b786e00-12c0-4627-8b2a-cc3482072eb7)

### インサイト

過去最大 30 日間のリソース使用状況を表示します。
使用率は 1 分単位で計算されます。

![image](https://github.com/user-attachments/assets/dd849d54-37a0-4f00-bec8-9c7f994d49fa)

![image](https://github.com/user-attachments/assets/7c3f9ddd-37c1-45b1-9c3a-9f661817e797)

![image](https://github.com/user-attachments/assets/2d3d2045-ccc0-46ee-9a3a-6cde3e13981e)

### カスタムグラフ

柔軟なグラフのカスタマイズが可能です。

![image](https://github.com/user-attachments/assets/b6b2436b-c4c7-4252-9654-c5f2ca89e499)

### 背景画像

![image](https://github.com/user-attachments/assets/6ab09e8a-ebef-449a-b73f-07ae44626e20)

## 権限とセキュリティについて

| 項目               | 理由                                                   |
| ------------------ | ------------------------------------------------------ |
| Linux の sudo 権限 | 特定のデバイスファイル（GPU、センサー）へのアクセス    |
| Windows の WMI     | メモリ・システムの詳細なメトリクス取得                 |
| Windows の PDH     | GPU エンジン使用率                                     |
| 外部送信なし       | テレメトリなし。アプリは外部へデータを一切送信しません |

## ロードマップ

| 項目                             | ステータス |
| -------------------------------- | ---------- |
| macOS への対応                   | ✅ 完了    |
| AMD GPU への対応                 | ✅ 完了    |
| 全ベンダー共通のファン・温度制御 | 調査中     |
| ゲームモード                     | 計画中     |
| 消費電力の推定機能               | 検討中     |
| プラグインシステム               | 検討中     |

## コントリビューション

詳細は [CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

## コード署名ポリシー（英語版のみ）

署名状況の詳細は [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md) をご覧ください。
チェックサムと provenance の確認方法は [ダウンロード検証ガイド](docs/download-verification.ja.md) を参照してください。

## ライセンス

[MIT License](LICENSE)
