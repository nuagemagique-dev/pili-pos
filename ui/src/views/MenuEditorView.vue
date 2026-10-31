<script setup lang="ts">
// Menu editor (managers, V11-D17): categories on the left, the products of the chosen category
// in the middle, the edit form on the right with the on-screen keyboard (text) or the NumPad
// (price) under it. A product opens its option groups and their options. Nothing is deleted:
// "Visible à la caisse" off hides an entry. Every save is logged by the engine ("Historique").
// Saves are refused (and the form locked) while a catalog manages the menu.
// A category also carries its kitchen course (set_course_settings), a product or an option its
// allergen list (set_allergens); both are written by the same Enregistrer, after the entry.
import { computed, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import * as ipc from '../api/ipc'
import AppHeader from '../components/AppHeader.vue'
import NumPad from '../components/NumPad.vue'
import TextKeyboard from '../components/TextKeyboard.vue'
import { useI18n } from '../i18n'
import { useToastStore } from '../stores/toast'
import { describeError } from '../utils/errorMessages'
import { formatDayTime, formatTimestamp } from '../utils/datetime'
import { formatPrice } from '../utils/format'

type Form =
  | { kind: 'category'; data: ipc.SaveCategoryInput }
  | { kind: 'product'; data: ipc.SaveProductInput }
  | { kind: 'group'; data: ipc.SaveModifierGroupInput }
  | { kind: 'option'; data: ipc.SaveModifierInput }

const router = useRouter()
const toast = useToastStore()
const { t } = useI18n()

const menu = ref<ipc.MenuAdmin | null>(null)
const selectedCategory = ref<number | null>(null)
const form = ref<Form | null>(null)
const showHistory = ref(false)
const changes = ref<ipc.MenuChange[]>([])
const error = ref<string | null>(null)
const busy = ref(false)
// The field the keyboard types into: `name:<lang>`, `code` or `price`.
const focus = ref<string | null>(null)
const priceDigits = ref('')
// The 14 allergens in Annex order, for the allergen chips.
const legend = ref<ipc.AllergenEntry[]>([])
// The course setting as stored; null when it could not be read (the picker is then hidden).
const courseSettings = ref<ipc.CourseSettings | null>(null)
// The open form's kitchen course (category) and allergen list (product, option).
const courseDraft = ref(0)
interface AllergenDraft {
  declared: boolean
  contains: string[]
  may_contain: string[]
}
const allergenDraft = ref<AllergenDraft>({ declared: false, contains: [], may_contain: [] })

const VAT_CODES = ['A', 'B', 'C', 'D', 'X'] as const
const PALETTE = ['#E94560', '#FFB020', '#22C55E', '#00D4FF', '#9B59B6', '#D7FF3A', '#F2F4F2']
const MAX_PRICE_DIGITS = 6
/** Languages typed on the on-screen keyboard; Chinese has no input method on the kiosk. */
const KEYBOARD_LANGS = new Set(['fr', 'nl', 'en', 'de'])

const locked = computed(() => typeof menu.value?.managed_by === 'string')
const languages = computed(() => menu.value?.languages ?? ['fr'])
const products = computed(() =>
  (menu.value?.products ?? []).filter((p) => p.category_id === selectedCategory.value),
)
const groupsOf = (productId: number) =>
  (menu.value?.groups ?? []).filter((g) => g.menu_item_id === productId)
const optionsOf = (groupId: number) =>
  (menu.value?.options ?? []).filter((o) => o.group_id === groupId)

/** The name to show for an entry: the establishment's language, then any. */
function labelOf(names: ipc.MenuNames): string {
  const first = languages.value[0] ?? 'fr'
  return names[first] ?? names.fr ?? Object.values(names)[0] ?? ''
}

function langLabel(lang: string): string {
  return t(`menu_ed_lang_${lang}`)
}

async function load() {
  try {
    menu.value = await ipc.menuAdminGet()
    if (
      selectedCategory.value === null ||
      !menu.value.categories.some((c) => c.id === selectedCategory.value)
    ) {
      selectedCategory.value = menu.value.categories[0]?.id ?? null
    }
  } catch (e) {
    error.value = describeError(e)
  }
}

onMounted(async () => {
  await load()
  try {
    legend.value = await ipc.listAllergens()
  } catch {
    // No legend: the allergen section stays empty.
  }
  try {
    courseSettings.value = await ipc.getCourseSettings()
  } catch {
    // No course setting: the picker is hidden.
  }
})

// ── Kitchen course of a category and allergens of a product or an option ──

function courseOf(categoryId: number | null): number {
  if (categoryId === null) return 0
  return (
    courseSettings.value?.category_courses.find((c) => c.category_id === categoryId)?.course ?? 0
  )
}

type AllergenTarget = ipc.AdminAllergens['entity_type']

function storedAllergens(type: AllergenTarget, id: number | null): AllergenDraft {
  const found =
    id === null
      ? undefined
      : menu.value?.allergens.find((a) => a.entity_type === type && a.entity_id === id)
  return found
    ? { declared: true, contains: [...found.contains], may_contain: [...found.may_contain] }
    : { declared: false, contains: [], may_contain: [] }
}

/** The level of `code` in the draft: contains, traces or none. */
function levelOf(code: string): 'contains' | 'traces' | 'none' {
  if (allergenDraft.value.contains.includes(code)) return 'contains'
  if (allergenDraft.value.may_contain.includes(code)) return 'traces'
  return 'none'
}

/** A tap cycles an allergen: none, contains, traces, none. */
function cycleAllergen(code: string) {
  if (locked.value) return
  const d = allergenDraft.value
  const level = levelOf(code)
  const contains = d.contains.filter((c) => c !== code)
  const may = d.may_contain.filter((c) => c !== code)
  if (level === 'none') contains.push(code)
  else if (level === 'contains') may.push(code)
  allergenDraft.value = { declared: true, contains, may_contain: may }
}

function toggleDeclared() {
  if (locked.value) return
  allergenDraft.value = { ...allergenDraft.value, declared: !allergenDraft.value.declared }
}

/** Same list, whatever the order of the codes. */
function sameAllergens(a: AllergenDraft, b: AllergenDraft): boolean {
  if (a.declared !== b.declared) return false
  if (!a.declared) return true
  const same = (x: string[], y: string[]) => x.length === y.length && x.every((c) => y.includes(c))
  return same(a.contains, b.contains) && same(a.may_contain, b.may_contain)
}

/** Writes the draft list of the saved entry when it changed; true when it wrote. */
async function saveAllergens(type: AllergenTarget, id: number): Promise<boolean> {
  const draft = allergenDraft.value
  if (sameAllergens(draft, storedAllergens(type, id))) return false
  await ipc.setAllergens({
    entity_type: type,
    entity_id: id,
    declared: draft.declared,
    contains: draft.declared ? draft.contains : [],
    may_contain: draft.declared ? draft.may_contain : [],
  })
  return true
}

/** Writes the course of the saved category when it changed. */
async function saveCourse(categoryId: number) {
  const stored = courseSettings.value
  if (!stored || courseOf(categoryId) === courseDraft.value) return
  const others = stored.category_courses.filter((c) => c.category_id !== categoryId)
  const category_courses =
    courseDraft.value > 0
      ? [...others, { category_id: categoryId, course: courseDraft.value }]
      : others
  courseSettings.value = await ipc.setCourseSettings({ enabled: stored.enabled, category_courses })
}

function selectCategory(id: number) {
  selectedCategory.value = id
}

/** A row of the lists: select it and, unless HubRise manages the menu, open it in the form. */
function openCategory(c: ipc.AdminCategory) {
  selectCategory(c.id)
  showHistory.value = false
  if (!locked.value) editCategory(c)
}

function newCategory() {
  showHistory.value = false
  editCategory()
}

function openProduct(p: ipc.AdminProduct) {
  showHistory.value = false
  if (!locked.value) editProduct(p)
}

function newProduct() {
  showHistory.value = false
  editProduct()
}

function startPrice(cents: number) {
  priceDigits.value = cents > 0 ? String(cents) : ''
}

function newNames(): ipc.MenuNames {
  return {}
}

function editCategory(c?: ipc.AdminCategory) {
  error.value = null
  focus.value = null
  courseDraft.value = courseOf(c?.id ?? null)
  form.value = {
    kind: 'category',
    data: c
      ? { ...c, names: { ...c.names } }
      : {
          id: null,
          names: newNames(),
          sort_order: (menu.value?.categories.length ?? 0) + 1,
          color: null,
          active: true,
        },
  }
}

function editProduct(p?: ipc.AdminProduct) {
  if (!p && selectedCategory.value === null) return
  error.value = null
  focus.value = null
  allergenDraft.value = storedAllergens('menu_item', p?.id ?? null)
  form.value = {
    kind: 'product',
    data: p
      ? { ...p, names: { ...p.names } }
      : {
          id: null,
          category_id: selectedCategory.value as number,
          names: newNames(),
          price_cents: 0,
          vat_code: 'B',
          vat_code_takeaway: null,
          kitchen_printer: true,
          code: null,
          open_price: false,
          sold_by_weight: false,
          scale_tare_grams: null,
          sort_order: products.value.length + 1,
          active: true,
        },
  }
  startPrice(p?.price_cents ?? 0)
}

function editGroup(productId: number, g?: ipc.AdminGroup) {
  error.value = null
  focus.value = null
  form.value = {
    kind: 'group',
    data: g
      ? { ...g, names: { ...g.names } }
      : {
          id: null,
          menu_item_id: productId,
          names: newNames(),
          min_select: 0,
          max_select: 1,
          sort_order: groupsOf(productId).length + 1,
          active: true,
        },
  }
}

function editOption(groupId: number, o?: ipc.AdminOption) {
  error.value = null
  focus.value = null
  allergenDraft.value = storedAllergens('modifier', o?.id ?? null)
  form.value = {
    kind: 'option',
    data: o
      ? { ...o, names: { ...o.names } }
      : {
          id: null,
          group_id: groupId,
          names: newNames(),
          price_cents: 0,
          sort_order: optionsOf(groupId).length + 1,
          active: true,
        },
  }
  startPrice(o?.price_cents ?? 0)
}

/** From a group or an option back to its product (or group). */
function backToParent() {
  const f = form.value
  if (!f || !menu.value) return
  if (f.kind === 'option') {
    const g = menu.value.groups.find((x) => x.id === f.data.group_id)
    if (g) return editGroup(g.menu_item_id, g)
  }
  if (f.kind === 'group') {
    const p = menu.value.products.find((x) => x.id === f.data.menu_item_id)
    if (p) return editProduct(p)
  }
  form.value = null
}

function cancel() {
  form.value = null
  focus.value = null
  error.value = null
}

const hasParent = computed(() => form.value?.kind === 'group' || form.value?.kind === 'option')

// ── Text and price entry ─────────────────────────────────────────

const names = computed(() => form.value?.data.names ?? {})

function setName(lang: string, value: string) {
  if (form.value) form.value.data.names = { ...form.value.data.names, [lang]: value }
}

const keyboardValue = computed(() => {
  const f = form.value
  const key = focus.value
  if (!f || !key) return ''
  if (key.startsWith('name:')) return f.data.names[key.slice(5)] ?? ''
  if (key === 'code' && f.kind === 'product') return f.data.code ?? ''
  return ''
})

function onKeyboard(value: string) {
  const f = form.value
  const key = focus.value
  if (!f || !key) return
  if (key.startsWith('name:')) setName(key.slice(5), value)
  else if (key === 'code' && f.kind === 'product') f.data.code = value
}

const keyboardMax = computed(() => (focus.value === 'code' ? 12 : 60))
const showKeyboard = computed(
  () => focus.value !== null && focus.value !== 'price' && !locked.value,
)
const showNumPad = computed(() => focus.value === 'price' && !locked.value)

function onPriceKey(key: string) {
  if (key === 'BACKSPACE') priceDigits.value = priceDigits.value.slice(0, -1)
  else if (key === 'CLEAR') priceDigits.value = ''
  else if (
    priceDigits.value.length < MAX_PRICE_DIGITS &&
    !(priceDigits.value === '' && key === '0')
  ) {
    priceDigits.value += key
  }
  const f = form.value
  const cents = Number(priceDigits.value || '0')
  if (f && (f.kind === 'product' || f.kind === 'option')) f.data.price_cents = cents
}

function step(delta: number) {
  const f = form.value
  if (!f) return
  f.data.sort_order = Math.max(0, f.data.sort_order + delta)
}

// A product has one way to get its price: typed, or from the scale.
function toggleOpenPrice() {
  const f = form.value
  if (!f || f.kind !== 'product') return
  f.data.open_price = !f.data.open_price
  if (f.data.open_price) {
    f.data.sold_by_weight = false
    f.data.scale_tare_grams = null
  }
}

function toggleSoldByWeight() {
  const f = form.value
  if (!f || f.kind !== 'product') return
  f.data.sold_by_weight = !f.data.sold_by_weight
  if (f.data.sold_by_weight) f.data.open_price = false
  else f.data.scale_tare_grams = null
}

/** The engine keeps a tare between 0 and 9 999 g; none at zero. */
function stepTare(delta: number) {
  const f = form.value
  if (!f || f.kind !== 'product') return
  const next = Math.min(9995, Math.max(0, (f.data.scale_tare_grams ?? 0) + delta))
  f.data.scale_tare_grams = next === 0 ? null : next
}

function stepSelect(field: 'min_select' | 'max_select', delta: number) {
  const f = form.value
  if (!f || f.kind !== 'group') return
  const floor = field === 'max_select' ? 1 : 0
  f.data[field] = Math.max(floor, f.data[field] + delta)
  if (f.data.min_select > f.data.max_select) {
    if (field === 'min_select') f.data.max_select = f.data.min_select
    else f.data.min_select = f.data.max_select
  }
}

// ── Save ─────────────────────────────────────────────────────────

function trimmed(n: ipc.MenuNames): ipc.MenuNames {
  const out: ipc.MenuNames = {}
  for (const [lang, value] of Object.entries(n)) {
    const v = value.trim()
    if (v) out[lang] = v
  }
  return out
}

async function save() {
  const f = form.value
  if (!f || busy.value || locked.value) return
  busy.value = true
  error.value = null
  try {
    let saved: ipc.MenuSaved
    if (f.kind === 'category') {
      saved = await ipc.menuSaveCategory({ ...f.data, names: trimmed(f.data.names) })
    } else if (f.kind === 'product') {
      saved = await ipc.menuSaveProduct({ ...f.data, names: trimmed(f.data.names) })
    } else if (f.kind === 'group') {
      saved = await ipc.menuSaveModifierGroup({ ...f.data, names: trimmed(f.data.names) })
    } else {
      saved = await ipc.menuSaveModifier({ ...f.data, names: trimmed(f.data.names) })
    }
    menu.value = saved.menu
    // The course of a category and the allergens of a product or an option, once the entry has its id.
    if (f.kind === 'category') {
      await saveCourse(saved.id)
    } else if (f.kind === 'product' || f.kind === 'option') {
      const type: AllergenTarget = f.kind === 'product' ? 'menu_item' : 'modifier'
      if (await saveAllergens(type, saved.id)) menu.value = await ipc.menuAdminGet()
    }
    toast.show(t('menu_ed_saved'), 'success', 2500)
    const now = menu.value
    // Reopen what was saved, so a new product shows its option groups.
    if (f.kind === 'category') {
      selectedCategory.value = saved.id
      editCategory(now.categories.find((c) => c.id === saved.id))
    } else if (f.kind === 'product') {
      selectedCategory.value = f.data.category_id
      editProduct(now.products.find((p) => p.id === saved.id))
    } else if (f.kind === 'group') {
      editGroup(
        f.data.menu_item_id,
        now.groups.find((g) => g.id === saved.id),
      )
    } else {
      editOption(
        f.data.group_id,
        now.options.find((o) => o.id === saved.id),
      )
    }
  } catch (e) {
    error.value = describeError(e)
  } finally {
    busy.value = false
  }
}

// ── History ──────────────────────────────────────────────────────

async function openHistory() {
  showHistory.value = true
  form.value = null
  focus.value = null
  try {
    changes.value = await ipc.listMenuChanges(100)
  } catch (e) {
    error.value = describeError(e)
  }
}

function closeHistory() {
  showHistory.value = false
}

function fieldsOf(c: ipc.MenuChange): string {
  return c.changed_fields.map((f) => t(`menu_ed_field_${f}`)).join(', ')
}

const time = (iso: string) => formatTimestamp(iso, formatDayTime)

const vatLabel = (code: string) => t(`menu_ed_vat_${code.toLowerCase()}`)

const formTitle = computed(() => {
  const f = form.value
  if (!f) return ''
  const named = labelOf(f.data.names)
  if (named) return named
  return t(
    `menu_ed_new_${f.kind === 'category' ? 'category' : f.kind === 'product' ? 'product' : f.kind === 'group' ? 'group' : 'option'}`,
  )
})

const currentProduct = computed(() => {
  const f = form.value
  if (!f || f.kind !== 'product') return null
  const { id } = f.data
  return id === null ? null : { ...f.data, id }
})
const currentGroup = computed(() => {
  const f = form.value
  if (!f || f.kind !== 'group') return null
  const { id } = f.data
  return id === null ? null : { ...f.data, id }
})

// Plain object so the template can refer to it without the ref wrapper.
const ui = reactive({ colors: PALETTE })
</script>

<template>
  <div class="menu-editor" data-testid="menu-editor-view">
    <AppHeader />

    <div class="bar">
      <button class="bar-btn" data-testid="menu-ed-back" @click="router.push({ name: 'settings' })">
        {{ t('back') }}
      </button>
      <h1>{{ t('menu_ed_title') }}</h1>
      <button
        class="bar-btn"
        :class="{ active: showHistory }"
        data-testid="menu-ed-history-button"
        @click="showHistory ? closeHistory() : openHistory()"
      >
        {{ t('menu_ed_history') }}
      </button>
    </div>
    <p v-if="locked" class="locked" data-testid="menu-ed-locked">
      {{
        t('menu_editor_locked', {
          source: menu?.managed_by === 'hubrise' ? 'HubRise' : (menu?.managed_by ?? ''),
        })
      }}
    </p>

    <div class="columns">
      <!-- Categories -->
      <section class="col col-cats">
        <h2>{{ t('menu_ed_categories') }}</h2>
        <ul class="list scroll-touch">
          <li v-for="c in menu?.categories ?? []" :key="c.id">
            <button
              class="row"
              :class="{ selected: selectedCategory === c.id, hidden: !c.active }"
              data-testid="menu-ed-category-row"
              @click="openCategory(c)"
            >
              <span class="swatch" :style="{ background: c.color ?? 'transparent' }"></span>
              <span class="row-name">{{ labelOf(c.names) }}</span>
              <span v-if="!c.active" class="tag">{{ t('menu_ed_hidden') }}</span>
            </button>
          </li>
        </ul>
        <button
          v-if="!locked"
          class="add-btn"
          data-testid="menu-ed-add-category"
          @click="newCategory"
        >
          + {{ t('menu_ed_add_category') }}
        </button>
      </section>

      <!-- Products of the category -->
      <section class="col col-products">
        <h2>{{ t('menu_ed_products') }}</h2>
        <p v-if="products.length === 0" class="empty">{{ t('menu_ed_empty_products') }}</p>
        <ul v-else class="list scroll-touch">
          <li v-for="p in products" :key="p.id">
            <button
              class="row"
              :class="{ selected: currentProduct?.id === p.id, hidden: !p.active }"
              data-testid="menu-ed-product-row"
              @click="openProduct(p)"
            >
              <span class="row-name">{{ labelOf(p.names) }}</span>
              <span v-if="!p.active" class="tag">{{ t('menu_ed_hidden') }}</span>
              <span class="row-price">{{
                p.open_price
                  ? t('menu_ed_open_price')
                  : `${formatPrice(p.price_cents)} ${p.sold_by_weight ? '€/kg' : '€'}`
              }}</span>
            </button>
          </li>
        </ul>
        <button
          v-if="!locked"
          class="add-btn"
          data-testid="menu-ed-add-product"
          :disabled="selectedCategory === null"
          @click="newProduct"
        >
          + {{ t('menu_ed_add_product') }}
        </button>
      </section>

      <!-- Edit form, or the history -->
      <section class="col col-form">
        <template v-if="showHistory">
          <h2>{{ t('menu_ed_history_title') }}</h2>
          <p v-if="changes.length === 0" class="empty" data-testid="menu-ed-history-empty">
            {{ t('menu_ed_history_empty') }}
          </p>
          <ul v-else class="history scroll-touch">
            <li
              v-for="c in changes"
              :key="c.id"
              class="history-row"
              data-testid="menu-ed-history-row"
            >
              <div class="h-main">
                <strong>{{ t(`menu_ed_action_${c.action}`) }}</strong>
                <span>{{ t(`menu_ed_entity_${c.entity_type}`) }}</span>
                <span class="h-label">{{ c.label }}</span>
              </div>
              <div class="h-meta">
                {{ time(c.changed_at) }} ·
                {{ t('menu_ed_history_by', { name: c.changed_by_name }) }}
                <template v-if="c.action === 'update'"> · {{ fieldsOf(c) }}</template>
              </div>
            </li>
          </ul>
        </template>

        <template v-else-if="form">
          <div class="form-head">
            <button
              v-if="hasParent"
              class="link-btn"
              data-testid="menu-ed-parent"
              @click="backToParent"
            >
              &lsaquo; {{ t('back') }}
            </button>
            <h2 data-testid="menu-ed-form-title">{{ formTitle }}</h2>
          </div>

          <div class="form scroll-touch">
            <!-- Names -->
            <div v-for="(lang, i) in languages" :key="lang" class="field">
              <label>
                {{ t('menu_ed_name', { lang: langLabel(lang) }) }}
                <em v-if="i === 0">({{ t('menu_ed_required') }})</em>
              </label>
              <input
                v-if="KEYBOARD_LANGS.has(lang)"
                class="input"
                :class="{ focused: focus === `name:${lang}` }"
                :value="names[lang] ?? ''"
                readonly
                :data-testid="`menu-ed-name-${lang}`"
                @click="focus = `name:${lang}`"
              />
              <input
                v-else
                class="input"
                :value="names[lang] ?? ''"
                :data-testid="`menu-ed-name-${lang}`"
                :disabled="locked"
                @input="setName(lang, ($event.target as HTMLInputElement).value)"
              />
            </div>
            <p v-if="languages.includes('zh')" class="hint">{{ t('menu_ed_zh_note') }}</p>

            <!-- Category -->
            <template v-if="form.kind === 'category'">
              <div class="field">
                <label>{{ t('menu_ed_color') }}</label>
                <div class="swatches">
                  <button
                    class="chip"
                    :class="{ on: form.data.color === null }"
                    data-testid="menu-ed-color-none"
                    @click="form.data.color = null"
                  >
                    {{ t('menu_ed_color_none') }}
                  </button>
                  <button
                    v-for="c in ui.colors"
                    :key="c"
                    class="chip swatch-chip"
                    :class="{ on: form.data.color === c }"
                    :style="{ background: c }"
                    :aria-label="c"
                    @click="form.data.color = c"
                  ></button>
                </div>
              </div>
              <div v-if="courseSettings" class="field" data-testid="menu-ed-course">
                <label>{{ t('menu_ed_course') }}</label>
                <div class="chips">
                  <button
                    v-for="n in [0, 1, 2, 3]"
                    :key="n"
                    class="chip"
                    :class="{ on: courseDraft === n }"
                    :data-testid="`menu-ed-course-${n}`"
                    @click="courseDraft = n"
                  >
                    {{ t(`course_${n}`) }}
                  </button>
                </div>
                <p v-if="!courseSettings.enabled" class="hint" data-testid="menu-ed-course-off">
                  {{ t('menu_ed_course_off') }}
                </p>
              </div>
            </template>

            <!-- Product -->
            <template v-if="form.kind === 'product'">
              <div class="field">
                <label>{{ t('menu_ed_category') }}</label>
                <div class="chips">
                  <button
                    v-for="c in menu?.categories ?? []"
                    :key="c.id"
                    class="chip"
                    :class="{ on: form.data.category_id === c.id }"
                    @click="form.data.category_id = c.id"
                  >
                    {{ labelOf(c.names) }}
                  </button>
                </div>
              </div>
              <div class="field">
                <label>{{
                  t(form.data.sold_by_weight ? 'menu_ed_price_per_kilo' : 'menu_ed_price')
                }}</label>
                <input
                  class="input mono"
                  :class="{ focused: focus === 'price' }"
                  :value="formatPrice(form.data.price_cents)"
                  readonly
                  data-testid="menu-ed-price"
                  @click="focus = 'price'"
                />
              </div>
              <div class="field">
                <label>{{ t('menu_ed_vat') }}</label>
                <div class="chips">
                  <button
                    v-for="code in VAT_CODES"
                    :key="code"
                    class="chip"
                    :class="{ on: form.data.vat_code === code }"
                    :data-testid="`menu-ed-vat-${code}`"
                    @click="form.data.vat_code = code"
                  >
                    {{ vatLabel(code) }}
                  </button>
                </div>
              </div>
              <div class="field">
                <label>{{ t('menu_ed_vat_takeaway') }}</label>
                <div class="chips">
                  <button
                    class="chip"
                    :class="{ on: form.data.vat_code_takeaway === null }"
                    data-testid="menu-ed-vat-takeaway-same"
                    @click="form.data.vat_code_takeaway = null"
                  >
                    {{ t('menu_ed_vat_same') }}
                  </button>
                  <button
                    v-for="code in VAT_CODES"
                    :key="code"
                    class="chip"
                    :class="{ on: form.data.vat_code_takeaway === code }"
                    :data-testid="`menu-ed-vat-takeaway-${code}`"
                    @click="form.data.vat_code_takeaway = code"
                  >
                    {{ vatLabel(code) }}
                  </button>
                </div>
              </div>
              <div class="field">
                <label>{{ t('menu_ed_code') }}</label>
                <input
                  class="input mono"
                  :class="{ focused: focus === 'code' }"
                  :value="form.data.code ?? ''"
                  readonly
                  data-testid="menu-ed-code"
                  @click="focus = 'code'"
                />
              </div>
              <button
                class="toggle"
                :class="{ on: form.data.kitchen_printer }"
                data-testid="menu-ed-kitchen"
                @click="form.data.kitchen_printer = !form.data.kitchen_printer"
              >
                {{ t('menu_ed_kitchen') }}
              </button>
              <button
                class="toggle"
                :class="{ on: form.data.open_price }"
                data-testid="menu-ed-open-price"
                @click="toggleOpenPrice"
              >
                {{ t('menu_ed_open_price') }}
              </button>
              <button
                class="toggle"
                :class="{ on: form.data.sold_by_weight }"
                data-testid="menu-ed-sold-by-weight"
                @click="toggleSoldByWeight"
              >
                {{ t('menu_ed_sold_by_weight') }}
              </button>
              <div v-if="form.data.sold_by_weight" class="field">
                <label>{{ t('menu_ed_tare') }}</label>
                <div class="stepper">
                  <button class="step-btn" data-testid="menu-ed-tare-minus" @click="stepTare(-5)">
                    &minus;
                  </button>
                  <span class="step-value" data-testid="menu-ed-tare"
                    >{{ form.data.scale_tare_grams ?? 0 }} g</span
                  >
                  <button class="step-btn" data-testid="menu-ed-tare-plus" @click="stepTare(5)">
                    +
                  </button>
                </div>
              </div>
            </template>

            <!-- Option group -->
            <template v-if="form.kind === 'group'">
              <div class="field">
                <label>{{ t('menu_ed_group_min') }}</label>
                <div class="stepper">
                  <button
                    class="step-btn"
                    data-testid="menu-ed-min-minus"
                    @click="stepSelect('min_select', -1)"
                  >
                    &minus;
                  </button>
                  <span class="step-value" data-testid="menu-ed-min">{{
                    form.data.min_select
                  }}</span>
                  <button
                    class="step-btn"
                    data-testid="menu-ed-min-plus"
                    @click="stepSelect('min_select', 1)"
                  >
                    +
                  </button>
                </div>
              </div>
              <div class="field">
                <label>{{ t('menu_ed_group_max') }}</label>
                <div class="stepper">
                  <button
                    class="step-btn"
                    data-testid="menu-ed-max-minus"
                    @click="stepSelect('max_select', -1)"
                  >
                    &minus;
                  </button>
                  <span class="step-value" data-testid="menu-ed-max">{{
                    form.data.max_select
                  }}</span>
                  <button
                    class="step-btn"
                    data-testid="menu-ed-max-plus"
                    @click="stepSelect('max_select', 1)"
                  >
                    +
                  </button>
                </div>
              </div>
            </template>

            <!-- Option -->
            <template v-if="form.kind === 'option'">
              <div class="field">
                <label>{{ t('menu_ed_price') }}</label>
                <input
                  class="input mono"
                  :class="{ focused: focus === 'price' }"
                  :value="formatPrice(form.data.price_cents)"
                  readonly
                  data-testid="menu-ed-price"
                  @click="focus = 'price'"
                />
              </div>
            </template>

            <!-- Position and visibility, for every entity -->
            <div class="field">
              <label>{{ t('menu_ed_position') }}</label>
              <div class="stepper">
                <button class="step-btn" data-testid="menu-ed-pos-minus" @click="step(-1)">
                  &minus;
                </button>
                <span class="step-value" data-testid="menu-ed-pos">{{ form.data.sort_order }}</span>
                <button class="step-btn" data-testid="menu-ed-pos-plus" @click="step(1)">+</button>
              </div>
            </div>
            <button
              class="toggle"
              :class="{ on: form.data.active }"
              data-testid="menu-ed-active"
              @click="form.data.active = !form.data.active"
            >
              {{ t('menu_ed_active') }}
            </button>

            <!-- Allergens of a saved product or option (set_allergens) -->
            <div
              v-if="form.kind === 'product' || form.kind === 'option'"
              class="sub"
              data-testid="menu-ed-allergens"
            >
              <h3>{{ t('menu_ed_allergens') }}</h3>
              <p v-if="form.data.id === null" class="hint">
                {{ t('menu_ed_allergens_save_first') }}
              </p>
              <template v-else>
                <button
                  class="toggle"
                  :class="{ on: allergenDraft.declared }"
                  data-testid="menu-ed-allergens-declared"
                  @click="toggleDeclared"
                >
                  {{ t('menu_ed_allergens_declared') }}
                </button>
                <p
                  v-if="!allergenDraft.declared"
                  class="hint"
                  data-testid="menu-ed-allergens-unknown"
                >
                  {{ t('menu_ed_allergens_unknown') }}
                </p>
                <template v-else>
                  <p class="hint">{{ t('menu_ed_allergens_hint') }}</p>
                  <div class="allergen-chips">
                    <button
                      v-for="a in legend"
                      :key="a.code"
                      class="allergen-chip"
                      :class="levelOf(a.code)"
                      :data-testid="`menu-ed-allergen-${a.code}`"
                      :data-level="levelOf(a.code)"
                      @click="cycleAllergen(a.code)"
                    >
                      <span class="a-num">{{ a.number }}</span>
                      <span class="a-name">{{ t(`allergen_${a.code}`) }}</span>
                      <span v-if="levelOf(a.code) !== 'none'" class="a-level">
                        {{
                          levelOf(a.code) === 'contains'
                            ? t('allergens_contains')
                            : t('allergens_traces')
                        }}
                      </span>
                    </button>
                  </div>
                  <p
                    v-if="
                      allergenDraft.contains.length === 0 && allergenDraft.may_contain.length === 0
                    "
                    class="hint"
                    data-testid="menu-ed-allergens-none"
                  >
                    {{ t('allergens_none') }}
                  </p>
                </template>
              </template>
            </div>

            <!-- Option groups of a saved product -->
            <div v-if="form.kind === 'product'" class="sub">
              <h3>{{ t('menu_ed_groups') }}</h3>
              <p v-if="!currentProduct" class="hint">{{ t('menu_ed_save_first') }}</p>
              <template v-else>
                <div v-for="g in groupsOf(currentProduct.id)" :key="g.id" class="sub-row">
                  <span class="sub-name" :class="{ hidden: !g.active }">
                    {{ labelOf(g.names) }} ({{ g.min_select }}-{{ g.max_select }})
                  </span>
                  <button
                    class="link-btn"
                    data-testid="menu-ed-edit-group"
                    @click="editGroup(currentProduct.id, g)"
                  >
                    {{ t('menu_ed_edit') }}
                  </button>
                </div>
                <button
                  class="add-btn"
                  data-testid="menu-ed-add-group"
                  @click="editGroup(currentProduct.id)"
                >
                  + {{ t('menu_ed_add_group') }}
                </button>
              </template>
            </div>

            <!-- Options of a saved group -->
            <div v-if="form.kind === 'group'" class="sub">
              <h3>{{ t('menu_ed_options') }}</h3>
              <p v-if="!currentGroup" class="hint">{{ t('menu_ed_save_first') }}</p>
              <template v-else>
                <div v-for="o in optionsOf(currentGroup.id)" :key="o.id" class="sub-row">
                  <span class="sub-name" :class="{ hidden: !o.active }">
                    {{ labelOf(o.names) }} · {{ formatPrice(o.price_cents) }} €
                  </span>
                  <button
                    class="link-btn"
                    data-testid="menu-ed-edit-option"
                    @click="editOption(currentGroup.id, o)"
                  >
                    {{ t('menu_ed_edit') }}
                  </button>
                </div>
                <button
                  class="add-btn"
                  data-testid="menu-ed-add-option"
                  @click="editOption(currentGroup.id)"
                >
                  + {{ t('menu_ed_add_option') }}
                </button>
              </template>
            </div>
          </div>

          <p v-if="error" class="error" role="alert" data-testid="menu-ed-error">{{ error }}</p>
          <div class="actions">
            <button class="btn btn-cancel" data-testid="menu-ed-cancel" @click="cancel">
              {{ t('menu_ed_cancel') }}
            </button>
            <button
              class="btn btn-save"
              data-testid="menu-ed-save"
              :disabled="busy || locked"
              @click="save"
            >
              {{ t('menu_ed_save') }}
            </button>
          </div>

          <div v-if="showKeyboard" class="pad">
            <TextKeyboard
              :model-value="keyboardValue"
              :max-length="keyboardMax"
              case-toggle
              @update:model-value="onKeyboard"
              @submit="focus = null"
            />
          </div>
          <div v-else-if="showNumPad" class="pad">
            <NumPad :model-value="priceDigits" @update:model-value="onPriceKey" />
          </div>
        </template>

        <p v-else class="empty" data-testid="menu-ed-hint">{{ t('menu_ed_select_hint') }}</p>
        <p v-if="error && !form" class="error" role="alert">{{ error }}</p>
      </section>
    </div>
  </div>
</template>

<style scoped>
.menu-editor {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  background: var(--bg-primary);
}
.bar {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-sm) var(--space-md);
  flex-shrink: 0;
}
.bar h1 {
  flex: 1;
  font-size: var(--font-size-xl);
  font-weight: var(--font-weight-heavy);
}
.bar-btn {
  min-height: 56px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.bar-btn.active {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.locked {
  margin: 0 var(--space-md);
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-md);
  background: var(--accent-warning);
  color: var(--text-on-accent);
  font-weight: var(--font-weight-bold);
}

.columns {
  display: flex;
  gap: var(--space-md);
  padding: var(--space-sm) var(--space-md) var(--space-md);
  flex: 1;
  min-height: 0;
}
.col {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  min-height: 0;
  padding: var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.col h2 {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.col-cats {
  width: 340px;
  flex-shrink: 0;
}
.col-products {
  width: 460px;
  flex-shrink: 0;
}
.col-form {
  flex: 1;
  min-width: 0;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}
.row {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  width: 100%;
  min-height: 64px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  text-align: left;
}
.row.selected {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.row.hidden {
  opacity: 0.55;
}
.row-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.row-price {
  font-family: var(--font-mono);
}
.swatch {
  width: 14px;
  height: 32px;
  border-radius: 4px;
  border: 1px solid var(--border);
  flex-shrink: 0;
}
.tag {
  font-size: var(--font-size-xs);
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--bg-pressed);
  color: var(--text-secondary);
}
.add-btn {
  min-height: 56px;
  border-radius: var(--radius-md);
  background: var(--bg-primary);
  color: var(--text-primary);
  border: 2px dashed var(--accent-info);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.add-btn:disabled {
  opacity: 0.4;
}
.empty {
  color: var(--text-secondary);
  font-size: var(--font-size-md);
}

.form-head {
  display: flex;
  align-items: center;
  gap: var(--space-md);
}
.link-btn {
  min-height: 48px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--accent-info);
  font-weight: var(--font-weight-bold);
}
.form {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}
.field label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.field label em {
  font-style: normal;
  color: var(--accent-warning);
}
.input {
  height: 52px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  border: 2px solid transparent;
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
}
.input.focused {
  border-color: var(--accent-info);
}
.mono {
  font-family: var(--font-mono);
}
.hint {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.chips,
.swatches {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-xs);
}
.chip {
  min-height: 52px;
  min-width: 64px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border: 2px solid transparent;
}
.chip.on {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.swatch-chip {
  width: 52px;
  min-width: 52px;
  padding: 0;
}
.swatch-chip.on {
  border-color: var(--text-primary);
}
.toggle {
  min-height: 52px;
  padding: 0 var(--space-md);
  border-radius: var(--radius-md);
  text-align: left;
  background: var(--bg-surface);
  color: var(--text-secondary);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.toggle.on {
  background: var(--accent-success);
  color: var(--text-on-accent);
}
.stepper {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
}
.step-btn {
  width: 64px;
  height: 52px;
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.step-value {
  min-width: 48px;
  text-align: center;
  font-family: var(--font-mono);
  font-size: var(--font-size-lg);
}
.sub {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  margin-top: var(--space-sm);
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border);
}
.sub h3 {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.sub-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-sm);
}
.sub-name.hidden {
  opacity: 0.55;
}
.allergen-chips {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-xs);
}
.allergen-chip {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  min-height: 52px;
  padding: 0 var(--space-sm);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-bold);
  text-align: left;
  border: 2px solid transparent;
}
.allergen-chip.contains {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.allergen-chip.traces {
  border-color: var(--accent-warning);
}
.a-num {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  flex-shrink: 0;
  border-radius: 50%;
  background: var(--accent-info);
  color: var(--text-on-accent);
  font-family: var(--font-mono);
  font-size: 12px;
}
.a-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.a-level {
  font-size: var(--font-size-xs);
  color: inherit;
  opacity: 0.85;
}

.error {
  color: var(--accent-danger);
  font-size: var(--font-size-md);
  overflow-wrap: anywhere;
}
.actions {
  display: flex;
  gap: var(--space-sm);
}
.btn {
  flex: 1;
  height: 56px;
  border-radius: var(--radius-md);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.btn:active {
  transform: scale(var(--tap-scale));
}
.btn-cancel {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
.btn-save {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.btn-save:disabled {
  opacity: 0.4;
}
.pad {
  display: flex;
  justify-content: center;
  flex-shrink: 0;
  padding-top: var(--space-xs);
}

.history {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}
.history-row {
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
}
.h-main {
  display: flex;
  gap: var(--space-sm);
  align-items: baseline;
}
.h-label {
  color: var(--text-secondary);
}
.h-meta {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
</style>
