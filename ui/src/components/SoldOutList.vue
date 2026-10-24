<script setup lang="ts">
// "Produits épuisés": every product sold out now, each one re-enabled with one tap.
import { onMounted, ref } from 'vue'
import { useMenuStore } from '../stores/menu'
import { useI18n } from '../i18n'
import { endOfDayLabel } from '../utils/soldOut'

const emit = defineEmits<{ done: [] }>()

const { t } = useI18n()
const menu = useMenuStore()
const busy = ref<number | null>(null)

onMounted(() => menu.loadSoldOut())

async function reenable(menuItemId: number) {
  if (busy.value !== null) return
  busy.value = menuItemId
  await menu.setAvailability(menuItemId, 'available')
  busy.value = null
}
</script>

<template>
  <div class="backdrop" @click.self="emit('done')">
    <div
      class="dialog"
      role="dialog"
      data-testid="soldout-list"
      :aria-label="t('soldout_list_title')"
    >
      <h2 class="title">{{ t('soldout_list_title') }}</h2>
      <p v-if="menu.soldOut.length === 0" class="empty" data-testid="soldout-list-empty">
        {{ t('soldout_list_empty') }}
      </p>
      <ul v-else class="rows scroll-touch">
        <li v-for="p in menu.soldOut" :key="p.menu_item_id" class="row" data-testid="soldout-row">
          <div class="info">
            <span class="name">{{ p.name }}</span>
            <span class="until">
              {{
                p.until
                  ? t('soldout_status_day', { time: endOfDayLabel(p.until) })
                  : t('soldout_status_reenabled')
              }}
            </span>
          </div>
          <button
            class="reenable"
            data-testid="soldout-reenable"
            :disabled="busy !== null"
            @click="reenable(p.menu_item_id)"
          >
            {{ t('soldout_reenable') }}
          </button>
        </li>
      </ul>
      <button class="btn-cancel" data-testid="soldout-list-close" @click="emit('done')">
        {{ t('back') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.6);
  z-index: 50;
}
.dialog {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  width: 620px;
  max-width: 94vw;
  max-height: 90vh;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  text-align: center;
}
.empty {
  text-align: center;
  color: var(--text-secondary);
  font-size: var(--font-size-md);
  padding: var(--space-lg);
}
.rows {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  overflow-y: auto;
  min-height: 0;
  list-style: none;
  padding: 0;
  margin: 0;
}
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}
.info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.name {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.until {
  font-size: 14px;
  color: var(--text-secondary);
}
.reenable {
  flex-shrink: 0;
  min-width: 160px;
  min-height: 56px;
  padding: 0 var(--space-md);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.reenable:active {
  transform: scale(var(--tap-scale));
}
.reenable:disabled {
  opacity: 0.5;
}
.btn-cancel {
  min-height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-secondary);
}
</style>
