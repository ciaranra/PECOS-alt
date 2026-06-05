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
$LlvmConfigReal = Join-Path $InstallDir "bin\llvm-config.real.exe"

function Find-SevenZip {
    foreach ($Name in @("7z.exe", "7zz.exe", "7za.exe")) {
        $Command = Get-Command $Name -ErrorAction SilentlyContinue
        if ($Command) {
            return $Command.Source
        }
    }

    $Candidates = @()
    if ($env:ProgramFiles) {
        $Candidates += Join-Path $env:ProgramFiles "7-Zip\7z.exe"
    }
    if (${env:ProgramFiles(x86)}) {
        $Candidates += Join-Path ${env:ProgramFiles(x86)} "7-Zip\7z.exe"
    }
    if ($env:ChocolateyInstall) {
        $Candidates += Join-Path $env:ChocolateyInstall "bin\7z.exe"
    }

    foreach ($Candidate in $Candidates) {
        if ($Candidate -and (Test-Path $Candidate)) {
            return $Candidate
        }
    }

    return $null
}

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

function Install-LlvmConfigWrapper {
    $WrapperSource = Join-Path $PSScriptRoot "llvm-config-wrapper.rs"
    if (-not (Test-Path $WrapperSource)) {
        throw "LLVM config wrapper source not found: $WrapperSource"
    }

    if (-not (Test-Path $LlvmConfigReal)) {
        if (-not (Test-Path $LlvmConfig)) {
            throw "llvm-config.exe not found at $LlvmConfig"
        }
        Move-Item -Force -Path $LlvmConfig -Destination $LlvmConfigReal
    }
    elseif (Test-Path $LlvmConfig) {
        Remove-Item -Force $LlvmConfig
    }

    $Rustc = Get-Command rustc.exe -ErrorAction SilentlyContinue
    if (-not $Rustc) {
        throw "rustc.exe is required to repair the LLVM Windows llvm-config system library output"
    }

    Write-Host "Installing PECOS llvm-config wrapper"
    & $Rustc.Source --edition=2021 -O -o $LlvmConfig $WrapperSource
    if ($LASTEXITCODE -ne 0) {
        throw "rustc failed to build llvm-config wrapper with exit code $LASTEXITCODE"
    }

    $SystemLibs = (& $LlvmConfig --system-libs --link-static).Trim()
    $Libxml2Static = Join-Path $InstallDir "lib\libxml2s.lib"
    if ((-not (Test-Path $Libxml2Static)) -and $SystemLibs -match "(^|\s)libxml2s\.lib($|\s)") {
        throw "LLVM config wrapper did not filter missing libxml2s.lib from --system-libs"
    }
}

if (Test-Path $LlvmConfig) {
    $FoundVersion = (& $LlvmConfig --version).Trim()
    if ($FoundVersion.StartsWith($RequiredVersion)) {
        Install-LlvmConfigWrapper
        Write-Host "LLVM $FoundVersion already installed at $InstallDir"
        exit 0
    }
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) "pecos-llvm-$([System.Guid]::NewGuid())"
$Archive = Join-Path $TempDir $Asset
$TarDir = Join-Path $TempDir "tar"
$ExtractDir = Join-Path $TempDir "extract"

New-Item -ItemType Directory -Force -Path $TempDir | Out-Null
New-Item -ItemType Directory -Force -Path $TarDir | Out-Null
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

    $SevenZip = Find-SevenZip
    if (-not $SevenZip) {
        throw "7-Zip is required to extract $Asset on Windows. Windows tar.exe can hang for hours on this archive in CI; install 7-Zip or provide an existing LLVM 21.1 install."
    }

    Write-Host "Extracting compressed LLVM archive with $SevenZip"
    & $SevenZip x -y -bb0 "-o$TarDir" $Archive
    if ($LASTEXITCODE -ne 0) {
        throw "7-Zip failed to decompress $Asset with exit code $LASTEXITCODE"
    }

    $TarArchive = Get-ChildItem -Path $TarDir -File -Filter "*.tar" | Select-Object -First 1
    if (-not $TarArchive) {
        throw "7-Zip did not produce a .tar payload from $Asset"
    }

    Write-Host "Extracting LLVM payload with $SevenZip"
    & $SevenZip x -y -bb0 "-o$ExtractDir" $TarArchive.FullName
    if ($LASTEXITCODE -ne 0) {
        throw "7-Zip failed to extract $($TarArchive.Name) with exit code $LASTEXITCODE"
    }

    $PayloadRoots = @(Get-ChildItem -Path $ExtractDir -Directory)
    if ($PayloadRoots.Count -ne 1) {
        throw "Expected one LLVM payload directory in $ExtractDir, found $($PayloadRoots.Count)"
    }
    $PayloadDir = $PayloadRoots[0].FullName

    if (Test-Path $InstallDir) {
        Remove-Item -Recurse -Force $InstallDir
    }
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $InstallDir) | Out-Null
    Move-Item -Path $PayloadDir -Destination $InstallDir

    & $LlvmConfig --version
    & $LlvmConfig --shared-mode
    Install-LlvmConfigWrapper
    Write-Host "Installed LLVM $Version to $InstallDir"
}
finally {
    if (Test-Path $TempDir) {
        Remove-Item -Recurse -Force $TempDir
    }
}
