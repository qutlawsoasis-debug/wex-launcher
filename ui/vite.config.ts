/// <reference types="vitest" />
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'
import { readFileSync } from 'node:fs'

const tauriConfig = JSON.parse(readFileSync(new URL('../app/src-tauri/tauri.conf.json', import.meta.url), 'utf8'))

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  preview: {
    headers: { 'Content-Security-Policy': tauriConfig.app.security.csp },
  },
  resolve: {
    conditions: ['browser'],
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    rollupOptions: {
      external: ['@tauri-apps/plugin-notification'],
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/*.{test,spec}.{js,ts}'],
  },
})
