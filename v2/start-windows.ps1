param(
    [string]$DataDir = '',
    [switch]$CheckOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'This launcher is for Windows. See v2/start-ubuntu.sh for Ubuntu.'
}

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$tauriDir = Join-Path $repoRoot 'src-tauri'
$viteScript = Join-Path $repoRoot 'node_modules\vite\bin\vite.js'
$v2Config = Join-Path $PSScriptRoot 'vite.config.ts'
$node = Get-Command node.exe -ErrorAction Stop
$cargo = Get-Command cargo.exe -ErrorAction Stop

if (-not (Test-Path -LiteralPath $viteScript -PathType Leaf)) {
    throw "Vite is missing at $viteScript. Install the project dependencies with a working npm installation before starting the app."
}
if (-not (Test-Path -LiteralPath (Join-Path $tauriDir 'Cargo.toml') -PathType Leaf)) {
    throw "Tauri source is missing from $tauriDir."
}
if (-not (Test-Path -LiteralPath $v2Config -PathType Leaf)) { throw "v2 Vite configuration is missing at $v2Config." }

function Test-DevPort {
    $client = [Net.Sockets.TcpClient]::new()
    try {
        $client.Connect('127.0.0.1', 1420)
        return $true
    } catch {
        return $false
    } finally {
        $client.Dispose()
    }
}

Write-Host "Project v2: $repoRoot"
Write-Host "Node: $($node.Source)"
Write-Host "Cargo: $($cargo.Source)"
if ($CheckOnly) {
    Write-Host 'Prerequisite check passed. No processes were started.'
    if (Test-DevPort) { Write-Warning 'Port 1420 is already in use; close that server before launching.' }
    return
}
if (Test-DevPort) {
    throw 'Port 1420 is already in use. Close the existing server before launching this app.'
}

$previousDataDir = [Environment]::GetEnvironmentVariable('UTOPIA_DATA_DIR', 'Process')
$vite = $null
try {
    if ($DataDir) {
        $requested = if ([IO.Path]::IsPathRooted($DataDir)) { $DataDir } else { Join-Path $repoRoot $DataDir }
        $env:UTOPIA_DATA_DIR = [IO.Path]::GetFullPath($requested)
        Write-Host "App data: $env:UTOPIA_DATA_DIR"
    }

    $logDir = Join-Path $repoRoot '.run-data\startup-logs'
    New-Item -ItemType Directory -Path $logDir -Force | Out-Null
    $runTag = '{0}-{1}' -f (Get-Date -Format 'yyyyMMdd-HHmmss'), ([Guid]::NewGuid().ToString('N').Substring(0, 8))
    $viteOut = Join-Path $logDir "vite-v2-$runTag.out.log"
    $viteErr = Join-Path $logDir "vite-v2-$runTag.err.log"
    $vite = Start-Process -FilePath $node.Source `
        -ArgumentList @('.\node_modules\vite\bin\vite.js', '--config', 'v2/vite.config.ts', '--host', '127.0.0.1', '--port', '1420', '--strictPort') `
        -WorkingDirectory $repoRoot -RedirectStandardOutput $viteOut -RedirectStandardError $viteErr `
        -WindowStyle Hidden -PassThru

    $ready = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {
        if ($vite.HasExited) { break }
        try {
            $response = Invoke-WebRequest -Uri 'http://127.0.0.1:1420/' -UseBasicParsing -TimeoutSec 1
            if ($response.StatusCode -eq 200 -and $response.Content -match 'Utopia Company v2') { $ready = $true; break }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    if (-not $ready) {
        $detail = if (Test-Path -LiteralPath $viteErr) { (Get-Content -LiteralPath $viteErr -Tail 20) -join [Environment]::NewLine } else { '' }
        throw "Vite did not become ready. Log: $viteErr`n$detail"
    }

    Write-Host 'Vite is ready at http://127.0.0.1:1420/'
    Write-Host "Vite logs: $viteOut and $viteErr"
    Write-Host 'Building and opening Utopia Company v2. Close the app window to stop this launcher.'
    Push-Location $tauriDir
    try {
        & $cargo.Source run --locked --bin ai-organization-os
        if ($LASTEXITCODE -ne 0) { throw "cargo run failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
} finally {
    if ($vite -and -not $vite.HasExited) {
        Stop-Process -Id $vite.Id -ErrorAction SilentlyContinue
        Write-Host 'Stopped the Vite process started by this launcher.'
    }
    if ($null -eq $previousDataDir) {
        Remove-Item Env:UTOPIA_DATA_DIR -ErrorAction SilentlyContinue
    } else {
        $env:UTOPIA_DATA_DIR = $previousDataDir
    }
}
