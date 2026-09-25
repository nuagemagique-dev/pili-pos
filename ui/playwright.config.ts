import { defineConfig, devices } from '@playwright/test';

// PW_PORT lets two checkouts (e.g. git worktrees) run their suites at
// once: with reuseExistingServer, a shared port would silently test
// the other checkout's UI.
const port = Number(process.env.PW_PORT ?? 1420);

export default defineConfig({
  testDir: './e2e',
  timeout: 15_000,
  retries: 0,

  projects: [
    // Layer 2: Mock IPC tests against Vite dev server.
    {
      name: 'e2e',
      use: {
        baseURL: `http://localhost:${port}`,
        browserName: 'webkit',
        viewport: { width: 1920, height: 1080 },
        locale: 'fr-BE',
        hasTouch: true,
      },
      testMatch: /\.spec\.ts$/,
      testIgnore: /integration/,
    },
    // Layer 3: Real binary via CDP (CI only, see test.yml).
    {
      name: 'integration',
      use: {
        browserName: 'webkit',
        viewport: { width: 1920, height: 1080 },
        locale: 'fr-BE',
        hasTouch: true,
        // connectOverCDP reads this from env.
        ...(process.env.INTEGRATION_CDP_URL
          ? { connectOptions: { wsEndpoint: process.env.INTEGRATION_CDP_URL } }
          : {}),
      },
      testMatch: /integration\.spec\.ts$/,
    },
  ],

  // Vite dev server for the e2e project.
  webServer: {
    command: `npm run dev -- --port ${port}`,
    port,
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
