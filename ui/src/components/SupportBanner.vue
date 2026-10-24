<script setup lang="ts">
// Shown on every screen while a remote support session runs, so the
// staff know someone may be watching or acting on the terminal. Logged
// out, anyone may stop it here.
import { computed } from 'vue'
import { useSupportStore } from '../stores/support'
import { useAuthStore } from '../stores/auth'
import { useI18n } from '../i18n'

const support = useSupportStore()
const auth = useAuthStore()
const { t } = useI18n()
const running = computed(() => support.status?.running === true)
const loggedOut = computed(() => !auth.currentUser)

async function stop() {
  try {
    await support.stop()
  } catch {
    // The next poll shows whether it still runs.
  }
}
</script>

<template>
  <div v-if="running" class="support-banner" role="status">
    <span>
      {{
        support.status?.startedBy
          ? t('support_banner', { name: support.status.startedBy })
          : t('support_banner_anonymous')
      }}
    </span>
    <button v-if="loggedOut" type="button" class="stop" @click="stop">
      {{ t('support_stop') }}
    </button>
  </div>
</template>

<style scoped>
.support-banner {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 10001;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-md);
  padding: 6px;
  text-align: center;
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  letter-spacing: 0.05em;
  background: var(--accent-danger);
  color: var(--text-on-accent);
}

.stop {
  min-height: 48px;
  padding: 0 var(--space-md);
  border: 2px solid var(--text-on-accent);
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-on-accent);
  font: inherit;
}
</style>
