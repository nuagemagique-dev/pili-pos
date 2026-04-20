<script setup lang="ts">
import { usePosStore } from "../stores/pos";
import { useI18n } from "../i18n";

const pos = usePosStore();
const { t } = useI18n();
</script>

<template>
  <!-- UX Guide §9: impossible à confondre avec le mode normal -->
  <div v-if="pos.trainingMode" class="training-overlay">
    <div class="training-watermark">TRAINING</div>
    <div class="training-banner">
      {{ t('training_banner') }}
    </div>
  </div>
</template>

<style scoped>
.training-overlay {
  position: fixed;
  inset: 0;
  pointer-events: none;
  z-index: 9999;
}

.training-watermark {
  position: absolute;
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
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  background: var(--accent-training);
  color: var(--text-on-accent);
  text-align: center;
  font-weight: var(--font-weight-bold);
  font-size: var(--font-size-sm);
  padding: 6px;
  letter-spacing: 0.05em;
  pointer-events: auto;
  z-index: 10000;
}
</style>
