<script setup lang="ts">
import { onMounted, onUnmounted, watchEffect } from "vue";
import { useRoute, useRouter } from "vue-router";
import { usePosStore } from "./stores/pos";
import { useAuthStore } from "./stores/auth";
import * as ipc from "./api/ipc";
import { useOrderChime } from "./composables/useOrderChime";
import { useBadgeReader } from "./composables/useBadgeReader";
import { useBadgeStore } from "./stores/badge";
import TrainingBanner from "./components/TrainingBanner.vue";
import ToastStack from "./components/ToastStack.vue";
import ConfigProblemsBanner from "./components/ConfigProblemsBanner.vue";
import SupportBanner from "./components/SupportBanner.vue";
import EftOverlay from "./components/EftOverlay.vue";
import { useSupportStore } from "./stores/support";
import { useFdmStore } from "./stores/fdm";
import { useOrderStore } from "./stores/order";
import { useToastStore } from "./stores/toast";
import { useI18n } from "./i18n";
import { describeError } from "./utils/errorMessages";

const pos = usePosStore();
const auth = useAuthStore();
const router = useRouter();
const route = useRoute();

// Training mode (UX guide §9): violet tokens on every screen.
watchEffect(() => {
  document.documentElement.classList.toggle("training", pos.trainingMode);
});

// Two-tone chime when a new webshop order arrives (C5 → E5); none
// arrives with HubRise switched off.
useOrderChime();

// The 5 minute idle lock belongs to the engine (it registers the open
// order and emits `session-locked`, handled below). The UI only tells it
// the operator is active, at most once per second.
const ACTIVITY_EVENTS = ["touchstart", "mousedown", "keydown"] as const;
const ACTIVITY_THROTTLE_MS = 1000;
let lastActivity = Number.NEGATIVE_INFINITY;

function forwardActivity() {
  const now = Date.now();
  if (now - lastActivity < ACTIVITY_THROTTLE_MS) return;
  lastActivity = now;
  try {
    ipc.userActivity().catch(() => {});
  } catch {
    /* no engine to tell */
  }
}

// USB badge reader: its capture listener is installed while the reader
// is `usb`, before any other keyboard listener, and swallows the scans so
// they are neither typed nor activity. Keys it held that were typing
// after all count as activity here.
const badge = useBadgeStore();
useBadgeReader(forwardActivity);

// Remote support banner: polled on every screen, logged in or not.
const support = useSupportStore();
let supportPoll: ReturnType<typeof setInterval> | undefined;

const fdm = useFdmStore();
const order = useOrderStore();
const toast = useToastStore();
const { t } = useI18n();

onMounted(() => {
  pos.loadSettings();
  void badge.load();
  void fdm.listenSignMessages();
  void fdm.listenStatus();
  void auth.listenPresenceFailed();
  for (const evt of ACTIVITY_EVENTS) document.addEventListener(evt, forwardActivity, { passive: true });
  // The engine locked the session (the order is already registered
  // there): forget the local order and go back to the login screen.
  void auth.listenSessionLocked((e) => {
    order.resetLocal();
    if (e.orderError) {
      toast.show(`${t("order_register_failed")} : ${describeError(e.orderError)}`, "error", 6000);
    }
    void router.push({ name: "login" });
  });
  const pollSupport = () => void (auth.currentUser ? support.refresh() : support.refreshRunning());
  pollSupport();
  supportPoll = setInterval(pollSupport, 5000);
});

onUnmounted(() => {
  clearInterval(supportPoll);
  for (const evt of ACTIVITY_EVENTS) document.removeEventListener(evt, forwardActivity);
});
</script>

<template>
  <!-- Staff screens show the banner under their header (AppHeader). -->
  <TrainingBanner v-if="route.name === 'login'" />
  <RouterView />
  <div v-if="pos.trainingMode" class="training-watermark" aria-hidden="true">TRAINING</div>
  <EftOverlay />
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
