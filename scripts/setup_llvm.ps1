#!/usr/bin/env pwsh
# Setup script for LLVM 14.0.6 on Windows
# This script extracts LLVM and sets up the required environment variable for building PECOS

$ErrorActionPreference = "Stop"

# Get the repository root (parent of scripts directory)
$RepoRoot = Split-Path -Parent $PSScriptRoot
$LLVMDir = Join-Path $RepoRoot "llvm"
$LLVMArchive = Join-Path $RepoRoot "LLVM-14.0.6-win64.7z"
$LLVMConfigPath = Join-Path $LLVMDir "bin\llvm-config.exe"

Write-Host "PECOS LLVM 14.0.6 Setup for Windows" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host ""

# Check if LLVM is already extracted
if (Test-Path $LLVMConfigPath) {
    Write-Host "[OK] LLVM is already extracted in the repository" -ForegroundColor Green
} else {
    # Check if archive exists, if not download it
    if (-not (Test-Path $LLVMArchive)) {
        Write-Host "[INFO] LLVM archive not found, downloading..." -ForegroundColor Yellow
        $DownloadUrl = "https://github.com/PLC-lang/llvm-package-windows/releases/download/v14.0.6/LLVM-14.0.6-win64.7z"

        try {
            Write-Host "Downloading from: $DownloadUrl" -ForegroundColor Cyan
            Write-Host "This may take several minutes (~450MB download)..." -ForegroundColor Yellow

            # Use Invoke-WebRequest with progress
            $ProgressPreference = 'SilentlyContinue'  # Faster downloads
            Invoke-WebRequest -Uri $DownloadUrl -OutFile $LLVMArchive -UseBasicParsing
            $ProgressPreference = 'Continue'

            Write-Host "[OK] Download completed" -ForegroundColor Green
        } catch {
            Write-Host "[ERROR] Failed to download LLVM archive: $_" -ForegroundColor Red
            Write-Host "Please manually download from: $DownloadUrl" -ForegroundColor Yellow
            Write-Host "And place it at: $LLVMArchive" -ForegroundColor Yellow
            exit 1
        }
    } else {
        Write-Host "[OK] LLVM archive found" -ForegroundColor Green
    }

    Write-Host "[INFO] Extracting LLVM archive..." -ForegroundColor Yellow
    Write-Host "This may take a few minutes..."

    # Check if 7z is available
    $7zPath = Get-Command "7z" -ErrorAction SilentlyContinue
    if (-not $7zPath) {
        Write-Host "[ERROR] 7-Zip (7z command) not found in PATH" -ForegroundColor Red
        Write-Host "Please install 7-Zip from https://www.7-zip.org/" -ForegroundColor Yellow
        exit 1
    }

    # Create llvm directory if it doesn't exist
    if (-not (Test-Path $LLVMDir)) {
        New-Item -ItemType Directory -Path $LLVMDir | Out-Null
    }

    # Extract the archive to llvm directory
    Push-Location $RepoRoot
    try {
        & 7z x $LLVMArchive -o"$LLVMDir" -y | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "7z extraction failed with exit code $LASTEXITCODE"
        }
    } catch {
        Write-Host "[ERROR] Failed to extract LLVM archive: $_" -ForegroundColor Red
        Pop-Location
        exit 1
    }
    Pop-Location

    # Verify extraction
    if (Test-Path $LLVMConfigPath) {
        Write-Host "[OK] LLVM extracted successfully" -ForegroundColor Green

        # Clean up the archive to save disk space
        try {
            Remove-Item $LLVMArchive -Force
            Write-Host "[OK] Cleaned up archive file (saved ~450MB)" -ForegroundColor Green
        } catch {
            Write-Host "[WARNING] Failed to delete archive file: $_" -ForegroundColor Yellow
            Write-Host "You can manually delete: $LLVMArchive" -ForegroundColor Yellow
        }
    } else {
        Write-Host "[ERROR] LLVM extraction completed but llvm-config.exe not found" -ForegroundColor Red
        exit 1
    }
}

Write-Host ""
Write-Host "Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "LLVM has been extracted to: $LLVMDir" -ForegroundColor Cyan
Write-Host ""
Write-Host "The Makefile will automatically use this LLVM installation when building and testing." -ForegroundColor Green
Write-Host "No manual environment variable configuration is needed." -ForegroundColor Green
Write-Host ""
Write-Host "You can now run:" -ForegroundColor Cyan
Write-Host "  make dev" -ForegroundColor White
Write-Host ""
Write-Host "Note: This LLVM installation is local to this project and won't interfere" -ForegroundColor Yellow
Write-Host "      with other LLVM installations on your system." -ForegroundColor Yellow
