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

// Severity tier - what the UI does about a code.
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

// One poller for the whole app, whatever the screens showing the FDM
// indicator: every 30 s while the FDM answers, backing off to 5 min
// while it does not, so a down or remote FDM is not hammered.
const POLL_OK_MS = 30_000;
const POLL_MAX_MS = 300_000;

export function pollDelay(failures: number): number {
  return failures === 0 ? POLL_OK_MS : Math.min(POLL_OK_MS * 2 ** failures, POLL_MAX_MS);
}

export const useFdmStore = defineStore("fdm", () => {
  const connected = ref(false);
  const checking = ref(false);
  const lastError = ref<string | null>(null); // FR message or raw code
  const severity = ref<Severity>("info");

  /** Query the FDM status. Returns false when the FDM did not answer
   * (transport failure), true when it answered, even with an error. */
  async function checkStatus(): Promise<boolean> {
    checking.value = true;
    try {
      const status = (await ipc.checkFdmStatus()) as FdmStatusEnvelope;
      // 1) Hard transport failure (Rust returned connected=false + error string).
      if (status.connected === false || status.error) {
        connected.value = false;
        lastError.value = status.error ?? "FDM_NOT_OPERATIONAL";
        severity.value = "blocking";
        return false;
      }
      // 2) Status query succeeded - inspect warnings/errors arrays.
      const { severity: sev, code } = pickPrimaryMessage(status);
      severity.value = sev;
      lastError.value = code;
      // Connected = transport OK AND no blocking condition.
      connected.value = status.initialized !== false && sev !== "blocking";
      return true;
    } catch {
      connected.value = false;
      lastError.value = "FDM_NOT_OPERATIONAL";
      severity.value = "blocking";
      return false;
    } finally {
      checking.value = false;
    }
  }

  // Screens showing the indicator; the poller runs while there is one.
  let subscribers = 0;
  let failures = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  // Bumped on each start and stop, so a check still in flight when
  // polling stopped (or restarted) schedules nothing.
  let generation = 0;

  async function poll(gen: number) {
    const answered = await checkStatus();
    if (gen !== generation || subscribers === 0) return;
    failures = answered ? 0 : failures + 1;
    timer = setTimeout(() => void poll(gen), pollDelay(failures));
  }

  /** A screen showing the FDM indicator appeared: check now, then poll. */
  function startPolling() {
    subscribers += 1;
    if (subscribers > 1) return;
    generation += 1;
    failures = 0;
    void poll(generation);
  }

  /** That screen went away: polling stops with the last one. */
  function stopPolling() {
    subscribers = Math.max(0, subscribers - 1);
    if (subscribers > 0) return;
    generation += 1;
    if (timer) clearTimeout(timer);
    timer = null;
  }

  return {
    connected,
    checking,
    lastError,
    severity,
    checkStatus,
    startPolling,
    stopPolling,
  };
});
