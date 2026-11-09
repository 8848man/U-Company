# The default app is v4 (ADR-0069). This entry stays for existing instructions and forwards to it.
param(
    [string]$DataDir = '',
    [switch]$CheckOnly
)
$ErrorActionPreference = 'Stop'
$launcher = Join-Path (Split-Path -Parent $PSScriptRoot) 'v4\start-windows.ps1'
& $launcher -DataDir $DataDir -CheckOnly:$CheckOnly
