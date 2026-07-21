# InfinityConnect-Windows — карта проекта

> Справочник по файлам, слоям и связям. **Читать перед тем, как разбираться в коде
> или писать новый.** Обновлять при добавлении/переносе файлов и изменении контрактов.

Windows-десктоп-клиент VPN на **Rust + Tauri 2** (фронт — React + TypeScript + Vite).
Самостоятельный проект; Android (`InfinityConnect-Android`) — только референс логики
и форматов (см. его `ARCHITECTURE.md` и извлечённый контракт). Два ядра: **Xray**
(VLESS/Reality/XHTTP) и **Hysteria2** (QUIC) — как sidecar-процессы. Туннель — `wintun`.

## Архитектура

```
Frontend (React/TS)  ──invoke()──►  Backend (Rust / src-tauri)
     ▲                                     │
     └──────── Tauri events ◄──────────────┘   (state.rs эмитит tunnel://state)
   (listen: состояние туннеля + статистика; зеркало Android VpnStateHolder)
```

**Принцип:** во фронте нет логики, кроме отображения. Вся сеть/туннель/ядра — в Rust.
Единый источник состояния для UI — Rust `state.rs` → Tauri events.

---

## Frontend — `src/`

| Файл | За что отвечает | Статус |
|---|---|---|
| `main.tsx` | Точка входа React. | ✅ Фаза 0 |
| `App.tsx` | Роутер всех экранов + восстановление сессии/статуса туннеля + подписки на `tunnel://state`/`tunnel://stats`. | ✅ Фаза 4 |
| `api/commands.ts` | Типы и вызовы всех Tauri-команд (invoke) + listen. Единственная точка общения с бэком. | ✅ |
| `state/appStore.ts` | Zustand-стор: route (7 маршрутов), ключи, серверы, выбор, туннель, статистика. | ✅ Фаза 4 |
| `screens/AuthScreen.tsx` | Вход: discovery + логин, фирменный стиль. | ✅ Фаза 4 |
| `screens/HomeScreen.tsx` | Hero (ConnectHero) + панель статистики + аккордеон Happ (KeyCard/ServerRow, бейдж «⚡ Быстрейший»). | ✅ Фаза 4 |
| `screens/ProfileScreen.tsx` | Аккаунт, подписка, разлогин. | ✅ Фаза 4 |
| `screens/SettingsScreens.tsx` | Хаб → Маршрутизация (Фаза 6) / Пинг (Фаза 5) / О приложении (версии ядер + автозапуск). | 🟡 Фаза 4 |
| `components/` | `ui.tsx` (GlassCard/StatusPill/EmojiBadge/Chip/Eyebrow), `ConnectHero.tsx` (SVG-свечение/кольцо/пульс), `Scaffold.tsx`. | ✅ Фаза 4 |
| `theme/colors.ts` | Точная палитра InfinityColors (перенос из Android) + градиенты + `pingColor()` (по качеству). | ✅ Фаза 4 |
| `util/format.ts` | Форматтеры байт/скорости. | ✅ |

---

## Backend — `src-tauri/src/`

| Файл | За что отвечает | Статус |
|---|---|---|
| `main.rs` | Бинарь; прячет консоль в релизе, зовёт `lib::run()`. | ✅ Фаза 0 |
| `lib.rs` | Сборка Tauri-приложения: плагины, трей, `.manage(ApiClient)`, `invoke_handler`. | ✅ |
| `commands.rs` | `#[tauri::command]` — мост: ping/discover/login/logout/is_authorized/user_info/keys/key_servers. | ✅ Фаза 1 |
| `state.rs` | Источник состояния туннеля → эмит `tunnel://state` (аналог VpnStateHolder). | ✅ Фаза 0 |
| `error.rs` | `AppError`/`AppResult` (аналог Android AppResult), сериализуется во фронт. | ✅ Фаза 1 |
| `api/` | reqwest-клиент: discovery→base_url, login/refresh (Bearer, авто-refresh 401), keys/config/user, тело подписки (HWID-заголовки), офлайн-фолбэк на кэш. `dto.rs` — все DTO. | ✅ Фаза 1 |
| `subscription/` | Парсер тела (JSON-конфиги панели/base64/URI) + `vless_uri`/`hysteria2_uri`/`uri`. RawXray для сложных, XHTTP extra без интерпретации. | ✅ Фаза 1 |
| `engine/` | Модель `EngineConfig` + `xray_config.rs` (Xray JSON, tun через wintun) + `hysteria2_config.rs` (Hy2 JSON, tun-режим + trafficStats) + `selector.rs` (выбор ядра). Юнит-тесты формы (4). | ✅ Фаза 3 |
| `connection.rs` | BuildConnection: ключ+индекс → EngineConfig (подписка → fallback /v1/config). | ✅ Фаза 2 |
| `store/` | Токены (DPAPI: `dpapi.rs`) + офлайн-кэши discovery/ключей/тел подписок на %APPDATA%. | ✅ Фаза 1 |
| `device.rs` | HWID (MachineGuid из реестра, UPPER) + метаданные ОС для заголовков. | ✅ Фаза 1 |
| `elevation.rs` | Проверка admin + само-relaunch через UAC (wintun/маршруты требуют admin). | ✅ Фаза 2 |
| `tunnel/` | Оркестратор connect/disconnect: старт ядра → ожидание wintun → маршруты ОС (`routes.rs`) → фоновый опрос статистики. kill-switch/смена сети — Фаза 7. | 🟡 Фаза 2 (MVP) |
| `sidecar/` | Trait `CoreProcess` + `XrayProcess` (stats через `xray api statsquery`) + `HysteriaProcess` (stats через HTTP `/traffic`). Запуск ядер как std::process. | ✅ Фаза 3 |
| `ping/` | 4 метода: `proxy.rs` (GET/HEAD через временный xray SOCKS-sidecar + режимы Default/Double/Keepalive), TCP (медиана), `icmp.rs` (IcmpSendEcho). `model.rs` — PingMethod/Mode/Settings. Замеры сериализованы. | ✅ Фаза 5 |
| `routing/` | Split-tunnel (WFP) + домены (Xray routing.rules). | ⬜ Фаза 6 |

