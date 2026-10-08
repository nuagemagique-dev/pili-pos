#!/usr/bin/env node
// pili-pos - Vue to Android strings converter (plan P4.3, PD105).
//
// Reads the Vue texts (ui/src/i18n*.ts, ui/src/utils/fdmMessages.ts) and the engine error
// registry (crates/pos-engine/src/commands/errors.rs) with a strict parser of its own (node
// built-ins only, no TypeScript dependency) and writes, under android/app:
//   res/values{,-en,-zh}/strings_vue_<area>.xml   areas core errors checkout reports settings webshop
//   res/values/strings_vue_fdm.xml                FDM texts (French and Chinese, translatable=false)
//   kotlin/dev/nuagemagique/pilipos/i18n/generated/{VueStringIds,FdmMessageTables,EngineErrorCodes}.kt
//   src/test/resources/vue_strings.json           snapshot compared by the unit tests
//
// Usage: node scripts/android/vue_to_android.mjs [--write | --check] [--repo DIR] [--out DIR]
//   (default)  print the summary line only
//   --write    write the outputs (and remove stale strings_vue_*.xml / generated *.kt files)
//   --check    regenerate in memory and compare byte for byte with the committed files;
//              exit 1 on a difference, a missing file or a stale file
//   --repo DIR root holding ui/ and crates/ (default: this repository)
//   --out DIR  root holding android/ and receiving the outputs (default: --repo)
// Exit 0 on PASS, 1 on FAIL (a rule broken or a drift), 2 on a usage error.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const INPUTS = {
  core: "ui/src/i18n.ts",
  checkout: "ui/src/i18n-checkout.ts",
  reports: "ui/src/i18n-reports.ts",
  settings: "ui/src/i18n-settings.ts",
  webshop: "ui/src/i18n-webshop.ts",
};
const FDM_FILE = "ui/src/utils/fdmMessages.ts";
const ERRORS_FILE = "crates/pos-engine/src/commands/errors.rs";
const LEAF_OBJECTS = {
  coreMessages: "core",
  checkoutMessages: "checkout",
  reportsMessages: "reports",
  settingsMessages: "settings",
  webshopMessages: "webshop",
};
const AREAS = ["core", "errors", "checkout", "reports", "settings", "webshop"];
const LANGS = ["fr", "en", "zh"];
const LOCALE_DIR = { fr: "values", en: "values-en", zh: "values-zh" };
const RES = "android/app/src/main/res";
const GEN = "android/app/src/main/kotlin/dev/nuagemagique/pilipos/i18n/generated";
const JSON_OUT = "android/app/src/test/resources/vue_strings.json";
const RESERVED = ["shell_", "engine_", "prov_", "kiosk_", "ui_", "plat_", "fdm_msg_", "fdm_zh_", "fdm_fallback_"];
const KEYWORDS = new Set(
  ("as break class continue do else false for fun if in interface is null object package return super this " +
    "throw true try typealias typeof val var when while abstract assert boolean byte case catch char const " +
    "default double enum extends final finally float goto implements import instanceof int long native new " +
    "private protected public short static strictfp switch synchronized throws transient void volatile").split(" "),
);
const CHUNK = 250;
const EM_DASH = new RegExp(String.fromCharCode(0x2014), "g");

class Fail extends Error {}
const fail = (file, line, msg) => new Fail(`${file}:${line}: ${msg}`);

// ── tokenizer (TypeScript subset and Rust subset) ──────────────────────────────────────
const ESCAPES = { n: "\n", t: "\t", r: "\r", "'": "'", '"': '"', "\\": "\\", "`": "`", "0": "\0" };

