<script setup lang="ts">
// Option picker (HubRise builds only): the product's option groups from
// the mirrored catalog. A group with a maximum of one is a single
// choice; the others toggle. The price shown is a preview: the backend
// prices the options and names the line.
//
// With `forWeight` (a product sold by weight, a malatang bowl and its
// broth) nothing is added here: "Peser" hands the picks to the order
// screen, which weighs the bowl with them.
import { computed, onMounted, ref } from 'vue'
import type { MenuItem } from '../types'
import {
  getItemModifiers,
  addOrderLineWithModifiers,
  type ModifierGroup,
} from '../api/ipc-modifiers'
import { useOrderStore } from '../stores/order'
import { usePosStore } from '../stores/pos'
import { useI18n } from '../i18n'
import { formatPrice } from '../utils/format'
import AllergenBadges from './AllergenBadges.vue'

const { item, forWeight = false } = defineProps<{ item: MenuItem; forWeight?: boolean }>()
const emit = defineEmits<{ done: []; weigh: [modifierIds: number[]] }>()

const { t } = useI18n()
const order = useOrderStore()
const pos = usePosStore()

const groups = ref<ModifierGroup[]>([])
const picked = ref<Set<number>>(new Set())
const loaded = ref(false)
const busy = ref(false)

onMounted(async () => {
  try {
    groups.value = (await getItemModifiers(item.id, pos.language)) ?? []
  } finally {
    loaded.value = true
  }
  // Nothing to pick: the plain product, as without options.
  if (groups.value.length === 0) {
    if (forWeight) emit('weigh', [])
    else await order.addItem(item)
    emit('done')
  }
})

function countIn(group: ModifierGroup): number {
  return group.options.filter((o) => picked.value.has(o.id)).length
}

function toggle(group: ModifierGroup, id: number) {
  const next = new Set(picked.value)
  if (next.has(id)) {
    next.delete(id)
  } else {
    // A single choice replaces the previous one.
    if (group.max === 1) for (const o of group.options) next.delete(o.id)
    else if (group.max !== null && countIn(group) >= group.max) return
    next.add(id)
  }
  picked.value = next
}

function rule(group: ModifierGroup): string {
  if (group.min > 0 && group.max === group.min) return t('modifier_required', { min: group.min })
  if (group.min > 0)
    return group.max === null
      ? t('modifier_required', { min: group.min })
      : t('modifier_range', { min: group.min, max: group.max })
  return group.max === null ? t('modifier_optional') : t('modifier_up_to', { max: group.max })
}

const valid = computed(() =>
  groups.value.every((g) => {
    const n = countIn(g)
    return n >= g.min && (g.max === null || n <= g.max)
  }),
)

const optionsCents = computed(() =>
  groups.value
    .flatMap((g) => g.options)
    .filter((o) => picked.value.has(o.id))
    .reduce((sum, o) => sum + Math.max(o.price_cents, 0), 0),
)

const total = computed(() => item.price_cents + optionsCents.value)

// A weighed product: its price per kilo, plus what the paid options add.
const weighLabel = computed(() => {
  const perKilo = t('modifier_per_kilo', { price: formatPrice(item.price_cents) })
  return optionsCents.value > 0 ? `${perKilo} + ${formatPrice(optionsCents.value)} €` : perKilo
})

async function confirm() {
  if (!valid.value || busy.value) return
  if (forWeight) {
    emit('weigh', [...picked.value])
    emit('done')
    return
  }
  busy.value = true
  const ids = [...picked.value]
  await order.addLineWith(item, 1, (orderId) => addOrderLineWithModifiers(orderId, item.id, 1, ids))
  busy.value = false
  emit('done')
}
</script>

<template>
  <div v-if="loaded && groups.length > 0" class="backdrop" @click.self="emit('done')">
    <div
      class="dialog"
      role="dialog"
      :aria-label="t('modifier_title')"
      data-testid="modifier-dialog"
    >
      <h2 class="title">{{ item.name }}</h2>
      <section v-for="group in groups" :key="group.id" class="group">
        <header class="group-head">
          <span class="group-name">{{ group.name }}</span>
          <span class="group-rule">{{ rule(group) }}</span>
        </header>
        <div class="options">
          <button
            v-for="o in group.options"
            :key="o.id"
            class="option"
            :class="{ selected: picked.has(o.id) }"
            :aria-pressed="picked.has(o.id)"
            @click="toggle(group, o.id)"
          >
            <span>{{ o.name }}</span>
            <span v-if="o.price_cents > 0" class="price">+{{ formatPrice(o.price_cents) }} €</span>
            <AllergenBadges v-if="o.allergens?.length" :codes="o.allergens" />
          </button>
        </div>
      </section>
      <div class="actions">
        <button class="btn btn-cancel" @click="emit('done')">{{ t('back') }}</button>
        <button
          class="btn btn-confirm"
          :disabled="!valid || busy"
          data-testid="modifier-confirm"
          @click="confirm"
        >
          <template v-if="forWeight">{{ t('modifier_weigh') }} · {{ weighLabel }}</template>
          <template v-else>{{ t('modifier_add') }} · {{ formatPrice(total) }} €</template>
        </button>
      </div>
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
  gap: var(--space-md);
  width: min(720px, 92vw);
  max-height: 88vh;
  overflow-y: auto;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
}
.group {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}
.group-head {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: var(--space-sm);
}
.group-name {
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
}
.group-rule {
  font-size: 14px;
  color: var(--text-secondary);
}
.options {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: var(--space-sm);
}
.option {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: 2px;
  min-height: 64px;
  padding: 8px 14px;
  border-radius: var(--radius-md);
  border: 2px solid var(--border);
  background: var(--bg-surface);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  text-align: left;
  transition: transform var(--tap-duration) ease;
}
.option:active {
  transform: scale(var(--tap-scale));
}
.option.selected {
  border-color: var(--accent-primary);
  background: color-mix(in srgb, var(--accent-primary) 18%, var(--bg-surface));
}
.price {
  font-size: 14px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}
.actions {
  display: flex;
  gap: var(--space-sm);
}
.btn {
  flex: 1;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  transition: transform var(--tap-duration) ease;
}
.btn:active {
  transform: scale(var(--tap-scale));
}
.btn-cancel {
  background: var(--bg-surface);
  color: var(--text-secondary);
}
.btn-confirm {
  background: var(--accent-primary);
  color: var(--text-on-accent);
}
.btn-confirm:disabled {
  opacity: 0.4;
}
</style>
