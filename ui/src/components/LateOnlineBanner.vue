<script setup lang="ts">
/**
 * Online orders nobody answered for 5 minutes (UX audit O3): a banner under the header of every
 * staff screen with a "Voir" button to the board. Amber, like every waiting state, with the cyan
 * of online orders on the button. The board has its own alarm, so it is hidden there. The count
 * comes from the webshop store (`lateCount`, `utils/webshopLate.ts`); the chime repeats from
 * `useOrderChime`.
 */
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useWebshopOrdersStore } from '../stores/webshopOrders'
import { useI18n } from '../i18n'

const route = useRoute()
const router = useRouter()
const store = useWebshopOrdersStore()
const { t } = useI18n()

const late = computed(() => (route.name === 'webshop-orders' ? 0 : store.lateCount))
const text = computed(() =>
  late.value === 1 ? t('online_late_one') : t('online_late_many', { count: late.value }),
)
</script>

<template>
  <div v-if="late > 0" class="late-banner" role="alert" data-testid="late-online-banner">
    <span class="bang">!</span>
    <span class="msg">{{ text }}</span>
    <button
      class="late-view"
      data-testid="late-online-view"
      @click="router.push({ name: 'webshop-orders' })"
    >
      {{ t('online_late_view') }}
    </button>
  </div>
</template>

<style scoped>
.late-banner {
  height: 56px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 20px;
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
}
.bang {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--text-on-accent);
  color: var(--accent-warning);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 900;
  flex-shrink: 0;
}
.msg {
  flex: 1;
  min-width: 0;
}
.late-view {
  height: 48px;
  min-width: 96px;
  padding: 0 24px;
  border-radius: var(--radius-sm);
  background: var(--accent-info);
  color: var(--text-on-accent);
  font-size: 18px;
  font-weight: var(--font-weight-heavy);
  transition: transform var(--tap-duration) ease;
}
.late-view:active {
  transform: scale(var(--tap-scale));
}
</style>

<style>
/* Above the blocking FDM veil (900) so the banner stays readable. */
.fdm-blocking .late-banner {
  position: relative;
  z-index: 901;
}
</style>