function tokenize(src, file, rust = false) {
  const toks = [];
  let i = 0;
  let line = 1;
  const n = src.length;
  while (i < n) {
    const c = src[i];
    if (c === "\n") { line++; i++; continue; }
    if (/\s/.test(c)) { i++; continue; }
    if (c === "/" && src[i + 1] === "/") { while (i < n && src[i] !== "\n") i++; continue; }
    if (c === "/" && src[i + 1] === "*") {
      const end = src.indexOf("*/", i + 2);
      if (end < 0) throw fail(file, line, "unterminated comment");
      for (const ch of src.slice(i, end)) if (ch === "\n") line++;
      i = end + 2;
      continue;
    }
    if (c === '"' || c === "'" || c === "`") {
      if (rust && c === "'" && !/^'(\\.|[^\\'\n])'/.test(src.slice(i, i + 4))) { toks.push({ k: "p", v: c, line }); i++; continue; }
      const start = line;
      let v = "";
      let dynamic = false;
      i++;
      for (;;) {
        if (i >= n || (c !== "`" && src[i] === "\n")) throw fail(file, start, "unterminated string");
        const d = src[i];
        if (d === c) { i++; break; }
        if (d === "\n") line++;
        if (c === "`" && d === "$" && src[i + 1] === "{") {
          // A template with ${}: skipped to the end of its braces; the parser refuses the token.
          dynamic = true;
          for (let depth = 0; i < n; i++) {
            if (src[i] === "{") depth++;
            else if (src[i] === "}" && --depth === 0) break;
            else if (src[i] === "\n") line++;
          }
          i++;
          continue;
        }
        if (d !== "\\") { v += d; i++; continue; }
        const e = src[i + 1];
        if (e === "u") {
          const m = /^\\u(?:([0-9a-fA-F]{4})|\{([0-9a-fA-F]{1,6})\})/.exec(src.slice(i, i + 12));
          if (!m) throw fail(file, line, "bad \\u escape");
          v += String.fromCodePoint(parseInt(m[1] ?? m[2], 16));
          i += m[0].length;
        } else if (e in ESCAPES) { v += ESCAPES[e]; i += 2; } else throw fail(file, line, `unsupported escape \\${e}`);
      }
      toks.push({ k: dynamic ? "tpl" : "str", v, line: start });
      continue;
    }
    if (/[A-Za-z_$]/.test(c)) {
      let j = i + 1;
      while (j < n && /[\w$]/.test(src[j])) j++;
      toks.push({ k: "id", v: src.slice(i, j), line });
      i = j;
      continue;
    }
    if (/[0-9]/.test(c)) {
      let j = i + 1;
      while (j < n && /[\w.]/.test(src[j])) j++;
      toks.push({ k: "num", v: src.slice(i, j), line });
      i = j;
      continue;
    }
    toks.push({ k: "p", v: c, line });
    i++;
  }
  return toks;
}

// ── TypeScript subset parser ───────────────────────────────────────────────────────────
class Parser {
  constructor(toks, file) { this.t = toks; this.i = 0; this.file = file; }
  get cur() { return this.t[this.i]; }
  err(msg, tok = this.cur ?? this.t[this.t.length - 1]) {
    return fail(this.file, tok ? tok.line : 0, tok?.k === "tpl" ? "template literal with ${} is not supported" : msg);
  }
  isP(v, off = 0) { const x = this.t[this.i + off]; return !!x && x.k === "p" && x.v === v; }
  eatP(v) { if (!this.isP(v)) throw this.err(`expected '${v}', found ${this.cur ? `'${this.cur.v}'` : "end of file"}`); this.i++; }
  str() {
    if (this.cur?.k !== "str") throw this.err("expected a string literal");
    let v = this.t[this.i++].v;
    while (this.isP("+")) { this.i++; if (this.cur?.k !== "str") throw this.err("expected a string literal after '+'"); v += this.t[this.i++].v; }
    return v;
  }
  value() {
    const tok = this.cur;
    if (tok?.k === "str") return { kind: "str", v: this.str(), line: tok.line };
    if (this.isP("{")) return { kind: "obj", entries: this.object(), line: tok.line };
    if (tok?.k === "id" && this.isP(".", 1) && this.t[this.i + 2]?.k === "id") {
      const ref = { kind: "ref", obj: tok.v, prop: this.t[this.i + 2].v, line: tok.line };
      this.i += 3;
      return ref;
    }
    throw this.err(`unsupported syntax '${tok ? tok.v : "end of file"}'`);
  }
  object() {
    this.eatP("{");
    const entries = [];
    const seen = new Set();
    while (!this.isP("}")) {
      const tok = this.cur;
      if (!tok) throw this.err("unterminated object");
      if (this.isP(".") && this.isP(".", 1) && this.isP(".", 2)) {
        this.i += 3;
        if (this.cur?.k !== "id") throw this.err("expected a name after '...'");
        entries.push({ spread: this.t[this.i++].v, line: tok.line });
      } else {
        if (tok.k !== "id" && tok.k !== "str") throw this.err(`unsupported syntax '${tok.v}'`);
        this.i++;
        if (seen.has(tok.v)) throw this.err(`duplicate key '${tok.v}' in the same object`, tok);
        seen.add(tok.v);
        this.eatP(":");
        entries.push({ key: tok.v, value: this.value(), line: tok.line });
      }
      if (this.isP(",")) this.i++;
      else if (!this.isP("}")) throw this.err(`expected ',' or '}', found '${this.cur?.v}'`);
    }
    this.i++;
    return entries;
  }
  // The object literal assigned to `const <name>`.
  named(name) {
    const at = this.t.findIndex((x, j) => x.k === "id" && x.v === "const" && this.t[j + 1]?.v === name);
    if (at < 0) throw this.err(`no 'const ${name}' found`, this.t[0]);
    this.i = at + 2;
    while (this.cur && !this.isP("=")) this.i++;
    this.i++;
    return this.object();
  }
  // The string literals returned by `function <name>`, in order.
  returns(name) {
    const at = this.t.findIndex((x, j) => x.k === "id" && x.v === "function" && this.t[j + 1]?.v === name);
    if (at < 0) throw this.err(`no 'function ${name}' found`, this.t[0]);
    this.i = at + 2;
    while (this.cur && !this.isP("{")) this.i++;
    let depth = 0;
    const out = [];
    for (; this.cur; ) {
      if (this.isP("{")) depth++;
      else if (this.isP("}") && --depth === 0) break;
      if (this.cur.k === "id" && this.cur.v === "return" && this.t[this.i + 1]?.k === "str") {
        this.i++;
        out.push(this.str());
        this.eatP(";");
        continue;
      }
      this.i++;
    }
    return out;
  }
}

