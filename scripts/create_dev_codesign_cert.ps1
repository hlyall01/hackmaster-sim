param(
    [string]$GameName = "Hackmaster Sim"
)

$ErrorActionPreference = "Stop"

$password = $env:CODESIGN_PFX_PASSWORD
if ([string]::IsNullOrWhiteSpace($password)) {
    Write-Error "CODESIGN_PFX_PASSWORD is required."
    exit 1
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$outputDir = Join-Path $repoRoot "secrets\codesign"
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

$subject = "CN=$GameName Dev"
$cert = New-SelfSignedCertificate -Type CodeSigningCert -Subject $subject -CertStoreLocation "Cert:\CurrentUser\My"

$securePassword = ConvertTo-SecureString -String $password -AsPlainText -Force
$pfxPath = Join-Path $outputDir "mygame-dev.pfx"
Export-PfxCertificate -Cert $cert -FilePath $pfxPath -Password $securePassword | Out-Null

$cerPath = Join-Path $outputDir "mygame-dev.cer"
Export-Certificate -Cert $cert -FilePath $cerPath | Out-Null

$thumbprintPath = Join-Path $outputDir "thumbprint.txt"
Set-Content -LiteralPath $thumbprintPath -Value $cert.Thumbprint -Encoding ASCII

Write-Host "Subject: $($cert.Subject)"
Write-Host "Thumbprint: $($cert.Thumbprint)"
Write-Host "PFX: $pfxPath"
Write-Host "CER: $cerPath"
Write-Host "Thumbprint file: $thumbprintPath"
