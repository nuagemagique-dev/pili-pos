// Self-test of vue_to_android.mjs. Run: node --test scripts/android/test_vue_to_android.mjs
// Every case works on a copy of the synthetic repository under testdata/vue_to_android/base,
// so the real tree is never touched (except by the last case, which only reads it).
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT = path.join(here, "vue_to_android.mjs");
const BASE = path.join(here, "testdata/vue_to_android/base");
const REAL_REPO = path.resolve(here, "../..");
const RES = "android/app/src/main/res";
const GEN = "android/app/src/main/kotlin/dev/nuagemagique/pilipos/i18n/generated";
const roots = [];

test.after(() => { for (const r of roots) fs.rmSync(r, { recursive: true, force: true }); });

function setup() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "vta-"));
  roots.push(root);
  const repo = path.join(root, "repo");
  fs.cpSync(BASE, repo, { recursive: true });
  return { repo, out: path.join(root, "out") };
}

function run(env, ...args) {
  const r = spawnSync("node", [SCRIPT, "--repo", env.repo, "--out", env.out, ...args], { encoding: "utf8" });
  return { code: r.status, out: r.stdout, err: r.stderr };
}

function edit(env, rel, from, to) {
  const file = path.join(env.repo, rel);
  const text = fs.readFileSync(file, "utf8");
  assert.ok(text.includes(from), `fixture lacks ${JSON.stringify(from)}`);
  fs.writeFileSync(file, text.replace(from, to));
}

const read = (env, rel) => fs.readFileSync(path.join(env.out, rel), "utf8");
const exists = (env, rel) => fs.existsSync(path.join(env.out, rel));
const CORE = `${RES}/values/strings_vue_core.xml`;
const I18N = "ui/src/i18n.ts";
const FDM = "ui/src/utils/fdmMessages.ts";

function written() {
  const env = setup();
  const r = run(env, "--write");
  assert.equal(r.code, 0, r.out + r.err);
  return env;
}

test("default mode prints the summary and writes nothing", () => {
  const env = setup();
  const r = run(env);
  assert.equal(r.code, 0);
  assert.match(r.out, /VUE \| total \| PASS \| 22 keys \(core 14, errors 3, checkout 2, reports 1, settings 1, webshop 1\), fdm 10, error codes 2 \+ 1/);
  assert.equal(fs.existsSync(env.out), false);
});

