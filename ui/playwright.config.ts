import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30000,
  expect: {
    timeout: 5000,
  },
  fullyParallel: false,
  retries: 0,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:4173',
    channel: 'chrome',
    viewport: { width: 1200, height: 800 },
    trace: 'off',
  },
  projects: [
    {
      name: 'chromium',
      use: {
        channel: 'chrome',
      },
    },
  ],
  webServer: {
    command: 'node ./node_modules/vite/bin/vite.js preview --port 4173 --host 127.0.0.1',
    port: 4173,
    reuseExistingServer: true,
    timeout: 15000,
  },
});
