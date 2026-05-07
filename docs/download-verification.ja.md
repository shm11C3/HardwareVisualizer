# ダウンロードの検証

[English](download-verification.md) | [日本語](download-verification.ja.md)

公式配布チャネルからダウンロードした HardwareVisualizer のリリースファイルを検証するためのガイドです。

## 公式配布元

公式のダウンロードおよびインストールは、以下のチャネルからのみ提供されます。

- GitHub Releases: <https://github.com/shm11C3/HardwareVisualizer/releases>
- 公式ウェブサイト: <https://hardviz.com/>
- Windows の Winget（利用可能な場合）

サードパーティのミラーサイト、ダウンロードサイト、ファイル共有リンク、YouTube の説明欄のリンク、
短縮 URL、パスワード付きアーカイブは公式配布チャネルではありません。
また、公式ダウンロードページを装った偽サイトから悪意のあるインストーラーを配布する攻撃も確認されています。
ダウンロード前にドメインを慎重に確認し、利用可能な場合は以下の方法で GitHub Release の assets を検証してください。

## SHA-256 チェックサム

v1.8.1 以降の GitHub Release では、リリース assets の正規チェックサム一覧として
`SHA256SUMS.txt` を Assets セクションに含める予定です。

インストーラーと同じ GitHub Release から `SHA256SUMS.txt` をダウンロードし、
対象ファイル名の SHA-256 値と照合してください。

Windows:

```powershell
Get-FileHash .\HardwareVisualizer_x.x.x_x64_en-US.msi -Algorithm SHA256
```

macOS:

```bash
shasum -a 256 HardwareVisualizer_x.x.x_aarch64.dmg
```

Linux:

```bash
sha256sum hardware-visualizer_x.x.x_amd64.deb
```

v1.8.1 より前のリリースでは、`SHA256SUMS.txt` が提供されていない場合があります。

## GitHub Artifact Attestations

v1.8.1 以降のリリース assets で GitHub Artifact Attestations を生成する予定です。

これは高度な検証手順です。多くのユーザーはまず、ファイルの SHA-256 が
`SHA256SUMS.txt` に記載された値と一致することを確認してください。

この確認には GitHub CLI と GitHub へのネットワークアクセスが必要です。`-R` フラグは、
このリポジトリに関連付けられた attestation に検証範囲を限定します。このコマンドは、
ローカルファイルに対してデフォルトの SLSA provenance predicate を検証します。

```bash
gh attestation verify ./HardwareVisualizer_x.x.x_x64_en-US.msi -R shm11C3/HardwareVisualizer
```

v1.8.1 より前のリリースでは、GitHub Artifact Attestations が提供されていない場合があります。

## macOS の署名と notarization

macOS 向けのダウンロードは Apple Developer ID で署名され、Apple により notarization
済みです。

ダウンロードしたディスクイメージの署名を検証するには、次を実行します。

```bash
codesign --verify --verbose=2 HardwareVisualizer_x.x.x_aarch64.dmg
```

ディスクイメージに対する Gatekeeper の判定と notarization 状態を検証するには、次を実行します。

```bash
spctl -a -vv --type open HardwareVisualizer_x.x.x_aarch64.dmg
```

すでにアプリを `/Applications` にコピーしている場合は、インストール済み app bundle の署名も検証できます。

```bash
codesign --verify --deep --strict --verbose=2 /Applications/HardwareVisualizer.app
```

`spctl` の出力に `accepted` が含まれ、詳細出力に Developer ID の情報が表示されれば成功です。

## Winget

Winget は、パッケージが利用可能な場合の Windows 公式インストール経路です。

```powershell
winget install shm11C3.HardwareVisualizer
winget show shm11C3.HardwareVisualizer
```

Winget はインストールチャネルです。Authenticode 署名、SHA-256 チェックサム、
GitHub Artifact Attestations の代替ではありません。

v1.8.1 以降の Winget manifest を確認する場合は、`SHA256SUMS.txt` にある
Windows インストーラーの SHA-256 値を `InstallerSha256` の入力または検証値として使用してください。
