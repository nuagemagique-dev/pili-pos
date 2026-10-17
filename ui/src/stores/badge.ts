import { defineStore } from "pinia";
import { computed, ref } from "vue";
import * as ipc from "../api/ipc";
import router from "../router";
import { useAuthStore } from "./auth";
import { useToastStore } from "./toast";
import { useBadgeCue } from "../composables/useBadgeCue";
import { useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";
import { formatTime, formatTimestamp } from "../utils/datetime";
import { goAfterLogin } from "../utils/postLogin";

/** A manager's badge on the login screen: the PIN pad opens for them. */
export interface PinRequest {
  insz: string;
  name: string;
}

/**
 * The badge reader as the screens see it (plan 4.4): the reader setting
 * (read at start-up, also with no session), the enrolment capture, the
 * end-of-shift flag of the login screen, and what a scan does.
 */
export const useBadgeStore = defineStore("badge", () => {
  const settings = ref<ipc.BadgeSettings | null>(null);
  const overview = ref<ipc.BadgeOverview | null>(null);
  // Set by LoginView while its "Fin de service" mode is on.
  const endShiftMode = ref(false);
  // Set by a manager's badge on the login screen; LoginView clears it.
  const pinRequest = ref<PinRequest | null>(null);
  // Bumped after a registered clock-in or out, for the screens that
  // show who is on shift.
  const presenceVersion = ref(0);

  // Linux has no NFC source: a stored `nfc` (a restore from other hardware)
  // reads nothing here, so it shows no badge hint nor "Lier" button either.
  const readerOn = computed(() => settings.value?.reader === "usb");
  const loginOn = computed(() => readerOn.value && settings.value?.loginEnabled === true);

  async function load() {
    try {
      settings.value = await ipc.getBadgeSettings();
    } catch {
      // No engine yet: the reader stays off.
    }
  }

  async function save(input: ipc.BadgeSettingsInput) {
    settings.value = await ipc.setBadgeSettings(input);
    await loadOverview();
  }

  /** The active badges and the log (managers only). */
  async function loadOverview() {
    try {
      overview.value = await ipc.listBadges();
    } catch {
      overview.value = null;
    }
  }

  async function link(employeeInsz: string, token: string, operatorPin: string) {
    const r = await ipc.linkBadge(employeeInsz, token, operatorPin);
    await loadOverview();
    return r;
  }

  async function revoke(employeeInsz: string) {
    await ipc.revokeBadge(employeeInsz);
    await loadOverview();
  }

  // ── Enrolment capture: the next scan is resolved here, not sent ──
  let captureResolve: ((token: string) => void) | null = null;
  const capturing = ref(false);

  /** Wait for the next scan and hand its token back (no engine call). */
  function startCapture(): { token: Promise<string>; cancel: () => void } {
    cancelCapture();
    capturing.value = true;
    const token = new Promise<string>((resolve) => {
      captureResolve = resolve;
    });
    return { token, cancel: cancelCapture };
  }

  function cancelCapture() {
    captureResolve = null;
    capturing.value = false;
  }

  // ── A scan ──
  async function onScan(token: string) {
    if (captureResolve) {
      const resolve = captureResolve;
      cancelCapture();
      resolve(token);
      return;
    }
    const { t } = useI18n();
    const toast = useToastStore();
    const cue = useBadgeCue();
    let outcome: ipc.BadgeOutcome;
    try {
      outcome = await ipc.badgeTap(token, endShiftMode.value);
    } catch (e) {
      toast.show(describeError(e), "error", 5000);
      cue.refused();
      return;
    }
    const name = outcome.name ?? "";
    const time = formatTimestamp(outcome.at ?? new Date().toISOString(), formatTime);
    switch (outcome.kind) {
      case "ignored":
        return;
      case "logged_in":
        if (outcome.user) {
          useAuthStore().currentUser = outcome.user;
          cue.ok();
          await goAfterLogin(router);
        }
        return;
      case "pin_required":
        pinRequest.value = { insz: outcome.insz ?? "", name };
        return;
      case "clock_in":
      case "clock_out":
        toast.show(t(outcome.kind === "clock_in" ? "badge_clock_in" : "badge_clock_out", { name, time }), "success", 4000);
        cue.ok();
        presenceVersion.value++;
        return;
      case "presence_failed":
        toast.show(t("badge_presence_failed", { name, error: describeError(outcome.error ?? "") }), "error", 6000);
        cue.refused();
        return;
    }
  }

  return {
    settings,
    overview,
    endShiftMode,
    pinRequest,
    presenceVersion,
    capturing,
    readerOn,
    loginOn,
    load,
    save,
    loadOverview,
    link,
    revoke,
    startCapture,
    cancelCapture,
    onScan,
  };
});
