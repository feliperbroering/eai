#!/usr/bin/env pwsh
$ErrorActionPreference = "Stop"

$Repo = "feliperbroering/eai"
$CargoBin = Join-Path $HOME ".cargo\bin"

function Info([string]$Message) {
    Write-Host "▶ $Message" -ForegroundColor Cyan
}

function Ok([string]$Message) {
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Fail([string]$Message) {
    Write-Host "✗ $Message" -ForegroundColor Red
    exit 1
}

function Ensure-Cargo {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        return
    }

    Fail "cargo was not found. Install Rust first: https://rustup.rs"
}

function Ensure-CargoOnPath {
    $pathParts = $env:Path -split ';'
    if ($pathParts -contains $CargoBin) {
        return
    }

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ([string]::IsNullOrWhiteSpace($userPath)) {
        [Environment]::SetEnvironmentVariable("Path", $CargoBin, "User")
    } elseif (($userPath -split ';') -notcontains $CargoBin) {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$CargoBin", "User")
    }

    Ok "Added $CargoBin to user PATH (open a new terminal)."
}

Info "Installing eai for Windows..."
Ensure-Cargo

Info "Running cargo install..."
& cargo install --git "https://github.com/$Repo" --locked
if ($LASTEXITCODE -ne 0) {
    Fail "cargo install failed."
}

Ensure-CargoOnPath
Ok "Done! Run: eai setup"
