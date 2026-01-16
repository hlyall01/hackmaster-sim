param(
    [string]$InnoPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot

function Test-IsWslPath {
    param([string]$Path)
    return $Path -match '^\\\\wsl\.localhost\\' -or $Path -match '^\\\\wsl\\'
}

function Get-WslDistroFromPath {
    param([string]$Path)
    if ($Path -match '^\\\\wsl\.localhost\\([^\\]+)\\') {
        return $Matches[1]
    }
    if ($Path -match '^\\\\wsl\\([^\\]+)\\') {
        return $Matches[1]
    }
    return $null
}

function Get-WslPath {
    param(
        [string]$WindowsPath,
        [string]$Distro
    )

    if ($WindowsPath -match '^\\\\wsl\.localhost\\[^\\]+\\(.+)$') {
        return ("/" + ($Matches[1] -replace '\\', '/'))
    }
    if ($WindowsPath -match '^\\\\wsl\\[^\\]+\\(.+)$') {
        return ("/" + ($Matches[1] -replace '\\', '/'))
    }

    $args = @()
    if ($Distro) {
        $args += @("-d", $Distro)
    }
    $args += @("wslpath", "-u", $WindowsPath)
    $wslPath = (& wsl.exe @args) 2>$null
    if (-not $wslPath) {
        return $null
    }
    return $wslPath.Trim()
}

function Invoke-WslCargoBuild {
    param(
        [string]$RepoRootPath,
        [string]$WinTarget,
        [string]$Distro
    )

    $wslRepoRoot = Get-WslPath -WindowsPath $RepoRootPath -Distro $Distro
    if (-not $wslRepoRoot) {
        throw "Failed to resolve WSL path for repo root: $RepoRootPath"
    }

    $cmd = "cd '$wslRepoRoot' && cargo build --release --target $WinTarget"
    $args = @()
    if ($Distro) {
        $args += @("-d", $Distro)
    }
    $args += @("--", "bash", "-lc", $cmd)
    & wsl.exe @args
    if ($LASTEXITCODE -ne 0) {
        throw "WSL cargo build failed."
    }
}

function Find-SignTool {
    if ($env:HACKMASTER_SIGNTOOL -and (Test-Path -LiteralPath $env:HACKMASTER_SIGNTOOL)) {
        return $env:HACKMASTER_SIGNTOOL
    }

    $pf86 = [Environment]::GetFolderPath('ProgramFilesX86')
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
        $candidate = Join-Path -Path $pf86 -ChildPath $suffix
        if (Test-Path -LiteralPath $candidate) {
            return $candidate
        }
    }

    return $null
}

Push-Location $RepoRoot
try {
    $winTarget = $env:WIN_TARGET
    if (-not $winTarget) {
        $winTarget = $env:CARGO_BUILD_TARGET
    }

    $useWslBuild = Test-IsWslPath -Path $RepoRoot
    if ($useWslBuild) {
        if (-not $winTarget) {
            $winTarget = "x86_64-pc-windows-gnu"
        }
        $distro = Get-WslDistroFromPath -Path $RepoRoot
        Invoke-WslCargoBuild -RepoRootPath $RepoRoot -WinTarget $winTarget -Distro $distro
    } else {
        $cargoArgs = @("build", "--release", "--bins")
        if ($winTarget) {
            $cargoArgs += @("--target", $winTarget)
        }
        & cargo @cargoArgs
    }

    if (-not (Test-Path $InnoPath)) {
        throw "Inno Setup not found at: $InnoPath"
    }

    $issPath = Join-Path $PSScriptRoot "hackmaster_sim.iss"
    if ($winTarget) {
        & $InnoPath "/DBuildTarget=$winTarget" $issPath
    } else {
        & $InnoPath $issPath
    }

    $thumbprint = $env:HACKMASTER_SIGN_THUMBPRINT
    if (-not $thumbprint) {
        $thumbprintFile = Join-Path $RepoRoot "secrets\codesign\thumbprint.txt"
        if (Test-Path -LiteralPath $thumbprintFile) {
            $thumbprint = (Get-Content -LiteralPath $thumbprintFile -ErrorAction SilentlyContinue | Select-Object -First 1).Trim()
        }
    }
    if ($thumbprint) {
        $signtool = Find-SignTool
        if (-not $signtool) {
            throw "signtool.exe not found. Set HACKMASTER_SIGNTOOL or install the Windows SDK/Build Tools with Signing Tools."
        }
        $binDir = if ($winTarget) {
            Join-Path -Path $RepoRoot -ChildPath ("target\{0}\release" -f $winTarget)
        } else {
            Join-Path -Path $RepoRoot -ChildPath "target\release"
        }
        $files = @(
            (Join-Path -Path $binDir -ChildPath "sim_gui.exe"),
            (Join-Path -Path $binDir -ChildPath "autobattler.exe"),
            (Join-Path -Path $binDir -ChildPath "sim_cli.exe"),
            (Join-Path -Path $binDir -ChildPath "hackmaster_sim.exe"),
            (Join-Path -Path $RepoRoot -ChildPath "installer\dist\HackmasterSimSetup.exe")
        )
        foreach ($file in $files) {
            if (Test-Path $file) {
                & $signtool sign /fd SHA256 /a /sha1 $thumbprint /tr http://timestamp.digicert.com /td SHA256 $file
            }
        }
    }
} finally {
    Pop-Location
}