### Конфиги
| Файл | Назначение |
|---|---|
| `Cargo.toml` | Зависимости: tauri, tauri-plugin-autostart, serde. |
| `tauri.conf.json` | Окно 420×720, трей, бандл NSIS+MSI (perMachine), ресурсы `binaries/*`. |
| `capabilities/default.json` | Разрешения окна: invoke/emit, window show/hide/focus, autostart. |
| `binaries/` | Sidecar: xray.exe, hysteria.exe, wintun.dll (bundled, не в git). |
| `icons/` | Иконки приложения/трея (фиолетовая «I»). |

---

## Контракт с Android (что переносится «даром» vs Windows-специфика)

**Даром (тот же JSON/логика):** vless-outbound + streamSettings + routing.rules,
парсинг подписки/URI, DTO/эндпоинты API + discovery + авторизация (Bearer, refresh
при 401), схема пинга (4 метода + Default/Double/Keepalive), проброс RawXray/автовыбора
целиком, XHTTP `extra` без интерпретации.

**Переписать под Windows:**
- **Inbound:** Android TUN-инбаунд форка xray-core (`protocol:"tun"`, fd через env) →
  на Windows **SOCKS-инбаунд + wintun→SOCKS** (tun2socks, как Hiddify/v2rayN).
  Единственная часть XrayConfigBuilder, не переносимая дословно.
- **Ядро:** libv2ray in-process → sidecar `xray.exe`/`hysteria.exe`.
- **HWID:** ANDROID_ID → MachineGuid (`HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid`), UPPER.
- **Заголовки подписки:** обязательны (иначе панель = заглушка); os=Windows, свой UA.
- **Токены:** Keystore → DPAPI. **Маршруты:** IP Helper API. **Смена сети:** события
  маршрута ОС. **kill-switch / split-tunnel:** WFP.

---

## Фазы (порядок работ)

- **Фаза 0 — Каркас Tauri.** ✅ Окно, трей, autostart, мост invoke/emit end-to-end (`ping`).
- **Фаза 1 — Аккаунт и подписки.** ✅ `api/` + `subscription/` + `engine/` (модель) + `store/` +
  `device.rs`. Логин, discovery, ключи, список серверов подписки, HWID, токены (DPAPI),
  офлайн-кэши. Экраны Auth/Home.
- **Фаза 2 — MVP-туннель.** ✅ `engine/xray_config` (Xray JSON, tun через wintun) + `sidecar/`
  xray.exe + `tunnel/` (маршруты ОС, статистика) + `connection.rs` + admin-элевация. connect/
  disconnect VLESS/RawXray, hero-кнопка, тикающая статистика. Ядро xray.exe 26.3.27 (не в git —
  `scripts/fetch-binaries.ps1`). Проверено: xray -test принимает генерируемый JSON.
- **Фаза 3 — Hysteria2 + RawXray.** ✅ hysteria.exe sidecar (tun-режим + trafficStats),
  `hysteria2_config` + `selector` (Vless/RawXray→Xray, Hy2→Hysteria), trait `CoreProcess`.
  RawXray-проброс готов с Фазы 2 (`build_raw`). Паритет по протоколам. Hy2 2.10.0.
  Проверено: hysteria -c принимает генерируемый JSON.
- **Фаза 4 — UI-паритет.** ✅ Home (hero + аккордеон Happ + бейдж «⚡ Быстрейший») / Auth /
  Profile / Settings (хаб). Точная палитра InfinityColors + градиенты. Трей (показать/отключить/
  выход, клик по иконке, сворачивание при закрытии). Автозапуск (команды + переключатель в About).
- **Фаза 5 — Пинг.** ✅ 4 метода (proxy GET/HEAD через временный xray SOCKS-sidecar + режимы
  Default/Double/Keepalive, TCP-медиана, ICMP через IP Helper) + таймаут. PingScreen (метод/режим/
  URL/таймаут). Автопинг в Home, пилл по качеству, бейдж «⚡ Быстрейший» по минимуму.
- **Фаза 6 — Маршрутизация.** По сайтам (routing.rules) + по приложениям (WFP).
- **Фаза 7 — Офлайн-кэш + kill-switch + установщик + элевация.**

---

## Инварианты (поведение как на Android)

- **Xray-конфиг — единственная «правда» о трафике.** Генерировать тот же JSON.
- **Подписка первична**, `/v1/config` — только fallback.
- **RawXray** — в ядро целиком (balancer/WHITE/автовыбор), не схлопывать.
- **XHTTP extra** — пробрасывать без интерпретации.
- **HWID** — стабильный, совместимый с сервером (лимит устройств считается по нему).
- **Тема фиолетовая**; пинг-пилл — по качеству. Список серверов раскрыт как в Happ.
- Единый источник состояния туннеля для UI (Rust `state.rs` → Tauri events).
