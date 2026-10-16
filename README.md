# Audio Shortcut Switch

<img src="assets/app.svg" alt="PON アイコン" width="72" height="72">

デスクトップのショートカットから、Windows の再生出力をワンクリックで切り替えるアプリです。Bluetooth イヤホンなら接続も行います。スピーカー、USB DAC、HDMI 出力などにもショートカットを作れます。

[Windows 10/11 x64 用 ZIP をダウンロード](https://github.com/ivgtr/audio-shortcut-switch/releases/latest)

## 使い方

1. ZIP を書き込み可能な場所に展開し、`AudioShortcutSwitch.exe` を起動します。Bluetooth 機器は先に Windows でペアリングします。
2. 出力先とショートカット名を選び、「作成」を押します。
3. 使いたい出力先のショートカットをクリックします。通常の出力はそのまま既定に設定し、Bluetooth は接続と Stereo 出力の確認後に設定します。

作成時は出力を切り替えません。Bluetooth 機器の Stereo 出力がまだ見つからない場合は、イヤホンの電源を入れ、「接続確認」で一度接続してから作成してください。確認ボタンは既定の出力を変更しません。

## 補足

- Console と Multimedia の再生用出力を切り替えます。通話用出力は変更しません。
- アプリのフォルダーを移動した場合は、ショートカットを作り直してください。
- 成功時は画面を出さず、失敗時はエラーを表示して `logs/switch.log` に記録します。