// ── Rust registry parser ───────────────────────────────────────────────────────────────
// Returns { '': {consts, all}, hubrise: {consts, all} } (consts: name to value, all: names).
function parseRegistry(src, file) {
  const t = tokenize(src, file, true);
  const scopes = {};
  const stack = [];
  const at = (i, k, v) => t[i]?.k === k && (v === undefined || t[i].v === v);
  for (let i = 0; i < t.length; i++) {
    const x = t[i];
    if (x.k === "p" && x.v === "{") { stack.push(null); continue; }
    if (x.k === "p" && x.v === "}") { stack.pop(); continue; }
    if (!at(i, "id", "pub")) continue;
    if (at(i + 1, "id", "mod") && at(i + 2, "id") && at(i + 3, "p", "{")) {
      if (stack.length) continue;
      scopes[t[i + 2].v] ??= { consts: {}, all: null, line: x.line };
      stack.push(t[i + 2].v);
      i += 3;
      continue;
    }
    if (!at(i + 1, "id", "const")) continue;
    const scope = stack.length === 0 ? "" : stack.length === 1 && stack[0] !== null ? stack[0] : undefined;
    if (scope === undefined) continue;
    scopes[scope] ??= { consts: {}, all: null, line: x.line };
    const name = t[i + 2]?.v;
    // pub const NAME: &str = "value";
    if (at(i + 3, "p", ":") && at(i + 4, "p", "&") && at(i + 5, "id", "str") && at(i + 6, "p", "=") && at(i + 7, "str")) {
      if (name in scopes[scope].consts) throw fail(file, x.line, `constant ${name} defined twice`);
      scopes[scope].consts[name] = t[i + 7].v;
      continue;
    }
    // pub const ALL: &[&str] = &[ A, B, ];
    if (name === "ALL") {
      let j = i + 3;
      for (const [k, v] of [["p", ":"], ["p", "&"], ["p", "["], ["p", "&"], ["id", "str"], ["p", "]"], ["p", "="], ["p", "&"], ["p", "["]]) {
        if (!at(j, k, v)) throw fail(file, x.line, "unsupported form of the ALL list");
        j++;
      }
      const list = [];
      while (!at(j, "p", "]")) {
        if (!at(j, "id")) throw fail(file, t[j]?.line ?? x.line, "unsupported item in the ALL list");
        list.push({ name: t[j].v, line: t[j].line });
        j++;
        if (at(j, "p", ",")) j++;
      }
      scopes[scope].all = list;
      i = j;
    }
  }
  const out = {};
  for (const scope of ["", "hubrise"]) {
    const s = scopes[scope];
    if (!s?.all) throw fail(file, s?.line ?? 1, `no ALL list found in ${scope === "" ? "the top level" : `mod ${scope}`}`);
    const codes = s.all.map((e) => {
      if (!(e.name in s.consts)) throw fail(file, e.line, `unknown constant ${e.name} in the ALL list`);
      return s.consts[e.name];
    });
    if (new Set(codes).size !== codes.length) throw fail(file, s.line, "a code is listed twice");
    out[scope] = codes;
  }
  return out;
}

