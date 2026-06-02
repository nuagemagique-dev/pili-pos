import { defineStore } from "pinia";
import { ref } from "vue";
import * as ipc from "../api/ipc";

// Shape of an FDM message item as returned by the SCE 2.0 spec
// (Status.warnings[i] and Status.errors[i]).
interface FdmMessage {
  message?: string;
  extensions?: {
    category?: string;
    code?: string;
    showPos?: string;
  };
}

interface FdmStatusEnvelope {
  // Set by Rust when status() fails outright (network down, GraphQL refused).
  connected?: boolean;
  error?: string;
  // Set by the FDM when the Status query succeeds.
  initialized?: boolean;
  warnings?: FdmMessage[] | null;
  errors?: FdmMessage[] | null;
  device?: {
    fdmId?: string;
    bufferCapacityUsed?: number;
  };
}

// Severity tier — what the UI does about a code.
// "blocking": red pill + checkout-blocking modal.
// "warning":  yellow pill + tooltip, no modal.
// "info":     informational, no UI change.
type Severity = "blocking" | "warning" | "info";

const BLOCKING_CODES = new Set([
  "BUFFER_FULL",
  "FDM_LOCKED",
  "FDM_NOT_OPERATIONAL",
  "UNKNOWN_POS",
]);

function pickPrimaryMessage(env: FdmStatusEnvelope): {
  severity: Severity;
  code: string | null;
} {
  // Prefer errors over warnings; within either, prefer mandatory-display codes.
  const errors = env.errors ?? [];
  const warnings = env.warnings ?? [];

  // Errors first.
  const mandatoryErr = errors.find(
    (e) => e.extensions?.showPos === "MANDATORY",
  );
  if (mandatoryErr?.extensions?.code) {
    const code = mandatoryErr.extensions.code;
    return { severity: BLOCKING_CODES.has(code) ? "blocking" : "warning", code };
  }
  const anyErr = errors.find((e) => e.extensions?.showPos !== "NEVER");
  if (anyErr?.extensions?.code) {
    const code = anyErr.extensions.code;
    return { severity: BLOCKING_CODES.has(code) ? "blocking" : "warning", code };
  }

  // Then warnings.
  const mandatoryWarn = warnings.find(
    (w) => w.extensions?.showPos === "MANDATORY",
  );
  if (mandatoryWarn?.extensions?.code) {
    return { severity: "warning", code: mandatoryWarn.extensions.code };
  }
  const anyWarn = warnings.find((w) => w.extensions?.showPos !== "NEVER");
  if (anyWarn?.extensions?.code) {
    return { severity: "warning", code: anyWarn.extensions.code };
  }

  return { severity: "info", code: null };
}

export const useFdmStore = defineStore("fdm", () => {
  const connected = ref(false);
  const checking = ref(false);
  const lastError = ref<string | null>(null); // FR message or raw code
  const severity = ref<Severity>("info");

  async function checkStatus() {
    checking.value = true;
    try {
      const status = (await ipc.checkFdmStatus()) as FdmStatusEnvelope;
      // 1) Hard transport failure (Rust returned connected=false + error string).
      if (status.connected === false || status.error) {
        connected.value = false;
        lastError.value = status.error ?? "FDM_NOT_OPERATIONAL";
        severity.value = "blocking";
        return;
      }
      // 2) Status query succeeded — inspect warnings/errors arrays.
      const { severity: sev, code } = pickPrimaryMessage(status);
      severity.value = sev;
      lastError.value = code;
      // Connected = transport OK AND no blocking condition.
      connected.value = status.initialized !== false && sev !== "blocking";
    } catch {
      connected.value = false;
      lastError.value = "FDM_NOT_OPERATIONAL";
      severity.value = "blocking";
    } finally {
      checking.value = false;
    }
  }

  return { connected, checking, lastError, severity, checkStatus };
});
