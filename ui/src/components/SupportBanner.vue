<script setup lang="ts">
// Shown on every screen while a remote support session runs, so the
// staff know someone may be watching or acting on the terminal.
import { computed } from "vue";
import { useSupportStore } from "../stores/support";
import { useI18n } from "../i18n";

const support = useSupportStore();
const { t } = useI18n();
const running = computed(() => support.status?.running === true);
</script>

<template>
  <div v-if="running" class="support-banner" role="status">
    {{ t('support_banner', { name: support.status?.startedBy ?? '' }) }}
  </div>
</template>

<style scoped>
.support-banner {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 10001;
  padding: 6px;
  text-align: center;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  letter-spacing: 0.05em;
  background: var(--accent-danger);
  color: var(--text-on-accent);
}
</style>
