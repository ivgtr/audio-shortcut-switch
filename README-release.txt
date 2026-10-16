Earbuds PC Switch 0.2.0 (Windows 10/11 x64)

使い方
1. この ZIP を展開し、EarbudsPC フォルダーを %LOCALAPPDATA% など
   書き込み可能な場所に置きます。
   例: C:\Users\<ユーザー名>\AppData\Local\EarbudsPC\EarbudsPC.exe
2. Bluetooth イヤホンを Windows でペアリングします。
3. EarbudsPC.exe を開き、機器とショートカット名を選んで「作成」を押します。
   作成時にイヤホンへ接続しません。
4. 接続を事前に試す場合は、イヤホンの電源を入れて「接続確認」を押します。
   Bluetooth接続とStereo出力を確認します。既定出力は変更しません。
5. デスクトップに作られたショートカットをクリックすると、その機器へ接続し、
   Stereo 再生出力を Console と Multimedia の既定に設定します。

Windows が Stereo 出力 ID をまだ保持していない場合は、
一度「接続確認」を実行してからショートカットを作成してください。
同じ名前のショートカットが既にある場合は、別の名前を入力してください。
別の機器用のショートカットも同じ設定画面で作成できます。
アプリのフォルダーを移動した場合は、ショートカットを作り直してください。

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
