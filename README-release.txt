Earbuds PC Switch 0.1.0 (Windows 10/11 x64)

使い方
1. この ZIP を展開し、EarbudsPC フォルダーを %LOCALAPPDATA% に置きます。
   例: C:\Users\<ユーザー名>\AppData\Local\EarbudsPC\EarbudsPC.exe
2. Bluetooth イヤホンを Windows でペアリングし、電源を入れます。
3. EarbudsPC.exe を開き、機器とショートカット名を選んで「作成」を押します。
4. デスクトップに作られたショートカットをクリックすると、その機器へ接続し、
   Stereo 再生出力を Console と Multimedia の既定に設定します。

設定を作るときは対象機器の Stereo 出力が使える状態である必要があります。
同じ名前のショートカットが既にある場合は、別の名前を入力してください。
別の機器用のショートカットも同じ設定画面で作成できます。

成功時は画面を出しません。失敗時はエラーを表示し、
logs\switch.log に結果を残します。configs\*.json には機器固有の ID を保存します。
機器の設定を解除するには、その機器のショートカットと対応する設定ファイルを削除します。

この配布物には、次の補助ツールを同梱しています。
- ToothTrayCLI 0.2-cli-20260701 (BSD-2-Clause)
  https://github.com/shunf4/ToothTrayCli/releases/tag/0.2-cli-20260701
  ライセンス: tools\ToothTrayCLI-LICENSE.md
- SoundVolumeView 2.53 (NirSoft freeware)
  https://www.nirsoft.net/utils/sound_volume_view.html
  配布元 ZIP の全ファイルを tools\SoundVolumeView に改変せず収録しています。
  利用条件: tools\SoundVolumeView\readme.txt

Rust 依存ライブラリのライセンス情報: tools\Rust-Crates-LICENSES.txt
