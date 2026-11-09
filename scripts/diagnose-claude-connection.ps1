# Reproduces the app's Claude connection check (discovery.rs) outside the app and prints timings,
# exit codes and output shape. Account identifiers from `auth status` are never printed.
# Read-only: it runs `--version` and `auth status` only. `-TryLogin` additionally opens the same
# login console the app's "로그인 창 열기" button opens.
param(
    [string]$Path = (Join-Path $env:APPDATA 'npm\claude.cmd'),
    [int]$AppTimeoutSeconds = 3,
    [int]$MaxWaitSeconds = 60,
    [switch]$TryLogin
)
$ErrorActionPreference = 'Stop'

function Invoke-Probe([string]$File, [string[]]$Arguments, [string]$Label) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $File
    $info.Arguments = ($Arguments -join ' ')
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.CreateNoWindow = $true
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $process = [System.Diagnostics.Process]::Start($info)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $finished = $process.WaitForExit($MaxWaitSeconds * 1000)
    $watch.Stop()
    if (-not $finished) { try { $process.Kill() } catch {} }
    $seconds = [math]::Round($watch.Elapsed.TotalSeconds, 2)
    $result = [pscustomobject]@{
        Label = $Label
        Seconds = $seconds
        Finished = $finished
        ExitCode = $(if ($finished) { $process.ExitCode } else { $null })
        WithinAppTimeout = ($finished -and $seconds -le $AppTimeoutSeconds)
        Stdout = $(if ($finished) { $stdout.Result } else { '' })
        Stderr = $(if ($finished) { $stderr.Result } else { '' })
    }
    Write-Host ''
    Write-Host "[$Label] $File $($info.Arguments)"
    Write-Host ("  시간 {0}s · 종료 {1} · 종료 코드 {2} · 앱 제한({3}s) 안에 끝남: {4}" -f $result.Seconds, $result.Finished, $result.ExitCode, $AppTimeoutSeconds, $result.WithinAppTimeout)
    return $result
}

Write-Host '== Claude 연결 진단 (앱과 같은 명령) =='
Write-Host "Windows 사용자: $([Environment]::UserName)"
Write-Host "등록 경로: $Path (존재: $(Test-Path -LiteralPath $Path -PathType Leaf))"
foreach ($name in 'CLAUDE_CONFIG_DIR', 'ANTHROPIC_API_KEY', 'ANTHROPIC_AUTH_TOKEN', 'CLAUDE_CODE_USE_BEDROCK', 'CLAUDE_CODE_USE_VERTEX') {
    Write-Host ("환경 변수 {0}: {1}" -f $name, $(if ([Environment]::GetEnvironmentVariable($name)) { '설정됨(값 숨김)' } else { '없음' }))
}

# Same resolution as runtime_command(): a .cmd wrapper runs the package's native claude.exe directly.
$native = Join-Path (Split-Path -Parent $Path) 'node_modules\@anthropic-ai\claude-code\bin\claude.exe'
Write-Host "앱이 auth status에 쓰는 실행 파일: $native (존재: $(Test-Path -LiteralPath $native -PathType Leaf))"

$version = Invoke-Probe $Path @('--version') 'version (앱: 등록 경로로 실행, 제한 3초)'
Write-Host "  출력: $(($version.Stdout + ' ' + $version.Stderr).Trim())"

if (Test-Path -LiteralPath $native -PathType Leaf) {
    $auth = Invoke-Probe $native @('auth', 'status') 'auth status (앱: claude.exe 직접 실행, stderr 무시, 제한 3초)'
    $parsed = $null
    try { $parsed = $auth.Stdout | ConvertFrom-Json } catch {}
    if ($null -eq $parsed) {
        Write-Host '  stdout이 JSON이 아닙니다. 앞부분(계정 정보가 있으면 직접 가리고 공유하세요):'
        foreach ($line in ($auth.Stdout.Trim() -split "`n" | Select-Object -First 5)) { Write-Host "  $line" }
    } else {
        $keys = ($parsed.PSObject.Properties | ForEach-Object { $_.Name }) -join ', '
        Write-Host "  JSON 키: $keys"
        Write-Host "  loggedIn: $($parsed.loggedIn)"
        if ($parsed.PSObject.Properties.Name -contains 'authMethod') { Write-Host "  authMethod: $($parsed.authMethod)" }
    }
    if ($auth.Stderr.Trim()) { Write-Host "  stderr 앞부분: $((($auth.Stderr.Trim() -split "`n") | Select-Object -First 3) -join ' / ')" }
    Write-Host ''
    if (-not $auth.Finished -or -not $auth.WithinAppTimeout) {
        Write-Host "판정: auth status가 앱 제한 $AppTimeoutSeconds 초를 넘었습니다. 로그인 문제가 아니라 앱의 확인 시간 제한 문제입니다."
    } elseif ($null -eq $parsed) {
        Write-Host '판정: auth status 출력 형식이 앱이 기대하는 JSON(loggedIn)이 아닙니다.'
    } elseif ($parsed.loggedIn -eq $true -and $auth.ExitCode -eq 0) {
        Write-Host '판정: 이 실행에서는 앱 조건을 통과합니다. 앱에서 다시 검사해도 실패하면 앱 실행 환경 차이를 확인해야 합니다.'
    } elseif ($parsed.loggedIn -eq $true) {
        Write-Host "판정: loggedIn=true지만 종료 코드가 $($auth.ExitCode)라 앱이 실패로 봅니다."
    } else {
        Write-Host '판정: 이 Windows 사용자로는 로그인되지 않았습니다. -TryLogin으로 로그인 창을 열어 보세요.'
    }
}

if ($TryLogin) {
    $literal = $Path.Replace("'", "''")
    $script = "& '$literal' auth login; Write-Host ''; Write-Host '로그인을 마친 뒤 앱에서 이 경로로 다시 검사를 누르세요.'"
    Write-Host ''
    Write-Host '앱과 같은 로그인 콘솔을 새 창으로 엽니다.'
    Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoProfile', '-NoExit', '-Command', $script)
}
