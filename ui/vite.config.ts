/// <reference types="vitest/config" />
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
  build: {
    // skinview3d (three.js, ~525 kB) is a lazy chunk fetched only by the wardrobe;
    // it cannot be split further, so the warning limit sits just above it.
    chunkSizeWarningLimit: 600,
  },
  resolve: {
    conditions: ['browser'],
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/*.{test,spec}.{js,ts}'],
  },
})
