param(
    [string]$DataDir = '.run-data\meeting-qa',
    [switch]$CheckOnly
)

# QA-38 launcher: starts v2 with an isolated data directory and the debug-only
# offline meeting fixture (labelled "not AI"). The fixture transmits nothing; real AI
# connections stay blocked by the closed-text gate. See qa/MANUAL-QA.md QA-38.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$launcher = Join-Path $PSScriptRoot 'start-windows.ps1'
if (-not (Test-Path -LiteralPath $launcher -PathType Leaf)) { throw "v2 launcher is missing at $launcher." }

$previousFixture = [Environment]::GetEnvironmentVariable('UTOPIA_MEETING_FIXTURE', 'Process')
try {
    $env:UTOPIA_MEETING_FIXTURE = '1'
    Write-Host 'Meeting QA: offline fixture enabled for this run only (debug build, not AI).'
    if ($CheckOnly) { & $launcher -DataDir $DataDir -CheckOnly } else { & $launcher -DataDir $DataDir }
} finally {
    if ($null -eq $previousFixture) {
        Remove-Item Env:UTOPIA_MEETING_FIXTURE -ErrorAction SilentlyContinue
    } else {
        $env:UTOPIA_MEETING_FIXTURE = $previousFixture
    }
}
