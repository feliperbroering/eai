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
    $pathParts = $env:Path -split ';'
    if ($pathParts -contains $InstallDir) {
        return
    }

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) {
        [Environment]::SetEnvironmentVariable("Path", $InstallDir, "User")
    }
    elseif (($userPath -split ';') -notcontains $InstallDir) {
        $newUserPath = $userPath + ";" + $InstallDir
        [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    }

    Ok "Added $InstallDir to user PATH (open a new terminal)."
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
