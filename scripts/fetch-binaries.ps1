# Скачивает sidecar-бинарники в src-tauri/binaries: xray.exe, wintun.dll,
# geoip.dat, geosite.dat. Версия Xray синхронизирована с Android BuildFlags.
# Запуск:  powershell -ExecutionPolicy Bypass -File scripts/fetch-binaries.ps1

$ErrorActionPreference = "Stop"

$XrayVersion = "26.3.27"  # = Android BuildFlags.XRAY_CORE_VERSION
$BinDir = Join-Path $PSScriptRoot "..\src-tauri\binaries"
$BinDir = [System.IO.Path]::GetFullPath($BinDir)
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

$Url = "https://github.com/XTLS/Xray-core/releases/download/v$XrayVersion/Xray-windows-64.zip"
$Zip = Join-Path $env:TEMP "xray-$XrayVersion.zip"
$Extract = Join-Path $env:TEMP "xray-$XrayVersion"

Write-Host "Скачиваю Xray $XrayVersion..."
Invoke-WebRequest -Uri $Url -OutFile $Zip

Write-Host "Распаковка..."
if (Test-Path $Extract) { Remove-Item -Recurse -Force $Extract }
Expand-Archive -Path $Zip -DestinationPath $Extract

foreach ($f in @("xray.exe", "wintun.dll", "geoip.dat", "geosite.dat")) {
    Copy-Item -Path (Join-Path $Extract $f) -Destination (Join-Path $BinDir $f) -Force
    Write-Host "  → $f"
}

Remove-Item $Zip -Force
Remove-Item -Recurse -Force $Extract
Write-Host "Готово: $BinDir"
