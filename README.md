# Earbuds PC Switch

Windows 10/11 x64 で、ペアリング済みの Bluetooth イヤホンへ接続し、Stereo/A2DP 再生出力を Console と Multimedia の既定に設定します。成功時は画面を出さず、失敗時はエラーを表示して `logs/switch.log` に記録します。

## 公開版 v0.1.0

1. [Releases](https://github.com/ivgtr/earbuds-pc-switch/releases/latest) の `EarbudsPC-0.1.0-win-x64.zip` をダウンロードします。GitHub の自動生成 Source code ZIP には実行ファイルがありません。
2. ZIP を展開し、`EarbudsPC` フォルダーを `%LOCALAPPDATA%` など書き込み可能な場所に置きます。
3. Windows でイヤホンをペアリングして電源を入れ、`EarbudsPC.exe` を起動します。
4. 機器とショートカット名を選んで「作成」を押します。v0.1.0 では作成時に接続と Stereo 出力を確認します。

同じ名前のリンクがある場合は別名を入力してください。複数の機器のリンクも作成できます。設定は `configs/*.json` に保存され、機器 ID と Stereo 出力 ID を使って対象を照合します。ショートカット作成後にアプリのフォルダーを移動した場合は、ショートカットを作り直してください。

## v0.2.0（実機検証待ち）

[Windows Actions の最新の成功実行](https://github.com/ivgtr/earbuds-pc-switch/actions/workflows/windows-release.yml)には、作成時の接続確認を外し、任意の「接続確認」ボタンを追加した v0.2.0 の ZIP があります。接続確認では Bluetooth 接続と Stereo 出力を確認し、既定出力は変更しません。Windows が Stereo 出力 ID をまだ保持していない機器は、初回接続後に作成する必要があります。未接続での作成と確認ボタンの実機試験が終わるまで Release には公開しません。

## 開発と配布物

実装は Rust です。配布用 EXE と ZIP は [GitHub Actions の Windows ワークフロー](.github/workflows/windows-release.yml)で作成します。ワークフローは Rust 1.99.1 と MSVC ターゲットを使い、EXE 内のユーザーフォルダーパスを検査します。

ローカルでは `mise exec -- cargo test` で単体テストを実行できます。配布物のビルドは Windows ワークフローを使います。

配布 ZIP には [ToothTrayCLI 0.2-cli-20260701](https://github.com/shunf4/ToothTrayCli/releases/tag/0.2-cli-20260701) と [SoundVolumeView 2.53](https://www.nirsoft.net/utils/sound_volume_view.html) を同梱します。`scripts/package_release.py` は指定した公式 ZIP の SHA-256 を検証し、SoundVolumeView の配布 ZIP の全ファイルと ToothTrayCLI のライセンスを含めてまとめます。補助ツールと実機の設定・ログはソース管理に含めません。
