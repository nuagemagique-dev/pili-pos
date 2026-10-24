<script setup lang="ts">
import { computed } from 'vue'
import { usePosStore } from '../stores/pos'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'

const pos = usePosStore()
const { t } = useI18n()

// pos.json holds an identity the FDM would refuse: every sale would
// fail. Each problem is a coded error (`config_<setting>|detail=...`):
// its text is in the screen's language and ends with the English detail
// for the installer.
const problems = computed(() => pos.settings?.configProblems ?? [])
</script>

<template>
  <div v-if="problems.length" class="config-problems" role="alert">
    <strong>{{ t('config_problems') }}</strong>
    <ul>
      <li v-for="p in problems" :key="p">{{ describeError(p) }}</li>
    </ul>
  </div>
</template>

<style scoped>
.config-problems {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 10001;
  background: var(--accent-danger);
  color: var(--text-on-accent);
  padding: 8px 16px;
  font-size: var(--font-size-sm);
}

.config-problems ul {
  margin: 4px 0 0;
  padding-left: 20px;
  font-family: monospace;
}
</style>
