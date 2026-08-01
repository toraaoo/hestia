import { fileURLToPath, URL } from 'node:url';

import { paraglideVitePlugin } from '@inlang/paraglide-js';
import tailwindcss from '@tailwindcss/vite';
import { devtools } from '@tanstack/devtools-vite';
import { tanstackRouter } from '@tanstack/router-plugin/vite';

import viteReact from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

// Tauri drives the frontend: it opens a webview at `server` in dev and bundles
// the static `build.outDir` (dist) in release. There is no Node server at
// runtime, so this is a plain client SPA.
const host = process.env.TAURI_DEV_HOST;

const config = defineConfig({
  plugins: [
    devtools(),
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/paraglide',
      strategy: ['localStorage', 'baseLocale'],
    }),
    tanstackRouter({ target: 'react', autoCodeSplitting: true }),
    viteReact(),
    tailwindcss(),
  ],

  build: {
    rolldownOptions: {
      output: {
        advancedChunks: {
          groups: [
            {
              name: 'three-core',
              test: /[\\/]node_modules[\\/]three[\\/]build[\\/]three\.core\.js/,
            },
            {
              name: 'three',
              test: /[\\/]node_modules[\\/]three[\\/]build[\\/]three\.module\.js/,
            },
          ],
        },
      },
    },
  },

  // `@/*` and `#/*` map to `src/*` (mirrors tsconfig paths + components.json).
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '#': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },

  test: {
    environment: 'jsdom',
    setupFiles: ['./tests/setup.ts'],
    restoreMocks: true,
  },

  // Tauri expects a fixed port (tauri.conf.json `devUrl`) and its own console.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
  },
});

export default config;
