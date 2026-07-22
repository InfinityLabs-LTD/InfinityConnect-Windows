/** Готовые наборы приложений для split-tunnel. Имена exe работают по process_name
 *  независимо от установки — если приложение появится позже, оно сразу попадёт под
 *  правило. Установленные приложения дополняются вспомогательными exe в рантайме. */

export interface AppPreset {
  id: string;
  name: string;
  icon: string;
  hint: string;
  /** Имена exe (process_name). Регистр не важен. */
  exes: string[];
}

export const APP_PRESETS: AppPreset[] = [
  {
    id: "ru-blocked",
    name: "Недоступное в РФ",
    icon: "🚫",
    hint: "Discord, ChatGPT, Claude, соцсети и AI-инструменты",
    exes: [
      // Discord (основной + обновлятор + хелперы)
      "Discord.exe", "Update.exe", "DiscordCanary.exe", "DiscordPTB.exe",
      // Spotify
      "Spotify.exe",
      // ChatGPT (desktop)
      "ChatGPT.exe",
      // Claude (desktop + code CLI + VS Code расширение)
      "claude.exe", "Claude.exe", "cowork-svc.exe",
      // OpenAI Codex
      "Codex.exe", "codex.exe",
      // VS Code (в нём живёт расширение Claude Code)
      "Code.exe",
      // Android Studio + Gemini-плагин работает внутри студии и её JBR
      "studio64.exe", "adb.exe", "fsnotifier.exe",
      // Соцсети/мессенджеры, часто заблокированные
      "Telegram.exe", "Instagram.exe",
    ],
  },
  {
    id: "browsers",
    name: "Браузеры",
    icon: "🌐",
    hint: "Chrome, Edge, Firefox, Opera, Brave",
    exes: [
      "chrome.exe", "msedge.exe", "firefox.exe", "opera.exe", "brave.exe",
      "vivaldi.exe", "yandex.exe",
    ],
  },
  {
    id: "ai-tools",
    name: "AI-инструменты",
    icon: "🤖",
    hint: "ChatGPT, Claude, Codex, Copilot, Gemini",
    exes: [
      "ChatGPT.exe", "claude.exe", "Claude.exe", "cowork-svc.exe",
      "Codex.exe", "codex.exe", "Code.exe", "studio64.exe",
    ],
  },
  {
    id: "gaming",
    name: "Игры и лаунчеры",
    icon: "🎮",
    hint: "Steam, Epic, Battle.net, Riot и др.",
    exes: [
      "steam.exe", "steamwebhelper.exe", "EpicGamesLauncher.exe",
      "Battle.net.exe", "RiotClientServices.exe", "GalaxyClient.exe",
    ],
  },
  {
    id: "messengers",
    name: "Мессенджеры",
    icon: "💬",
    hint: "Discord, Telegram, WhatsApp, Signal",
    exes: [
      "Discord.exe", "Update.exe", "Telegram.exe", "WhatsApp.exe",
      "Signal.exe", "Slack.exe",
    ],
  },
];
