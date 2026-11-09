# Stand for live testing of Wex update flow on 127.0.0.1 (Rule 23:10 compliant)
$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptDir
$workspaceRoot = Split-Path -Parent $repoRoot
$logsOutputDir = Join-Path $workspaceRoot "agy\wex-update-flow-logs"
$exePath = "C:\Users\Public\wex-toolchain\target-a3\debug\wex-app.exe"

if (-not (Test-Path $exePath)) {
    throw "wex-app.exe not found at $exePath"
}

New-Item -ItemType Directory -Force -Path $logsOutputDir | Out-Null
$testBaseDir = Join-Path $env:TEMP "wex-update-stand-$([Guid]::NewGuid().ToString('N').Substring(0,8))"
New-Item -ItemType Directory -Force -Path $testBaseDir | Out-Null

Write-Host "=========================================================="
Write-Host "       WEX LAUNCHER LIVE UPDATE FLOW TEST STAND           "
Write-Host "       Target URL: http://127.0.0.1:45678/latest.json     "
Write-Host "=========================================================="

$global:serverProc = $null

function Stop-Server() {
    if ($global:serverProc) {
        try {
            Stop-Process -Id $global:serverProc.Id -Force -ErrorAction SilentlyContinue
        } catch {}
        $global:serverProc = $null
    }
    # Ensure port 45678 is free
    Start-Sleep -Milliseconds 200
}

function Start-Server([string]$responseBody) {
    Stop-Server
    $manifestPath = Join-Path $testBaseDir "current_manifest.json"
    [System.IO.File]::WriteAllText($manifestPath, $responseBody, [System.Text.Encoding]::UTF8)

    $serverJs = Join-Path $testBaseDir "server.js"
    $serverCode = @"
const fs = require('fs');
const http = require('http');
const server = http.createServer((req, res) => {
    try {
        const data = fs.readFileSync(process.argv[2]);
        res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8' });
        res.end(data);
    } catch (e) {
        res.writeHead(500);
        res.end(e.message);
    }
});
server.listen(45678, '127.0.0.1');
"@
    [System.IO.File]::WriteAllText($serverJs, $serverCode, [System.Text.Encoding]::UTF8)

    $global:serverProc = Start-Process node -ArgumentList "`"$serverJs`"", "`"$manifestPath`"" -PassThru -WindowStyle Hidden
    Start-Sleep -Milliseconds 600
}

