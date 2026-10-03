/**
 * Layer 3, online orders: the REAL pili-pos binary (certification
 * build, HubRise switched on in pos.json) against pos-hubrise-mock,
 * pos-fdm-mock and pos-printer-mock, driven over WebDriver
 * (tauri-driver + WebKitWebDriver), offline.
 *
 * One session, one app launch, each step starting where the previous
 * left: connect HubRise through the login dialog, take over the menu,
 * a prepaid platform order (accepted and signed N, cancelled by the
 * platform, refunded by the manager), an unpaid webshop order (P at
 * acceptance, cash N at handover), then a catalog change reaching the
 * menu.
 *
 * Run by scripts/e2e_hubrise.sh (Linux container), which starts the
 * three mocks, Xvfb and tauri-driver and seeds the database
 * (e2e/fixtures/seed_hubrise.sql). Skipped elsewhere, so the CI
 * integration job's glob leaves it out:
 *
 *   TAURI_DRIVER_URL=http://127.0.0.1:4444 PILI_POS_BINARY=... \
 *   HUBRISE_MOCK_URL=http://127.0.0.1:3001 PRINTER_JOBS_URL=http://127.0.0.1:9180 \
 *   PILI_POS_DB=/tmp/pili-hubrise/pos.db node --test e2e/real-binary/hubrise.test.mjs
 */
import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";

const DRIVER = process.env.TAURI_DRIVER_URL;
const BINARY = process.env.PILI_POS_BINARY;
const HUBRISE = process.env.HUBRISE_MOCK_URL;
const PRINTER = process.env.PRINTER_JOBS_URL;
const DB = process.env.PILI_POS_DB;
const ELEMENT = "element-6066-11e4-a52e-4f735466cecf";
const skip =
  !DRIVER || !BINARY || !HUBRISE || !PRINTER || !DB
    ? "needs the HubRise mocks (scripts/e2e_hubrise.sh)"
    : false;

/** The seeded manager (ui/e2e/fixtures/seed_hubrise.sql). */
const MANAGER = { name: "Gerant E2E", insz: "94060242576", pin: "1234" };
/** Orders reach the POS at its next poll (every 30 s) or event. */
const ORDER_WAIT = 75_000;

let session;

// ── WebDriver ───────────────────────────────────────────────────

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
function elementId(ref) {
  const id = ref?.[ELEMENT];
  if (!id) throw new Error(`unexpected element reference: ${JSON.stringify(ref)}`);
  return id;
}
const findAll = async (css) =>
  (await cmd("POST", "/elements", { using: "css selector", value: css })).map(elementId);
const js = (script, ...args) => cmd("POST", "/execute/sync", { script, args });
// textContent: WebKitWebDriver's rendered text drops line-clamped text.
const text = async (id) =>
  (await js("return arguments[0].textContent;", { [ELEMENT]: id })).replace(/\s+/g, " ").trim();
const click = (id) => cmd("POST", `/element/${id}/click`, {});
const type = (id, value) => cmd("POST", `/element/${id}/value`, { text: value });
const go = (hash) => js(`window.location.hash = ${JSON.stringify(hash)};`);

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
    await new Promise((r) => setTimeout(r, 300));
  }
  assert.fail(`timed out waiting for ${what} (last: ${last})`);
}

/** The first `css` match whose text contains `label`. */
async function find(css, label) {
  for (const id of await findAll(css)) {
    if (label === undefined || (await text(id)).includes(label)) return id;
  }
  return null;
}
const waitFind = (css, label, timeoutMs) =>
  waitFor(`${css}${label ? ` "${label}"` : ""}`, () => find(css, label), timeoutMs);
async function tap(css, label, timeoutMs) {
  await click(await waitFind(css, label, timeoutMs));
}

// ── Mocks and the POS database ─────────────────────────────────

