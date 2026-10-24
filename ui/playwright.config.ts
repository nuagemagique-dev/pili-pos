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
      testIgnore: [/real-binary/, /captures/],
    },
    // Layer 3 (the real binary) is not Playwright: WebKitGTK is driven
    // over WebDriver by tauri-driver, see e2e/real-binary/.
    // Certification dossier screenshot tour (docs/SUBMISSION_ARTIFACTS).
    // Not part of the normal gate - run explicitly with `npm run captures`.
    // Playwright runs every configured project when no --project filter
    // is given, so this project only exists in the config when PW_CAPTURES
    // is set (the `captures` npm script sets it); otherwise a bare
    // `playwright test` never discovers captures.spec.ts. Viewport
    // matches the kiosk's 15.6" FHD touchscreen. Longer timeout than the
    // default 15s: several captures walk a full sale (login, tables,
    // order, checkout) in one test, screenshotting along the way.
    ...(process.env.PW_CAPTURES
      ? [
          {
            name: 'captures',
            timeout: 60_000,
            use: {
              baseURL: `http://localhost:${port}`,
              browserName: 'webkit' as const,
              viewport: { width: 1920, height: 1080 },
              locale: 'fr-BE',
              hasTouch: true,
            },
            testMatch: /captures(-features)?\.spec\.ts$/,
          },
        ]
      : []),
  ],

  // Vite dev server for the e2e project.
  webServer: {
    command: `npm run dev -- --port ${port}`,
    port,
    reuseExistingServer: true,
    timeout: 30_000,
  },
});
