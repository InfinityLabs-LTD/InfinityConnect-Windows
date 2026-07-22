# Code-signs a single binary with the "Infinity Labs" self-signed certificate.
# Called by Tauri's bundler (signCommand) once per binary; %1 -> -Path.
#
# Certificate source, in order:
#   1. WINDOWS_CERT_PFX_BASE64 + WINDOWS_CERT_PASSWORD env (CI) -> imported to a temp store.
#   2. A pfx already imported into CurrentUser\My matching the subject.
# If no certificate is available (e.g. a contributor building locally without
# secrets), signing is SKIPPED so the build still succeeds.
#
# NOTE: this is a SELF-SIGNED cert. It sets the publisher name to "Infinity Labs"
# but does NOT establish chain trust, so Windows SmartScreen will still warn
# until a real OV/EV certificate is used. See UPDATER.md / INSTALLER.md.

param(
    [Parameter(Mandatory = $true)][string]$Path
)

$ErrorActionPreference = "Stop"

function Find-SignTool {
    $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    $roots = @(
        "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
        "${env:ProgramFiles}\Windows Kits\10\bin"
    )
    foreach ($r in $roots) {
        if (Test-Path $r) {
            $st = Get-ChildItem -Path $r -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
                  Where-Object { $_.FullName -match "x64" } |
                  Sort-Object FullName -Descending | Select-Object -First 1
            if ($st) { return $st.FullName }
        }
    }
    return $null
}

$b64 = $env:WINDOWS_CERT_PFX_BASE64
$pass = $env:WINDOWS_CERT_PASSWORD

if ([string]::IsNullOrWhiteSpace($b64)) {
    Write-Host "sign.ps1: no WINDOWS_CERT_PFX_BASE64 set - skipping signing of $Path"
    exit 0
}

$signtool = Find-SignTool
if (-not $signtool) {
    Write-Host "sign.ps1: signtool.exe not found - skipping signing of $Path"
    exit 0
}

# Materialize the pfx into a temp file.
$pfxPath = Join-Path $env:TEMP ("infinity-codesign-" + [guid]::NewGuid().ToString("N") + ".pfx")
[System.IO.File]::WriteAllBytes($pfxPath, [System.Convert]::FromBase64String($b64))

try {
    $tsUrls = @(
        "http://timestamp.digicert.com",
        "http://timestamp.sectigo.com",
        "http://time.certum.pl"
    )

    $signed = $false
    foreach ($ts in $tsUrls) {
        & $signtool sign `
            /f $pfxPath /p $pass `
            /fd SHA256 /td SHA256 /tr $ts `
            /d "Infinity Connect" `
            "$Path"
        if ($LASTEXITCODE -eq 0) { $signed = $true; break }
        Write-Host "sign.ps1: timestamp $ts failed, trying next..."
    }

    if (-not $signed) {
        # Last resort: sign without a timestamp (still sets publisher name).
        & $signtool sign /f $pfxPath /p $pass /fd SHA256 /d "Infinity Connect" "$Path"
        if ($LASTEXITCODE -ne 0) { throw "signtool failed for $Path (exit $LASTEXITCODE)" }
    }

    Write-Host "sign.ps1: signed $Path"
}
finally {
    Remove-Item $pfxPath -Force -ErrorAction SilentlyContinue
}
