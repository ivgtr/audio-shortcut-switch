param(
    [Parameter(Mandatory = $true)] [string]$ConfigPath,
    [Parameter(Mandatory = $true)] [string]$ShortcutName
)

$ErrorActionPreference = 'Stop'
if ($ShortcutName -notmatch '^[^\\/:*?"<>|]{1,100}$' -or $ShortcutName.EndsWith(' ')) {
    throw 'Invalid shortcut name.'
}
$config = (Resolve-Path -LiteralPath $ConfigPath).Path
$base = Split-Path -Parent $MyInvocation.MyCommand.Path
$launcher = Join-Path $base 'Launch-Earbuds.vbs'
$desktop = [Environment]::GetFolderPath('Desktop')
$link = Join-Path $desktop ($ShortcutName + '.lnk')
if (Test-Path -LiteralPath $link) { throw "Shortcut already exists: $link" }
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($link)
$shortcut.TargetPath = Join-Path $env:WINDIR 'System32\wscript.exe'
$shortcut.Arguments = '"' + $launcher + '" "' + $config + '"'
$shortcut.WorkingDirectory = $base
$shortcut.Description = '登録済みBluetooth機器へ接続し、Stereo再生出力へ切り替える'
$shortcut.Save()
Write-Output $link
