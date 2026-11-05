# 外部コンポーネント

[English](external-components.md) | [日本語](external-components.ja.md)

このページは External Component Guidance の日本語版の元コンテンツです。現時点では、アプリは GitHub 上で表示されるこのドキュメントを直接開きます。将来的には HardwareVisualizer の website がこのページを元にした内容を公開し、website URL が用意できた時点でアプリ側のリンク先を切り替えます。

暫定的なアプリ内詳細リンク:

- PawnIO:
  `https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.ja.md#pawnio`
- smartctl:
  `https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.ja.md#smartctl`

HardwareVisualizer は、多くのハードウェア情報を任意の外部コンポーネントなしで取得できます。一部のより深いセンサー経路では、別途インストールされるツールやドライバーに依存します。HardwareVisualizer は、それらのコンポーネントを自動でインストール、同梱、有効化することはありません。Windows では、設定画面から求めた場合に限り External Component Setup が PawnIO をインストールできます。[HardwareVisualizer から PawnIO をインストールする](#hardwarevisualizer-から-pawnio-をインストールする) を参照してください。

External Component Guidance は、HardwareVisualizer が任意コンポーネントを使おうとして使えず、さらにフォールバック取得でもユーザーに表示するハードウェア情報が不足する場合にだけ表示されます。フォールバック取得で必要な情報を取得できた場合、アプリはこの案内を表示しません。

## ダイアログの意味

ダイアログでは次の内容を案内します。

- 使えなかった任意コンポーネント
- 取得できず表示できないハードウェア情報
- その情報を表示できるようにするための対応

このダイアログは情報提供のためのものです。ハードウェア監視は、利用できる最善のフォールバックデータで継続します。

## Windowsセンサーサービス

Program Files配下のMSI、または **設定 → 高度な設定 → Windowsセンサーサービス**
から、任意のサービスを明示的に導入できます。PawnIOの導入とは別の選択です。
導入・修復には管理者の承認が必要ですが、通常の起動・トレイ常駐・再接続では
本体を昇格させる必要はありません。サイレントMSIでは
`INSTALL_SENSOR_SERVICE=1` を指定します。PawnIOも導入する場合は
`EXTERNAL_COMPONENT_PAWNIO=1` を併せて指定します。

**新規インストール**でCPU・マザーボードのセンサーを取得するには、PawnIOと
モジュール、およびサービスの両方が必要です。MSIで両方を選ぶか、
**設定 → 高度な設定 → 任意コンポーネントのセットアップ**でPawnIOをセットアップしてから
サービスを追加してください。サービスを選択しなくても通常の監視は継続します。

**本体で直接PawnIOを使用していた版からの更新**では、既存のPawnIOがあっても
サービスは自動追加しません。フォールバック後もセンサー値が取得できない場合、
案内の「センサー設定を開く」から導入欄へ進めます。案内の非表示設定は維持します。

センサーの案内は、実際の取得失敗により現在の画面で必要な値が不足した場合に、
操作を妨げない画面内の案内として表示します。通常のセンサー更新に従い、復旧すると
消えます。導入ダイアログ、起動後の時間制限、案内専用の定期確認はありません。
閉じると今回の起動中はセンサーの案内を再表示しません。「今後表示しない」は、
そのとき案内している取得不足について保存します。設定からの状態確認や導入・修復は
いつでも利用できます。

「管理者として起動」が有効だった場合は、サービス接続後に「次回から一般権限で起動」
を選択し、終了後に通常起動してください。現在のプロセスの権限はその場では下がりません。
他の特権機能では取得情報が減る場合があります。最高権限のタスクや互換性設定を
ユーザーが作成していた場合は、ご自身で変更してください。

管理者として起動する設定が有効で、現在のプロセスも管理者権限の場合は、同じ案内欄から
サービスの導入状況と起動設定の確認へ進めます。これはサービスの準備完了を示す案内では
ありません。一般権限への切替は、設定で準備状況を確認してから行います。
設定画面では案内欄を表示しません。

サービスがなくても、通常の監視と利用可能なACPIフォールバックは継続します。
NSIS版・ポータブル版は導入済みの互換サービスを利用できますが、初回導入には
保護されたMSI版が必要です。本体のアンインストール時はサービスを削除し、共有の
PawnIOは残します。SYSTEMサービスが読み込むモジュールは、アプリが識別できる
署名済みリリースに限ります。未署名のテストモジュールや未認識のファイルは
読み込まず、元のファイルを勝手に置き換えることもしません。

## PawnIO

PawnIO は、対応している CPU で PawnIO 経由の取得経路が利用できる場合に、Windows の CPU パッケージ温度と CPU パッケージ電力を取得するために使われます。

ダッシュボードで CPU 温度を表示できない場合、PawnIO が関係していることがあります。HardwareVisualizer が ACPI thermal zone 経由で CPU 温度を表示できている場合、PawnIO の案内は表示しません。

### PawnIO で表示できる項目

PawnIO は、CPU ごとのセンサー経路から Windows の CPU 情報を取得できます。

- Intel MSR モジュール経由の Intel パッケージ温度
- Intel MSR RAPL レジスター経由の Intel CPU パッケージ電力
- Ryzen SMU モジュール経由の AMD Family 17h / Family 19h パッケージ温度
- AMDFamily17 RAPL モジュール経由の AMD CPU パッケージ電力

### HardwareVisualizer から PawnIO をインストールする

Windows では、**設定 → 高度な設定 → 任意コンポーネントのセットアップ** に PawnIO ランタイムと HardwareVisualizer が使うモジュールファイルの状態が表示され、必要に応じてインストールや更新を選べます。選ぶと HardwareVisualizer は次を行います。

1. 固定バージョンの PawnIO ランタイムインストーラーと PawnIO.Modules リリースを、それぞれの公式 GitHub リリースからダウンロードします。
2. 各ダウンロードをアプリに記録されたサイズと SHA-256 ダイジェストで検証し、一致しなければ破棄します。
3. Windows の管理者承認を求めたうえで、ランタイムが未インストールの場合だけインストーラーを無人モードで実行します。
4. 不足しているモジュールファイルを PawnIO のインストール先へコピーし、SHA-256 が過去の公式リリースと一致する古いファイルを置き換えます。現行のファイルや、より新しいバージョンを含む未認識のファイルは保持します。

Program Files 配下の MSI インストールを更新する場合は、無人更新でもアプリのインストール成功後に古いモジュールファイルを更新します。この処理はランタイムのインストールや不足ファイルの追加を行いません。モジュール更新が失敗してもアプリの更新は成功し、設定から再試行できます。NSIS でインストールした場合は設定から更新してください。

完了後は HardwareVisualizer を再起動してください。Windows の再起動が必要と表示された場合は、先に Windows を再起動します。

HardwareVisualizer をアンインストールしても PawnIO やモジュールファイルは削除されません。不要になった場合は Windows の「アプリと機能」から PawnIO を削除してください。

### ユーザー側で必要な準備

手動で準備する場合、PawnIO 経由で CPU パッケージ温度または電力を取得するには、対象の PC に次のものが必要です。

- 正しく動作する PawnIO driver/runtime
- PawnIO runtime に含まれる `PawnIOLib.dll`
- PawnIO.Modules のリリースアセットに含まれる署名済み CPU 別モジュール
  - Intel パッケージ温度と電力では `IntelMSR.bin`
  - AMD パッケージ温度では `RyzenSMU.bin`
  - AMD パッケージ電力では `AMDFamily17.bin`
- アプリに代わってPawnIOを開く、接続済みのWindowsセンサーサービス

Windowsセンサーサービスは、識別できる署名済みの `.bin` だけを読み込みます。
未署名の `.amx` やテスト用モジュールは使用しません。

モジュールを配置する手順:

1. PawnIO runtime をインストールします。
2. [namazso/PawnIO.Modules](https://github.com/namazso/PawnIO.Modules/releases) からリリースアセットをダウンロードします。
3. アーカイブを展開し、必要な CPU モジュールファイルを `C:\Program Files\PawnIO` にコピーします。
   - Intel パッケージ温度または電力を取得する場合は、`IntelMSR.bin` をコピーします。
   - 対応 AMD パッケージ温度を取得する場合は、`RyzenSMU.bin` をコピーします。
   - AMD パッケージ電力を取得する場合は、`AMDFamily17.bin` をコピーします。
   - どの CPU 経路が必要かわからない場合は、必要な署名済みモジュールをすべてコピーしても問題ありません。
4. HardwareVisualizer を再起動します。

`C:\Program Files\PawnIO` への書き込みには、通常は管理者権限が必要です。

PawnIOを導入済みでも取得できない場合は、高度な設定でWindowsセンサーサービスを確認・修復してください。本体の昇格では、別プロセスのサービスは修復されません。

### 公式の入手先

- PawnIO runtime: <https://github.com/namazso/PawnIO>
- PawnIO modules: <https://github.com/namazso/PawnIO.Modules/releases>

実装上の詳細は [`../architecture/windows-sensor-external-components.md`](../architecture/windows-sensor-external-components.md) に記録しています。

### PawnIO Motherboard Sensors

PawnIO は、対応している Super I/O のマザーボード温度とファン回転数を Windows で取得する場合にも使われます。この経路では PawnIO LpcIO module と、WindowsセンサーサービスがPawnIO driverを開ける権限が必要です。

PawnIO 経由でマザーボードセンサーを取得するには、対象の PC に次のものが必要です。

- 正しく動作する PawnIO driver/runtime
- PawnIO runtime に含まれる `PawnIOLib.dll`
- 対応する署名済みリリースの `LpcIO.bin`
- アプリに代わってPawnIOを開く、接続済みのWindowsセンサーサービス

PawnIOを導入済みでもマザーボードの値を取得できない場合は、Windowsセンサーサービスと対応する署名済みLpcIOモジュールを確認・修復してください。

この案内は読み取り専用のマザーボードセンサー経路だけを対象にしています。ファン制御、PWM 書き込み、電圧、未対応の Super I/O chip、vendor 固有の embedded controller sensor は対象外です。

## smartctl

`smartctl` は smartmontools に含まれるツールです。HardwareVisualizer は、OS 標準の経路だけでは十分なデバイス健康状態を取得できない場合に、Storage Health の取得元の 1 つとして `smartctl` を使用できます。

Storage Health が少なくとも 1 つのストレージデバイスで重要な健康状態シグナルを取得できない場合、`smartctl` の案内が表示されることがあります。OS 標準またはフォールバックの取得元で重要な健康状態シグナルを取得できている場合、この案内は表示しません。

`smartctl:storage-health:v1` の案内キーはクロスプラットフォームです。Windows、Linux、macOS のいずれでも、HardwareVisualizer が実際に `smartctl` を使おうとして、利用可能な取得元では重要な Storage Health シグナルを取得できなかった場合に表示されます。

Live Storage Health は、利用できる場合は軽量な OS 標準の読み取り経路を使用します。ライブポーリングの周期では `smartctl` を使用しません。

### smartctl で改善できる項目

Storage Health では、HardwareVisualizer は次の情報を重要な健康状態シグナルとして扱います。

- SMART 全体健康状態
- 温度
- NVMe 使用率
- NVMe 予備領域
- 代替処理済みセクター数
- 現在保留中のセクター数
- オフライン訂正不能セクター数
- NVMe メディアエラー

電源投入時間、電源投入回数、異常シャットダウン回数、エラーログエントリ数などの値も UI 上では有用な場合があります。ただし、それらだけが不足している場合は外部コンポーネント案内を表示しません。

### ユーザー側で必要な準備

`smartctl` を使うには、利用している OS 向けの smartmontools をインストールし、`smartctl` 実行ファイルを `PATH` から利用できるようにしてください。

一部のストレージデバイスや OS では、SMART データの読み取りに昇格権限が必要な場合があります。`smartctl` をインストール済みでも HardwareVisualizer が Storage Health シグナルを取得できない場合は、利用している OS とデバイスで必要な権限で HardwareVisualizer を実行してください。

`smartctl` のインストールまたは設定修正後、Storage Device Refresh が利用できる場合は実行してください。そうでない場合は HardwareVisualizer を再起動してください。日次の Storage Health 収集も、次回のスケジュール実行時に再試行します。

### 公式の入手先

- smartmontools: <https://www.smartmontools.org/>

## 案内理由

アプリは、利用できないコンポーネントを次のように分類することがあります。

- `missing`: 実行ファイル、DLL、またはモジュールファイルが見つからない
- `permission`: コンポーネントは存在するが、現在の権限では開けない
- `misconfigured`: runtime は存在するが、必要な driver、service、module が不足している
- `failed`: コンポーネントは実行されたが、対象デバイスまたはセンサーの有効なデータを返さなかった

website では、これらの分類を使って、raw error string を主なユーザー向けメッセージとして表示するのではなく、具体的な次の対応を表示します。
