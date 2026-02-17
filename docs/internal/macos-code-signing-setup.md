# macOS コード署名・公証 セットアップ手順

このドキュメントでは、GitHub Actions で macOS 向けビルドにコード署名と Apple 公証 (Notarization) を適用するための手順を説明します。

## 前提条件

- [Apple Developer Program](https://developer.apple.com/programs/) に加入済み（年額 $99）
- Mac で Keychain Access が使用可能

## 1. 証明書の作成

### 1.1 Certificate Signing Request (CSR) の生成

1. Mac で **Keychain Access** を開く ※`/System/Library/CoreServices/Applications/Keychain Access.app` にある
2. メニューから **キーチェーンアクセス > 証明書アシスタント > 認証局に証明書を要求** を選択
3. 以下を入力:
   - **User Email Address**: Apple Developer Program に登録したメールアドレス
   - **Common Name（固有名）**: 任意の識別名（例: `HardwareVisualizer Distribution`）
   - **CA Email Address**: 空欄のまま
4. **Request is** で **Saved to disk** を選択
5. 詳細オプション:
   - **Key Size（鍵のサイズ）**: **2048 bits**
   - **Algorithm（アルゴリズム）**: **RSA**
6. CSR ファイルを保存

### 1.2 Developer ID Application 証明書の作成

1. [Apple Developer - Certificates](https://developer.apple.com/account/resources/certificates/list) にアクセス
2. 「+」ボタンで新しい証明書を作成
3. **Developer ID Application** を選択（App Store 外での配布用）
4. 先ほど作成した CSR ファイルをアップロード
5. 証明書 (`.cer`) をダウンロードし、ダブルクリックで Keychain に追加

### 1.3 署名 ID の確認

```bash
security find-identity -v -p codesigning
```

出力例:

```txt
1) XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX "Developer ID Application: Your Name (TEAMID)"
```

`"Developer ID Application: Your Name (TEAMID)"` の部分が署名 ID です。

## 2. App Store Connect API Key の作成

1. [App Store Connect - Keys](https://appstoreconnect.apple.com/access/integrations/api) にアクセス
2. ページ上部の **Issuer ID** を控える
3. 「Generate API Key」をクリック
4. 名前を入力し、アクセス権限は **Developer** を選択
5. 作成後、**Key ID** を控える
6. **Download API Key** をクリックして `.p8` ファイルをダウンロード

> `.p8` ファイルは一度しかダウンロードできないため、安全な場所に保管してください。

## 3. 証明書の base64 エンコード

### 3.1 .p12 ファイルのエクスポート

1. **Keychain Access** を開く
2. **デフォルトキーチェーン > 自分の証明書** カテゴリで「Developer ID Application」証明書を見つける
3. 証明書を展開し、秘密鍵を含む状態で右クリック > **Export**
4. `.p12` 形式で保存し、パスワードを設定

### 3.2 base64 への変換

```bash
openssl base64 -A -in certificate.p12 -out certificate-base64.txt
```

`certificate-base64.txt` の内容がシークレットに設定する値です。

## 4. GitHub Secrets の設定

GitHub リポジトリの **Settings > Secrets and variables > Actions** で以下のシークレットを追加します。

| シークレット名               | 値                                                                      |
| ---------------------------- | ----------------------------------------------------------------------- |
| `APPLE_CERTIFICATE`          | `.p12` ファイルの base64 エンコード文字列（手順 3.2）                   |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` エクスポート時に設定したパスワード                               |
| `APPLE_SIGNING_IDENTITY`     | 署名 ID（手順 1.3、例: `Developer ID Application: Your Name (TEAMID)`） |
| `APPLE_API_ISSUER`           | App Store Connect の Issuer ID（手順 2）                                |
| `APPLE_API_KEY`              | API Key ID（手順 2）                                                    |
| `APPLE_API_KEY_CONTENT`      | `.p8` ファイルの内容をそのまま貼り付け                                  |

## 5. 動作確認

alpha または beta タグをプッシュして、`publish-tauri-macos` ジョブが正常に完了することを確認します。

```bash
git tag v1.0.0-alpha.1
git push origin v1.0.0-alpha.1
```

GitHub Actions のログで以下を確認:

- **certificate import**: `APPLE_CERTIFICATE` からの証明書インポートが成功していること
- **codesign**: バイナリへの署名が実行されていること
- **notarytool**: Apple への公証リクエストが送信され、承認されていること

## トラブルシューティング

### 「The certificate is not valid」エラー

- `APPLE_CERTIFICATE` の base64 文字列に改行が含まれていないか確認（`openssl base64 -A` の `-A` フラグが重要）
- 証明書の有効期限が切れていないか確認

### 「Unable to notarize」エラー

- `APPLE_API_KEY_CONTENT` が `.p8` ファイルの完全な内容（`-----BEGIN PRIVATE KEY-----` から `-----END PRIVATE KEY-----` まで）であることを確認
- API Key のアクセス権限が **Developer** 以上であることを確認

### 署名されているかの確認方法

ビルド成果物をダウンロードした後、以下のコマンドで確認できます:

```bash
# 署名の確認
codesign -dv --verbose=4 /path/to/HardwareVisualizer.app

# 公証の確認
spctl -a -v /path/to/HardwareVisualizer.app
```

## 参考URL

- <https://v2.tauri.app/ja/distribute/sign/macos/>
