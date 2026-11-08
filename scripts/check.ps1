param()

$ErrorActionPreference = 'Continue'
$repoRoot = Split-Path -Parent $PSScriptRoot
$coreRoot = Join-Path $repoRoot 'core'
$uiRoot = Join-Path $repoRoot 'ui'
$results = [System.Collections.Generic.List[object]]::new()

try {
    (Get-Process -Id $PID).PriorityClass = [System.Diagnostics.ProcessPriorityClass]::BelowNormal
} catch {
    Write-Warning "Could not lower check script priority: $($_.Exception.Message)"
}

$env:CARGO_BUILD_JOBS = '2'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:npm_config_maxsockets = '2'

function Invoke-Check {
    param(
        [Parameter(Mandatory)] [string] $Name,
        [Parameter(Mandatory)] [string] $WorkingDirectory,
        [Parameter(Mandatory)] [string] $Executable,
        [Parameter()] [string[]] $Arguments = @()
    )

    Write-Host "`n==> $Name"
    $exitCode = 1
    Push-Location -LiteralPath $WorkingDirectory
    try {
        if (-not (Get-Command $Executable -ErrorAction SilentlyContinue)) {
            throw "Required command '$Executable' was not found on PATH."
        }
        $global:LASTEXITCODE = 1
        & $Executable @Arguments
        if ($null -ne $LASTEXITCODE) {
            $exitCode = $LASTEXITCODE
        }
    } catch {
        Write-Error "${Name}: $($_.Exception.Message)"
    } finally {
        Pop-Location
    }

    $passed = $exitCode -eq 0
    $results.Add([pscustomobject]@{ Name = $Name; Passed = $passed; ExitCode = $exitCode })
    if ($passed) {
        Write-Host "PASS $Name" -ForegroundColor Green
    } else {
        Write-Host "FAIL $Name (exit $exitCode)" -ForegroundColor Red
    }
}

# Root and core are separate Cargo workspaces, so each gets the same Rust gates.
Invoke-Check 'core: cargo fmt --check' $coreRoot 'cargo' @('fmt', '--all', '--', '--check')
Invoke-Check 'app: cargo fmt --check' $repoRoot 'cargo' @('fmt', '--all', '--', '--check')
Invoke-Check 'core: cargo clippy' $coreRoot 'cargo' @('clippy', '--workspace', '-j', '2', '--', '-D', 'warnings')
Invoke-Check 'app: cargo clippy' $repoRoot 'cargo' @('clippy', '--workspace', '-j', '2', '--', '-D', 'warnings')
Invoke-Check 'core: cargo test' $coreRoot 'cargo' @('test', '--workspace', '-j', '2')
Invoke-Check 'app: cargo test' $repoRoot 'cargo' @('test', '--workspace', '-j', '2')

if (-not (Test-Path -LiteralPath (Join-Path $uiRoot 'package-lock.json'))) {
    Write-Host 'FAIL ui: npm ci (package-lock.json not found)' -ForegroundColor Red
    $results.Add([pscustomobject]@{ Name = 'ui: npm ci'; Passed = $false; ExitCode = 1 })
} else {
    Invoke-Check 'ui: npm ci' $repoRoot 'npm' @('--prefix', 'ui', '--workspaces=false', 'ci', '--foreground-scripts')
    Invoke-Check 'ui: npm test' $repoRoot 'npm' @('--prefix', 'ui', '--workspaces=false', 'test', '--', '--maxWorkers=2')
    Invoke-Check 'ui: npm run check' $repoRoot 'npm' @('--prefix', 'ui', '--workspaces=false', 'run', 'check')
}

Write-Host "`n================ Check summary ================"
foreach ($result in $results) {
    $label = if ($result.Passed) { 'PASS' } else { 'FAIL' }
    $color = if ($result.Passed) { 'Green' } else { 'Red' }
    Write-Host "$label $($result.Name)" -ForegroundColor $color
}

$failedCount = @($results | Where-Object { -not $_.Passed }).Count
if ($failedCount -eq 0) {
    Write-Host "PASS: all $($results.Count) checks passed" -ForegroundColor Green
    exit 0
}

Write-Host "FAIL: $failedCount of $($results.Count) checks failed" -ForegroundColor Red
exit 1
