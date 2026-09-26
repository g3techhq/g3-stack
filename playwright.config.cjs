const { defineConfig, devices } = require("@playwright/test");

const port = Number(process.env.PLAYWRIGHT_PORT || 8080);
const baseURL = process.env.PLAYWRIGHT_BASE_URL || `http://127.0.0.1:${port}`;

module.exports = defineConfig({
  testDir: "./tests/ui",
  outputDir: "./test-results/playwright",
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 2 : 0,
  // One worker: the tests share a single dev server and one database.
  workers: 1,
  preserveOutput: "failures-only",
  timeout: 60_000,
  expect: { timeout: 10_000 },
  reporter: [["list"], ["html", { outputFolder: "playwright-report", open: "never" }]],
  use: {
    baseURL,
    actionTimeout: 10_000,
    navigationTimeout: 30_000,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  // Set PLAYWRIGHT_BASE_URL to test against a server you started yourself
  // (a `dx serve` you are already watching, or a deployed environment).
  webServer: process.env.PLAYWRIGHT_BASE_URL
    ? undefined
    : {
        command: `dx serve --web --addr 127.0.0.1 --port ${port} --open false --interactive false`,
        url: baseURL,
        reuseExistingServer: !process.env.CI,
        // A cold Rust + WASM build is slow the first time.
        timeout: 300_000,
      },
  projects: [
    // Mobile first, because the stack is: g3-ui's shell is a phone layout
    // that widens into a desktop rail, not the other way around.
    { name: "mobile-chromium", use: { ...devices["Pixel 7"] } },
    {
      name: "desktop-chromium",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } },
    },
  ],
});