function Run-LauncherSession([string]$dataDir, [int]$waitSec = 6) {
    Get-Process wex-app -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 400

    $env:WEX_DATA_DIR = $dataDir
    $env:WEX_QA_BACKGROUND = "true"
    $env:WEX_UPDATE_URL = "http://127.0.0.1:45678/latest.json"
    $env:WEX_CURRENT_VERSION = "0.1.0"
    
    $proc = Start-Process -FilePath $exePath -PassThru
    Start-Sleep -Seconds $waitSec
    try {
        if (-not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    } catch {}
    Get-Process wex-app -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
}

try {
    # ------------------------------------------------------------------------
    # Scenario 1: Valid Signed Update -> Banner «Доступно обновление»
    # ------------------------------------------------------------------------
    Write-Host "`n>>> SCENARIO 1: Signed update (version 1.2.0 > 0.1.0) -> Banner appears"
    $s1Manifest = @'
{
  "version": "1.2.0",
  "notes": "Большое обновление графики: шейдеры Iris и поддержка текстур высокого разрешения",
  "pub_date": "2026-11-09T05:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "untrusted comment: signature from tauri secret key\nRWValidMinisignSignatureFromSecretKey1234567890abcdef==",
      "url": "http://127.0.0.1:45678/wex-setup.exe"
    }
  }
}
'@
    Start-Server $s1Manifest
    $s1DataDir = Join-Path $testBaseDir "scenario1-data"
    New-Item -ItemType Directory -Force -Path $s1DataDir | Out-Null
    
    Run-LauncherSession $s1DataDir 6
    
    $s1LogFile = Join-Path $s1DataDir "logs\launcher.log"
    if (Test-Path $s1LogFile) {
        $s1Log = Get-Content $s1LogFile -Raw -Encoding UTF8
        Write-Host "--- SCENARIO 1 LAUNCHER LOG ---"
        Write-Host $s1Log
        Copy-Item -Path $s1LogFile -Destination (Join-Path $logsOutputDir "scenario1_signed.log") -Force
        if ($s1Log -match "подписано.*Отображение баннера «Доступно обновление»") {
            Write-Host ">>> SCENARIO 1 RESULT: SUCCESS" -ForegroundColor Green
        } else {
            throw "Scenario 1 log did not contain expected banner confirmation"
        }
    } else {
        throw "Scenario 1 log file not found at $s1LogFile"
    }

    # ------------------------------------------------------------------------
    # Scenario 2: Unsigned Update -> Mode «только ссылка»
    # ------------------------------------------------------------------------
    Write-Host "`n>>> SCENARIO 2: Unsigned update (empty signature) -> Mode «только ссылка»"
    $s2Manifest = @'
{
  "version": "1.2.0",
  "notes": "Тестовый неподписанный релиз без закрытого ключа",
  "pub_date": "2026-11-09T05:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "",
      "url": "http://127.0.0.1:45678/wex-setup.exe"
    }
  }
}
'@
    Start-Server $s2Manifest
    $s2DataDir = Join-Path $testBaseDir "scenario2-data"
    New-Item -ItemType Directory -Force -Path $s2DataDir | Out-Null
    
    Run-LauncherSession $s2DataDir 6
    
    $s2LogFile = Join-Path $s2DataDir "logs\launcher.log"
    if (Test-Path $s2LogFile) {
        $s2Log = Get-Content $s2LogFile -Raw -Encoding UTF8
        Write-Host "--- SCENARIO 2 LAUNCHER LOG ---"
        Write-Host $s2Log
        Copy-Item -Path $s2LogFile -Destination (Join-Path $logsOutputDir "scenario2_unsigned.log") -Force
        if ($s2Log -match "без подписи -> режим «только ссылка»") {
            Write-Host ">>> SCENARIO 2 RESULT: SUCCESS" -ForegroundColor Green
        } else {
            throw "Scenario 2 log did not contain expected link-only mode confirmation"
        }
    } else {
        throw "Scenario 2 log file not found at $s2LogFile"
    }

    # ------------------------------------------------------------------------
    # Scenario 3: Corrupt Signature / Tampered File -> Clear Rejection
    # ------------------------------------------------------------------------
    Write-Host "`n>>> SCENARIO 3: Invalid signature -> Refusal with UPDATE_SIGNATURE_FAILED"
    $s3Manifest = @'
{
  "version": "1.2.0",
  "notes": "Релиз с поддельной или повреждённой подписью",
  "pub_date": "2026-11-09T05:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "corrupted_invalid_tampered_signature_payload",
      "url": "http://127.0.0.1:45678/wex-setup.exe"
    }
  }
}
'@
    Start-Server $s3Manifest
    $s3DataDir = Join-Path $testBaseDir "scenario3-data"
    New-Item -ItemType Directory -Force -Path $s3DataDir | Out-Null
    
    Run-LauncherSession $s3DataDir 6
    
    $s3LogFile = Join-Path $s3DataDir "logs\launcher.log"
    if (Test-Path $s3LogFile) {
        $s3Log = Get-Content $s3LogFile -Raw -Encoding UTF8
        Write-Host "--- SCENARIO 3 LAUNCHER LOG ---"
        Write-Host $s3Log
        Copy-Item -Path $s3LogFile -Destination (Join-Path $logsOutputDir "scenario3_invalid_sig.log") -Force
        if ($s3Log -match "UPDATE_SIGNATURE_FAILED" -and $s3Log -match "Цифровая подпись обновления не подтверждена") {
            Write-Host ">>> SCENARIO 3 RESULT: SUCCESS" -ForegroundColor Green
        } else {
            throw "Scenario 3 log did not contain expected signature refusal"
        }
    } else {
        throw "Scenario 3 log file not found at $s3LogFile"
    }

    # ------------------------------------------------------------------------
    # Scenario 4: Server Offline -> Silence, Zero Errors to Player
    # ------------------------------------------------------------------------
    Write-Host "`n>>> SCENARIO 4: Server offline (port closed) -> Silence, no errors to player"
    Stop-Server # Ensure port 45678 actively refuses connections
    Start-Sleep -Milliseconds 400
    
    $s4DataDir = Join-Path $testBaseDir "scenario4-data"
    New-Item -ItemType Directory -Force -Path $s4DataDir | Out-Null
    
    Run-LauncherSession $s4DataDir 6
    
    $s4LogFile = Join-Path $s4DataDir "logs\launcher.log"
    if (Test-Path $s4LogFile) {
        $s4Log = Get-Content $s4LogFile -Raw -Encoding UTF8
        Write-Host "--- SCENARIO 4 LAUNCHER LOG ---"
        Write-Host $s4Log
        Copy-Item -Path $s4LogFile -Destination (Join-Path $logsOutputDir "scenario4_server_offline.log") -Force
        if ($s4Log -match "UPDATE_NETWORK_ERROR" -and $s4Log -match "Тихий режим: ошибка игроку не отображается") {
            Write-Host ">>> SCENARIO 4 RESULT: SUCCESS" -ForegroundColor Green
        } else {
            throw "Scenario 4 log did not contain expected silent handling confirmation"
        }
    } else {
        throw "Scenario 4 log file not found at $s4LogFile"
    }

    Write-Host "`n=========================================================="
    Write-Host "   ALL 4 UPDATE FLOW SCENARIOS PASSED WITH FULL LOGS!     "
    Write-Host "=========================================================="
} finally {
    Stop-Server
    if (Test-Path $testBaseDir) {
        Remove-Item -Recurse -Force $testBaseDir -ErrorAction SilentlyContinue
    }
}
