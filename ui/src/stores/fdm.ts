import { defineStore } from "pinia";
import { ref } from "vue";
import * as ipc from "../api/ipc";

export const useFdmStore = defineStore("fdm", () => {
  const connected = ref(false);
  const checking = ref(false);
  const lastError = ref<string | null>(null);

  async function checkStatus() {
    checking.value = true;
    try {
      const status = (await ipc.checkFdmStatus()) as Record<string, unknown>;
      connected.value = status.connected !== false && !status.error;
      lastError.value = (status.error as string) ?? null;
    } catch {
      connected.value = false;
      lastError.value = "Failed to check FDM status";
    } finally {
      checking.value = false;
    }
  }

  return { connected, checking, lastError, checkStatus };
});
