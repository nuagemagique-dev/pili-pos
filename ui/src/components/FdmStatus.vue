<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useFdmStore } from "../stores/fdm";
import { useI18n } from "../i18n";
import { translateFdmError as translateError } from "../utils/fdmMessages";

const fdm = useFdmStore();
const { t } = useI18n();
const displayMessage = computed(() => translateError(fdm.lastError));

// UX Guide §4.2: no continuous loops. Pulse once on transition
// into a degraded state (warn/err) to catch the operator's eye,
// then leave the dot in its static color as the persistent cue.
type FdmVisualState = "ok" | "warn" | "err";
const visualState = computed<FdmVisualState>(() => {
  if (fdm.severity === "blocking") return "err";
  if (fdm.severity === "warning" || !fdm.connected) return "warn";
  return "ok";
});
const pulseActive = ref(false);
watch(visualState, (next, prev) => {
  if (next !== prev && (next === "warn" || next === "err")) {
    pulseActive.value = true;
    setTimeout(() => {
      pulseActive.value = false;
    }, 180);
  }
});


// The store runs one poller for every screen showing this indicator.
onMounted(() => fdm.startPolling());
onUnmounted(() => fdm.stopPolling());
</script>

<template>
  <!-- UX Guide §8.1: permanent FDM indicator in header -->
  <div class="fdm" :class="visualState">
    <span class="dot" :class="{ pulse: pulseActive }"></span>
    <span class="label">{{ t('status_register') }}</span>
    <span v-if="displayMessage" class="msg">{{ displayMessage }}</span>
  </div>
</template>

<style scoped>
.fdm {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.ok .dot { background: var(--accent-success); }
.warn .dot { background: var(--accent-warning); }
.err .dot { background: var(--accent-danger); }

/* One-shot scale pulse fired on transition into warn/err. §4.2
   allows informative animations ≤ 200 ms - no `infinite`. */
.dot.pulse { animation: fdm-pulse 150ms ease-out; }

.msg {
  color: var(--accent-warning);
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* A blocking state reads red, like its dot. */
.err .msg { color: var(--accent-danger); }

@keyframes fdm-pulse {
  0%   { transform: scale(1); }
  50%  { transform: scale(1.6); }
  100% { transform: scale(1); }
}
</style>
