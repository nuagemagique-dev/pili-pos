<script setup lang="ts">
/**
 * The one header of every staff screen (UX guide §2.3, §3): logo,
 * device lights, operator, table, clock, workspaces and the lock.
 * 64 px high so its buttons can be 48 px (§4.1, §13).
 */
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from 'vue'
import { useIntegrationsStore } from '../stores/integrations'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useOrderStore } from '../stores/order'
import { useTablePlanStore } from '../stores/tablePlan'
import { useToastStore } from '../stores/toast'
import { useI18n } from '../i18n'
import { describeError } from '../utils/errorMessages'
import { formatTime } from '../utils/datetime'
import PiliLogo from './PiliLogo.vue'
import StatusLights from './StatusLights.vue'
import PinDialog from './PinDialog.vue'
import FdmAlerts from './FdmAlerts.vue'
import TrainingBanner from './TrainingBanner.vue'
import IdleLockBanner from './IdleLockBanner.vue'
import LateOnlineBanner from './LateOnlineBanner.vue'
import TableMover from './TableMover.vue'
import TakeawayCustomer from './TakeawayCustomer.vue'
import UserMenu from './UserMenu.vue'
import UiIcon from './UiIcon.vue'

// While HubRise runs, or online orders are still to settle.
const OnlineOrdersNavButton = defineAsyncComponent(() => import('./OnlineOrdersNavButton.vue'))
const integrations = useIntegrationsStore()

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const order = useOrderStore()
const plan = useTablePlanStore()
const toast = useToastStore()
const { t } = useI18n()

const clock = ref(formatTime(new Date()))
let clockTimer: ReturnType<typeof setInterval> | undefined
onMounted(() => {
  clockTimer = setInterval(() => (clock.value = formatTime(new Date())), 10_000)
})
onUnmounted(() => clearInterval(clockTimer))

const onOrderScreen = computed(() => route.name === 'order' || route.name === 'checkout')
// An unpaid online order at the checkout (UX audit O2) is neither a table nor a numbered takeaway.
const collectingOnline = computed(() => onOrderScreen.value && order.onlineCollect !== null)
const orderType = computed(() =>
  onOrderScreen.value && !collectingOnline.value ? order.currentOrder?.order_type : undefined,
)
const tableNumber = computed(() =>
  orderType.value === 'dine_in' ? order.currentOrder?.table_number : null,
)
// "A-19" in a zone, "19" without one (UX audit T1).
const tableLabel = computed(() => plan.label(tableNumber.value))

// "You are here" (§2.3): a dine-in order belongs to Tables.
const tablesActive = computed(() => route.name === 'tables' || orderType.value === 'dine_in')
const takeawayActive = computed(() => orderType.value === 'takeaway')

const roleLabel = computed(() => (auth.currentUser ? t(`role_${auth.currentUser.role}`) : ''))

// UX audit C8: the table chip is the way to move the order to another table.
const canMoveTable = computed(() => order.canTransfer)
const moving = ref(false)

// The operator changing their own PIN (from the menu under their name).
const pinOpen = ref(false)
const userMenuOpen = ref(false)
const userButton = ref<HTMLButtonElement | null>(null)
// The menu hangs under the user button.
const userMenuLeft = ref(0)

function toggleUserMenu() {
  userMenuLeft.value = Math.round(userButton.value?.getBoundingClientRect().left ?? 0)
  userMenuOpen.value = !userMenuOpen.value
}

/** Start a fresh takeaway order. A second tap while already on a
 * takeaway is a no-op instead of silently replacing the cart. */
async function startTakeaway() {
  if (route.name === 'order' && takeawayActive.value) return
  if (route.name === 'order') {
    // Staying on the order screen: the engine registers the table order
    // (P) here, since the router guard only runs when leaving the screen.
    try {
      await order.leave('switch_order')
    } catch (e) {
      toast.show(`${t('order_register_failed')} : ${describeError(e)}`, 'error', 6000)
      return
    }
  }
  try {
    await order.startOrder(null)
  } catch (e) {
    toast.show(describeError(e), 'error', 5000)
    return
  }
  if (route.name !== 'order') await router.push({ name: 'order' })
}

/** Lock the screen. The operator stays on shift: clocking out is done
 * from the login screen ("Fin de service"). */
async function lock() {
  // The engine registers the open order (P) while the session is open,
  // then closes the session; a failed registration never blocks the lock.
  const outcome = await auth.lock()
  if (outcome.orderError) {
    toast.show(
      `${t('order_register_failed')} : ${describeError(outcome.orderError)}`,
      'error',
      6000,
    )
  }
  order.resetLocal()
  await router.push({ name: 'login' })
}

/** "Fin de service" from the menu: what was ordered is registered, the engine signs the
 * clock-out of the logged-in employee (no PIN again: the session proves it), then the
 * till locks and the login screen shows them off shift. */
async function endShift() {
  userMenuOpen.value = false
  const name = auth.currentUser?.name ?? ''
  try {
    await order.leave('navigate')
  } catch (e) {
    toast.show(`${t('order_register_failed')} : ${describeError(e)}`, 'error', 6000)
  }
  const done = await auth.endOwnShift()
  if (done) toast.show(t('shift_ended', { name }), 'success')
  order.resetLocal()
  await router.push({ name: 'login' })
}

function menuLock() {
  userMenuOpen.value = false
  void lock()
}

