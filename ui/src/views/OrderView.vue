<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch } from 'vue'
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
import CodePad from '../components/CodePad.vue'
import {
  codeLetters,
  parseCodeEntry,
  pickByCode,
  openDigitWindow,
  searchByCode,
  stepDigitWindow,
  takesQuantity,
  typedCode,
  windowLabel,
  type CodePadTone,
  type DigitWindow,
} from '../utils/codeEntry'
import { formatPrice } from '../utils/format'

const router = useRouter()
const menu = useMenuStore()
const order = useOrderStore()
const auth = useAuthStore()
const { t } = useI18n()
const allergens = useAllergensStore()
const giftVouchers = useGiftVouchersStore()
const pos = usePosStore()

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
  if (codePadOpen.value) await loadCodeItems()
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
    // A bowl's paid option (its broth) is not one more bowl.
    if (line.bowl_line_id !== null && line.bowl_line_id !== undefined) continue
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
const pricingQuantity = ref(1)

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

async function handleAddItem(item: MenuItem, quantity = 1) {
  // Another product is added: a digit starts a code again.
  digitWindow.value = null
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
    if (!scaleReady()) return
    // A bowl with options (its broth): picked first, then weighed with them,
    // so the bowl never reaches the order without its broth.
    if (item.has_modifiers) {
      choosing.value = item
      return
    }
    await weigh(item)
    return
  }
  if (item.open_price) {
    pricing.value = item
    pricingQuantity.value = quantity
    return
  }
  // The option picker opens for a product that has option groups (V11-D18), whoever manages the
  // menu: the local editor creates them too.
  if (item.has_modifiers) {
    choosing.value = item
    return
  }
  await addPlain(item, quantity)
}

// ── Digit after an add = quantity (C7) ──────────────────────────────
// Right after a plain product is added, for 3 s, a digit typed on the pad
// (or a physical keyboard) while the code field is empty says how many of
// that product the add gives: extra units of the same line. Products with
// options, a free price or a weight never open the window.
const digitWindow = ref<DigitWindow | null>(null)
const digitItem = ref<MenuItem | null>(null)
// The extra units go one after the other: each reads the line as the one before left it.
let digitQueue: Promise<unknown> = Promise.resolve()

/** Adds a plain product and opens the digit window on the line it went to. */
async function addPlain(item: MenuItem, quantity: number) {
  const add = await order.addItemTracked(item, quantity)
  if (!add) return
  digitItem.value = item
  digitWindow.value = openDigitWindow(
    { lineId: add.lineId, itemId: item.id, quantity: add.units },
    Date.now(),
  )
}

/** Every key of the pad first: true when a digit was taken as the quantity of the last add. */
function takePadKey(key: string): boolean {
  const open = digitWindow.value
  const item = digitItem.value
  const step = codeText.value === '' ? stepDigitWindow(open, key, Date.now()) : null
  if (step?.kind !== 'set' || open === null || item === null) {
    digitWindow.value = null
    return false
  }
  digitWindow.value = step.window
  const delta = step.window.quantity - open.quantity
  digitQueue = digitQueue.then(async () => {
    if (await order.addUnits(step.window.lineId, delta)) {
      codeOutcome.value = { tone: 'done', text: windowLabel(item.name, step.window.quantity) }
    } else if (digitWindow.value === step.window) {
      digitWindow.value = open
    }
  })
  return true
}

// ── Order entry by menu code (the "KeyB" pad of Xudox) ──────────────
// The pad stays open or closed per terminal, like the product label.
const CODE_PAD_KEY = 'pili-pos-code-pad'

function readCodePadFlag(): boolean {
  try {
    return localStorage.getItem(CODE_PAD_KEY) === '1'
  } catch {
    return false
  }
}

const codePadOpen = ref(readCodePadFlag())
const codeText = ref('')
// What the last OK did or why it was refused, until the next key.
const codeOutcome = ref<{ tone: CodePadTone; text: string } | null>(null)
// Every product of the menu (all categories), for the lookups by code.
const codeItems = ref<MenuItem[]>([])
const codeKeys = computed(() => codeLetters(codeItems.value))

async function loadCodeItems() {
  try {
    codeItems.value = await ipc.getMenuItems(undefined, pos.language)
  } catch {
    // The next OK reads them again.
  }
}

async function toggleCodePad() {
  codePadOpen.value = !codePadOpen.value
  try {
    localStorage.setItem(CODE_PAD_KEY, codePadOpen.value ? '1' : '0')
  } catch {
    // Remembering the pad is a convenience only.
  }
  codeText.value = ''
  codeOutcome.value = null
  if (codePadOpen.value) await loadCodeItems()
}

watch(codeText, () => {
  codeOutcome.value = null
})
watch(
  () => pos.language,
  () => {
    if (codePadOpen.value) void loadCodeItems()
  },
)

function lineLabel(quantity: number, item: MenuItem): string {
  return `${quantity} × ${item.name} · ${formatPrice(item.price_cents * quantity)} €`
}

const codeEntry = computed(() => parseCodeEntry(codeText.value))

/** While a code is typed, the products whose code starts with it (all categories); null shows the category. */
const codeResults = computed<MenuItem[] | null>(() => {
  if (!codePadOpen.value) return null
  const code = typedCode(codeEntry.value)
  return code === null ? null : searchByCode(codeItems.value, code)
})

/** The product OK would add, highlighted in the results. */
const codePick = computed<MenuItem | null>(() => {
  const code = typedCode(codeEntry.value)
  if (code === null) return null
  const match = pickByCode(codeItems.value, code)
  return match.kind === 'found' ? match.item : null
})

