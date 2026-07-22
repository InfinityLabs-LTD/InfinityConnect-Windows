import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Установщик Infinity Setup — отдельный фронт (frameless-окно).
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1430, strictPort: true },
  build: { target: "chrome105", minify: "esbuild" },
});
