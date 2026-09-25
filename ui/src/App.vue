<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { usePosStore } from "./stores/pos";
import { useAuthStore } from "./stores/auth";
import { useIdleTimer } from "./composables/useIdleTimer";
import { useOrderChime } from "./composables/useOrderChime";
import { HUBRISE_ENABLED } from "./features";
import TrainingBanner from "./components/TrainingBanner.vue";
import ToastStack from "./components/ToastStack.vue";
import ConfigProblemsBanner from "./components/ConfigProblemsBanner.vue";

const pos = usePosStore();
const auth = useAuthStore();
const router = useRouter();

// Two-tone chime when a new webshop order arrives (C5 → E5).
if (HUBRISE_ENABLED) useOrderChime();

// Auto-lock after 5 minutes of inactivity — redirects to login
// silently. Prevents unauthorized POS use when the operator walks
// away from the terminal. Only fires if someone is actually logged in.
useIdleTimer(5 * 60 * 1000, () => {
  if (auth.currentUser) {
    // Navigate first: the router guard registers the open order (P)
    // while the operator's session is still open.
    void router.push({ name: "login" }).then(() => auth.logout());
  }
});

onMounted(() => {
  pos.loadSettings();
});
</script>

<template>
  <TrainingBanner />
  <RouterView />
  <ToastStack />
  <ConfigProblemsBanner />
</template>
