<script setup lang="ts">
// The installation's HubRise switch (commands/integrations.rs): its
// state, the manager's button and the log of every change. Switching
// off waits until the online orders are settled (the backend says how
// many).
import { computed, onMounted, ref } from 'vue'
import { useIntegrationsStore } from '../stores/integrations'
import { useToastStore } from '../stores/toast'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'

const integrations = useIntegrationsStore()
const toast = useToastStore()
const { t } = useI18n()
const busy = ref(false)

const on = computed(() => integrations.hubriseEnabled)
const open = computed(() => integrations.state?.online_orders_open ?? 0)
const log = computed(() => integrations.state?.hubrise_log ?? [])

onMounted(() => void integrations.load())

async function toggle() {
  if (busy.value) return
  busy.value = true
  try {
    await integrations.setHubrise(!on.value)
  } catch (e) {
    toast.show(describeError(e), 'error')
  } finally {
    busy.value = false
  }
}

function when(at: string): string {
  // SQLite datetime('now') is UTC without a zone.
  const d = new Date(at.includes('T') ? at : `${at.replace(' ', 'T')}Z`)
  return d.toLocaleString('fr-BE', {
    timeZone: 'Europe/Brussels',
    dateStyle: 'short',
    timeStyle: 'short',
  })
}
</script>

<template>
  <section class="integrations" data-testid="integrations">
    <div class="panel-head">
      <h2>{{ t('integrations_title') }}</h2>
    </div>
    <div class="row">
      <div class="what">
        <span class="name">HubRise</span>
        <span class="pill" :class="on ? 'is-on' : 'is-off'"
          ><span class="dot"></span>{{ on ? t('integrations_on') : t('integrations_off') }}</span
        >
      </div>
      <button class="switch-btn" :class="{ on }" :disabled="busy" @click="toggle">
        {{ on ? t('integrations_turn_off') : t('integrations_turn_on') }}
      </button>
    </div>
    <p class="panel-text">{{ t('integrations_hubrise_text') }}</p>
    <p v-if="open > 0" class="settle">{{ t('integrations_settle_first', { count: open }) }}</p>
    <div v-if="log.length" class="log">
      <div class="section-label">{{ t('integrations_log') }}</div>
      <ul>
        <li v-for="(c, i) in log.slice(0, 5)" :key="i">
          <span class="log-when">{{ when(c.changed_at) }}</span>
          <span>{{ c.enabled ? t('integrations_on') : t('integrations_off') }}</span>
          <span class="log-who">{{
            c.source === 'admin'
              ? t('integrations_source_admin', { who: c.changed_by ?? '' })
              : t('integrations_source_config')
          }}</span>
        </li>
      </ul>
    </div>
  </section>
</template>

<style scoped>
.panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
}
.what {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}
.name {
  font-weight: var(--font-weight-bold);
}
.pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: var(--font-size-sm);
}
.pill .dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}
.is-on {
  color: var(--accent-success);
}
.is-off {
  color: var(--text-secondary);
}
.switch-btn {
  min-height: 48px;
  min-width: 120px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-weight: var(--font-weight-bold);
  font-size: var(--font-size-md);
}
.switch-btn.on {
  border-color: var(--accent-danger);
  color: var(--accent-danger);
}
.switch-btn:disabled {
  opacity: 0.5;
}
.settle {
  color: var(--accent-warning);
  font-weight: var(--font-weight-bold);
}
.log ul {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: var(--space-xs);
}
.log li {
  display: flex;
  gap: var(--space-sm);
  font-size: var(--font-size-sm);
}
.log-when {
  font-family: var(--font-mono);
  color: var(--text-secondary);
}
.log-who {
  color: var(--text-secondary);
}
</style>
