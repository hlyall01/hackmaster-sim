param(
    [Parameter(Mandatory = $true)]
    [string]$File,
    [string]$PfxPath
)

$ErrorActionPreference = "Stop"

function Find-SignTool {
    if ($env:SIGNTOOL_PATH -and (Test-Path -LiteralPath $env:SIGNTOOL_PATH)) {
        return $env:SIGNTOOL_PATH
    }

    $pf86 = [Environment]::GetFolderPath('ProgramFilesX86')
    $candidates = @()
    foreach ($suffix in @(
        "Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe",
        "Windows Kits\10\bin\10.0.22000.0\x64\signtool.exe",
        "Windows Kits\10\bin\10.0.19041.0\x64\signtool.exe",
        "Windows Kits\10\bin\x64\signtool.exe",
        "Windows Kits\10\bin\x86\signtool.exe",
        "Microsoft SDKs\Windows\v10.0A\bin\NETFX 4.8 Tools\signtool.exe",
        "Microsoft SDKs\ClickOnce\SignTool\signtool.exe"
    )) {
        if ([string]::IsNullOrWhiteSpace($pf86)) {
            break
        }
        $candidates += (Join-Path -Path $pf86 -ChildPath $suffix)
    }

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate) {
            return $candidate
        }
    }

    $kitsRoot = if ([string]::IsNullOrWhiteSpace($pf86)) { $null } else { Join-Path -Path $pf86 -ChildPath "Windows Kits\10\bin" }
    if (Test-Path -LiteralPath $kitsRoot) {
        $versions = Get-ChildItem -LiteralPath $kitsRoot -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
            Sort-Object Name -Descending
        foreach ($version in $versions) {
            $candidate = Join-Path $version.FullName "x64\signtool.exe"
            if (Test-Path -LiteralPath $candidate) {
                return $candidate
            }
        }
    }

    return $null
}

function Resolve-LocalPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [string]$Label,
        [switch]$CopyBack
    )

    $resolved = (Resolve-Path -LiteralPath $Path).ProviderPath
    if ($resolved.StartsWith("\\\\")) {
        $stagingRoot = Join-Path -Path $env:TEMP -ChildPath "hackmaster_sim_sign"
        New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null
        $fileName = Split-Path -Path $resolved -Leaf
        $stageName = if ($Label) { "$Label-$fileName" } else { $fileName }
        $localPath = Join-Path -Path $stagingRoot -ChildPath $stageName
        Copy-Item -LiteralPath $resolved -Destination $localPath -Force
        return @{
            Original = $resolved
            Local = $localPath
            CopyBack = $CopyBack.IsPresent
        }
    }

    return @{
        Original = $resolved
        Local = $resolved
        CopyBack = $false
    }
}

if (-not $PfxPath) {
    $repoRoot = Split-Path -Parent $PSScriptRoot
    $PfxPath = Join-Path $repoRoot "secrets\codesign\mygame-dev.pfx"
}

if (-not (Test-Path -LiteralPath $PfxPath)) {
    Write-Host "signing skipped: PFX not found at $PfxPath"
    exit 0
}

$password = $env:CODESIGN_PFX_PASSWORD
if ([string]::IsNullOrWhiteSpace($password)) {
    Write-Host "signing skipped: CODESIGN_PFX_PASSWORD not set"
    exit 0
}

if (-not (Test-Path -LiteralPath $File)) {
    Write-Error "File not found: $File"
    exit 1
}

$signTool = Find-SignTool
if (-not $signTool) {
    Write-Error "signtool.exe not found. Install the Windows 10/11 SDK or Visual Studio Build Tools with the Windows SDK and Signing Tools components."
    exit 1
}

$fileInfo = Resolve-LocalPath -Path $File -Label "exe" -CopyBack
$pfxInfo = Resolve-LocalPath -Path $PfxPath -Label "pfx"
$filePath = $fileInfo.Local
$pfxPathResolved = $pfxInfo.Local

Write-Host "Signing: $filePath"
& $signTool sign /fd SHA256 /f $pfxPathResolved /p $password /tr http://timestamp.digicert.com /td SHA256 $filePath
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

& $signTool verify /pa /v $filePath
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if ($fileInfo.CopyBack -and ($fileInfo.Original -ne $fileInfo.Local)) {
    Copy-Item -LiteralPath $fileInfo.Local -Destination $fileInfo.Original -Force
}
