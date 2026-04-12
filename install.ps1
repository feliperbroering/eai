#!/usr/bin/env pwsh
$ErrorActionPreference = "Stop"

$Repo = "feliperbroering/eai"
$AssetPattern = '^eai-windows-amd64(\.exe)?$'
$InstallDir = if ($env:EAI_INSTALL_DIR) { $env:EAI_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "eai\bin" }
$BinaryPath = Join-Path $InstallDir "eai.exe"

function Info([string]$Message) {
    Write-Host "[INFO] $Message" -ForegroundColor Cyan
}

function Ok([string]$Message) {
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Fail([string]$Message) {
    Write-Host "[ERROR] $Message" -ForegroundColor Red
    exit 1
}

function Ensure-InstallDir {
    if (-not (Test-Path -LiteralPath $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }
}

function Add-ToPath {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if (-not [string]::IsNullOrWhiteSpace($userPath)) {
        $parts = $userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }

    # Keep only one entry for the install dir and force it to the front,
    # so older installs in other folders don't shadow the latest binary.
    $filtered = $parts | Where-Object { $_ -ne $InstallDir }
    $newUserPath = @($InstallDir) + $filtered
    [Environment]::SetEnvironmentVariable("Path", ($newUserPath -join ';'), "User")

    # Also update current session PATH so `eai` works immediately.
    $sessionParts = $env:Path -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) -and $_ -ne $InstallDir }
    $env:Path = (@($InstallDir) + $sessionParts) -join ';'

    $resolved = Get-Command eai -ErrorAction SilentlyContinue
    if ($resolved -and $resolved.Source -ne $BinaryPath) {
        Write-Host "[WARN] eai resolves to $($resolved.Source). Reopen terminal if needed." -ForegroundColor Yellow
    }

    Ok "Set $InstallDir as first entry in user PATH."
}

Info "Installing eai for Windows..."
Info "Fetching latest release metadata..."

$release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "eai-install-script" }
$asset = $release.assets | Where-Object { $_.name -match $AssetPattern } | Select-Object -First 1
if ($null -eq $asset) {
    Fail "Could not find Windows release asset matching '$AssetPattern'."
}

Info "Version: $($release.tag_name)"
Info "Downloading $($asset.name)..."
Ensure-InstallDir
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $BinaryPath

if (-not (Test-Path -LiteralPath $BinaryPath)) {
    Fail "Download failed: $BinaryPath was not created."
}

Add-ToPath
Ok "Installed eai to $BinaryPath"
Ok "Done! Run: eai setup"
