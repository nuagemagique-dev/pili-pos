import { defineStore } from "pinia";
import { ref } from "vue";
import type { User } from "../types";
import * as ipc from "../api/ipc";

export const useAuthStore = defineStore("auth", () => {
  const currentUser = ref<User | null>(null);
  const error = ref<string | null>(null);

  async function loginWithInsz(insz: string, pin?: string) {
    error.value = null;
    try {
      currentUser.value = await ipc.login(insz, pin);
      // Event S: register ONSS clock-in (best-effort, non-blocking)
      ipc.workIn(insz).catch(() => {});
    } catch (e) {
      error.value = String(e);
      throw e;
    }
  }

  function logout() {
    if (currentUser.value) {
      // Event S: register ONSS clock-out (best-effort, non-blocking)
      ipc.workOut(currentUser.value.insz).catch(() => {});
    }
    currentUser.value = null;
  }

  return { currentUser, error, loginWithInsz, logout };
});