/** What OK would do with the typed text, shown under the field. */
const codeMessage = computed<{ tone: CodePadTone; text: string }>(() => {
  if (codeOutcome.value) return codeOutcome.value
  const entry = codeEntry.value
  if (entry.kind === 'empty') return { tone: 'hint', text: t('code_pad_hint') }
  if (entry.kind === 'multiplier') {
    return { tone: 'hint', text: t('code_pad_multiplier', { n: entry.quantity }) }
  }
  if (entry.kind === 'invalid') return { tone: 'error', text: t('code_pad_invalid') }
  if (codePick.value) return { tone: 'match', text: lineLabel(entry.quantity, codePick.value) }
  const n = codeResults.value?.length ?? 0
  return n === 0
    ? { tone: 'error', text: t('code_pad_no_result') }
    : { tone: 'hint', text: t('code_pad_results', { n }) }
})

// No physical key reaches the pad while a dialog of the screen is open.
const codePadListening = computed(
  () =>
    !pricing.value &&
    !choosing.value &&
    !soldOutItem.value &&
    !showSoldOutList.value &&
    !weighing.value &&
    !sellingVoucher.value &&
    !allergens.sheetItem,
)

function refuseCode(text: string) {
  codeOutcome.value = { tone: 'error', text }
}

/** Adds [quantity] of [item] from the pad; the screen's rules (modes, sold out, dialogs) apply. */
async function addFromPad(item: MenuItem, quantity: number) {
  const modeTakesTap = soldOutMode.value || allergens.mode
  if (!modeTakesTap && quantity > 1 && !takesQuantity(item)) {
    refuseCode(t('code_pad_no_multiplier'))
    return
  }
  if (!modeTakesTap && item.sold_out === true) {
    refuseCode(t('err_product_sold_out'))
    return
  }
  codeText.value = ''
  await handleAddItem(item, modeTakesTap ? 1 : quantity)
  const plain = !modeTakesTap && !item.open_price && takesQuantity(item)
  if (plain) codeOutcome.value = { tone: 'done', text: `✓ ${lineLabel(quantity, item)}` }
}

/** OK on the pad: the exact code, else the only product starting with it. */
async function submitCode() {
  const entry = codeEntry.value
  if (entry.kind === 'empty' || entry.kind === 'multiplier') return
  if (entry.kind === 'invalid') {
    refuseCode(t('code_pad_invalid'))
    return
  }
  // Read again: the sold-out flags may have changed since the pad opened.
  await loadCodeItems()
  const match = pickByCode(codeItems.value, entry.code)
  if (match.kind === 'unknown') refuseCode(t('code_pad_unknown', { code: entry.code }))
  else if (match.kind === 'ambiguous') refuseCode(t('code_pad_ambiguous', { code: entry.code }))
  else await addFromPad(match.item, entry.quantity)
}

/** A tap in the grid: the quantity on the pad ("3 ×", or "3 × A1" over the results) applies to it. */
async function handleGridTap(item: MenuItem) {
  const entry = codePadOpen.value ? codeEntry.value : null
  if (entry?.kind === 'multiplier' || entry?.kind === 'add') {
    await addFromPad(item, entry.quantity)
    return
  }
  await handleAddItem(item)
}

/** A category tab leaves the results of a typed code. */
async function selectCategory(id: number) {
  if (typedCode(codeEntry.value) !== null) codeText.value = ''
  await menu.selectCategory(id)
}

// Product on the scale: the dialog stays up until the engine has the line or a refusal.
const weighing = ref<MenuItem | null>(null)

function scaleReady(): boolean {
  if (pos.settings?.scaleConfigured === true) return true
  useToastStore().show(t('err_scale_not_configured'), 'error', 4000)
  return false
}

/** "Peser" in the option picker: the bowl being chosen, with its options. */
async function weighChosen(modifierIds: number[]) {
  const item = choosing.value
  if (item) await weigh(item, modifierIds)
}

async function weigh(item: MenuItem, modifierIds: number[] = []) {
  if (!scaleReady()) return
  if (weighing.value) return
  weighing.value = item
  await order.addLineWith(item, 1, (orderId) =>
    ipc.scaleStartWeighing(orderId, item.id, modifierIds),
  )
  weighing.value = null
}

async function handlePriceConfirmed(priceCents: number) {
  const item = pricing.value
  pricing.value = null
  if (item) await order.addItem(item, pricingQuantity.value, priceCents)
  pricingQuantity.value = 1
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
          @select="selectCategory($event)"
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
            :class="{ on: codePadOpen }"
            :aria-pressed="codePadOpen"
            data-testid="code-pad-toggle"
            @click="toggleCodePad"
          >
            {{ t('code_pad_toggle') }}
          </button>
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
        <div class="menu-body">
          <CodePad
            v-if="codePadOpen"
            v-model="codeText"
            :letters="codeKeys"
            :message="codeMessage.text"
            :tone="codeMessage.tone"
            :listening="codePadListening"
            :take-key="takePadKey"
            @submit="submitCode"
          />
          <MenuGrid
            :items="codeResults ?? menu.items"
            :category-color="codeResults ? null : selectedCategoryColor"
            :cart-quantities="cartQuantities"
            :columns="codePadOpen ? 3 : 5"
            :highlight-id="codeResults ? (codePick?.id ?? null) : null"
            :empty-text="codeResults ? t('code_pad_no_result') : undefined"
            @add-item="handleGridTap"
          />
        </div>
      </div>
      <CartSidebar />
    </div>

    <component
      :is="ModifierDialog"
      v-if="choosing"
      :key="choosing.id"
      :item="choosing"
      :for-weight="choosing.sold_by_weight === true"
      @weigh="weighChosen"
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

.menu-body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.menu-section {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