// ── model ──────────────────────────────────────────────────────────────────────────────
function validKey(file, line, key) {
  if (!/^[a-z][a-z0-9_]*$/.test(key)) throw fail(file, line, `invalid key '${key}' (must match ^[a-z][a-z0-9_]*$)`);
  const bad = RESERVED.find((p) => key.startsWith(p));
  if (bad) throw fail(file, line, `key '${key}' uses the reserved prefix '${bad}'`);
  if (KEYWORDS.has(key)) throw fail(file, line, `key '${key}' is a Java or Kotlin keyword and cannot be a resource name`);
}

function leafMessages(entries, file, area) {
  const out = [];
  for (const e of entries) {
    if (e.spread) throw fail(file, e.line, "spread is only supported in the merge objects of i18n.ts");
    validKey(file, e.line, e.key);
    if (e.value.kind !== "obj") throw fail(file, e.line, `key '${e.key}': expected an object { fr, en, zh }`);
    const langs = {};
    for (const l of e.value.entries) {
      if (l.spread || !LANGS.includes(l.key) || l.value.kind !== "str") throw fail(file, l.line, `key '${e.key}': only fr, en and zh strings are supported`);
      langs[l.key] = l.value.v;
    }
    if (langs.fr === undefined) throw fail(file, e.line, `key '${e.key}' has no French text`);
    out.push({ key: e.key, langs, file, line: e.line, area: e.key.startsWith("err_") ? "errors" : area });
  }
  return out;
}

function readText(repo, rel) {
  try { return fs.readFileSync(path.join(repo, rel), "utf8"); } catch (e) { throw new Fail(`${rel}: cannot read (${e.code ?? e.message})`); }
}

function collectMessages(repo, warns) {
  const leaves = {};
  const merges = {};
  for (const [area, rel] of Object.entries(INPUTS)) {
    const p = new Parser(tokenize(readText(repo, rel), rel), rel);
    for (const [name, a] of Object.entries(LEAF_OBJECTS)) {
      if (a !== area) continue;
      leaves[name] = leafMessages(p.named(name), rel, area);
    }
    if (area === "core") {
      for (const name of ["screenMessages", "messages"]) {
        merges[name] = p.named(name).map((e) => {
          if (!e.spread) throw fail(rel, e.line, `${name} may only hold spreads`);
          return e.spread;
        });
      }
    }
  }
  const merged = new Map();
  const apply = (name, depth = 0) => {
    if (depth > 8) throw new Fail(`${INPUTS.core}: merge objects nest too deep at ${name}`);
    if (merges[name]) { for (const s of merges[name]) apply(s, depth + 1); return; }
    if (!leaves[name]) throw new Fail(`${INPUTS.core}: spread of unknown object '${name}'`);
    for (const m of leaves[name]) {
      const old = merged.get(m.key);
      if (old) warns.push(`WARN | duplicate | ${m.key} | ${old.file},${m.file}`);
      merged.delete(m.key);
      merged.set(m.key, m);
    }
  };
  apply("messages");
  return merged;
}

const codeRe = /^[A-Z][A-Z0-9_]*$/;

