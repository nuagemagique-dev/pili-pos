<script setup lang="ts">
// Availability of one product ("Épuisé", V11-D16): sold out until the end of the business
// day, sold out until re-enabled, or available. Opened from the order screen's "Épuisé" mode.
import { computed, ref } from "vue";
import type { MenuItem } from "../types";
import { useMenuStore } from "../stores/menu";
import { useI18n } from "../i18n";
import { useBilingualName } from "../utils/names";
import { availabilityOf, endOfDayLabel, type Availability } from "../utils/soldOut";

const props = defineProps<{ item: MenuItem }>();
const emit = defineEmits<{ done: [] }>();

const { t } = useI18n();
const menu = useMenuStore();
const bilingual = useBilingualName();
const names = computed(() => bilingual(props.item));
const current = computed(() => availabilityOf(props.item));
const busy = ref(false);

const choices: { value: Availability; label: string }[] = [
  { value: "day", label: "soldout_until_day" },
  { value: "until", label: "soldout_until_reenabled" },
  { value: "available", label: "soldout_available" },
];

async function choose(choice: Availability) {
  if (busy.value) return;
  if (choice === current.value) {
    emit("done");
    return;
  }
  busy.value = true;
  const ok = await menu.setAvailability(props.item.id, choice);
  busy.value = false;
  if (ok) emit("done");
}
</script>

<template>
  <div class="backdrop" @click.self="emit('done')">
    <div class="dialog" role="dialog" data-testid="soldout-dialog" :aria-label="t('soldout_mode')">
      <h2 class="title">{{ names.primary }}</h2>
      <p v-if="item.sold_out" class="status" data-testid="soldout-status">
        {{
          item.sold_out_until
            ? t('soldout_status_day', { time: endOfDayLabel(item.sold_out_until) })
            : t('soldout_status_reenabled')
        }}
      </p>
      <button
        v-for="c in choices"
        :key="c.value"
        class="choice"
        :class="{ current: c.value === current, danger: c.value !== 'available' }"
        :data-testid="`soldout-choice-${c.value}`"
        :disabled="busy"
        @click="choose(c.value)"
      >
        {{ t(c.label) }}
      </button>
      <button class="btn-cancel" data-testid="soldout-cancel" @click="emit('done')">
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
  width: 480px;
  max-width: 94vw;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); text-align: center; }
.status { text-align: center; color: var(--text-secondary); font-size: var(--font-size-md); }
.choice {
  min-height: 64px;
  padding: 0 var(--space-md);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  border: 2px solid transparent;
  transition: transform var(--tap-duration) ease;
}
.choice:active { transform: scale(var(--tap-scale)); }
.choice.current { border-color: var(--accent-primary); }
.choice.danger.current { border-color: var(--accent-warning); }
.choice:disabled { opacity: 0.5; }
.btn-cancel {
  min-height: 56px;
  margin-top: var(--space-sm);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-secondary);
}
.btn-cancel:active { transform: scale(var(--tap-scale)); }
</style>
