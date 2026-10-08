// Synthetic fixture of the FDM message tables.
export const ERROR_MESSAGES: Record<string, string> = {
  BUFFER_FULL: "La boîte noire est pleine.",
  UNAUTHORIZED: "La caisse n'est pas autorisée.",
};

export const POS_CHECK_MESSAGES: Record<string, string> = {
  FDM_PENDING:
    "La caisse attend " + "la réponse.",
};

export const VENDOR_MESSAGES: Record<string, string> = {
  "Not a valid Token": ERROR_MESSAGES.UNAUTHORIZED,
};

export function translateFdmError(raw: string | null): string | null {
  if (!raw) return null;
  for (const [code, msg] of Object.entries(POS_CHECK_MESSAGES)) {
    if (raw.includes(code)) return msg;
  }
  if (raw.includes("connect") || raw.includes("timeout")) {
    return "Connexion perdue.";
  }
  // generic fallback
  return "Erreur boîte noire.";
}

const MESSAGES_ZH: Record<string, string> = {
  FDM_PENDING: "收银机正在等待。",
  "Not a valid Token": "收银机无权通信。",
  BUFFER_FULL: "黑匣子已满。",
};

export function translateFdmErrorZh(raw: string | null): string | null {
  if (!raw) return null;
  for (const [code, msg] of Object.entries(MESSAGES_ZH)) {
    if (raw.includes(code)) return msg;
  }
  if (raw.includes("connect") || raw.includes("timeout")) {
    return "连接中断。";
  }
  return "黑匣子错误。";
}
