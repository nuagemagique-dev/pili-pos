<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useOrderStore } from "../stores/order";
import { useAuthStore } from "../stores/auth";
import * as ipc from "../api/ipc";
import { useI18n } from "../i18n";
import { describeError } from "../utils/errorMessages";
import { useToastStore } from "../stores/toast";
import { formatPrice } from "../utils/format";

const router = useRouter();
const { t } = useI18n();
const order = useOrderStore();
const auth = useAuthStore();

const toast = useToastStore();
const tables = ref<ipc.TableInfo[]>([]);

onMounted(async () => {
  if (!auth.currentUser) { router.push({ name: "login" }); return; }
  tables.value = await ipc.listTables(20);
});

async function selectTable(table: ipc.TableInfo) {
  try {
    if (table.orderId) {
      // Rehydrate the existing order AND its lines so the cart is
      // repopulated (fixes the bug where tapping a busy table after
      // a takeaway switch showed an empty cart).
      await order.loadOrder(table.orderId);
    } else {
      await order.startOrder(table.tableNumber);
    }
  } catch (e) {
    toast.show(describeError(e), "error", 5000);
    return;
  }
  router.push({ name: "order" });
}
</script>

<template>
  <div class="tables-view">
    <header class="header">
      <h1>{{ t('tables') }}</h1>
      <button class="nav-btn" @click="router.push({ name: 'order' })">{{ t('back') }}</button>
    </header>

    <div class="grid">
      <button
        v-for="tbl in tables"
        :key="tbl.tableNumber"
        class="table-card"
        :class="{ occupied: tbl.status === 'occupied' }"
        @click="selectTable(tbl)"
      >
        <span class="table-num">{{ tbl.tableNumber }}</span>
        <span v-if="tbl.status === 'occupied'" class="table-info">{{ tbl.lineCount }} {{ t('items_short') }}</span>
        <span v-if="tbl.status === 'occupied'" class="table-total">{{ formatPrice(tbl.totalCents) }}</span>
        <span v-else class="table-free">{{ t('free') }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.tables-view { display: flex; flex-direction: column; height: 100vh; background: var(--bg-primary); }
.header { display: flex; align-items: center; justify-content: space-between; padding: 0 var(--space-md); background: var(--bg-secondary); border-bottom: 1px solid rgba(160,160,176,0.1); height: 48px; }
.header h1 { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); }
.nav-btn { padding: var(--space-xs) var(--space-md); font-size: var(--font-size-xs); font-weight: var(--font-weight-bold); border-radius: var(--radius-sm); background: var(--bg-surface); color: var(--text-secondary); min-height: 36px; transition: transform var(--tap-duration) ease; }
.nav-btn:active { transform: scale(var(--tap-scale)); }

.grid { flex: 1; display: grid; grid-template-columns: repeat(5, 1fr); gap: var(--space-md); padding: var(--space-lg); overflow-y: auto; align-content: start; }

.table-card {
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: var(--space-xs);
  padding: var(--space-lg); min-height: 120px; border-radius: var(--radius-lg);
  background: var(--bg-surface); color: var(--text-primary); border: 2px solid transparent;
  transition: transform var(--tap-duration) ease;
}
.table-card:active { transform: scale(var(--tap-scale)); }
.table-card.occupied { border-color: var(--accent-primary); }

.table-num { font-size: var(--font-size-xl); font-weight: var(--font-weight-bold); }
.table-info { font-size: var(--font-size-xs); color: var(--text-secondary); }
.table-total { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); color: var(--accent-primary); }
.table-free { font-size: var(--font-size-sm); color: var(--text-secondary); }
</style>
