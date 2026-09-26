import { defineStore } from "pinia";
import { ref } from "vue";
import * as ipc from "../api/ipc";

/** Remote support session (commands/support.rs). Polled while someone
 * is logged in, so every screen shows the banner while it runs. */
export const useSupportStore = defineStore("support", () => {
  const status = ref<ipc.SupportStatus | null>(null);

  async function refresh() {
    try {
      status.value = await ipc.remoteSupportStatus();
    } catch {
      status.value = null;
    }
  }

  async function start(insz: string, pin: string) {
    status.value = await ipc.startRemoteSupport(insz, pin);
  }

  async function stop() {
    status.value = await ipc.stopRemoteSupport();
  }

  return { status, refresh, start, stop };
});
