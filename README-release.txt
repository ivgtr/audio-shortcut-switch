Audio Shortcut Switch 0.3.2 (Windows 10/11 x64)

アプリ本体と作成するショートカットには PON アイコンを使用します。

使い方
1. この ZIP を展開し、AudioShortcutSwitch フォルダーを %LOCALAPPDATA% など
   書き込み可能な場所に置きます。
   例: C:\Users\<ユーザー名>\AppData\Local\AudioShortcutSwitch\AudioShortcutSwitch.exe
2. AudioShortcutSwitch.exe を開き、出力先とショートカット名を選んで「作成」を押します。
   Bluetooth 機器は先に Windows でペアリングします。作成時は切り替えません。
3. デスクトップのショートカットをクリックします。通常の出力機器はそのまま、
   Bluetooth 機器は接続と Stereo 出力の確認後に、Console と Multimedia の既定出力に設定します。

Bluetooth 機器の Stereo 出力 ID を Windows がまだ保持していない場合は、
イヤホンの電源を入れ、「接続確認」を実行してから作成してください。
通常の出力機器では「利用可能か確認」で使用可能な状態を確認できます。
確認ボタンは既定出力を変更しません。
同じ名前のショートカットが既にある場合は、別の名前を入力してください。
アプリのフォルダーを移動した場合は、ショートカットを作り直してください。

成功時は画面を出しません。失敗時はエラーを表示し、
logs\switch.log に結果を残します。configs\*.json には機器固有の ID を保存します。
出力先の設定を解除するには、そのショートカットと対応する設定ファイルを削除します。

この配布物には、次の補助ツールを同梱しています。
- ToothTrayCLI 0.2-cli-20260701 (BSD-2-Clause)
  https://github.com/shunf4/ToothTrayCli/releases/tag/0.2-cli-20260701
  ライセンス: tools\ToothTrayCLI-LICENSE.md
- SoundVolumeView 2.53 (NirSoft freeware)
  https://www.nirsoft.net/utils/sound_volume_view.html
  配布元 ZIP の全ファイルを tools\SoundVolumeView に改変せず収録しています。
  利用条件: tools\SoundVolumeView\readme.txt

Rust 依存ライブラリのライセンス情報: tools\Rust-Crates-LICENSES.txt
