/**
 * Layer 3: the REAL moyun-pos binary (certification build), its real
 * SQLite database and IPC, no mock: WebKitGTK driven over W3C WebDriver
 * through tauri-driver, the way Tauri documents end-to-end testing on
 * Linux. Playwright cannot drive a WebKitGTK webview.
 *
 * Run by .github/workflows/test.yml (job `integration`), which builds
 * the binary (scripts/build_and_hash.sh), seeds the database
 * (e2e/fixtures/seed.sql) and starts Xvfb + tauri-driver first:
 *
 *   TAURI_DRIVER_URL=http://127.0.0.1:4444 \
 *   MOYUN_POS_BINARY=$PWD/../target/release/moyun-pos \
 *   npm run test:real-binary
 *
 * One session, one app launch: each step starts where the previous left.
 */
import { after, before, test } from "node:test";
import assert from "node:assert/strict";

const DRIVER = process.env.TAURI_DRIVER_URL;
const BINARY = process.env.MOYUN_POS_BINARY;
const ELEMENT = "element-6066-11e4-a23f-da3e2b66b0f7";
const skip = !DRIVER || !BINARY ? "needs TAURI_DRIVER_URL and MOYUN_POS_BINARY (CI only)" : false;

let session;

async function wd(method, path, body) {
  const res = await fetch(`${DRIVER}${path}`, {
    method,
    headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const json = await res.json();
  if (!res.ok) throw new Error(`${method} ${path}: ${JSON.stringify(json.value)}`);
  return json.value;
}

const cmd = (method, path, body) => wd(method, `/session/${session}${path}`, body);
const findAll = async (css) =>
  (await cmd("POST", "/elements", { using: "css selector", value: css })).map((e) => e[ELEMENT]);
const text = (id) => cmd("GET", `/element/${id}/text`);
const click = (id) => cmd("POST", `/element/${id}/click`, {});
const url = () => cmd("GET", "/url");

/** Polls `probe` until it returns a truthy value, or fails with `what`. */
async function waitFor(what, probe, timeoutMs = 30_000) {
  const end = Date.now() + timeoutMs;
  let last;
  while (Date.now() < end) {
    try {
      last = await probe();
      if (last) return last;
    } catch (e) {
      last = e;
    }
    await new Promise((r) => setTimeout(r, 250));
  }
  assert.fail(`timed out waiting for ${what} (last: ${last})`);
}

async function texts(css) {
  return Promise.all((await findAll(css)).map(text));
}

before(async () => {
  if (skip) return;
  const created = await wd("POST", "/session", {
    capabilities: { alwaysMatch: { "tauri:options": { application: BINARY } } },
  });
  session = created.sessionId;
});

after(async () => {
  if (session) await wd("DELETE", `/session/${session}`).catch(() => {});
});

test("the login screen lists the seeded operator", { skip }, async () => {
  const names = await waitFor("user tiles", async () => {
    const t = await texts(".user-name");
    return t.length > 0 && t;
  });
  assert.ok(names.includes("CI Server"), `tiles: ${names.join(", ")}`);
});

test("tapping the tile logs in and shows the 20 tables", { skip }, async () => {
  const tiles = await findAll(".user-btn");
  const names = await Promise.all(tiles.map(text));
  const tile = tiles[names.findIndex((n) => n.includes("CI Server"))];
  assert.ok(tile, "CI Server tile");
  await click(tile);
  await waitFor("the tables route", async () => /#\/tables/.test(await url()));
  await waitFor("20 table cards", async () => (await findAll(".table-card")).length === 20);
});

test("a table opens an order with the seeded menu", { skip }, async () => {
  const [first] = await findAll(".table-card");
  await click(first);
  await waitFor("the order route", async () => /#\/order/.test(await url()));
  // The first category (Sushi) holds two of the three seeded items.
  const items = await waitFor("the Sushi items", async () => {
    const t = await texts(".item-btn");
    return t.length === 2 && t;
  });
  assert.ok(items.some((t) => t.includes("Maki saumon x8")), `items: ${items.join(" | ")}`);
});

test("adding an item writes an order line and its total", { skip }, async () => {
  const buttons = await findAll(".item-btn");
  const labels = await Promise.all(buttons.map(text));
  await click(buttons[labels.findIndex((t) => t.includes("Maki saumon x8"))]);
  const lines = await waitFor("one order line", async () => {
    const t = await texts(".line-name");
    return t.length === 1 && t;
  });
  assert.equal(lines[0], "Maki saumon x8");
  // 12,00: the seeded price (1200 cents), totalled by the backend.
  await waitFor("the 12,00 total", async () => {
    const [total] = await texts(".total-amount");
    return total && total.includes("12,00");
  });
});
