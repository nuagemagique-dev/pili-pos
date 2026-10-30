<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useMenuStore } from '../stores/menu'
import { useOrderStore } from '../stores/order'
import { useAuthStore } from '../stores/auth'
import { useI18n } from '../i18n'
import * as ipc from '../api/ipc'
import AppHeader from '../components/AppHeader.vue'
import CategoryTabs from '../components/CategoryTabs.vue'
import MenuGrid from '../components/MenuGrid.vue'
import CartSidebar from '../components/CartSidebar.vue'
import PriceDialog from '../components/PriceDialog.vue'
import WeighDialog from '../components/WeighDialog.vue'
import SoldOutDialog from '../components/SoldOutDialog.vue'
import SoldOutList from '../components/SoldOutList.vue'
import { useToastStore } from '../stores/toast'
import { usePosStore } from '../stores/pos'
import AllergenSheetDialog from '../components/AllergenSheetDialog.vue'
import { useAllergensStore } from '../stores/allergens'
import { useGiftVouchersStore } from '../stores/giftVouchers'
import type { MenuItem } from '../types'

const router = useRouter()
const menu = useMenuStore()
const order = useOrderStore()
const auth = useAuthStore()
const { t } = useI18n()
const allergens = useAllergensStore()
const giftVouchers = useGiftVouchersStore()

onMounted(async () => {
  if (!auth.currentUser) {
    router.push({ name: 'login' })
    return
  }
  void giftVouchers.load()
  await menu.loadCategories()
  if (menu.selectedCategoryId) {
    await menu.loadItems(menu.selectedCategoryId)
  }
  if (auth.canTakePayment) await menu.loadSoldOut()
  // Z-report reminder - check on mount + every 5 min.
  checkZReport()
  zCheckInterval = setInterval(checkZReport, 5 * 60 * 1000)

  // No active order → bounce to Tables so the operator consciously
  // picks a table OR taps "À emporter". Avoids silently creating a
  // phantom Table 1 when the user just navigated here.
  if (!order.currentOrder) {
    router.push({ name: 'tables' })
  }
})

onUnmounted(() => clearInterval(zCheckInterval))

const selectedCategoryColor = computed(() => {
  const cat = menu.categories.find((c) => c.id === menu.selectedCategoryId)
  return cat?.color ?? null
})

/** Quantity of each product already in the cart, for the badges on
 * the menu buttons. */
const cartQuantities = computed(() => {
  const map = new Map<number, number>()
  for (const line of order.lines) {
    map.set(line.menu_item_id, (map.get(line.menu_item_id) ?? 0) + line.quantity)
  }
  return map
})

// Z-report daily reminder - polls every 5 min, only shows after 22:00.
const showZWarning = ref(false)
let zCheckInterval: ReturnType<typeof setInterval>

async function checkZReport() {
  try {
    showZWarning.value = await ipc.needsZReport()
  } catch {
    // Non-critical - degrade silently.
  }
}

// Selling a gift voucher: PriceDialog open for its amount.
const sellingVoucher = ref(false)

async function handleVoucherAmount(cents: number) {
  sellingVoucher.value = false
  await order.addGiftVoucher(cents)
}

// Free-price product waiting for its price (PriceDialog open).
const pricing = ref<MenuItem | null>(null)

// Option picker for a product with option groups (from a mirrored catalog or made in the menu
// editor); its own chunk.
const ModifierDialog = defineAsyncComponent(() => import('../components/ModifierDialog.vue'))
// Product whose options are being picked; the dialog adds it plainly
// when it has none.
const choosing = ref<MenuItem | null>(null)

// "Épuisé" mode (cashier and manager): a tap on a product opens its availability dialog
// instead of adding it. Out of the mode a sold-out product is greyed and a tap shows why
// it cannot be added.
const soldOutMode = ref(false)
const soldOutItem = ref<MenuItem | null>(null)
const showSoldOutList = ref(false)

// The "Épuisé" and "Allergènes" modes both take over a tap on a product:
// switching one on switches the other off.
async function toggleSoldOutMode() {
  soldOutMode.value = !soldOutMode.value
  if (soldOutMode.value && allergens.mode) await allergens.toggleMode()
}

async function toggleAllergenMode() {
  await allergens.toggleMode()
  if (allergens.mode) soldOutMode.value = false
}

async function handleAddItem(item: MenuItem) {
  if (soldOutMode.value) {
    soldOutItem.value = item
    return
  }
  // Allergen mode: a tap shows the product's allergen sheet instead of adding it.
  if (allergens.mode) {
    await allergens.openSheet(item)
    return
  }
  if (item.sold_out) {
    useToastStore().show(t('err_product_sold_out'), 'error', 4000)
    // The end of the day may have passed since the grid was read.
    if (menu.selectedCategoryId) await menu.loadItems(menu.selectedCategoryId)
    return
  }
  if (item.sold_by_weight) {
    await weigh(item)
    return
  }
  if (item.open_price) {
    pricing.value = item
    return
  }
  // The option picker opens for a product that has option groups (V11-D18), whoever manages the
  // menu: the local editor creates them too.
  if (item.has_modifiers) {
    choosing.value = item
    return
  }
  await order.addItem(item)
}

// Product on the scale: the dialog stays up until the engine has the line or a refusal.
const pos = usePosStore()
const weighing = ref<MenuItem | null>(null)

async function weigh(item: MenuItem) {
  if (!pos.settings?.scaleConfigured) {
    useToastStore().show(t('err_scale_not_configured'), 'error', 4000)
    return
  }
  if (weighing.value) return
  weighing.value = item
  await order.addLineWith(item, 1, (orderId) => ipc.scaleStartWeighing(orderId, item.id))
  weighing.value = null
}

