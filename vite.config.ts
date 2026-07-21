import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Порт фиксирован — на него смотрит tauri.conf.json (devUrl).
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Tauri ожидает фиксированный порт и падает, если он занят.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // Не следим за Rust-исходниками — их пересобирает cargo.
      ignored: ["**/src-tauri/**"],
    },
  },
}));
