<script setup lang="ts">
import { usePosStore } from "../stores/pos";
import { useI18n } from "../i18n";

const pos = usePosStore();
const { t } = useI18n();
</script>

<template>
  <!-- UX Guide §9: impossible à confondre avec le mode normal.
       The banner takes its own row at the top of #app (a flex column),
       so the screen below starts under it and no control is hidden or
       blocked; the watermark lies over everything and lets taps through. -->
  <template v-if="pos.trainingMode">
    <div class="training-banner" role="status">
      {{ t('training_banner') }}
    </div>
    <div class="training-watermark">TRAINING</div>
  </template>
</template>

<style scoped>
.training-watermark {
  position: fixed;
  z-index: 9999;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%) rotate(-35deg);
  font-size: 120px;
  font-weight: 900;
  color: rgba(155, 89, 182, 0.08);
  white-space: nowrap;
  letter-spacing: 0.1em;
  pointer-events: none;
}

.training-banner {
  flex: none;
  background: var(--accent-training);
  color: var(--text-on-accent);
  text-align: center;
  font-weight: var(--font-weight-bold);
  font-size: var(--font-size-sm);
  padding: 6px;
  letter-spacing: 0.05em;
}
</style>