function menuPin() {
  userMenuOpen.value = false
  pinOpen.value = true
}
</script>

<template>
  <header class="app-header">
    <PiliLogo :height="24" color="var(--text-primary)" mark-color="var(--accent-primary)" />
    <span class="divider" />
    <StatusLights />
    <span class="divider" />
    <button
      ref="userButton"
      class="header-user"
      :aria-label="t('user_menu')"
      aria-haspopup="menu"
      :aria-expanded="userMenuOpen"
      data-testid="header-user"
      @click="toggleUserMenu"
    >
      <span class="user-name">{{ auth.currentUser?.name }}</span>
      <span class="user-role"
        >{{ roleLabel }}<UiIcon class="user-chevron" name="menuDown" :size="18"
      /></span>
    </button>
    <UserMenu
      v-if="userMenuOpen"
      :left="userMenuLeft"
      @close="userMenuOpen = false"
      @pin="menuPin"
      @end-shift="endShift"
      @lock="menuLock"
    />
    <button
      v-if="tableNumber && canMoveTable"
      class="header-table header-table-btn"
      data-testid="header-table"
      :aria-label="`${t('transfer_table')} · ${t('table')} ${tableLabel}`"
      @click="moving = true"
    >
      {{ t('table') }} {{ tableLabel }}
    </button>
    <span v-else-if="tableNumber" class="header-table" data-testid="header-table"
      >{{ t('table') }} {{ tableLabel }}</span
    >
    <span v-else-if="collectingOnline" class="header-table" data-testid="header-online">{{
      t('online_orders')
    }}</span>
    <TakeawayCustomer v-else-if="takeawayActive" />
    <span class="header-clock">{{ clock }}</span>

    <TableMover v-if="moving" @close="moving = false" />

    <nav class="header-nav">
      <div class="segmented">
        <button
          class="nav-btn"
          :class="{ 'nav-btn-active': tablesActive }"
          @click="router.push({ name: 'tables' })"
        >
          {{ t('tables') }}
        </button>
        <button
          class="nav-btn"
          :class="{ 'nav-btn-active': takeawayActive }"
          @click="startTakeaway"
        >
          {{ t('takeaway_new') }}
        </button>
        <OnlineOrdersNavButton v-if="integrations.hubriseVisible" />
      </div>
      <div v-if="auth.isManager" class="admin-nav">
        <button
          class="nav-btn nav-admin"
          :class="{ 'nav-btn-active': route.name === 'reports' }"
          @click="router.push({ name: 'reports' })"
        >
          {{ t('reports') }}
        </button>
        <button
          class="nav-btn nav-admin"
          :class="{ 'nav-btn-active': route.name === 'settings' }"
          @click="router.push({ name: 'settings' })"
        >
          {{ t('config') }}
        </button>
      </div>
      <button class="nav-btn nav-logout" @click="lock">{{ t('lock') }}</button>
    </nav>
  </header>
  <TrainingBanner />
  <IdleLockBanner />
  <LateOnlineBanner />
  <FdmAlerts />

  <PinDialog
    v-if="pinOpen && auth.currentUser"
    :insz="auth.currentUser.insz"
    :user-name="auth.currentUser.name"
    @done="pinOpen = false"
    @cancel="pinOpen = false"
  />
</template>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 0 20px;
  height: var(--header-height);
  flex-shrink: 0;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
}

.divider {
  width: 1px;
  height: 32px;
  background: var(--border);
  flex-shrink: 0;
}

.header-user {
  height: 48px;
  padding: 0 10px;
  border-radius: var(--radius-sm);
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  color: var(--text-primary);
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.header-user:active {
  transform: scale(var(--tap-scale));
  background: var(--bg-surface);
}
.user-name {
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  line-height: 1.15;
}
.user-role {
  display: flex;
  align-items: center;
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.2;
}

/* The current table: outlined in Brume, never an action colour. */
.header-table {
  height: 40px;
  padding: 0 14px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--text-primary);
  font-size: 17px;
  font-weight: var(--font-weight-heavy);
  display: flex;
  align-items: center;
  white-space: nowrap;
}
/* A table that can move is a button: 48 px high, same Brume outline. */
.header-table-btn {
  height: 48px;
  background: transparent;
  color: var(--text-primary);
}
.header-table-btn:active {
  transform: scale(var(--tap-scale));
}
.header-clock {
  font-family: var(--font-mono);
  font-size: 16px;
  font-weight: 500;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.header-nav {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Workspaces: one segmented control, the current one in Brume. */
.segmented {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--bg-surface);
  border-radius: 12px;
}
.segmented .nav-btn {
  height: 48px;
  padding: 0 22px;
  border-radius: var(--radius-sm);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  color: var(--text-primary);
  display: flex;
  align-items: center;
  gap: 10px;
  transition: transform var(--tap-duration) ease;
}

.admin-nav {
  display: flex;
  gap: 8px;
  margin-left: 12px;
}
.nav-admin {
  height: 48px;
  padding: 0 20px;
  border-radius: 12px;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}

.nav-btn:active {
  transform: scale(var(--tap-scale));
}
.segmented .nav-btn.nav-btn-active,
.nav-admin.nav-btn-active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}

.nav-logout {
  height: 48px;
  padding: 0 20px;
  margin-left: 20px;
  border-radius: 12px;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 16px;
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
</style>
