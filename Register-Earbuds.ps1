param(
    [Parameter(Mandatory = $true)] [string]$DeviceName,
    [Parameter(Mandatory = $true)] [string]$ShortcutName
)

$ErrorActionPreference = 'Stop'
$base = Split-Path -Parent $MyInvocation.MyCommand.Path
$bt = Join-Path $base 'tools\ToothTrayCLI.exe'
$sv = Join-Path $base 'tools\SoundVolumeView\SoundVolumeView.exe'
$desktop = [Environment]::GetFolderPath('Desktop')
$snapshot = Join-Path $base ('audio-registration-' + $PID + '.json')

if (-not (Test-Path -LiteralPath $bt) -or -not (Test-Path -LiteralPath $sv)) { throw 'Required tools are missing.' }
if ($DeviceName -eq '' -or $DeviceName.Contains('"')) { throw 'Invalid device name.' }
if ($ShortcutName -notmatch '^[^\\/:*?"<>|]{1,100}$' -or $ShortcutName.EndsWith(' ')) { throw 'Invalid shortcut name.' }
if (Test-Path -LiteralPath (Join-Path $desktop ($ShortcutName + '.lnk'))) { throw 'Shortcut already exists.' }

function Invoke-Bluetooth([string]$arguments, [int]$timeoutMs = 8000) {
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $bt
    $start.Arguments = $arguments
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::Start($start)
    try {
        if (-not $process.WaitForExit($timeoutMs)) {
            $process.Kill()
            throw "Bluetooth tool timed out: $arguments"
        }
        $output = $process.StandardOutput.ReadToEnd().Trim()
        if ($process.ExitCode -ne 0) { throw "Bluetooth tool failed ($($process.ExitCode)): $arguments" }
        return $output
    } finally { $process.Dispose() }
}

function Get-AudioEndpoint([string]$stereoName) {
    Remove-Item -LiteralPath $snapshot -Force -ErrorAction SilentlyContinue
    $process = Start-Process -FilePath $sv -ArgumentList @('/SaveFileEncoding', '3', '/sjson', $snapshot) -WindowStyle Hidden -PassThru
    try {
        if (-not $process.WaitForExit(8000)) { $process.Kill(); throw 'Audio export timed out.' }
        if ($process.ExitCode -ne 0) { throw 'Audio export failed.' }
    } finally { $process.Dispose() }
    if (-not (Test-Path -LiteralPath $snapshot)) { throw 'Audio export is missing.' }
    $all = Get-Content -LiteralPath $snapshot -Raw -Encoding UTF8 | ConvertFrom-Json
    $found = @($all | Where-Object { $_.Type -eq 'Device' -and $_.Direction -eq 'Render' -and $_.'Device Name' -ceq $stereoName -and $_.'Device State' -eq 'Active' })
    if ($found.Count -gt 1) { throw 'Stereo endpoint is not unique.' }
    if ($found.Count -eq 0) { return $null }
    $id = [string]$found[0].'Item ID'
    if ($id -cnotmatch '^\{0\.0\.0\.00000000\}\.\{[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}\}$') { throw 'Unexpected endpoint ID.' }
    return $id
}

try {
    $lines = Invoke-Bluetooth 'list'
    $entries = @(foreach ($line in ($lines -split '\r?\n')) {
        if ($line -match '^([01]) (\{[0-9A-Fa-f-]{36}\}) (.+)$' -and $Matches[3] -ceq $DeviceName) {
            [pscustomobject]@{ ContainerId = $Matches[2] }
        }
    })
    if ($entries.Count -ne 1) { throw 'Paired device name must match exactly one audio device.' }
    $containerId = $entries[0].ContainerId
    $configPath = Join-Path (Join-Path $base 'configs') ($containerId.Trim('{}').ToLowerInvariant() + '.json')
    if (Test-Path -LiteralPath $configPath) { throw "Configuration already exists: $configPath" }

    $a2dpPrefix = 'BTHENUM\{0000110B-0000-1000-8000-00805F9B34FB}'
    $stereoDrivers = @(foreach ($device in (Get-PnpDevice -Class MEDIA)) {
        if (-not $device.InstanceId.StartsWith($a2dpPrefix, [StringComparison]::OrdinalIgnoreCase)) { continue }
        $property = Get-PnpDeviceProperty -InstanceId $device.InstanceId -KeyName 'DEVPKEY_Device_ContainerId' -ErrorAction Stop
        if ($property.Data -ieq $containerId) { $device }
    })
    if ($stereoDrivers.Count -ne 1) { throw 'A2DP Stereo driver is not uniquely registered for this Bluetooth device.' }
    $stereoName = $stereoDrivers[0].FriendlyName

    $connected = Invoke-Bluetooth "is-connected-by-container-id $containerId"
    if ($connected -ne '0' -and $connected -ne '1') { throw 'Unexpected Bluetooth status.' }
    if ($connected -eq '0') { [void](Invoke-Bluetooth "connect-by-container-id $containerId" 15000) }
    $deadline = (Get-Date).AddSeconds(35)
    $endpointId = $null
    do {
        $connected = Invoke-Bluetooth "is-connected-by-container-id $containerId"
        if ($connected -eq '1') { $endpointId = Get-AudioEndpoint $stereoName }
        if ($endpointId) { break }
        Start-Sleep -Milliseconds 700
    } while ((Get-Date) -lt $deadline)
    if (-not $endpointId) { throw 'Stereo endpoint did not become available within 35 seconds.' }

    $config = [ordered]@{
        schemaVersion = 1
        displayName = $DeviceName
        bluetoothContainerId = $containerId
        renderEndpointId = $endpointId
        stereoDeviceName = $stereoName
    }
    $json = $config | ConvertTo-Json -Depth 2
    [IO.File]::WriteAllText($configPath, $json + "`r`n", (New-Object Text.UTF8Encoding($true)))
    try {
        & (Join-Path $base 'New-Shortcut.ps1') -ConfigPath $configPath -ShortcutName $ShortcutName
    } catch {
        Remove-Item -LiteralPath $configPath -Force
        throw
    }
    Write-Output "Configuration: $configPath"
} finally {
    Remove-Item -LiteralPath $snapshot -Force -ErrorAction SilentlyContinue
}
