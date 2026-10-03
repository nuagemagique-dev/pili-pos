import { defineStore } from "pinia";
import { ref } from "vue";
import * as ipc from "../api/ipc";

/** Remote support session (commands/support.rs). Polled on every
 * screen, the login screen included, so the banner shows while it runs. */
export const useSupportStore = defineStore("support", () => {
  const status = ref<ipc.SupportStatus | null>(null);

  async function refresh() {
    try {
      status.value = await ipc.remoteSupportStatus();
    } catch {
      status.value = null;
    }
  }

  /** Logged out: only whether a session runs, not who started it. */
  async function refreshRunning() {
    try {
      const seen = await ipc.remoteSupportRunning();
      status.value = { available: true, running: seen.running, startedBy: null, startedAt: seen.startedAt };
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

  return { status, refresh, refreshRunning, start, stop };
});
