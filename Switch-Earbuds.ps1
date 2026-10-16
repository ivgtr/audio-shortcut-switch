param(
    [Parameter(Mandatory = $true)]
    [string]$ConfigPath,
    [switch]$NoDialog
)

$ErrorActionPreference = 'Stop'
$base = Split-Path -Parent $MyInvocation.MyCommand.Path
$logDir = Join-Path $base 'logs'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$log = Join-Path $logDir 'switch.log'
$audioTool = Join-Path $base 'tools\SoundVolumeView\SoundVolumeView.exe'
$bluetoothTool = Join-Path $base 'tools\ToothTrayCLI.exe'
$snapshot = Join-Path $logDir ('audio-' + $PID + '.json')
$mutex = New-Object System.Threading.Mutex($false, 'Local\EarbudsPC.Switch')
$locked = $false
$exitCode = 0

function Write-Log([string]$message) {
    Add-Content -LiteralPath $log -Encoding UTF8 -Value ((Get-Date -Format 'yyyy-MM-dd HH:mm:ss.fff') + ' ' + $message)
}

function Invoke-Bluetooth([string]$arguments, [int]$timeoutMs = 8000) {
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $bluetoothTool
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
        $stdout = $process.StandardOutput.ReadToEnd().Trim()
        $stderr = $process.StandardError.ReadToEnd().Trim()
        if ($process.ExitCode -ne 0) {
            throw "Bluetooth tool failed ($($process.ExitCode)): $arguments $stderr $stdout"
        }
        return $stdout
    } finally {
        $process.Dispose()
    }
}

function Get-Connection([string]$id) {
    $result = Invoke-Bluetooth "is-connected-by-container-id $id"
    if ($result -ne '0' -and $result -ne '1') {
        throw "Unexpected Bluetooth status: $result"
    }
    return ($result -eq '1')
}

function Invoke-Audio([string[]]$arguments) {
    $process = Start-Process -FilePath $audioTool -ArgumentList $arguments -WindowStyle Hidden -PassThru
    try {
        if (-not $process.WaitForExit(8000)) {
            $process.Kill()
            throw 'Audio tool timed out.'
        }
        if ($process.ExitCode -ne 0) {
            throw "Audio tool failed ($($process.ExitCode))."
        }
    } finally {
        $process.Dispose()
    }
}

function Get-AudioState([string]$endpointId, [string]$stereoDeviceName) {
    Remove-Item -LiteralPath $snapshot -Force -ErrorAction SilentlyContinue
    Invoke-Audio @('/SaveFileEncoding', '3', '/sjson', $snapshot)
    if (-not (Test-Path -LiteralPath $snapshot)) {
        throw 'Audio tool did not export device state.'
    }
    $all = Get-Content -LiteralPath $snapshot -Raw -Encoding UTF8 | ConvertFrom-Json
    $matches = @($all | Where-Object { $_.Type -eq 'Device' -and $_.Direction -eq 'Render' -and $_.'Item ID' -ieq $endpointId })
    if ($matches.Count -gt 1) { throw 'Duplicate audio endpoint ID.' }
    if ($matches.Count -eq 0) {
        return @{ Available = $false; Console = $false; Multimedia = $false }
    }
    $item = $matches[0]
    if ($item.'Device Name' -cne $stereoDeviceName) {
        throw "Endpoint device name differs from configuration: $($item.'Device Name')"
    }
    return @{
        Available = ($item.'Device State' -eq 'Active')
        Console = ($item.Default -eq 'Render')
        Multimedia = ($item.'Default Multimedia' -eq 'Render')
    }
}

function Read-Config([string]$path) {
    $resolved = (Resolve-Path -LiteralPath $path).Path
    $config = Get-Content -LiteralPath $resolved -Raw -Encoding UTF8 | ConvertFrom-Json
    $keys = @($config.PSObject.Properties.Name | Sort-Object)
    $expected = @('bluetoothContainerId', 'displayName', 'renderEndpointId', 'schemaVersion', 'stereoDeviceName' | Sort-Object)
    if (($keys -join '|') -cne ($expected -join '|')) { throw 'Configuration fields are invalid.' }
    if ($config.schemaVersion -isnot [int] -or $config.schemaVersion -ne 1) { throw 'Unsupported configuration version.' }
    if ($config.bluetoothContainerId -cnotmatch '^\{[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}\}$') { throw 'Invalid Bluetooth container ID.' }
    if ($config.renderEndpointId -cnotmatch '^\{0\.0\.0\.00000000\}\.\{[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}\}$') { throw 'Invalid render endpoint ID.' }
    if ($config.displayName -isnot [string] -or [string]::IsNullOrWhiteSpace($config.displayName)) { throw 'Invalid display name.' }
    if ($config.stereoDeviceName -isnot [string] -or [string]::IsNullOrWhiteSpace($config.stereoDeviceName)) { throw 'Invalid stereo device name.' }
    return $config
}

