// French messages for the FDM's error codes (certification manual §V),
// shown by the header indicator and wherever an FDM refusal is
// reported. The dossier's §V table is copied from this file.

// UX Guide §8.3: human-readable FDM error messages.
// Covers every "showPos != NEVER" code listed in the SCE 2.0
// certification manual §V (mandatory and optional displays).
export const ERROR_MESSAGES: Record<string, string> = {
  // Mandatory (M) — must be displayed on the POS
  BUFFER_FULL: "La boîte noire est pleine. Contactez votre technicien.",
  FDM_LOCKED: "La boîte noire est verrouillée. Contactez votre technicien.",
  FDM_NOT_OPERATIONAL: "La boîte noire ne fonctionne pas. Impossible d'enregistrer les ventes.",
  UNKNOWN_POS: "Cette caisse n'est pas reconnue par la boîte noire.",
  BUFFER_NEAR_FULL: "La boîte noire est presque pleine. Programmez une intervention.",
  CLIENT_CERT_EXPIRED: "Le certificat de la boîte noire a expiré. Contactez votre technicien.",
  // Optional (O) — recommended display
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

export function translateFdmError(raw: string | null): string | null {
  if (!raw) return null;
  for (const [code, msg] of Object.entries(ERROR_MESSAGES)) {
    if (raw.includes(code)) return msg;
  }
  if (raw.includes("connect") || raw.includes("timeout")) {
    return "Connexion perdue avec la boîte noire. Vérifiez le câble réseau.";
  }
  // UX Guide §8.3: no raw technical jargon — generic fallback
  return "Erreur boîte noire. Contactez votre technicien.";
}

/** A backend error as the operator should read it: an FDM failure
 * (the backend prefixes those with "FDM") in plain French, anything
 * else (access, validation) as it is. */
export function describeError(raw: unknown): string {
  const text = String(raw ?? "");
  if (/^FDM\b/.test(text)) return translateFdmError(text) ?? text;
  return text;
}
