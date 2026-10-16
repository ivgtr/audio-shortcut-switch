# Earbuds PC Switch

Windows 10/11 x64 で、ペアリング済みの Bluetooth イヤホンへ接続し、Stereo/A2DP 再生出力を Console と Multimedia の既定に設定します。成功時は画面を出さず、失敗時はエラーを表示して `logs/switch.log` に記録します。

## ダウンロードと設定

1. [Releases](https://github.com/ivgtr/earbuds-pc-switch/releases/latest) の `EarbudsPC-0.1.0-win-x64.zip` をダウンロードします。GitHub の自動生成 Source code ZIP には実行ファイルがありません。
2. ZIP を展開し、`EarbudsPC` フォルダーを `%LOCALAPPDATA%` に置きます。
3. Windows でイヤホンをペアリングして電源を入れ、`EarbudsPC.exe` を起動します。
4. 機器とショートカット名を選んで「作成」を押します。デスクトップにできたリンクから接続できます。

設定時は対象機器の Stereo 出力が使える必要があります。同じ名前のリンクがある場合は別名を入力してください。複数の機器のリンクも作成できます。設定は `configs/*.json` に保存され、機器 ID と Stereo 出力 ID を使って対象を照合します。

## 開発と配布物

実装は Rust です。`mise.toml` は Rust 1.99.1 と Zig 0.15.1 を固定します。Windows EXE のクロスビルドには `cargo-zigbuild` 0.23.4 と `x86_64-pc-windows-gnu` ターゲットを使います。

```sh
mise install
rustup target add x86_64-pc-windows-gnu
cargo install cargo-zigbuild --version 0.23.4 --locked
mise exec -- cargo test
mise exec -- cargo zigbuild --target x86_64-pc-windows-gnu --release
```

配布 ZIP には [ToothTrayCLI 0.2-cli-20260701](https://github.com/shunf4/ToothTrayCli/releases/tag/0.2-cli-20260701) と [SoundVolumeView 2.53](https://www.nirsoft.net/utils/sound_volume_view.html) を同梱します。`scripts/package_release.py` は指定した公式 ZIP の SHA-256 を検証し、SoundVolumeView の配布 ZIP の全ファイルと ToothTrayCLI のライセンスを含めてまとめます。補助ツールと実機の設定・ログはソース管理に含めません。