function collectFdm(repo) {
  const p = new Parser(tokenize(readText(repo, FDM_FILE), FDM_FILE), FDM_FILE);
  const fdm = new Map(); // resource name to text
  const tables = { POS_CHECK: [], ERRORS: [], VENDOR: [], ZH: [] };
  const add = (name, text, line) => {
    if (fdm.has(name)) throw fail(FDM_FILE, line, `resource ${name} defined twice`);
    fdm.set(name, text);
  };
  const strings = (obj, table, prefix) => {
    for (const e of p.named(obj)) {
      if (e.spread || e.value.kind !== "str") throw fail(FDM_FILE, e.line, `${obj}: only string values are supported`);
      if (!codeRe.test(e.key)) throw fail(FDM_FILE, e.line, `${obj}: '${e.key}' is not a code (A-Z, 0-9, _)`);
      const res = `${prefix}${e.key.toLowerCase()}`;
      add(res, e.value.v, e.line);
      tables[table].push([e.key, res]);
    }
  };
  strings("POS_CHECK_MESSAGES", "POS_CHECK", "fdm_msg_");
  strings("ERROR_MESSAGES", "ERRORS", "fdm_msg_");
  const known = new Map([...tables.POS_CHECK, ...tables.ERRORS]);
  for (const e of p.named("VENDOR_MESSAGES")) {
    const v = e.value;
    if (e.spread || v?.kind !== "ref" || !["ERROR_MESSAGES", "POS_CHECK_MESSAGES"].includes(v.obj)) throw fail(FDM_FILE, e.line, "VENDOR_MESSAGES: expected ERROR_MESSAGES.<CODE>");
    if (!known.has(v.prop)) throw fail(FDM_FILE, e.line, `VENDOR_MESSAGES: unknown ${v.obj}.${v.prop}`);
    tables.VENDOR.push([e.key, known.get(v.prop)]);
  }
  let vendor = 0;
  for (const e of p.named("MESSAGES_ZH")) {
    if (e.spread || e.value.kind !== "str") throw fail(FDM_FILE, e.line, "MESSAGES_ZH: only string values are supported");
    const res = codeRe.test(e.key) ? `fdm_zh_${e.key.toLowerCase()}` : `fdm_zh_vendor_${++vendor}`;
    add(res, e.value.v, e.line);
    tables.ZH.push([e.key, res]);
  }
  const fallbacks = {};
  for (const [fn, prefix] of [["translateFdmError", "fdm_fallback_"], ["translateFdmErrorZh", "fdm_zh_fallback_"]]) {
    const r = p.returns(fn);
    if (r.length !== 2) throw fail(FDM_FILE, 1, `${fn}: expected 2 string returns (connection, generic), found ${r.length}`);
    for (const [kind, text] of [["connection", r[0]], ["generic", r[1]]]) {
      add(`${prefix}${kind}`, text, 1);
      fallbacks[`${prefix}${kind}`] = `${prefix}${kind}`;
    }
  }
  return { fdm, tables, fallbacks };
}

// ── rendering ──────────────────────────────────────────────────────────────────────────
const noEmDash = (s) => s.replace(EM_DASH, "\\u2014");

function xmlValue(text) {
  let s = text
    .replace(/\\/g, "\\\\").replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    .replace(/'/g, "\\'").replace(/"/g, '\\"').replace(/\n/g, "\\n").replace(/\t/g, "\\t").replace(/\r/g, "\\r");
  if (/^[@?]/.test(s)) s = `\\${s}`;
  if (/^\s|\s$| {2}/.test(text)) s = `"${s}"`;
  return noEmDash(s);
}

const INPUT_LIST = [...Object.values(INPUTS), FDM_FILE, ERRORS_FILE].join(", ");
const xmlHeader = `<?xml version="1.0" encoding="utf-8"?>\n<!-- Generated by scripts/android/vue_to_android.mjs from ${INPUT_LIST}. Do not edit; regenerate with node scripts/android/vue_to_android.mjs in write mode (the write flag). -->\n`;
const codeHeader = `// Generated by scripts/android/vue_to_android.mjs from ${INPUT_LIST}. Do not edit; regenerate with node scripts/android/vue_to_android.mjs --write.\n`;

function xmlFile(entries, extra = "") {
  const lines = entries.map(([name, text]) => {
    const attrs = `name="${name}"${extra}${text.includes("%") ? ' formatted="false"' : ""}`;
    return `    <string ${attrs}>${xmlValue(text)}</string>\n`;
  });
  return `${xmlHeader}<resources>\n${lines.join("")}</resources>\n`;
}

const kt = (s) => noEmDash(s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\$/g, "\\$").replace(/\n/g, "\\n"));

