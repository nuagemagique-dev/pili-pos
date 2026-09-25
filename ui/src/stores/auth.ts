import { defineStore } from "pinia";
import { ref } from "vue";
import type { User } from "../types";
import * as ipc from "../api/ipc";
import { useToastStore } from "./toast";
import { useI18n } from "../i18n";

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
    useToastStore().show(`${t("presence_failed")} — ${error}`, "error", 6000);
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

  function logout() {
    if (currentUser.value) {
      void registerPresence(currentUser.value.insz, false);
    }
    currentUser.value = null;
  }

  return { currentUser, error, loginWithInsz, logout };
});
