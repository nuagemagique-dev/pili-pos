<script setup lang="ts">
// "En ligne" nav button with the pending webshop-orders badge. Only
// rendered when HUBRISE_ENABLED (see features.ts). The button is the
// component root, so OrderView's scoped `.nav-btn` styles apply to it.
import { onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useWebshopOrdersStore } from "../stores/webshopOrders";
import { useI18n } from "../i18n";

const router = useRouter();
const webshop = useWebshopOrdersStore();
const { t } = useI18n();

// Brief pulse class toggled whenever pendingCount goes up - gives
// the red badge a visual punch when a new online order arrives
// without needing a continuous animation (UX Guide §4.2).
const pendingPulse = ref(false);
watch(
  () => webshop.pendingCount,
  (next, prev) => {
    if (next > (prev ?? 0)) {
      pendingPulse.value = true;
      setTimeout(() => {
        pendingPulse.value = false;
      }, 180);
    }
  },
);

onMounted(() => {
  // Keep the badge live even when the operator is heads-down on the
  // dine-in flow. Fire-and-forget - a slow webshop fetch must not
  // block the menu grid from rendering.
  webshop.attachListeners().catch((e) => console.error(e));
  webshop.fetchOrders().catch((e) => console.error(e));
});
</script>

<template>
  <button
    class="nav-btn"
    :class="{ 'nav-btn-raised': webshop.pendingCount > 0 }"
    @click="router.push({ name: 'webshop-orders' })"
  >
    {{ t('online_orders') }}
    <span
      v-if="webshop.pendingCount > 0"
      class="nav-pending-badge"
      :class="{ pulse: pendingPulse }"
    >{{ webshop.pendingCount }}</span>
  </button>
</template>