test("escapes: quotes, backslash, entities, leading @ and ?, newline, spaces, percent", () => {
  const env = written();
  const core = read(env, CORE);
  assert.match(core, /name="apos">L\\'été</);
  assert.match(core, /name="dquote">Dis \\"oui\\"</);
  assert.match(core, /name="backslash">a\\\\b</);
  assert.match(core, /name="markup">A &amp; B &lt; C &gt; D</);
  assert.match(core, /name="lead_at">\\@home</);
  assert.match(read(env, `${RES}/values-en/strings_vue_core.xml`), /name="lead_at">\\\?what</);
  assert.match(core, /name="newline">ligne1\\nligne2</);
  assert.match(core, /name="double_space">"a  b"</);
  assert.match(core, /name="edge_space">" lead"</);
  assert.match(read(env, `${RES}/values-en/strings_vue_core.xml`), /name="edge_space">"trail "</);
  assert.match(core, /name="percent" formatted="false">100 % sûr</);
  assert.doesNotMatch(read(env, `${RES}/values-en/strings_vue_core.xml`), /name="double_space">"/);
});

test("parser: comments, bare and quoted keys, concatenation, \\u escapes, template literal", () => {
  const env = written();
  const core = read(env, CORE);
  assert.match(core, /name="bare_key">Nu</);
  assert.match(core, /name="concat">long text</);
  assert.match(core, /name="unicode">été</);
  assert.match(read(env, `${RES}/values-en/strings_vue_core.xml`), /name="dquote">Say \\"yes\\"</);
});

test("a {name} placeholder is kept as it is", () => {
  const env = written();
  assert.match(read(env, CORE), /name="placeholder">Part \{n\}</);
  assert.match(read(env, `${RES}/values-zh/strings_vue_core.xml`), /name="placeholder">第\{n\}份</);
});

test("a missing language is omitted from that locale file", () => {
  const env = written();
  assert.doesNotMatch(read(env, `${RES}/values-zh/strings_vue_core.xml`), /name="backslash"/);
  assert.match(read(env, `${RES}/values-en/strings_vue_core.xml`), /name="backslash"/);
  const snap = JSON.parse(read(env, "android/app/src/test/resources/vue_strings.json"));
  assert.deepEqual(Object.keys(snap.keys.backslash), ["fr", "en", "area"]);
});

test("a key defined in two files: the later file wins and a WARN line is printed", () => {
  const env = setup();
  const r = run(env, "--write");
  assert.match(r.out, /^WARN \| duplicate \| dup \| ui\/src\/i18n\.ts,ui\/src\/i18n-checkout\.ts$/m);
  assert.doesNotMatch(read(env, CORE), /name="dup"/);
  assert.match(read(env, `${RES}/values/strings_vue_checkout.xml`), /name="dup">checkout</);
});

test("a duplicate key inside one object fails", () => {
  const env = setup();
  edit(env, I18N, '  "hello": {', '  "bare_key": { fr: "x", en: "x", zh: "x" },\n  "hello": {');
  const r = run(env);
  assert.equal(r.code, 1);
  assert.match(r.out, /VUE \| total \| FAIL \| ui\/src\/i18n\.ts:\d+: duplicate key 'bare_key'/);
});

test("an invalid key fails", () => {
  for (const bad of ["Hello", "9lives", "a-b"]) {
    const env = setup();
    edit(env, I18N, '"hello":', `"${bad}":`);
    const r = run(env);
    assert.equal(r.code, 1, bad);
    assert.match(r.out, /invalid key/, bad);
  }
});

test("a reserved prefix fails", () => {
  for (const key of ["shell_title", "engine_x", "prov_x", "kiosk_x", "ui_x", "plat_x", "fdm_msg_x", "fdm_zh_x", "fdm_fallback_x"]) {
    const env = setup();
    edit(env, I18N, '"hello":', `"${key}":`);
    const r = run(env);
    assert.equal(r.code, 1, key);
    assert.match(r.out, /reserved prefix/, key);
  }
});

test("unknown syntax fails with file:line", () => {
  const env = setup();
  edit(env, I18N, 'fr: "Bonjour"', "fr: compute()");
  const r = run(env);
  assert.equal(r.code, 1);
  assert.match(r.out, /VUE \| total \| FAIL \| ui\/src\/i18n\.ts:\d+: unsupported syntax 'compute'/);
  const line = Number(/i18n\.ts:(\d+)/.exec(r.out)[1]);
  assert.equal(fs.readFileSync(path.join(env.repo, I18N), "utf8").split("\n")[line - 1].includes("compute"), true);
  const env2 = setup();
  edit(env2, I18N, 'fr: "Bonjour"', "fr: `a ${b}`");
  assert.match(run(env2).out, /template literal with \$\{\} is not supported/);
  const env3 = setup();
  edit(env3, "ui/src/i18n-reports.ts", 'zh: "报表"', 'zh: "报表", jp: "x"');
  assert.match(run(env3).out, /i18n-reports\.ts:\d+: key 'report_title': only fr, en and zh/);
});

test("the vendor member reference is resolved to the referenced French message", () => {
  const env = written();
  const tables = read(env, `${GEN}/FdmMessageTables.kt`);
  assert.match(tables, /val VENDOR: List<Pair<String, Int>> =\s+listOf\(\s+"Not a valid Token" to R\.string\.fdm_msg_unauthorized,\s+\)/);
  assert.match(tables, /"Not a valid Token" to R\.string\.fdm_zh_vendor_1/);
  edit(env, FDM, "ERROR_MESSAGES.UNAUTHORIZED", "ERROR_MESSAGES.NOPE");
  assert.match(run(env, "--check").out, /unknown ERROR_MESSAGES\.NOPE/);
});

test("fallbacks are extracted and the FDM texts are French and Chinese only, translatable false", () => {
  const env = written();
  const fdm = read(env, `${RES}/values/strings_vue_fdm.xml`);
  for (const [name, text] of [["fdm_fallback_connection", "Connexion perdue."], ["fdm_fallback_generic", "Erreur boîte noire."], ["fdm_zh_fallback_connection", "连接中断。"], ["fdm_zh_fallback_generic", "黑匣子错误。"]]) {
    assert.ok(fdm.includes(`<string name="${name}" translatable="false">${text}</string>`), name);
  }
  assert.match(fdm, /name="fdm_msg_fdm_pending" translatable="false">La caisse attend la réponse\.</);
  assert.equal(exists(env, `${RES}/values-en/strings_vue_fdm.xml`), false);
  assert.equal(exists(env, `${RES}/values-zh/strings_vue_fdm.xml`), false);
  assert.match(read(env, `${GEN}/FdmMessageTables.kt`), /val ZH_FALLBACK_GENERIC: Int = R\.string\.fdm_zh_fallback_generic/);
  edit(env, FDM, 'return "Connexion perdue.";', "return null;");
  assert.match(run(env).out, /translateFdmError: expected 2 string returns/);
});

test("errors.rs scopes are resolved, in registry order, and err_ keys go to the errors area", () => {
  const env = written();
  const snap = JSON.parse(read(env, "android/app/src/test/resources/vue_strings.json"));
  assert.deepEqual(snap.engineErrorCodes, ["alpha", "beta"]);
  assert.deepEqual(snap.hubriseErrorCodes, ["hub_one"]);
  const codes = read(env, `${GEN}/EngineErrorCodes.kt`);
  assert.match(codes, /object EngineErrorCodes/);
  assert.match(codes, /val ALL: List<String> =\s+listOf\(\s+"alpha",\s+"beta",\s+\)/);
  assert.match(codes, /val HUBRISE: List<String> =\s+listOf\(\s+"hub_one",\s+\)/);
  const errors = read(env, `${RES}/values/strings_vue_errors.xml`);
  for (const k of ["err_alpha", "err_beta", "err_hub_one"]) assert.ok(errors.includes(`name="${k}"`), k);
  assert.equal(snap.keys.err_hub_one.area, "errors");
  edit(env, "crates/pos-engine/src/commands/errors.rs", "[HUB_ONE]", "[HUB_TWO]");
  assert.match(run(env).out, /errors\.rs:\d+: unknown constant HUB_TWO/);
});

test("an engine code without its fr, en or zh text fails and lists it", () => {
  const env = setup();
  edit(env, I18N, 'zh: "乙",', "");
  edit(env, "ui/src/i18n-webshop.ts", '  "err_hub_one": { fr: "Hub un", en: "Hub one", zh: "枢纽一" },\n', "");
  const r = run(env, "--write");
  assert.equal(r.code, 1);
  assert.match(r.out, /VUE \| engine-codes \| FAIL \| 2 without fr, en and zh: err_beta \(zh\); err_hub_one \(no key\)/);
  assert.equal(fs.existsSync(env.out), false);
});

test("--check passes on fresh outputs, then detects a modified, a missing and a stale file", () => {
  const env = written();
  assert.equal(run(env, "--check").code, 0);
  const core = path.join(env.out, CORE);
  fs.appendFileSync(core, "<!-- edit -->\n");
  fs.writeFileSync(path.join(env.out, RES, "values-en/strings_vue_x.xml"), "<resources/>\n");
  fs.rmSync(path.join(env.out, `${GEN}/EngineErrorCodes.kt`));
  fs.writeFileSync(path.join(env.out, GEN, "Old.kt"), "// old\n");
  const r = run(env, "--check");
  assert.equal(r.code, 1);
  assert.match(r.out, new RegExp(`VUE \\| drift \\| FAIL \\| modified ${CORE}`));
  assert.match(r.out, /drift \| FAIL \| stale android\/app\/src\/main\/res\/values-en\/strings_vue_x\.xml/);
  assert.match(r.out, /drift \| FAIL \| missing .*EngineErrorCodes\.kt/);
  assert.match(r.out, /drift \| FAIL \| stale .*Old\.kt/);
  assert.match(r.out, /VUE \| total \| FAIL \|/);
  // --write repairs everything, stale files included.
  assert.equal(run(env, "--write").code, 0);
  assert.equal(run(env, "--check").code, 0);
  assert.equal(exists(env, `${RES}/values-en/strings_vue_x.xml`), false);
});

test("outputs are deterministic: two runs give the same bytes", () => {
  const a = written();
  const b = written();
  const files = (env) => {
    const out = [];
    const walk = (d) => { for (const e of fs.readdirSync(d, { withFileTypes: true })) { const p = path.join(d, e.name); if (e.isDirectory()) walk(p); else out.push(p); } };
    walk(env.out);
    return out.map((p) => [path.relative(env.out, p), fs.readFileSync(p, "utf8")]).sort(([x], [y]) => (x < y ? -1 : 1));
  };
  assert.deepEqual(files(a), files(b));
  assert.equal(files(a).length, 7 + 6 + 6 + 3 + 1);
});

test("the snapshot is sorted, 2-space indented and ends with a newline; the Kotlin ids are chunked", () => {
  const env = written();
  const text = read(env, "android/app/src/test/resources/vue_strings.json");
  assert.ok(text.endsWith("}\n"));
  assert.ok(text.startsWith('{\n  "keys": {\n    "'));
  const snap = JSON.parse(text);
  assert.deepEqual(Object.keys(snap), ["keys", "fdm", "engineErrorCodes", "hubriseErrorCodes"]);
  assert.deepEqual(Object.keys(snap.keys), Object.keys(snap.keys).slice().sort());
  assert.deepEqual(Object.keys(snap.fdm), Object.keys(snap.fdm).slice().sort());
  const ids = read(env, `${GEN}/VueStringIds.kt`);
  assert.match(ids, /import dev\.nuagemagique\.pilipos\.R/);
  assert.match(ids, /internal object VueStringIds/);
  assert.match(ids, /this\["hello"\] = R\.string\.hello/);
  assert.match(ids, /fun id\(key: String\): Int\?/);
  // More than 250 keys split into several functions.
  const many = Array.from({ length: 600 }, (_, i) => `  "gen_${i}": { fr: "g${i}", en: "g${i}" },`).join("\n");
  edit(env, I18N, '  "hello":', `${many}\n  "hello":`);
  assert.equal(run(env, "--write").code, 0);
  const big = read(env, `${GEN}/VueStringIds.kt`);
  assert.match(big, /buildMap \{ part0\(\); part1\(\); part2\(\) \}/);
  assert.equal(big.split("\n").filter((l) => l.includes("] = R.string.")).length, 622);
  assert.doesNotMatch(big, new RegExp(String.fromCharCode(0x2014)));
});

test("usage errors exit 2", () => {
  const env = setup();
  assert.equal(run(env, "--nope").code, 2);
  assert.equal(spawnSync("node", [SCRIPT, "--repo"], { encoding: "utf8" }).status, 2);
});

test("the committed outputs of this repository are in sync with the Vue sources", { skip: !fs.existsSync(path.join(REAL_REPO, "ui/src/i18n.ts")) }, () => {
  const r = spawnSync("node", [SCRIPT, "--check"], { cwd: REAL_REPO, encoding: "utf8" });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  assert.match(r.stdout, /VUE \| total \| PASS \| \d+ keys/);
});
