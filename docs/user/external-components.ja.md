# 外部コンポーネント

[English](external-components.md) | [日本語](external-components.ja.md)

このページは External Component Guidance の日本語版の元コンテンツです。現時点では、アプリは GitHub 上で表示されるこのドキュメントを直接開きます。将来的には HardwareVisualizer の website がこのページを元にした内容を公開し、website URL が用意できた時点でアプリ側のリンク先を切り替えます。

暫定的なアプリ内詳細リンク:

- PawnIO:
  `https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.ja.md#pawnio`
- smartctl:
  `https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.ja.md#smartctl`

HardwareVisualizer は、多くのハードウェア情報を任意の外部コンポーネントなしで取得できます。一部のより深いセンサー経路では、ユーザーが別途インストールする必要があるツールやドライバーに依存します。HardwareVisualizer は、それらのコンポーネントを自動でダウンロード、インストール、同梱、有効化しません。

External Component Guidance は、HardwareVisualizer が任意コンポーネントを使おうとして使えず、さらにフォールバック取得でもユーザーに表示するハードウェア情報が不足する場合にだけ表示されます。フォールバック取得で必要な情報を取得できた場合、アプリはこの案内を表示しません。

## ダイアログの意味

ダイアログでは次の内容を案内します。

- 使えなかった任意コンポーネント
- 取得できず表示できないハードウェア情報
- その情報を表示できるようにするための対応

このダイアログは情報提供のためのものです。ハードウェア監視は、利用できる最善のフォールバックデータで継続します。

## PawnIO

PawnIO は、対応している CPU で PawnIO 経由の取得経路が利用できる場合に、Windows の CPU パッケージ温度を取得するために使われます。

ダッシュボードで CPU 温度を表示できない場合、PawnIO が関係していることがあります。HardwareVisualizer が ACPI thermal zone 経由で CPU 温度を表示できている場合、PawnIO の案内は表示しません。

### PawnIO で表示できる項目

PawnIO は、CPU ごとのセンサー経路から Windows の CPU パッケージ温度を取得できます。

- Intel MSR モジュール経由の Intel パッケージ温度
- Ryzen SMU モジュール経由の AMD Family 17h / Family 19h パッケージ温度

### ユーザー側で必要な準備

PawnIO 経由で CPU パッケージ温度を取得するには、対象の PC に次のものが必要です。

- 正しく動作する PawnIO driver/runtime
- PawnIO runtime に含まれる `PawnIOLib.dll`
- PawnIO.Modules のリリースアセットに含まれる CPU 別モジュール
  - 対応 Intel CPU では `IntelMSR.amx` または `IntelMSR.bin`
  - 対応 AMD CPU では `RyzenSMU.amx` または `RyzenSMU.bin`
- HardwareVisualizer のプロセスが PawnIO driver を開ける Windows 権限

モジュールを配置する手順:

1. PawnIO runtime をインストールします。
2. [namazso/PawnIO.Modules](https://github.com/namazso/PawnIO.Modules/releases) からリリースアセットをダウンロードします。
3. アーカイブを展開し、必要な CPU モジュールファイルを `C:\Program Files\PawnIO` にコピーします。
   - Intel パッケージ温度を取得する場合は、`IntelMSR.amx` または `IntelMSR.bin` をコピーします。
   - 対応 AMD パッケージ温度を取得する場合は、`RyzenSMU.amx` または `RyzenSMU.bin` をコピーします。
   - どちらの CPU 経路が必要かわからない場合は、両方のモジュールファイルをコピーしても問題ありません。
4. HardwareVisualizer を再起動します。

`C:\Program Files\PawnIO` への書き込みには、通常は管理者権限が必要です。

PawnIO をインストール済みでも HardwareVisualizer が権限不足を報告する場合は、HardwareVisualizer を管理者として実行してから再度確認してください。

### 公式の入手先

- PawnIO runtime: <https://github.com/namazso/PawnIO>
- PawnIO modules: <https://github.com/namazso/PawnIO.Modules/releases>

実装上の詳細は [`../architecture/windows-sensor-external-components.md`](../architecture/windows-sensor-external-components.md) に記録しています。

### PawnIO Motherboard Sensors

PawnIO は、対応している Super I/O のマザーボード温度とファン回転数を Windows で取得する場合にも使われます。この経路では PawnIO LpcIO module と、HardwareVisualizer のプロセスが PawnIO driver を開ける権限が必要です。

PawnIO 経由でマザーボードセンサーを取得するには、対象の PC に次のものが必要です。

- 正しく動作する PawnIO driver/runtime
- PawnIO runtime に含まれる `PawnIOLib.dll`
- PawnIO.Modules のリリースアセットに含まれる `LpcIO.bin` または `LpcIO.amx`
- HardwareVisualizer のプロセスが PawnIO driver を開ける Windows 権限

PawnIO をインストール済みでも HardwareVisualizer が権限不足を報告する場合は、HardwareVisualizer を管理者として実行するか、「管理者として起動」を有効にしてからアプリを再起動してください。

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
