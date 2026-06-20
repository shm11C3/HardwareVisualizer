# Windows コード署名 (SSL.com eSigner) セットアップ手順

このドキュメントでは、GitHub Actions で Windows 向けビルドに SSL.com eSigner の
クラウド署名 (CodeSignTool) を適用するための手順を説明します。

署名は Tauri の `bundle.windows.signCommand` に CodeSignTool を据える方式で行います。
これにより NSIS (`.exe`) / MSI、およびインストーラ内部にネストされたバイナリが、
単一の経路で署名されます。

## 前提条件

- SSL.com の **Individual Validation (IV)** コード署名証明書を保有していること。
- その証明書が **eSigner クラウド署名 (Cloud Signing) に enroll 済み**であること。
  - USB トークンに発行済みの証明書は後からクラウドへ移行できません。購入・発行の
    段階で eSigner Cloud Signing オプションを選択しておく必要があります。
- 2023-06-01 以降の CA/Browser Forum 要件によりコード署名鍵はハードウェア保管が
  必須ですが、eSigner のクラウド HSM がこれを満たすため、USB トークンを CI に
  配布せずヘッドレスで署名できます。

## 1. eSigner 認証情報の取得

ヘッドレス署名には次の 4 つが必要です。

- **ユーザー名 / パスワード**: SSL.com アカウントの認証情報。
- **credential_id**: 証明書が複数ある場合に必須（単一の場合は省略可）。
- **TOTP secret**: 非対話で OTP を生成するためのシークレット。

### TOTP secret の取得

1. eSigner のポータルにログインする。
2. 対象証明書の QR コードを表示する（登録 PIN の入力が必要）。
3. QR コードの横に表示される **secret code** の値を控える。これが TOTP secret です。

> 2FA アプリ (Authy 等) が QR コードから OTP を生成するのと同じ値です。CodeSignTool
> はこの値を使って署名時に OTP を自動生成します。

## 2. GitHub Secrets の設定

GitHub リポジトリの **Settings > Secrets and variables > Actions** で以下のシークレットを追加します。

| シークレット名   | 値                                                       |
| ---------------- | -------------------------------------------------------- |
| `ES_USERNAME`    | SSL.com / eSigner のユーザー名                           |
| `ES_PASSWORD`    | SSL.com / eSigner のパスワード                           |
| `ES_TOTP_SECRET` | 手順 1 で取得した TOTP secret                            |
| `CREDENTIAL_ID`  | eSigner の credential ID（証明書が複数ある場合に必須）   |

> パスワードに cmd.exe の特殊文字 (`& | < > ^ %`) を含めると署名ラッパー内の引数
> 受け渡しで問題になり得るため、避けることを推奨します。

## 3. 署名の有効化

`.github/workflows/publish.yml` の `ENABLE_WINDOWS_SIGNING` を `true` にします
（既定では安全のため `false`）。

```yaml
env:
  ENABLE_WINDOWS_SIGNING: true
```

## 4. 仕組み

- `Setup CodeSignTool (SSL.com eSigner)` ステップが CodeSignTool の Windows 版
  (`v1.3.2`、JRE 同梱のため別途 Java 不要) を GitHub Releases から固定バージョンで
  ダウンロード・展開し、`CODESIGNTOOL_DIR` を後続ステップへ公開します。
- `update-tauri-config.ts --sign` が `tauri.conf.json` の
  `bundle.windows.signCommand` に署名ラッパー
  (`pwsh ... .github/scripts/sign-codesigntool.ps1 %1`) を注入します。
- Tauri はビルド成果物ごとにこのラッパーを呼び出します。CodeSignTool は in-place
  署名ではなく出力ディレクトリへ署名するため、ラッパーは一時ディレクトリへ署名した
  うえで元のパスへ署名済みファイルを戻します。
- この経路により、`tauri-action` のビルドで生成される NSIS (`.exe`) / MSI が
  署名されます。
- CodeSignTool はクラウド署名時に RFC 3161 タイムスタンプを自動付与します
  （`tauri.conf.json` の `timestampUrl` は空のままで問題ありません）。

## 5. 動作確認

alpha または beta タグをプッシュして、`publish-tauri` の Windows ジョブが正常に
完了することを確認します。

```bash
git tag v1.0.0-alpha.1
git push origin v1.0.0-alpha.1
```

GitHub Actions のログで以下を確認します。

- **Setup CodeSignTool**: ダウンロードに成功し、`CODESIGNTOOL_DIR=...` が出力されている。
- **署名**: 成果物ごとに `Signing with CodeSignTool: ...` と `Signed: ...` が出力されている。

ダウンロードした成果物の署名は Windows 上で確認できます。

```powershell
Get-AuthenticodeSignature .\HardwareVisualizer_x.x.x_x64_en-US.msi | Format-List
signtool verify /pa /v .\HardwareVisualizer_x.x.x_x64-setup.exe
```

## トラブルシューティング

### `invalid/expired OTP` 系のエラー

- `ES_TOTP_SECRET` が QR コードの secret code と一致しているか確認する。
- runner の時刻ずれがないか確認する（TOTP は時刻ベース）。

### `ES_USERNAME is not set` / `ES_PASSWORD is not set`

- シークレットが未設定、または `ENABLE_WINDOWS_SIGNING` が `false` のままになっていないか確認する。

### `CodeSignTool failed (exit code ...)`

- 認証情報・`credential_id`・証明書の eSigner enroll 状態を確認する。
- `sign` コマンドは既定でマルウェアスキャンを行いません（スキャンは別コマンド
  `scan_code`）。スキャン起因のブロックではないかも合わせて確認する。

## ポリシー更新

署名を有効化して署名付きリリースを公開したら、ルートの
[`CODE_SIGNING_POLICY.md`](../../CODE_SIGNING_POLICY.md) の Windows の状態を
「Pending」から実状（Signed / Authenticode）へ更新してください。

## 参考 URL

- eSigner (Cloud Signing): <https://www.ssl.com/products/software-integrity/signing-service/>
- CodeSignTool コマンドガイド: <https://www.ssl.com/guide/esigner-codesigntool-command-guide/>
- GitHub Actions 統合ガイド: <https://www.ssl.com/how-to/cloud-code-signing-integration-with-github-actions/>
- Tauri Windows 署名: <https://v2.tauri.app/distribute/sign/windows/>
