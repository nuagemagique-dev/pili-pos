import { defineStore } from "pinia";
import { computed, ref } from "vue";
import * as ipc from "../api/ipc";

/**
 * The installation's HubRise switch (commands/integrations.rs). The
 * integration ships in every build; this says whether its screens are
 * shown. The online orders board stays reachable while orders taken
 * before a switch-off are still to settle.
 */
export const useIntegrationsStore = defineStore("integrations", () => {
  const state = ref<ipc.Integrations | null>(null);

  const hubriseEnabled = computed(() => state.value?.hubrise_enabled ?? false);
  const hubriseVisible = computed(
    () => hubriseEnabled.value || (state.value?.online_orders_open ?? 0) > 0,
  );

  async function load() {
    try {
      state.value = await ipc.getIntegrations();
    } catch {
      state.value = null;
    }
  }

  async function setHubrise(enabled: boolean) {
    state.value = await ipc.setHubriseEnabled(enabled);
  }

  return { state, hubriseEnabled, hubriseVisible, load, setHubrise };
});
