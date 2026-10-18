<script setup lang="ts">
// The allergen sheet of one product: what it contains and may contain
// (names in the UI language, then in French), its options, or "information
// not available" when nothing is declared (never "no allergen"). Buttons
// print this sheet or the whole menu's.
import { computed } from "vue";
import type { AllergenRef, AllergenSheetOption } from "../api/ipc";
import { useAllergensStore } from "../stores/allergens";
import { usePosStore } from "../stores/pos";
import { useI18n } from "../i18n";

const { t } = useI18n();
const store = useAllergensStore();
const pos = usePosStore();

const product = computed(() => store.sheet);

/** The UI-language names, with the French ones after when the language differs. */
function names(list: AllergenRef[], fr: AllergenRef[] | undefined): string {
  const own = list.map((a) => `${a.number} ${a.name}`);
  if (pos.language === "fr" || !fr || fr.length === 0) return own.join(", ");
  return `${own.join(", ")} (${fr.map((a) => a.name).join(", ")})`;
}

function frOption(option: AllergenSheetOption): AllergenSheetOption | undefined {
  return store.sheetFr?.options.find((o: AllergenSheetOption) => o.name === option.name && o.group === option.group) ?? undefined;
}

const outcome = computed(() => {
  switch (store.printResult) {
    case "printed":
      return { ok: true, text: t("allergens_printed") };
    case "no_printer":
      return { ok: false, text: t("allergens_no_printer") };
    case "nothing_to_print":
      return { ok: false, text: t("allergens_nothing") };
    case "failed":
      return { ok: false, text: t("allergens_print_failed") };
    default:
      return null;
  }
});
</script>

<template>
  <div v-if="store.sheetItem" class="backdrop" @click.self="store.closeSheet()">
    <div class="dialog" role="dialog" :aria-label="t('allergens_title')" data-testid="allergen-sheet">
      <h2 class="title">{{ store.sheetItem.name }}</h2>
      <p class="reg">{{ t('allergens_regulation') }}</p>

      <p v-if="store.error" class="error" data-testid="allergen-error">{{ store.error }}</p>

      <template v-if="product">
        <section class="block">
          <p v-if="!product.declared" class="unknown" data-testid="allergen-unknown">{{ t('allergens_unknown') }}</p>
          <template v-else>
            <p v-if="product.contains.length === 0 && product.traces.length === 0" class="none" data-testid="allergen-none">
              {{ t('allergens_none') }}
            </p>
            <p v-if="product.contains.length > 0" class="line" data-testid="allergen-contains">
              <span class="label">{{ t('allergens_contains') }}</span>
              <span>{{ names(product.contains, store.sheetFr?.contains) }}</span>
            </p>
            <p v-if="product.traces.length > 0" class="line" data-testid="allergen-traces">
              <span class="label">{{ t('allergens_traces') }}</span>
              <span>{{ names(product.traces, store.sheetFr?.traces) }}</span>
            </p>
          </template>
        </section>

        <section v-if="product.options.length > 0" class="block" data-testid="allergen-options">
          <h3 class="subtitle">{{ t('allergens_options') }}</h3>
          <div v-for="o in product.options" :key="`${o.group}|${o.name}`" class="option">
            <span class="option-name">{{ o.name }}</span>
            <span v-if="!o.declared" class="unknown">{{ t('allergens_unknown') }}</span>
            <span v-else-if="o.contains.length === 0 && o.traces.length === 0" class="none">{{ t('allergens_none') }}</span>
            <span v-else>
              <template v-if="o.contains.length > 0">
                {{ t('allergens_contains') }}: {{ names(o.contains, frOption(o)?.contains) }}
              </template>
              <template v-if="o.traces.length > 0">
                {{ o.contains.length > 0 ? " · " : "" }}{{ t('allergens_traces') }}: {{ names(o.traces, frOption(o)?.traces) }}
              </template>
            </span>
          </div>
        </section>
      </template>

      <p v-if="outcome" class="outcome" :class="{ ok: outcome.ok }" data-testid="allergen-print-outcome">{{ outcome.text }}</p>

      <div class="actions">
        <button class="btn" :disabled="store.busy" data-testid="allergen-print-one" @click="store.print(false)">
          {{ t('allergens_print_one') }}
        </button>
        <button class="btn" :disabled="store.busy" data-testid="allergen-print-all" @click="store.print(true)">
          {{ t('allergens_print_all') }}
        </button>
        <button class="btn btn-close" data-testid="allergen-close" @click="store.closeSheet()">
          {{ t('allergens_close') }}
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
  width: min(760px, 92vw);
  max-height: 88vh;
  overflow-y: auto;
  padding: var(--space-lg);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.title { font-size: var(--font-size-lg); font-weight: var(--font-weight-bold); }
.reg { font-size: var(--font-size-xs); color: var(--text-secondary); }
.subtitle { font-size: var(--font-size-md); font-weight: var(--font-weight-bold); }
.block { display: flex; flex-direction: column; gap: var(--space-sm); }
.line { display: flex; flex-direction: column; gap: 2px; font-size: var(--font-size-md); }
.label { font-size: var(--font-size-sm); font-weight: var(--font-weight-bold); color: var(--accent-info); }
.unknown { color: var(--accent-warning); font-weight: var(--font-weight-bold); }
.none { color: var(--accent-success); font-weight: var(--font-weight-bold); }
.option {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  background: var(--bg-surface);
  font-size: var(--font-size-sm);
}
.option-name { font-weight: var(--font-weight-bold); font-size: var(--font-size-md); }
.error { color: var(--accent-danger); font-weight: var(--font-weight-bold); }
.outcome { font-weight: var(--font-weight-bold); color: var(--accent-danger); }
.outcome.ok { color: var(--accent-success); }
.actions { display: flex; gap: var(--space-sm); }
.btn {
  flex: 1;
  height: 56px;
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}
.btn:active { transform: scale(var(--tap-scale)); }
.btn:disabled { opacity: 0.4; }
.btn-close { background: var(--selected-bg); color: var(--selected-fg); }
</style>