try {
    try { $locked = $mutex.WaitOne(0) } catch [System.Threading.AbandonedMutexException] { $locked = $true }
    if (-not $locked) {
        Write-Log 'Another switch is already running; duplicate launch ignored.'
    } else {
        if (-not (Test-Path -LiteralPath $audioTool) -or -not (Test-Path -LiteralPath $bluetoothTool)) {
            throw 'Required tool is missing.'
        }
        $config = Read-Config $ConfigPath
        Write-Log "Start: $($config.displayName), container=$($config.bluetoothContainerId), endpoint=$($config.renderEndpointId)"
        $devices = Invoke-Bluetooth 'list'
        $matchLine = @($devices -split '\r?\n' | Where-Object { $_ -match ('^[01] ' + [regex]::Escape($config.bluetoothContainerId) + ' ' + [regex]::Escape($config.displayName) + '$') })
        if ($matchLine.Count -ne 1) { throw 'Configured Bluetooth device is not uniquely registered.' }

        $connected = Get-Connection $config.bluetoothContainerId
        $audio = Get-AudioState $config.renderEndpointId $config.stereoDeviceName
        Write-Log "Initial: connected=$connected available=$($audio.Available) console=$($audio.Console) multimedia=$($audio.Multimedia)"
        if (-not $connected) {
            Write-Log 'Requesting Bluetooth connection.'
            [void](Invoke-Bluetooth "connect-by-container-id $($config.bluetoothContainerId)" 15000)
        }

        $deadline = (Get-Date).AddSeconds(35)
        while (-not ($connected -and $audio.Available)) {
            if ((Get-Date) -ge $deadline) { throw 'Earbuds did not become available within 35 seconds.' }
            Start-Sleep -Milliseconds 700
            $connected = Get-Connection $config.bluetoothContainerId
            $audio = Get-AudioState $config.renderEndpointId $config.stereoDeviceName
        }
        Write-Log 'Stereo endpoint is available.'

        if (-not $audio.Console) {
            Write-Log 'Setting Console default.'
            Invoke-Audio @('/SetDefault', $config.renderEndpointId, '0')
        }
        if (-not $audio.Multimedia) {
            Write-Log 'Setting Multimedia default.'
            Invoke-Audio @('/SetDefault', $config.renderEndpointId, '1')
        }

        $verified = $false
        $verifyDeadline = (Get-Date).AddSeconds(8)
        do {
            $connected = Get-Connection $config.bluetoothContainerId
            $audio = Get-AudioState $config.renderEndpointId $config.stereoDeviceName
            $verified = $connected -and $audio.Available -and $audio.Console -and $audio.Multimedia
            if ($verified) { break }
            Start-Sleep -Milliseconds 350
        } while ((Get-Date) -lt $verifyDeadline)
        if (-not $verified) { throw 'Final Bluetooth/audio state could not be confirmed.' }
        Write-Log 'Success: connected, Stereo active, Console and Multimedia default.'
    }
} catch {
    $exitCode = 1
    Write-Log ('ERROR: ' + $_.Exception.Message)
    if (-not $NoDialog) {
        Add-Type -AssemblyName System.Windows.Forms
        [void][System.Windows.Forms.MessageBox]::Show(
            "イヤホンへの切り替えに失敗しました。`r`n$($_.Exception.Message)`r`nログ: $log",
            'イヤホンをPCで使う',
            [System.Windows.Forms.MessageBoxButtons]::OK,
            [System.Windows.Forms.MessageBoxIcon]::Error
        )
    }
} finally {
    Remove-Item -LiteralPath $snapshot -Force -ErrorAction SilentlyContinue
    if ($locked) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}
exit $exitCode