function vueStringIdsKt(keys) {
  const chunks = [];
  for (let i = 0; i < keys.length; i += CHUNK) chunks.push(keys.slice(i, i + CHUNK));
  const body = chunks.map((c, n) => `\n    private fun MutableMap<String, Int>.part${n}() {\n${c.map((k) => `        this["${k}"] = R.string.${k}\n`).join("")}    }\n`).join("");
  const calls = chunks.map((_, n) => `part${n}()`).join("; ");
  return `${codeHeader}package dev.nuagemagique.pilipos.i18n.generated

import dev.nuagemagique.pilipos.R

/** Vue message key to Android string resource id, in the order of the Vue dictionary. */
internal object VueStringIds {
    private val ids: Map<String, Int> by lazy {
        buildMap { ${calls} }
    }

    /** Every Vue key, in dictionary order. */
    val keys: List<String> by lazy { ids.keys.toList() }

    /** The resource id of [key], or null when Vue has no such key. */
    fun id(key: String): Int? = ids[key]
${body}}
`;
}

function fdmTablesKt({ tables, fallbacks }) {
  const list = (name, doc, rows) => `    /** ${doc} */\n    val ${name}: List<Pair<String, Int>> =\n        listOf(\n${rows.map(([k, r]) => `            "${kt(k)}" to R.string.${r},\n`).join("")}        )\n`;
  const fb = (name, key) => `    val ${name}: Int = R.string.${fallbacks[key]}\n`;
  return `${codeHeader}package dev.nuagemagique.pilipos.i18n.generated

import dev.nuagemagique.pilipos.R

/**
 * FDM message tables, in the order of fdmMessages.ts: the Vue translateFdmError tests POS_CHECK, then
 * ERRORS, then VENDOR, taking the first entry whose key the raw text contains.
 */
internal object FdmMessageTables {
${list("POS_CHECK", "Refusals detected by the POS itself (French).", tables.POS_CHECK)}
${list("ERRORS", "FDM error codes (French).", tables.ERRORS)}
${list("VENDOR", "Vendor text to the id of the French message it maps to.", tables.VENDOR)}
${list("ZH", "Chinese line per FDM code or vendor text, in the order of MESSAGES_ZH.", tables.ZH)}
${fb("FALLBACK_CONNECTION", "fdm_fallback_connection")}${fb("FALLBACK_GENERIC", "fdm_fallback_generic")}${fb("ZH_FALLBACK_CONNECTION", "fdm_zh_fallback_connection")}${fb("ZH_FALLBACK_GENERIC", "fdm_zh_fallback_generic")}}
`;
}

function engineCodesKt(all, hubrise) {
  const list = (name, doc, codes) => `    /** ${doc} */\n    val ${name}: List<String> =\n        listOf(\n${codes.map((c) => `            "${c}",\n`).join("")}        )\n`;
  return `${codeHeader}package dev.nuagemagique.pilipos.i18n.generated

/** Engine error codes in registry order (crates/pos-engine/src/commands/errors.rs). */
object EngineErrorCodes {
${list("ALL", "errors::ALL.", all)}
${list("HUBRISE", "errors::hubrise::ALL.", hubrise)}}
`;
}

const sortObj = (o) => Object.fromEntries(Object.entries(o).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));

