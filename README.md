# Earbuds PC Switch

Windows 10で、登録済みBluetoothイヤホンへの接続と、Stereo/A2DP再生出力への切り替えをデスクトップのショートカットから行うツールです。ConsoleとMultimediaの既定出力だけを変更します。

## 現在の状態

このリポジトリには実機で動作確認したPowerShell版のソースがあります。EXEと設定GUI、GitHub Releases用の完成品はまだ作成していません。機器固有の設定、ログ、ダウンロードした実行ファイルはリポジトリに含めません。

PowerShell版は、登録済み機器のContainer IDと対応するStereo再生エンドポイントIDを設定ファイルに保存します。起動時に接続を要求し、期限内に使用可能になったことを確認してから既定出力を変更します。既に正しい状態なら接続や出力を変更しません。失敗時はログを残し、接続できない段階では現在の既定出力を維持します。

## ソースの構成

- `Switch-Earbuds.ps1`: ショートカットから実行する接続・出力切り替え処理
- `Register-Earbuds.ps1`: 登録済み機器を正確な識別子とStereoエンドポイントで設定する処理
- `New-Shortcut.ps1`: 実際のデスクトップの場所にショートカットを作成
- `Launch-Earbuds.vbs`: 成功時に端末を表示せず起動
- `configs/`: ローカルで作成した設定の保存先。設定の実データは公開しません。

このPCで検証した補助ツールは[ToothTrayCLI 0.2-cli-20260701](https://github.com/shunf4/ToothTrayCli/releases/tag/0.2-cli-20260701)と[SoundVolumeView 2.53](https://www.nirsoft.net/utils/sound_volume_view.html)です。補助ツールは開発元から取得し、`tools/ToothTrayCLI.exe`と`tools/SoundVolumeView/SoundVolumeView.exe`に配置します。両ツールの配布条件を確認した上で、公開用パッケージの構成を決めます。

GitHubで配布する完成品は、ソースの自動生成ZIPではなく、実機確認後のEXEを含むRelease assetにする予定です。
