param(
    [string]$InstallDir = (Join-Path $env:USERPROFILE ".pecos\deps\llvm-21.1"),
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

if (-not $Version) {
    if ($env:LLVM_RELEASE_VERSION) {
        $Version = $env:LLVM_RELEASE_VERSION
    }
    else {
        $Version = "21.1.8"
    }
}

$ExpectedSha256 = "749d22f565fcd5718dbed06512572d0e5353b502c03fe1f7f17ee8b8aca21a47"
$RequiredVersion = "21.1"
$Asset = "clang+llvm-$Version-x86_64-pc-windows-msvc.tar.xz"
$Url = "https://github.com/llvm/llvm-project/releases/download/llvmorg-$Version/$Asset"
$LlvmConfig = Join-Path $InstallDir "bin\llvm-config.exe"

function Get-Sha256Hex {
    param([string]$Path)

    $Stream = [System.IO.File]::OpenRead($Path)
    try {
        $Sha256 = [System.Security.Cryptography.SHA256]::Create()
        try {
            return -join ($Sha256.ComputeHash($Stream) | ForEach-Object { $_.ToString("x2") })
        }
        finally {
            $Sha256.Dispose()
        }
    }
    finally {
        $Stream.Dispose()
    }
}

if (Test-Path $LlvmConfig) {
    $FoundVersion = (& $LlvmConfig --version).Trim()
    if ($FoundVersion.StartsWith($RequiredVersion)) {
        Write-Host "LLVM $FoundVersion already installed at $InstallDir"
        exit 0
    }
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) "pecos-llvm-$([System.Guid]::NewGuid())"
$Archive = Join-Path $TempDir $Asset
$ExtractDir = Join-Path $TempDir "extract"

New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
New-Item -ItemType Directory -Force -Path $ExtractDir | Out-Null

try {
    Write-Host "Downloading official LLVM $Version Windows development archive: $Asset"
    $Curl = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($Curl) {
        & $Curl.Source --fail --location --retry 5 --retry-delay 5 --output $Archive $Url
    }
    else {
        Invoke-WebRequest -Uri $Url -OutFile $Archive
    }

    $ActualSha256 = Get-Sha256Hex $Archive
    if ($ActualSha256 -ne $ExpectedSha256) {
        throw "SHA256 mismatch for $Asset. Expected $ExpectedSha256, got $ActualSha256"
    }

    tar -xf $Archive -C $ExtractDir --strip-components=1

    if (Test-Path $InstallDir) {
        Remove-Item -Recurse -Force $InstallDir
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $InstallDir) | Out-Null
    Move-Item $ExtractDir $InstallDir

    & $LlvmConfig --version
    & $LlvmConfig --shared-mode
    Write-Host "Installed LLVM $Version to $InstallDir"
}
finally {
    if (Test-Path $TempDir) {
        Remove-Item -Recurse -Force $TempDir
    }
}
