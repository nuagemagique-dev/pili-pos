<script setup lang="ts">
import { onMounted, onUnmounted, watchEffect } from "vue";
import { useRoute, useRouter } from "vue-router";
import { usePosStore } from "./stores/pos";
import { useAuthStore } from "./stores/auth";
import { useIdleTimer } from "./composables/useIdleTimer";
import { useOrderChime } from "./composables/useOrderChime";
import TrainingBanner from "./components/TrainingBanner.vue";
import ToastStack from "./components/ToastStack.vue";
import ConfigProblemsBanner from "./components/ConfigProblemsBanner.vue";
import SupportBanner from "./components/SupportBanner.vue";
import { useSupportStore } from "./stores/support";
import { useFdmStore } from "./stores/fdm";

const pos = usePosStore();
const auth = useAuthStore();
const router = useRouter();
const route = useRoute();

// Training mode (UX guide §9): violet tokens on every screen.
watchEffect(() => {
  document.documentElement.classList.toggle("training", pos.trainingMode);
});

// Two-tone chime when a new webshop order arrives (C5 → E5).
if (__HUBRISE_ENABLED__) useOrderChime();

// Auto-lock after 5 minutes of inactivity - redirects to login
// silently. Prevents unauthorized POS use when the operator walks
// away from the terminal. Only fires if someone is actually logged in.
// Like the lock button, it keeps the operator on shift.
useIdleTimer(5 * 60 * 1000, () => {
  if (auth.currentUser) {
    // Navigate first: the router guard registers the open order (P)
    // while the operator's session is still open.
    void router.push({ name: "login" }).then(() => auth.lock());
  }
});

// Remote support banner: polled while someone is logged in.
const support = useSupportStore();
let supportPoll: ReturnType<typeof setInterval> | undefined;

const fdm = useFdmStore();

onMounted(() => {
  pos.loadSettings();
  void fdm.listenSignMessages();
  supportPoll = setInterval(() => {
    if (auth.currentUser) void support.refresh();
    else support.status = null;
  }, 5000);
});

onUnmounted(() => clearInterval(supportPoll));
</script>

<template>
  <!-- Staff screens show the banner under their header (AppHeader). -->
  <TrainingBanner v-if="route.name === 'login'" />
  <RouterView />
  <div v-if="pos.trainingMode" class="training-watermark" aria-hidden="true">TRAINING</div>
  <ToastStack />
  <ConfigProblemsBanner />
  <SupportBanner />
</template>

<style>
/* UX guide §9: "TRAINING" across the whole screen, taps go through. */
.training-watermark {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  pointer-events: none;
  font-family: var(--font-brand);
  font-size: 360px;
  font-weight: 900;
  letter-spacing: 0.04em;
  color: rgba(255, 255, 255, 0.06);
  white-space: nowrap;
  transform: rotate(-24deg);
}
</style>