async function handlePriceConfirmed(priceCents: number) {
  const item = pricing.value
  pricing.value = null
  if (item) await order.addItem(item, 1, priceCents)
}
</script>

<template>
  <!--
    UX guide §3: header | menu (categories above the grid) | cart 670 px
    on the right, main action at the bottom right.
  -->
  <div class="order-view">
    <AppHeader />

    <!-- Z-report daily reminder - §8.2 non-blocking warning banner -->
    <div v-if="showZWarning" class="z-warning">
      {{ t('z_report_warning') }}
    </div>

    <div class="main">
      <div class="menu-section">
        <CategoryTabs
          :categories="menu.categories"
          :selected-id="menu.selectedCategoryId"
          @select="menu.selectCategory($event)"
        />
        <div v-if="auth.canTakePayment" class="soldout-bar">
          <button
            class="soldout-btn"
            :class="{ active: soldOutMode }"
            :aria-pressed="soldOutMode"
            data-testid="soldout-mode"
            @click="toggleSoldOutMode"
          >
            {{ t('soldout_mode') }}
          </button>
          <span v-if="soldOutMode" class="soldout-hint" data-testid="soldout-hint">
            {{ t('soldout_mode_hint') }}
          </span>
          <button
            class="soldout-btn list"
            data-testid="soldout-list-button"
            @click="showSoldOutList = true"
          >
            {{ t('soldout_list_button', { n: menu.soldOut.length }) }}
          </button>
        </div>
        <!-- Allergen mode (EU Annex II): a tap on a product opens its sheet -->
        <div class="allergen-bar" data-testid="allergen-bar">
          <button
            class="allergen-toggle"
            :class="{ on: allergens.mode }"
            :aria-pressed="allergens.mode"
            data-testid="allergen-mode"
            @click="toggleAllergenMode"
          >
            {{ t('allergens_mode') }}
          </button>
          <button
            v-if="giftVouchers.enabled && order.currentOrder && auth.canTakePayment"
            class="allergen-toggle"
            data-testid="gift-voucher-sell"
            @click="sellingVoucher = true"
          >
            {{ t('gv_sell_button') }}
          </button>
          <template v-if="allergens.mode">
            <span class="allergen-hint">{{ t('allergens_hint') }}</span>
            <span
              v-if="allergens.error && !allergens.sheetItem"
              class="allergen-error"
              role="alert"
              data-testid="allergen-legend-error"
              >{{ allergens.error }}</span
            >
            <ul
              class="allergen-legend"
              data-testid="allergen-legend"
              :aria-label="t('allergens_legend')"
            >
              <li v-for="a in allergens.legend" :key="a.code" class="legend-item">
                <span class="legend-num">{{ a.number }}</span>
                <span>{{ t(`allergen_${a.code}`) }}</span>
              </li>
            </ul>
          </template>
        </div>
        <MenuGrid
          :items="menu.items"
          :category-color="selectedCategoryColor"
          :cart-quantities="cartQuantities"
          @add-item="handleAddItem"
        />
      </div>
      <CartSidebar />
    </div>

    <component
      :is="ModifierDialog"
      v-if="choosing"
      :key="choosing.id"
      :item="choosing"
      @done="choosing = null"
    />

    <SoldOutDialog v-if="soldOutItem" :item="soldOutItem" @done="soldOutItem = null" />
    <SoldOutList v-if="showSoldOutList" @done="showSoldOutList = false" />
    <AllergenSheetDialog />

    <WeighDialog v-if="weighing" :item-name="weighing.name" />
    <PriceDialog
      v-if="sellingVoucher"
      :title="t('gv_sell_title')"
      :item-name="t('gv_sell_item')"
      @confirm="handleVoucherAmount"
      @cancel="sellingVoucher = false"
    />

    <PriceDialog
      v-if="pricing"
      :item-name="pricing.name"
      @confirm="handlePriceConfirmed"
      @cancel="pricing = null"
    />
  </div>
</template>

<style scoped>
.order-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: var(--bg-primary);
}

/* §8.2 non-blocking warning - amber, full-width, gone once the Z is
   done. Fires only after 22:00. */
.z-warning {
  padding: 14px 20px;
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-size: 18px;
  font-weight: var(--font-weight-bold);
  flex-shrink: 0;
}

.soldout-bar {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: 8px 16px 0;
  flex-shrink: 0;
}
.soldout-btn {
  min-height: 56px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-primary);
  border: 2px solid transparent;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.soldout-btn:active {
  transform: scale(var(--tap-scale));
}
.soldout-btn.active {
  background: var(--accent-warning);
  color: var(--text-on-accent);
}
.soldout-btn.list {
  margin-left: auto;
}
.soldout-hint {
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}

.main {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.allergen-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px 12px;
  padding: 8px 16px 0;
}
.allergen-toggle {
  height: 44px;
  padding: 0 18px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  transition: transform var(--tap-duration) ease;
}
.allergen-toggle:active {
  transform: scale(var(--tap-scale));
}
.allergen-toggle.on {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.allergen-hint {
  font-size: var(--font-size-sm);
  color: var(--accent-info);
  font-weight: var(--font-weight-bold);
}
.allergen-error {
  font-size: var(--font-size-sm);
  color: var(--accent-danger);
  font-weight: var(--font-weight-bold);
}
.allergen-legend {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  width: 100%;
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: var(--font-size-xs);
  color: var(--text-secondary);
}
.legend-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.legend-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--accent-info);
  color: var(--text-on-accent);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: var(--font-weight-bold);
}

.menu-section {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
