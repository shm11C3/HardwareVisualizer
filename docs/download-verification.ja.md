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
`SHA256SUMS.txt` を Assets セクションに含めています。

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

v1.8.1 以降のリリース assets では GitHub Artifact Attestations を生成しています。

これは高度な検証手順です。多くのユーザーはまず、ファイルの SHA-256 が
`SHA256SUMS.txt` に記載された値と一致することを確認してください。

この確認には GitHub CLI と GitHub へのネットワークアクセスが必要です。`-R` フラグは、
このリポジトリに関連付けられた attestation に検証範囲を限定します。このコマンドは、
ローカルファイルに対してデフォルトの SLSA provenance predicate を検証します。

```bash
gh attestation verify ./HardwareVisualizer_x.x.x_x64_en-US.msi -R shm11C3/HardwareVisualizer
```

v1.8.1 より前のリリースでは、GitHub Artifact Attestations が提供されていない場合があります。

## Windows Authenticode 署名

Windows の `.exe` / `.msi` リリースインストーラーは、v1.9.0 以降 Authenticode
署名済みです。v1.9.0 より前の Windows リリースインストーラーは未署名の場合があります。

インストーラーの署名は PowerShell で検証できます。

```powershell
Get-AuthenticodeSignature .\HardwareVisualizer_x.x.x_x64_en-US.msi | Format-List
```

NSIS 形式のセットアップ実行ファイルを確認する場合:

```powershell
Get-AuthenticodeSignature .\HardwareVisualizer_x.x.x_x64-setup.exe | Format-List
```

成功している場合、出力に `Status: Valid` が表示されます。署名者証明書やタイムスタンプの詳細も
同じ出力で確認できます。

Windows SDK tools が入っている場合は、`signtool` でも同じポリシー検証を実行できます。

```powershell
signtool verify /pa /v .\HardwareVisualizer_x.x.x_x64-setup.exe
```

有効な署名が付いていても、発行元またはファイルの reputation が十分に確立するまでは
Windows SmartScreen の警告が表示される場合があります。

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

## Linux package signing

AppImage、`.deb`、`.rpm` などの Linux パッケージは、現時点では GPG、Sigstore/cosign、
リポジトリメタデータ署名などの Linux package signing では署名されていません。
Linux 向けダウンロードは、利用可能な場合 `SHA256SUMS.txt` と GitHub Artifact Attestations
で検証してください。

## Tauri updater の `.sig` assets

Release assets に含まれる `.sig` ファイルは、アプリ内アップデート経路で使用する
Tauri updater 署名です。Windows Authenticode 署名、macOS notarization、Linux package signing、
SHA-256 チェックサム、GitHub Artifact Attestations の代替ではありません。

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
