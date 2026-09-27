import { defineStore } from "pinia";
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import * as ipc from "../api/ipc";
import { ERROR_MESSAGES } from "../utils/fdmMessages";
import { useToastStore } from "./toast";

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

// Messages the FDM attached to a signed event (Rust
// `pos_fiscal::fdm_client::SignMessages`, event "fdm-sign-messages"),
// showPos NEVER already removed.
export interface SignMessages {
  mutation: string;
  bookingDate?: string | null;
  warnings: FdmMessage[];
  informations: FdmMessage[];
}

const SIGN_MESSAGE_TTL_MS = 8000;

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
  mandatory: boolean;
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
    return { severity: BLOCKING_CODES.has(code) ? "blocking" : "warning", code, mandatory: true };
  }
  const anyErr = errors.find((e) => e.extensions?.showPos !== "NEVER");
  if (anyErr?.extensions?.code) {
    const code = anyErr.extensions.code;
    return { severity: BLOCKING_CODES.has(code) ? "blocking" : "warning", code, mandatory: false };
  }

  // Then warnings.
  const mandatoryWarn = warnings.find(
    (w) => w.extensions?.showPos === "MANDATORY",
  );
  if (mandatoryWarn?.extensions?.code) {
    return { severity: "warning", code: mandatoryWarn.extensions.code, mandatory: true };
  }
  const anyWarn = warnings.find((w) => w.extensions?.showPos !== "NEVER");
  if (anyWarn?.extensions?.code) {
    return { severity: "warning", code: anyWarn.extensions.code, mandatory: false };
  }

  return { severity: "info", code: null, mandatory: false };
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
  // The FDM asks the POS to keep showing this message (showPos
  // MANDATORY, certification manual §V): it must not auto-dismiss.
  const mandatory = ref(false);

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
        mandatory.value = true;
        return false;
      }
      // 2) Status query succeeded - inspect warnings/errors arrays.
      const { severity: sev, code, mandatory: keep } = pickPrimaryMessage(status);
      severity.value = sev;
      // An FDM that says it is not initialized, with nothing more to
      // say, leaves the indicator amber: name the reason.
      lastError.value = code ?? (status.initialized === false ? "FDM_NOT_INITIALIZED" : null);
      mandatory.value = keep;
      // Connected = transport OK AND no blocking condition.
      connected.value = status.initialized !== false && sev !== "blocking";
      return true;
    } catch {
      connected.value = false;
      lastError.value = "FDM_NOT_OPERATIONAL";
      severity.value = "blocking";
      mandatory.value = true;
      return false;
    } finally {
      checking.value = false;
    }
  }

  // Screens showing the indicator; the poller runs while there is one.
  let subscribers = 0;
  let running = false;
  let failures = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  // Bumped on each start and stop, so a check still in flight when
  // polling stopped (or restarted) schedules nothing.
  let generation = 0;
  /** When the poller checks next (ms since epoch), for the countdown on
   * the blocking veil; null while it is not polling. */
  const nextCheckAt = ref<number | null>(null);

  async function poll(gen: number) {
    nextCheckAt.value = null;
    const answered = await checkStatus();
    if (gen !== generation || !running) return;
    failures = answered ? 0 : failures + 1;
    const delay = pollDelay(failures);
    nextCheckAt.value = Date.now() + delay;
    timer = setTimeout(() => void poll(gen), delay);
  }

  /** A screen showing the FDM indicator appeared: check now, then poll. */
  function startPolling() {
    subscribers += 1;
    if (running) return;
    running = true;
    generation += 1;
    failures = 0;
    void poll(generation);
  }

  /** That screen went away: polling stops with the last one. Every
   * screen shows the indicator, so the stop waits for the end of the
   * screen change: the next screen takes over the same poller. */
  function stopPolling() {
    subscribers = Math.max(0, subscribers - 1);
    if (subscribers > 0) return;
    queueMicrotask(() => {
      if (subscribers > 0 || !running) return;
      running = false;
      generation += 1;
      if (timer) clearTimeout(timer);
      timer = null;
      nextCheckAt.value = null;
    });
  }

  // POS description v2.2, Message: a warning received with a
  // mutation's response is shown at least once per bookingDate. Each
  // code is shown once per bookingDate, whichever event carried it.
  const shownSignMessages = new Set<string>();

  function showSignMessages(signed: SignMessages) {
    const toast = useToastStore();
    const groups: Array<[FdmMessage[], "warning" | "info"]> = [
      [signed.warnings ?? [], "warning"],
      [signed.informations ?? [], "info"],
    ];
    for (const [items, variant] of groups) {
      for (const item of items) {
        const code = item.extensions?.code;
        if (!code) continue;
        const key = `${signed.bookingDate ?? ""}|${code}`;
        if (shownSignMessages.has(key)) continue;
        shownSignMessages.add(key);
        const text = ERROR_MESSAGES[code] ?? item.message ?? code;
        toast.show(text, variant, SIGN_MESSAGE_TTL_MS);
        // The header indicator shows it too until the next status.
        if (variant === "warning" && severity.value === "info") {
          severity.value = "warning";
          lastError.value = code;
        }
      }
    }
  }

  let signMessagesListened = false;

  /** Show the messages of every signed event; call once at start-up. */
  async function listenSignMessages() {
    if (signMessagesListened) return;
    signMessagesListened = true;
    await listen<SignMessages>("fdm-sign-messages", (event) => showSignMessages(event.payload));
  }

  return {
    connected,
    checking,
    lastError,
    severity,
    mandatory,
    nextCheckAt,
    checkStatus,
    startPolling,
    stopPolling,
    showSignMessages,
    listenSignMessages,
  };
});
