# Push the current branch and open its pull request (rules: sdd/pr-workflow.md).
# PR text: .pr/<branch with / replaced by ->.md, first line = title, the rest = body.
# Usage: .\scripts\open-pr.ps1 [-Base main]
param([string]$Base = 'main')
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

$branch = (git rev-parse --abbrev-ref HEAD).Trim()
if ($branch -eq $Base -or $branch -eq 'HEAD') { throw "Switch to a work branch first (current: $branch)." }
if (git status --porcelain --untracked-files=no) { throw 'Commit tracked changes before opening a PR.' }

$file = Join-Path '.pr' (($branch -replace '/', '-') + '.md')
if (-not (Test-Path -LiteralPath $file)) { throw "PR text not found: $file" }
$lines = @(Get-Content -LiteralPath $file -Encoding UTF8)
$title = $lines[0].Trim()
if (-not $title) { throw "The first line of $file must be the PR title." }
$body = ((@($lines | Select-Object -Skip 1)) -join "`n").Trim()

Write-Host "Branch : $branch -> $Base"
Write-Host "Title  : $title"
git push -u origin $branch
if ($LASTEXITCODE -ne 0) { throw 'git push failed. See the message above.' }

$repo = ((git remote get-url origin).Trim() -replace '^git@github\.com:', 'https://github.com/') -replace '\.git$', ''
if (Get-Command gh -ErrorAction SilentlyContinue) {
  $tmp = [IO.Path]::GetTempFileName()
  [IO.File]::WriteAllText($tmp, $body, (New-Object Text.UTF8Encoding $false))
  try { gh pr create --base $Base --head $branch --title $title --body-file $tmp }
  finally { Remove-Item -LiteralPath $tmp -ErrorAction SilentlyContinue }
  if ($LASTEXITCODE -eq 0) { return }
  Write-Host 'gh could not create the PR; opening the web form instead.'
}
# Web fallback: title in the URL, body on the clipboard (bodies are too long for a URL).
Set-Clipboard -Value $body
$url = "$repo/compare/$Base...$($branch)?expand=1&title=$([Uri]::EscapeDataString($title))"
Start-Process $url
Write-Host 'Opened the PR form. The body is on the clipboard: paste it (Ctrl+V) into the description, then press Create pull request.'
