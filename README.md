# InfinityConnect для Windows

Десктоп-клиент VPN InfinityConnect для Windows на **Rust + Tauri 2** (фронтенд —
React + TypeScript). Самостоятельный проект; Android-версия
(`InfinityConnect-Android`) — только референс логики/форматов.

Два ядра: **Xray** (VLESS/Reality/XHTTP) и **Hysteria2** (QUIC) — как sidecar-процессы
`xray.exe` / `hysteria.exe`. Туннель — `wintun.dll`. Подписки — панель **Remnawave**,
стиль клиента — **Happ**.

## Стек

- **UI:** React + TypeScript + Vite (`src/`).
- **Backend:** Rust (`src-tauri/`) — вся логика, сеть, туннель, sidecar-процессы.
- **Мост:** фронт → Rust через `invoke()`, состояние/статистика обратно через
  Tauri events (`emit`/`listen`).

## Требования для сборки

- **Rust** (stable, `x86_64-pc-windows-msvc`) — [rustup.rs](https://rustup.rs).
- **Visual Studio Build Tools** (MSVC + Windows SDK).
- **Node.js** 18+ и npm.

## Разработка

```powershell
npm install                                              # зависимости фронта
powershell -ExecutionPolicy Bypass -File scripts/fetch-binaries.ps1  # xray.exe/wintun.dll/geo (не в git)
npm run tauri dev                                        # запуск дев-сборки (Vite + cargo)
```

> Приложение требует прав администратора (создание wintun-адаптера и правка
> маршрутов). При запуске без них — само перезапустится через UAC.

## Сборка релиза

```powershell
npm run tauri build         # NSIS + MSI установщики в src-tauri/target/release/bundle
```

> ⚠️ Для туннеля (wintun) и правки маршрутов нужны права администратора —
> решается манифестом элевации / helper-службой (Фаза 7).

## Статус

- **Фаза 0 — каркас.** ✅ Окно, трей, автозапуск, мост invoke/emit (`ping`).
- **Фаза 1 — аккаунт и подписки.** ✅ Логин, discovery, ключи, список серверов,
  HWID (MachineGuid), токены (DPAPI), офлайн-кэши.
- **Фаза 2 — MVP-туннель.** ✅ wintun + Xray JSON + sidecar xray.exe, connect/disconnect,
  статистика, admin-элевация.
- **Фаза 3 — Hysteria2 + RawXray.** ✅ hysteria.exe sidecar, выбор ядра, паритет протоколов.
- **Фаза 4 — UI-паритет.** ✅ Экраны Home (hero + аккордеон Happ) / Auth / Profile / Settings
  (хаб → Маршрутизация / Пинг / О приложении), фиолетовая тема, трей, автозапуск.
- **Фаза 5 — Пинг.** ✅ 4 метода (proxy GET/HEAD, TCP, ICMP) + режимы + таймаут, PingScreen,
  автопинг в Home, бейдж «⚡ Быстрейший».
- **Фаза 6 — Маршрутизация.** ✅ По сайтам (домены → Xray routing.rules) в RoutingScreen.
  По приложениям (WFP) — задел, реально на Фазе 7.
- **Далее — Фаза 7 (Офлайн-кэш + kill-switch + установщик):** WFP kill-switch и per-app,
  MSI/NSIS-установщик, полировка.

См. [ARCHITECTURE.md](ARCHITECTURE.md).
