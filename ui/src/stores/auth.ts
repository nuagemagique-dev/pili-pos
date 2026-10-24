import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { User } from "../types";
import * as ipc from "../api/ipc";
import { useToastStore } from "./toast";
import { useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";

export const useAuthStore = defineStore("auth", () => {
  const currentUser = ref<User | null>(null);
  const error = ref<string | null>(null);

  async function loginWithInsz(insz: string, pin?: string) {
    error.value = null;
    try {
      currentUser.value = await ipc.login(insz, pin);
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  /** First login with a PIN chosen now (a manager must have one). */
  async function loginWithNewPin(insz: string, pin: string) {
    error.value = null;
    try {
      currentUser.value = await ipc.loginWithNewPin(insz, pin);
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  // True while lock() runs: its own `session-locked` event is handled by
  // the caller, not by the listener.
  let locking = false;

  /** Lock the screen through the engine: it registers the open order
   * (it needs the open session to sign the P), ends the session and
   * keeps the operator on shift. A failed order registration comes back
   * in the outcome and never blocks the lock. */
  async function lock(): Promise<ipc.LockOutcome> {
    locking = true;
    try {
      const outcome = await ipc.lockSession();
      currentUser.value = null;
      return outcome;
    } catch {
      currentUser.value = null;
      try {
        await ipc.logout();
      } catch {
        /* the next login replaces the session anyway */
      }
      return { orderError: null };
    } finally {
      locking = false;
    }
  }

  let sessionLockedListened = false;

  /** The engine locked the session by itself (idle): drop the operator
   * and let the caller leave the screen. Call once at start-up. */
  async function listenSessionLocked(onLocked: (e: ipc.SessionLocked) => void) {
    if (sessionLockedListened) return;
    sessionLockedListened = true;
    await listen<ipc.SessionLocked>("session-locked", (event) => {
      if (locking || !currentUser.value) return;
      currentUser.value = null;
      onLocked(event.payload);
    });
  }

  let presenceFailedListened = false;

  /** The engine clocks the employee in itself after a login; when that
   * fails it says so here. Never blocks anything. Call once at start-up. */
  async function listenPresenceFailed() {
    if (presenceFailedListened) return;
    presenceFailedListened = true;
    await listen<ipc.PresenceFailed>("presence-failed", (event) => {
      const { t } = useI18n();
      useToastStore().show(`${t("presence_failed")} : ${describeError(event.payload.error)}`, "error", 6000);
    });
  }

  /** End the employee's shift from the login screen, through the engine:
   * it checks the PIN (a refused PIN is thrown for the PIN pad to show),
   * signs the clock-out (S) and always ends the session. Returns whether
   * the clock-out was registered. */
  async function endShift(insz: string, pin?: string): Promise<boolean> {
    error.value = null;
    const r = await ipc.endShift(insz, pin);
    currentUser.value = null;
    if (!r.success) {
      const { t } = useI18n();
      useToastStore().show(`${t("presence_failed")} : ${describeError(r.error ?? "FDM error")}`, "error", 6000);
    }
    return r.success;
  }

  // What the role may do (PROFILS_UTILISATEURS_FR.md §2). The backend
  // enforces it; the UI only hides what would be refused.
  const role = computed(() => currentUser.value?.role);
  const canTakePayment = computed(() => role.value === "admin" || role.value === "cashier");
  const isManager = computed(() => role.value === "admin");

  return {
    currentUser,
    error,
    loginWithInsz,
    loginWithNewPin,
    lock,
    listenSessionLocked,
    listenPresenceFailed,
    endShift,
    canTakePayment,
    isManager,
  };
});
