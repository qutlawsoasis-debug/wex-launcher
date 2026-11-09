import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30000,
  expect: {
    timeout: 7000,
  },
  fullyParallel: false,
  retries: process.env.CI ? 1 : 0, // 1 retry only for CI under heavy load
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: 'http://127.0.0.1:4173',
    channel: 'chrome',
    viewport: { width: 1200, height: 800 },
    trace: 'off',
    reducedMotion: 'reduce', // Apple HIG: emulate reduced motion to eliminate animation flakiness
    ignoreHTTPSErrors: true,
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
    reuseExistingServer: !process.env.CI,
    timeout: 60000,
  },
});