async function http(method, url, body, headers = {}) {
  const res = await fetch(url, {
    method,
    headers: { "content-type": "application/json", ...headers },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const raw = await res.text();
  if (!res.ok) throw new Error(`${method} ${url}: ${res.status} ${raw}`);
  return raw ? JSON.parse(raw) : null;
}
const mock = (method, path, body) => http(method, `${HUBRISE}${path}`, body);

/** What the POS asked HubRise, oldest first. */
async function requests() {
  const log = await mock("GET", "/mock/requests");
  return Array.isArray(log) ? log : (log.requests ?? []);
}
const patchesTo = async (orderId, status) =>
  (await requests()).filter(
    (r) =>
      r.method === "PATCH" &&
      r.path.endsWith(`/orders/${orderId}`) &&
      JSON.stringify(r.body ?? "").includes(`"${status}"`),
  );

/** Rows of the POS database (read-only; the app keeps it in WAL). */
function sql(query) {
  const out = execFileSync("sqlite3", ["-readonly", "-json", DB, query], { encoding: "utf8" });
  return out.trim() ? JSON.parse(out) : [];
}
/** The P, N and refund of an online order, as the POS links them. */
function eventsOf(hubriseId) {
  return sql(
    // The counters are the FDM's: in its archived response.
    `SELECT f.event_type, f.event_label, f.short_signature, f.event_data_json,
            json_extract(f.fdm_response_json, '$.fdmRef.eventCounter') AS event_counter,
            json_extract(f.fdm_response_json, '$.fdmRef.totalCounter') AS total_counter
       FROM fiscal_events f JOIN webshop_orders w
         ON f.id IN (w.p_event_id, w.n_event_id, w.refund_event_id)
      WHERE w.hubrise_order_id = '${hubriseId}' ORDER BY f.id`,
  ).map((e) => ({ ...e, request: JSON.parse(e.event_data_json) }));
}
/** The event's cost center, as `{ type, reference }`. */
const costCenter = (e) => {
  const found = JSON.stringify(e.request).match(/"costCenter":\{[^}]*\}/);
  const cc = found && JSON.parse(`{${found[0]}}`).costCenter;
  return cc && { type: cc.type, reference: cc.reference };
};
const paymentTypes = (e) => (e.request.financials ?? []).map((p) => p.type);

async function printed() {
  const jobs = await http("GET", `${PRINTER}/jobs`);
  return (Array.isArray(jobs) ? jobs : (jobs.jobs ?? [])).map((j) => j.text ?? "");
}

async function card(column) {
  return find(`.column-${column} .card`);
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

// ── The run ─────────────────────────────────────────────────────

test("the manager logs in with the PIN", { skip }, async () => {
  await tap(".user-btn", MANAGER.name);
  for (const digit of MANAGER.pin) await tap(".pin-overlay .nk", digit);
  await waitFor("the tables route", async () => /#\/tables/.test(await cmd("GET", "/url")));
});

test("HubRise connects through the login dialog", { skip }, async () => {
  await go("/settings");
  await tap("button", "Se connecter à HubRise");
  const dialog = '[data-testid="hubrise-authorize"]';
  await waitFind(`${dialog} .qr svg`);
  const url = await text(await waitFind(`${dialog} .url`));
  assert.ok(url.startsWith(`${HUBRISE}/oauth2/v1/authorize?`), url);
  // The installer logs in on a phone; HubRise shows the code.
  const { code } = await http("GET", url);
  await type(await waitFind(`${dialog} .field`), code);
  await tap(`${dialog} .btn-confirm`);
  await waitFor("the dialog closed", async () => (await findAll(dialog)).length === 0);
  await waitFind(".hubrise-section .pill.status-ok", "Connecté");
  // Connecting registered the passive callback.
  await waitFor("the callback registered", async () =>
    (await requests()).some((r) => r.method === "POST" && r.path.endsWith("/callback")),
  );
});

test("the manager takes over the menu from HubRise", { skip }, async () => {
  const adopt = await waitFor(
    "the take-over or the managed menu",
    async () =>
      (await find(".hubrise-section .action-btn", "Reprendre le menu")) ??
      ((await find('[data-testid="menu-managed"]')) && "managed"),
  );
  if (adopt !== "managed") await click(adopt);
  await waitFind('[data-testid="menu-managed"]', "Menu géré par HubRise");
  const active = sql("SELECT COUNT(*) AS n FROM menu_items WHERE active = 1 AND external_id IS NOT NULL");
  assert.ok(active[0].n > 0, "the HubRise products are on the menu");
});

let prepaid;

test("a prepaid platform order is accepted and signed N", { skip }, async () => {
  prepaid = (await mock("POST", "/mock/orders", { template: "deliveroo_prepaid" })).id;
  await go("/webshop-orders");
  await waitFor("the order on the board", () => card("new"), ORDER_WAIT);
  assert.equal(await text((await findAll('.column-new [data-testid="pay-badge"]'))[0]), "Payée");
  const jobsBefore = (await printed()).length;
  await tap(".column-new .btn-primary", "Accepter");
  await waitFor("the order in the kitchen", () => card("kitchen"));

  const [sale] = await waitFor("the signed sale", async () => {
    const e = eventsOf(prepaid);
    return e.length === 1 && e;
  });
  assert.equal(sale.event_type, "N");
  assert.ok(sale.short_signature, "signed by the FDM");
  assert.deepEqual(costCenter(sale), { type: "PLATFORM", reference: prepaid });
  assert.ok(paymentTypes(sale).length > 0 && paymentTypes(sale).every((t) => t === "APP"));

  // The kitchen ticket and the VAT ticket.
  const tickets = await waitFor("two tickets", async () => {
    const all = (await printed()).slice(jobsBefore);
    return all.length >= 2 && all;
  });
  assert.ok(tickets.some((t) => /TVA|BTW/.test(t)), tickets.join("\n----\n"));
  // HubRise hears it, through the outbox.
  await waitFor("PATCH accepted", async () => (await patchesTo(prepaid, "accepted")).length > 0);
});

test("the platform cancels; the manager refunds the signed sale", { skip }, async () => {
  await mock("POST", `/mock/orders/${prepaid}/cancel`);
  await waitFind('[data-testid="cancelled-banner"]', "remboursement", ORDER_WAIT);
  await tap(".btn-primary", "Rembourser");

  const [sale, refund] = await waitFor("the refund signed", async () => {
    const e = eventsOf(prepaid);
    return e.length === 2 && e;
  });
  assert.equal(refund.event_type, "N");
  assert.ok(refund.short_signature);
  // The refund cites the sale and goes back by the app.
  const cited = refund.request.fdmRef;
  assert.equal(cited.eventLabel, "N", JSON.stringify(cited));
  assert.equal(cited.eventCounter, sale.event_counter);
  assert.equal(cited.totalCounter, sale.total_counter);
  assert.ok(paymentTypes(refund).every((t) => t === "APP"));
  await waitFor("the card gone", async () => !(await find('[data-testid="cancelled-banner"]')));
});

let unpaid;

test("an unpaid webshop order is accepted and registered P", { skip }, async () => {
  unpaid = (await mock("POST", "/mock/orders", { template: "webshop_cash_collection" })).id;
  await waitFor("the order on the board", () => card("new"), ORDER_WAIT);
  assert.equal(await text((await findAll('.column-new [data-testid="pay-badge"]'))[0]), "À encaisser");
  await tap(".column-new .btn-primary", "Accepter");
  const [order] = await waitFor("the P", async () => {
    const e = eventsOf(unpaid);
    return e.length === 1 && e;
  });
  assert.equal(order.event_type, "P");
  assert.deepEqual(costCenter(order), { type: "WEBSHOP", reference: unpaid });
  await waitFor("PATCH accepted", async () => (await patchesTo(unpaid, "accepted")).length > 0);
});

test("the unpaid order is cashed in at handover and signed N", { skip }, async () => {
  // Through the kitchen to the counter, one tap a stage.
  const cash = await waitFor(
    "the cash button",
    async () => {
      const done = await find(".column-ready .btn-primary", "Encaisser espèces");
      if (done) return done;
      const next = await find(".column-kitchen .btn-primary");
      if (next) await click(next);
      return null;
    },
    45_000,
  );
  await click(cash);
  const [order, sold] = await waitFor("the sale", async () => {
    const e = eventsOf(unpaid);
    return e.length === 2 && e;
  });
  assert.equal(sold.event_type, "N");
  assert.ok(sold.short_signature);
  // The N closes the P: the same cost center.
  assert.deepEqual(costCenter(sold), costCenter(order));
  assert.deepEqual(paymentTypes(sold), ["CASH"]);
  await waitFor("PATCH completed", async () => (await patchesTo(unpaid, "completed")).length > 0);
});

test("a catalog change on HubRise reaches the menu", { skip }, async () => {
  const current = await mock("PUT", "/mock/catalog", {});
  const data = current.data ?? current;
  const sku = data.products[0].skus[0];
  const before = sql(`SELECT id, price_cents FROM menu_items WHERE external_id = '${sku.ref}'`)[0];
  assert.ok(before, `the menu item of ${sku.ref}`);
  const cents = before.price_cents + 100;
  sku.price = `${(cents / 100).toFixed(2)} EUR`;
  await mock("PUT", "/mock/catalog", { data });

  // HubRise announces it (catalog.update); the POS applies it at its
  // next poll, nobody touching the till.
  const after = await waitFor(
    "the new price",
    async () => {
      const row = sql(`SELECT id, price_cents FROM menu_items WHERE external_id = '${sku.ref}'`)[0];
      return row.price_cents === cents && row;
    },
    ORDER_WAIT,
  );
  // Same product, same id: the productId the FDM knows.
  assert.equal(after.id, before.id);
});
