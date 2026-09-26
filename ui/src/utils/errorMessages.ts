// A backend error as the operator should read it, in their language.
//
// The backend refuses with a code (`src-tauri/src/commands/errors.rs`),
// possibly with values: `code|name=value|...`. Each code has a text
// `err_<code>` in i18n.ts (fr, en, zh; checked by
// src-tauri/tests/error_codes.rs). FDM failures start with "FDM" and
// get their §V message (fdmMessages.ts). Anything else is unexpected:
// a generic line, with the raw text for the technician.
import { useI18n, hasMessage } from "../i18n";
import { translateFdmError } from "./fdmMessages";

export interface CodedError {
  code: string;
  vars: Record<string, string>;
}

const CODE = /^[a-z][a-z_]*$/;

/** `code|name=value|...` split back, or null when `text` is not a code. */
export function parseErrorCode(text: string): CodedError | null {
  const [code, ...parts] = text.trim().split("|");
  if (!code || !CODE.test(code)) return null;
  const vars: Record<string, string> = {};
  for (const part of parts) {
    const at = part.indexOf("=");
    if (at > 0) vars[part.slice(0, at)] = part.slice(at + 1);
  }
  return { code, vars };
}

/** The code of a backend error, or null. */
export function errorCode(raw: unknown): string | null {
  return parseErrorCode(String(raw ?? ""))?.code ?? null;
}

export function describeError(raw: unknown): string {
  const { t } = useI18n();
  const text = String(raw ?? "").trim();
  if (!text) return t("err_failed");
  if (/^FDM\b/.test(text)) return translateFdmError(text) ?? text;
  const coded = parseErrorCode(text);
  if (coded && hasMessage(`err_${coded.code}`)) return t(`err_${coded.code}`, coded.vars);
  return t("err_unexpected", { detail: text });
}