function build(repo) {
  const warns = [];
  const merged = collectMessages(repo, warns);
  const fdmData = collectFdm(repo);
  const registry = parseRegistry(readText(repo, ERRORS_FILE), ERRORS_FILE);
  const missing = [];
  for (const code of [...registry[""], ...registry.hubrise]) {
    const m = merged.get(`err_${code}`);
    const lacks = LANGS.filter((l) => !m?.langs[l]);
    if (lacks.length) missing.push(`err_${code} (${m ? lacks.join(", ") : "no key"})`);
  }
  const counts = Object.fromEntries(AREAS.map((a) => [a, 0]));
  for (const m of merged.values()) counts[m.area]++;
  const summary = `${merged.size} keys (${AREAS.map((a) => `${a} ${counts[a]}`).join(", ")}), fdm ${fdmData.fdm.size}, error codes ${registry[""].length} + ${registry.hubrise.length}`;
  if (missing.length) return { warns, summary, problems: [`VUE | engine-codes | FAIL | ${missing.length} without fr, en and zh: ${missing.join("; ")}`], outputs: new Map() };

  const outputs = new Map();
  for (const area of AREAS) {
    for (const l of LANGS) {
      const rows = [...merged.values()].filter((m) => m.area === area && m.langs[l] !== undefined).map((m) => [m.key, m.langs[l]]);
      if (rows.length) outputs.set(`${RES}/${LOCALE_DIR[l]}/strings_vue_${area}.xml`, xmlFile(rows));
    }
  }
  outputs.set(`${RES}/values/strings_vue_fdm.xml`, xmlFile([...fdmData.fdm], ' translatable="false"'));
  outputs.set(`${GEN}/VueStringIds.kt`, vueStringIdsKt([...merged.keys()]));
  outputs.set(`${GEN}/FdmMessageTables.kt`, fdmTablesKt(fdmData));
  outputs.set(`${GEN}/EngineErrorCodes.kt`, engineCodesKt(registry[""], registry.hubrise));
  const keys = {};
  for (const m of merged.values()) keys[m.key] = { fr: m.langs.fr, ...(m.langs.en !== undefined && { en: m.langs.en }), ...(m.langs.zh !== undefined && { zh: m.langs.zh }), area: m.area };
  const snapshot = { keys: sortObj(keys), fdm: sortObj(Object.fromEntries(fdmData.fdm)), engineErrorCodes: registry[""], hubriseErrorCodes: registry.hubrise };
  outputs.set(JSON_OUT, `${noEmDash(JSON.stringify(snapshot, null, 2))}\n`);
  return { warns, summary, problems: [], outputs };
}

// Generated files present on disk that this run would not produce.
function staleFiles(out, outputs) {
  const found = [];
  for (const dir of Object.values(LOCALE_DIR)) {
    const d = path.join(out, RES, dir);
    if (fs.existsSync(d)) for (const f of fs.readdirSync(d)) if (/^strings_vue_.*\.xml$/.test(f)) found.push(`${RES}/${dir}/${f}`);
  }
  const g = path.join(out, GEN);
  if (fs.existsSync(g)) for (const f of fs.readdirSync(g)) if (f.endsWith(".kt")) found.push(`${GEN}/${f}`);
  return found.filter((f) => !outputs.has(f)).sort();
}

function main(argv) {
  let mode = "summary";
  let repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
  let out = null;
  for (let i = 0; i < argv.length; i++) {
    const [flag, inline] = argv[i].split(/=(.*)/s);
    if (flag === "--write" || flag === "--check") mode = flag.slice(2);
    else if (flag === "--repo" || flag === "--out") {
      const v = inline ?? argv[++i];
      if (!v) { console.error(`${flag} needs a directory`); return 2; }
      if (flag === "--repo") repo = path.resolve(v); else out = path.resolve(v);
    } else { console.error(`unknown argument ${argv[i]}`); return 2; }
  }
  out ??= repo;
  let result;
  try { result = build(repo); } catch (e) {
    if (!(e instanceof Fail)) throw e;
    console.log(`VUE | total | FAIL | ${e.message}`);
    console.error(e.message);
    return 1;
  }
  for (const w of result.warns) console.log(w);
  const problems = [...result.problems];
  if (!problems.length && mode === "write") {
    for (const f of staleFiles(out, result.outputs)) { fs.rmSync(path.join(out, f)); console.log(`VUE | removed | ${f}`); }
    for (const [rel, content] of result.outputs) {
      fs.mkdirSync(path.dirname(path.join(out, rel)), { recursive: true });
      fs.writeFileSync(path.join(out, rel), content);
    }
  }
  if (!problems.length && mode === "check") {
    for (const [rel, content] of result.outputs) {
      const file = path.join(out, rel);
      if (!fs.existsSync(file)) problems.push(`VUE | drift | FAIL | missing ${rel}`);
      else if (fs.readFileSync(file, "utf8") !== content) problems.push(`VUE | drift | FAIL | modified ${rel}`);
    }
    for (const f of staleFiles(out, result.outputs)) problems.push(`VUE | drift | FAIL | stale ${f}`);
  }
  for (const p of problems) { console.log(p); console.error(p); }
  console.log(`VUE | total | ${problems.length ? "FAIL" : "PASS"} | ${result.summary}`);
  return problems.length ? 1 : 0;
}

process.exitCode = main(process.argv.slice(2));
