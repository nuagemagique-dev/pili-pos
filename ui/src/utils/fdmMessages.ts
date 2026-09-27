// French messages for the FDM's error codes (certification manual §V),
// shown by the header indicator and wherever an FDM refusal is
// reported. The dossier's §V table is copied from this file.

// UX Guide §8.3: human-readable FDM error messages.
// Covers every "showPos != NEVER" code listed in the SCE 2.0
// certification manual §V (mandatory and optional displays).
export const ERROR_MESSAGES: Record<string, string> = {
  // Mandatory (M) - must be displayed on the POS
  BUFFER_FULL: "La boîte noire est pleine. Contactez votre technicien.",
  FDM_LOCKED: "La boîte noire est verrouillée. Contactez votre technicien.",
  FDM_NOT_OPERATIONAL: "La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes.",
  UNKNOWN_POS: "Cette caisse n'est pas reconnue par la boîte noire.",
  BUFFER_NEAR_FULL: "La boîte noire est presque pleine. Programmez une intervention.",
  CLIENT_CERT_EXPIRED: "Le certificat de la boîte noire a expiré. Contactez votre technicien.",
  // Mandatory warnings of the POS description v2.2 tb_messages (not in
  // the manual's §V table): the identity configured on the POS does not
  // match the one the SPF gave the FDM.
  UPDATE_POS_VATNO:
    "Le numéro de TVA de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien.",
  UPDATE_POS_ESTNO:
    "Le numéro d'unité d'établissement de la caisse ne correspond pas à celui de la boîte noire. Contactez votre technicien.",
  // Optional (O) - recommended display
  CLIENT_CERT_NEAR_EXPIRATION: "Le certificat de la boîte noire expire bientôt.",
  UNAUTHORIZED: "La caisse n'est pas autorisée à parler à la boîte noire.",
  INVALID_REQUEST: "Erreur de communication avec la boîte noire. Contactez votre technicien.",
  INTERNAL_ERROR: "Erreur technique de la boîte noire. Contactez votre technicien.",
  UNDEFINED_ERROR: "Erreur indéfinie de la boîte noire. Contactez votre technicien.",
  UNDEFINED_OTHER: "Erreur de la boîte noire. Contactez votre technicien.",
  RTC_SYNC_FAILED: "L'horloge de la boîte noire n'a pas pu se synchroniser.",
  UPDATE_URLS_FAILED: "Mise à jour des URLs de la boîte noire échouée.",
  UPDATE_TRUST_CERT_FAILED: "Mise à jour des certificats de confiance échouée.",
  UPDATE_TASK_LIST_FAILED: "Téléchargement des tâches FDM échoué.",
  TASK_FEEDBACK_FAILED: "La boîte noire n'a pas pu signaler le résultat d'une tâche.",
  NOP_FAILED: "Communication intermédiaire avec la boîte noire échouée.",
  SERVER_CERT_RESOLVE_FAILED: "Impossible de résoudre le certificat du serveur FDM.",
  TRANSACTION_UPLOAD_FAILED: "La boîte noire échoue à transmettre les transactions au SPF Finances.",
  INITIALIZATION_FAILED: "L'initialisation de la boîte noire a échoué.",
};

// Refusals detected by the POS itself, not codes of the FDM (so not
// part of the §V table).
export const POS_CHECK_MESSAGES: Record<string, string> = {
  // The FDM answered, but not to the request sent: nothing was
  // recorded (pos_fiscal::response_check).
  FDM_RESPONSE_MISMATCH:
    "Réponse de la boîte noire incohérente. Vente non enregistrée. Contactez votre technicien.",
  // The FDM's status says `initialized: false` without any message
  // (module2's online simulator always does). It still signs.
  FDM_NOT_INITIALIZED: "La boîte noire n'est pas encore initialisée.",
  // HTTPS to the FDM failed on a certificate (pos_fiscal::fdm_tls):
  // its CA, or the client certificate it asks for.
  FDM_CERTIFICATE_REFUSED:
    "Connexion sécurisée à la boîte noire refusée (certificat). Contactez votre technicien.",
};

// Refusals an FDM words without a code: module2 answers a missing or
// wrong bearer token with this bare message.
export const VENDOR_MESSAGES: Record<string, string> = {
  "Not a valid Token": ERROR_MESSAGES.UNAUTHORIZED,
};

export function translateFdmError(raw: string | null): string | null {
  if (!raw) return null;
  for (const [code, msg] of Object.entries(POS_CHECK_MESSAGES)) {
    if (raw.includes(code)) return msg;
  }
  for (const [code, msg] of Object.entries(ERROR_MESSAGES)) {
    if (raw.includes(code)) return msg;
  }
  for (const [text, msg] of Object.entries(VENDOR_MESSAGES)) {
    if (raw.includes(text)) return msg;
  }
  if (raw.includes("connect") || raw.includes("timeout")) {
    return "Connexion perdue avec la boîte noire. Vérifiez le câble réseau.";
  }
  // UX Guide §8.3: no raw technical jargon - generic fallback
  return "Erreur boîte noire. Contactez votre technicien.";
}

// Chinese line shown under the French §V message when the interface
// is in Chinese. The French text stays: it is the certified wording.
const MESSAGES_ZH: Record<string, string> = {
  FDM_RESPONSE_MISMATCH: "黑匣子回应不一致，销售未登记。请联系技术人员。",
  BUFFER_FULL: "黑匣子已满。请联系技术人员。",
  FDM_LOCKED: "黑匣子已锁定。请联系技术人员。",
  FDM_NOT_OPERATIONAL: "黑匣子无法工作，无法登记销售。",
  UNKNOWN_POS: "黑匣子不认识这台收银机。",
  BUFFER_NEAR_FULL: "黑匣子快满了。请安排维护。",
  CLIENT_CERT_EXPIRED: "黑匣子证书已过期。请联系技术人员。",
  UPDATE_POS_VATNO: "收银机的增值税号与黑匣子不一致。请联系技术人员。",
  UPDATE_POS_ESTNO: "收银机的营业单位编号与黑匣子不一致。请联系技术人员。",
  CLIENT_CERT_NEAR_EXPIRATION: "黑匣子证书即将过期。",
  TRANSACTION_UPLOAD_FAILED: "黑匣子无法向财政部传送交易。请检查网络连接。",
};

export function translateFdmErrorZh(raw: string | null): string | null {
  if (!raw) return null;
  for (const [code, msg] of Object.entries(MESSAGES_ZH)) {
    if (raw.includes(code)) return msg;
  }
  if (raw.includes("connect") || raw.includes("timeout")) {
    return "与黑匣子的连接中断。请检查网线。";
  }
  return "黑匣子错误。请联系技术人员。";
}

/** The FDM code in `raw`, for the technician (shown small). */
export function fdmCode(raw: string | null): string | null {
  if (!raw) return null;
  const known = [...Object.keys(POS_CHECK_MESSAGES), ...Object.keys(ERROR_MESSAGES)];
  return known.find((code) => raw.includes(code)) ?? null;
}
