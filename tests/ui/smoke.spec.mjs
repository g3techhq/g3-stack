import { expect, test } from "@playwright/test";

/**
 * One pass through the whole stack.
 *
 * If these pass, the WASM client booted and hydrated cleanly, the session cookie
 * round-tripped, the server functions reached SurrealDB, and the router
 * navigated in both directions. That is most of what a browser test can usefully
 * tell you, and it fails loudly on any of the wiring the template exists to
 * provide. Everything else belongs in a Rust unit test.
 *
 * Requires a database — `just db-up` first. Playwright starts `dx serve` itself
 * unless PLAYWRIGHT_BASE_URL points at a server you already have.
 */

const TITLE = `Playwright note ${Date.now()}`;

/**
 * Fails the test on any hydration decode error.
 *
 * A server-rendered tree that differs from the client's does not crash: the
 * client logs "Error deserializing data" and carries on with components reading
 * each other's state. Nothing visible breaks at first, so only a check like
 * this notices.
 */
function watchHydration(page) {
  const errors = [];
  page.on("console", (message) => {
    if (message.text().includes("Error deserializing data")) errors.push(message.text());
  });
  return () => expect(errors, "hydration decode errors in the console").toEqual([]);
}

/** Guest sign-in, from a cold load. */
async function signIn(page) {
  await page.goto("/");

  // A signed-out load lands on the splash at `/`, which asks the server once
  // and routes onward — so the sign-in button is what to wait for, not a URL.
  const guest = page.getByRole("button", { name: /continue as guest/i });
  await expect(guest).toBeVisible();
  await guest.click();

  await expect(page.getByRole("heading", { name: "Notes", exact: true })).toBeVisible();
}

test("a guest can sign in and work through a note's whole life", async ({ page }) => {
  const expectCleanHydration = watchHydration(page);
  await signIn(page);

  // --- Create -------------------------------------------------------------
  await page.getByRole("button", { name: "New note" }).click();
  await expect(page.getByRole("heading", { name: "New note" })).toBeVisible();

  await page.getByLabel("Title").fill(TITLE);
  await page.getByLabel("Body").fill("Written by the smoke test.");
  await page.getByRole("button", { name: "Save" }).click();

  const row = page.getByText(TITLE);
  await expect(row).toBeVisible();

  // --- Read ---------------------------------------------------------------
  await row.click();
  await expect(page.getByText("Written by the smoke test.")).toBeVisible();

  // --- Update -------------------------------------------------------------
  await page.getByRole("button", { name: "Edit" }).click();
  await page.getByLabel("Body").fill("Edited by the smoke test.");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("Edited by the smoke test.")).toBeVisible();

  // --- Delete -------------------------------------------------------------
  await page.getByRole("button", { name: "Delete note" }).click();
  await page.getByRole("button", { name: "Delete", exact: true }).click();

  await expect(page.getByRole("heading", { name: "Notes", exact: true })).toBeVisible();
  await expect(page.getByText(TITLE)).toHaveCount(0);

  // A reload is a fresh server render and a fresh hydration, signed in.
  await page.reload();
  await expect(page.getByRole("heading", { name: "Notes", exact: true })).toBeVisible();
  expectCleanHydration();
});

test("signing in replaces the sign-in screen in history", async ({ page }) => {
  await page.goto("/");
  const guest = page.getByRole("button", { name: /continue as guest/i });
  await expect(guest).toBeVisible();
  const lengthOnSignIn = await page.evaluate(() => history.length);

  await guest.click();
  await expect(page.getByRole("heading", { name: "Notes", exact: true })).toBeVisible();

  // `Notes` declares `handoff_from = (Splash, SignIn)`. Had it pushed instead, Back
  // from the app would land on a sign-in screen for an account already signed in.
  expect(await page.evaluate(() => history.length)).toBe(lengthOnSignIn);
});

test("appearance settings apply without losing the session", async ({ page }) => {
  await signIn(page);

  await page.getByRole("button", { name: "Settings" }).click();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();

  // A g3-ui SegmentButton is a `radio` in a radiogroup, which is also how a
  // screen reader announces it.
  //
  // Switching mode re-themes the tree in place rather than remounting it, so the
  // tab bar has to survive the switch.
  const style = page.getByRole("radiogroup", { name: "Style" });
  const theme = page.getByRole("radiogroup", { name: "Theme" });

  // A new account follows the device until it picks otherwise.
  await expect(style.getByRole("radio", { name: "Auto" })).toBeChecked();
  await expect(theme.getByRole("radio", { name: "Auto" })).toBeChecked();

  await style.getByRole("radio", { name: "Material" }).click();
  await expect(page.getByRole("button", { name: "Notes" })).toBeVisible();

  await theme.getByRole("radio", { name: "Dark" }).click();

  // A regression guard, not a formality: a stored preference applied late used
  // to overwrite whatever the user had just picked — a toggle that flips itself
  // back. Waiting past that window and re-asserting is what catches its return.
  await page.waitForTimeout(1500);
  await expect(style.getByRole("radio", { name: "Material" })).toBeChecked();
  await expect(theme.getByRole("radio", { name: "Dark" })).toBeChecked();

  // Saved to the account and rendered by the server, so a reload comes back
  // signed in and still dark rather than at sign-in in the default theme.
  await page.reload();
  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(theme.getByRole("radio", { name: "Dark" })).toBeChecked();
});

test("a note can be pinned from its page", async ({ page }) => {
  await signIn(page);
  const title = `Pinned note ${Date.now()}`;
  await page.getByRole("button", { name: "New note" }).click();
  await page.getByLabel("Title").fill(title);
  await page.getByRole("button", { name: "Save" }).click();

  await page.getByText(title).click();
  await page.getByRole("button", { name: "Pin", exact: true }).click();
  await expect(page.getByRole("button", { name: "Unpin" })).toBeVisible();

  // The Pinned filter shows it, from the cache at once and after the refetch.
  await page.getByRole("button", { name: "Back" }).click();
  await page.getByRole("radio", { name: "Pinned" }).click();
  await expect(page.getByText(title)).toBeVisible();
});

test("a guest can delete their account", async ({ page }) => {
  await signIn(page);

  await page.getByRole("button", { name: "Settings" }).click();
  await page.getByRole("button", { name: "Delete account" }).click();
  await page.getByRole("button", { name: "Delete account" }).last().click();

  // The session is gone with the account, so the splash sends them to sign-in.
  await expect(page.getByRole("button", { name: /continue as guest/i })).toBeVisible();
  await page.goto("/notes");
  await expect(page.getByRole("button", { name: /continue as guest/i })).toBeVisible();
});
