<script setup lang="ts">
import { usePosStore } from '../stores/pos'
import { useI18n } from '../i18n'

const pos = usePosStore()
const { t } = useI18n()
</script>

<template>
  <!-- UX guide §9: impossible to mistake for the normal mode. The
       banner takes its own row, so nothing below it is hidden; the
       watermark (App.vue) lies over everything and lets taps through. -->
  <div v-if="pos.trainingMode" class="training-banner" role="status">
    {{ t('training_banner') }}
  </div>
</template>

<style scoped>
.training-banner {
  height: 48px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--accent-training);
  color: #ffffff;
  font-size: 18px;
  font-weight: var(--font-weight-heavy);
  letter-spacing: 0.04em;
}
</style>
