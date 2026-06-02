<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useFdmStore } from "../stores/fdm";
import { useI18n } from "../i18n";

const fdm = useFdmStore();
const { t } = useI18n();
const displayMessage = ref<string | null>(null);
let interval: ReturnType<typeof setInterval>;

// UX Guide §4.2: no continuous loops. Pulse once on transition
// into a degraded state (warn/err) to catch the operator's eye,
// then leave the dot in its static color as the persistent cue.
type FdmVisualState = "ok" | "warn" | "err";
const visualState = computed<FdmVisualState>(() => {
  if (fdm.severity === "blocking") return "err";
  if (fdm.severity === "warning" || !fdm.connected) return "warn";
  return "ok";
});
const pulseActive = ref(false);
watch(visualState, (next, prev) => {
  if (next !== prev && (next === "warn" || next === "err")) {
    pulseActive.value = true;
    setTimeout(() => {
      pulseActive.value = false;
    }, 180);
  }
});

// UX Guide §8.3: human-readable FDM error messages.
// Covers every "showPos != NEVER" code listed in the SCE 2.0
// certification manual §V (mandatory and optional displays).
const ERROR_MESSAGES: Record<string, string> = {
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

function translateError(raw: string | null): string | null {
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

onMounted(async () => {
  await fdm.checkStatus();
  displayMessage.value = translateError(fdm.lastError);
  interval = setInterval(async () => {
    await fdm.checkStatus();
    displayMessage.value = translateError(fdm.lastError);
  }, 30_000);
});

onUnmounted(() => clearInterval(interval));
</script>

<template>
  <!-- UX Guide §8.1: permanent FDM indicator in header -->
  <div class="fdm" :class="visualState">
    <span class="dot" :class="{ pulse: pulseActive }"></span>
    <span class="label">{{ t('status_register') }}</span>
    <span v-if="displayMessage" class="msg">{{ displayMessage }}</span>
  </div>
</template>

<style scoped>
.fdm {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.ok .dot { background: var(--accent-success); }
.warn .dot { background: var(--accent-warning); }
.err .dot { background: var(--accent-danger); }

/* One-shot scale pulse fired on transition into warn/err. §4.2
   allows informative animations ≤ 200 ms — no `infinite`. */
.dot.pulse { animation: fdm-pulse 150ms ease-out; }

.msg {
  color: var(--accent-warning);
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@keyframes fdm-pulse {
  0%   { transform: scale(1); }
  50%  { transform: scale(1.6); }
  100% { transform: scale(1); }
}
</style>
