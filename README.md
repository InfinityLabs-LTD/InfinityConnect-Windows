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
npm install                 # зависимости фронта
npm run tauri dev           # запуск дев-сборки (Vite + cargo)
```

## Сборка релиза

```powershell
npm run tauri build         # NSIS + MSI установщики в src-tauri/target/release/bundle
```

> ⚠️ Для туннеля (wintun) и правки маршрутов нужны права администратора —
> решается манифестом элевации / helper-службой (Фаза 7).

## Статус

- **Фаза 0 — каркас.** ✅ Окно, трей, автозапуск, мост invoke/emit (`ping`).
- **Фаза 1 — аккаунт и подписки.** ✅ Логин, discovery, ключи, список серверов
  подписки (стиль Happ), HWID (MachineGuid), токены (DPAPI), офлайн-кэши.
- **Далее — Фаза 2 (MVP-туннель):** wintun + engine (Xray JSON) + sidecar xray.exe.

См. [ARCHITECTURE.md](ARCHITECTURE.md).
