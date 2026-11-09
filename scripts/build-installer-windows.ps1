param([switch]$CheckOnly)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'This installer build is for Windows.' }
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$npm = Get-Command npm.cmd -ErrorAction Stop
$npx = Get-Command npx.cmd -ErrorAction Stop
$cargo = Get-Command cargo.exe -ErrorAction Stop
if (-not (Test-Path -LiteralPath (Join-Path $repoRoot 'src-tauri\tauri.installer.conf.json') -PathType Leaf)) { throw 'Tauri installer configuration is missing.' }
Write-Host "Project: $repoRoot"
Write-Host "npm: $($npm.Source)"
Write-Host "Cargo: $($cargo.Source)"
if ($CheckOnly) { Write-Host 'Installer build prerequisites found. No build started.'; return }
Push-Location $repoRoot
try {
    & $npx.Source tauri build --config src-tauri/tauri.installer.conf.json
    if ($LASTEXITCODE -ne 0) { throw "Installer build failed with exit code $LASTEXITCODE" }
    Write-Host 'Installer output (v4 app): src-tauri\target\release\bundle\nsis'
} finally {
    Pop-Location
}
