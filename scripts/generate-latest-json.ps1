param(
    [string]$Version,
    [string]$ArtifactsDir = "target/release/bundle",
    [string]$Notes = "Wex Launcher update",
    [string]$BaseUrl = "https://github.com/qutlawsoasis-debug/wex-launcher/releases/download",
    [string]$OutputFile = "latest.json"
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

# If version is not provided, read from app/src-tauri/tauri.conf.json
if (-not $Version) {
    $tauriConfPath = Join-Path $repoRoot "app/src-tauri/tauri.conf.json"
    if (Test-Path $tauriConfPath) {
        $tauriConf = Get-Content $tauriConfPath -Raw | ConvertFrom-Json
        $Version = $tauriConf.version
    }
}

if (-not $Version) {
    $Version = "0.9.0"
}

Write-Host "Generating latest.json for version $Version..."

# Resolve artifacts directory
$resolvedArtifactsDir = if ([System.IO.Path]::IsPathRooted($ArtifactsDir)) {
    $ArtifactsDir
} else {
    Join-Path $repoRoot $ArtifactsDir
}

# Look for installer and signature files
$nsisDir = Join-Path $resolvedArtifactsDir "nsis"
$msiDir = Join-Path $resolvedArtifactsDir "msi"

$installerFile = $null
$sigContent = ""
$installerName = ""

# Check for NSIS zip or exe (Tauri updater targets .nsis.zip or .exe)
if (Test-Path $nsisDir) {
    $nsisZip = Get-ChildItem -Path $nsisDir -Filter "*.nsis.zip" -File | Select-Object -First 1
    $nsisExe = Get-ChildItem -Path $nsisDir -Filter "*setup.exe" -File | Select-Object -First 1
    
    if ($nsisZip) {
        $installerFile = $nsisZip
    } elseif ($nsisExe) {
        $installerFile = $nsisExe
    }
}

if (-not $installerFile -and (Test-Path $msiDir)) {
    $msiFile = Get-ChildItem -Path $msiDir -Filter "*.msi" -File | Select-Object -First 1
    if ($msiFile) {
        $installerFile = $msiFile
    }
}

if ($installerFile) {
    $installerName = $installerFile.Name
    Write-Host "Found installer artifact: $($installerFile.FullName)"
    
    # Check for corresponding .sig file
    $sigFile = "$($installerFile.FullName).sig"
    if (Test-Path $sigFile) {
        $sigContent = (Get-Content -Path $sigFile -Raw).Trim()
        Write-Host "Found signature file: $sigFile"
    } else {
        Write-Warning "Signature file not found for $($installerFile.Name). Signature will be empty."
    }

    # Compute SHA256 hash
    $sha256 = (Get-FileHash -Path $installerFile.FullName -Algorithm SHA256).Hash
    Write-Host "Installer SHA256: $sha256"
} else {
    Write-Warning "No installer binary found in $resolvedArtifactsDir. Creating template latest.json."
    $installerName = "Wex_${Version}_x64-setup.nsis.zip"
}

$url = "$BaseUrl/v$Version/$installerName"
$pubDate = [DateTime]::UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ")

$latestObj = [ordered]@{
    version   = $Version
    notes     = $Notes
    pub_date  = $pubDate
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = $sigContent
            url       = $url
        }
    }
}

$jsonContent = $latestObj | ConvertTo-Json -Depth 5

$resolvedOutputFile = if ([System.IO.Path]::IsPathRooted($OutputFile)) {
    $OutputFile
} else {
    Join-Path $repoRoot $OutputFile
}

$outDir = Split-Path -Parent $resolvedOutputFile
if ($outDir -and -not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir -Force | Out-Null
}

$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($resolvedOutputFile, $jsonContent, $utf8NoBom)

Write-Host "Successfully generated $resolvedOutputFile"
Write-Host $jsonContent
