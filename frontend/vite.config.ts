import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri drives this build; the dev server must run on a fixed port so the shell
// can point its webview at it.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
