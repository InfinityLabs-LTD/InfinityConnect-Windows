# Downloads sidecar binaries into src-tauri/binaries:
#   xray.exe, wintun.dll, geoip.dat, geosite.dat, hysteria.exe, sing-box.exe
# ASCII-only on purpose: pwsh 7 on CI runners mis-decodes non-BOM Cyrillic and
# fails to parse the script. Run: powershell -ExecutionPolicy Bypass -File scripts/fetch-binaries.ps1

$ErrorActionPreference = "Stop"

$XrayVersion = "26.3.27"       # = Android BuildFlags.XRAY_CORE_VERSION
$HysteriaVersion = "2.10.0"    # apernet/hysteria 2.x
$SingboxVersion = "1.13.14"    # SagerNet/sing-box - TUN + per-app routing
$BinDir = Join-Path $PSScriptRoot "..\src-tauri\binaries"
$BinDir = [System.IO.Path]::GetFullPath($BinDir)
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

# --- Xray (zip: xray.exe + wintun.dll + geo) ---
$Url = "https://github.com/XTLS/Xray-core/releases/download/v$XrayVersion/Xray-windows-64.zip"
$Zip = Join-Path $env:TEMP "xray-$XrayVersion.zip"
$Extract = Join-Path $env:TEMP "xray-$XrayVersion"

Write-Host "Downloading Xray $XrayVersion..."
Invoke-WebRequest -Uri $Url -OutFile $Zip

Write-Host "Extracting..."
if (Test-Path $Extract) { Remove-Item -Recurse -Force $Extract }
Expand-Archive -Path $Zip -DestinationPath $Extract

foreach ($f in @("xray.exe", "wintun.dll", "geoip.dat", "geosite.dat")) {
    Copy-Item -Path (Join-Path $Extract $f) -Destination (Join-Path $BinDir $f) -Force
    Write-Host "  - $f"
}
Remove-Item $Zip -Force
Remove-Item -Recurse -Force $Extract

# --- Hysteria2 (single exe) ---
$HyUrl = "https://github.com/apernet/hysteria/releases/download/app/v$HysteriaVersion/hysteria-windows-amd64.exe"
Write-Host "Downloading Hysteria $HysteriaVersion..."
Invoke-WebRequest -Uri $HyUrl -OutFile (Join-Path $BinDir "hysteria.exe")
Write-Host "  - hysteria.exe"

# --- sing-box (zip: sing-box.exe) - TUN + per-app split-tunnel ---
$SingboxUrl = "https://github.com/SagerNet/sing-box/releases/download/v$SingboxVersion/sing-box-$SingboxVersion-windows-amd64.zip"
$SbZip = Join-Path $env:TEMP "sing-box-$SingboxVersion.zip"
$SbExtract = Join-Path $env:TEMP "sing-box-$SingboxVersion"
Write-Host "Downloading sing-box $SingboxVersion..."
Invoke-WebRequest -Uri $SingboxUrl -OutFile $SbZip
Expand-Archive -Path $SbZip -DestinationPath $SbExtract -Force
$SbExe = Get-ChildItem -Path $SbExtract -Recurse -Filter "sing-box.exe" | Select-Object -First 1
Copy-Item $SbExe.FullName (Join-Path $BinDir "sing-box.exe") -Force
Write-Host "  - sing-box.exe"
Remove-Item -Recurse -Force $SbExtract

Write-Host "Done: $BinDir"
