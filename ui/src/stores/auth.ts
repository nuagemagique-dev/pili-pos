import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { User } from "../types";
import * as ipc from "../api/ipc";
import { useToastStore } from "./toast";
import { useI18n } from "../i18n";
import { describeError } from "../utils/fdmMessages";

/** Event S (ONSS presence). Never blocks the login or logout, but an
 * unregistered clock-in/out is reported so it can be fixed. */
async function registerPresence(insz: string, isIn: boolean) {
  let error: string | null;
  try {
    const result = isIn ? await ipc.workIn(insz) : await ipc.workOut(insz);
    error = result.success ? null : (result.error ?? "FDM error");
  } catch (e) {
    error = String(e);
  }
  if (error) {
    const { t } = useI18n();
    useToastStore().show(`${t("presence_failed")} : ${describeError(error)}`, "error", 6000);
  }
}

export const useAuthStore = defineStore("auth", () => {
  const currentUser = ref<User | null>(null);
  const error = ref<string | null>(null);

  async function loginWithInsz(insz: string, pin?: string) {
    error.value = null;
    try {
      currentUser.value = await ipc.login(insz, pin);
      void registerPresence(insz, true);
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  /** Clock out, then end the backend session (in that order: the
   * clock-out is signed in the operator's name). */
  async function logout() {
    const user = currentUser.value;
    currentUser.value = null;
    if (user) await registerPresence(user.insz, false);
    try {
      await ipc.logout();
    } catch {
      /* the next login replaces the session anyway */
    }
  }

  // What the role may do (PROFILS_UTILISATEURS_FR.md §2). The backend
  // enforces it; the UI only hides what would be refused.
  const role = computed(() => currentUser.value?.role);
  const canTakePayment = computed(() => role.value === "admin" || role.value === "cashier");
  const isManager = computed(() => role.value === "admin");

  return { currentUser, error, loginWithInsz, logout, canTakePayment, isManager };
});
