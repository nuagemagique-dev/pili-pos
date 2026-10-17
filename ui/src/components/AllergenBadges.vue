<script setup lang="ts">
// The allergens a product (or an option) contains, as the numbered circles
// of the EU Annex II (1 to 14), in Annex order. At most `max` circles, then
// "+n". Numbers only, no drawn icon: the legend lists the 14 names.
import { computed } from "vue";
import { TILE_BADGES, tileBadges } from "../utils/allergens";
import { useI18n } from "../i18n";

const props = withDefaults(
  defineProps<{
    /** Allergen codes the item contains (traces are not shown on a tile). */
    codes: readonly string[];
    max?: number;
  }>(),
  { max: TILE_BADGES },
);

const { t } = useI18n();
const badges = computed(() => tileBadges(props.codes, props.max));
const label = computed(() =>
  `${t("allergens_contains")}: ${badges.value.shown.join(", ")}${badges.value.more ? ` +${badges.value.more}` : ""}`,
);
</script>

<template>
  <span v-if="badges.shown.length > 0" class="allergen-badges" data-testid="allergen-badges" :aria-label="label">
    <span v-for="n in badges.shown" :key="n" class="allergen-badge" data-testid="allergen-badge">{{ n }}</span>
    <span v-if="badges.more > 0" class="allergen-more" data-testid="allergen-more">+{{ badges.more }}</span>
  </span>
</template>

<style scoped>
.allergen-badges {
  display: inline-flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 3px;
}
.allergen-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--accent-info);
  color: var(--text-on-accent);
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: var(--font-weight-bold);
  line-height: 1;
}
.allergen-more {
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: var(--font-weight-bold);
  color: var(--text-secondary);
}
</style>
