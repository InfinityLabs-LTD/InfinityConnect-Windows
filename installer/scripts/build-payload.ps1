# Собирает payload для установщика Infinity Connect из выхлопа основного
# приложения. Payload — это ровно те файлы, что попадают в папку установки:
#   infinity-connect.exe + binaries\* + resources\*
#
# Запускать ПОСЛЕ сборки основного app (`tauri build`) из корня репозитория.
# Использование: powershell -File installer/scripts/build-payload.ps1

$ErrorActionPreference = "Stop"

$repo = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$release = Join-Path $repo "src-tauri\target\release"
$mainExe = Join-Path $release "infinity-connect.exe"
$binaries = Join-Path $repo "src-tauri\binaries"
$resources = Join-Path $release "resources"

$payload = Join-Path $repo "installer\payload"

Write-Host "Repo:     $repo"
Write-Host "Main exe: $mainExe"

if (-not (Test-Path $mainExe)) {
    throw "Не найден основной бинарник: $mainExe. Сначала соберите приложение (tauri build)."
}

# Чистим и создаём payload.
if (Test-Path $payload) { Remove-Item $payload -Recurse -Force }
New-Item -ItemType Directory -Force -Path $payload | Out-Null

# 1. Главный бинарник.
Copy-Item $mainExe (Join-Path $payload "infinity-connect.exe") -Force
Write-Host "  + infinity-connect.exe"

# 2. binaries\* (ядра, wintun, geo) — как в bundle.resources основного конфига.
New-Item -ItemType Directory -Force -Path (Join-Path $payload "binaries") | Out-Null
foreach ($f in @("xray.exe", "hysteria.exe", "sing-box.exe", "wintun.dll", "geoip.dat", "geosite.dat")) {
    $src = Join-Path $binaries $f
    if (Test-Path $src) {
        Copy-Item $src (Join-Path $payload "binaries\$f") -Force
        Write-Host "  + binaries\$f"
    } else {
        Write-Warning "нет $src (пропущен)"
    }
}

# 3. resources\* (иконка и прочее из bundle).
if (Test-Path $resources) {
    Copy-Item $resources (Join-Path $payload "resources") -Recurse -Force
    Write-Host "  + resources\ (скопирована)"
}

$size = [math]::Round((Get-ChildItem $payload -Recurse -File | Measure-Object Length -Sum).Sum / 1MB, 1)
Write-Host "Payload готов: $payload ($size MB)"

# Пакуем payload в ZIP для встраивания в single-file установщик (include_bytes!).
# build.rs подхватит installer/payload.zip и вошьёт в infinity-setup.exe.
$zip = Join-Path $repo "installer\payload.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Write-Host "Упаковка payload.zip…"
# Содержимое payload/* в корень архива (без папки payload сверху).
Compress-Archive -Path (Join-Path $payload "*") -DestinationPath $zip -CompressionLevel Optimal
$zsize = [math]::Round((Get-Item $zip).Length / 1MB, 1)
Write-Host "payload.zip готов: $zip ($zsize MB)"
