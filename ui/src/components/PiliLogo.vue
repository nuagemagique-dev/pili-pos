<script setup lang="ts">
/**
 * The Pili logo ("Frappe" with the pinyin tones, brand guide §1).
 *
 * - `wordmark`: "pili" in geometric lower case. A Z-shaped bolt cuts
 *   through the p; the dots of the two i are the tones of pī lì
 *   (flat, then falling), drawn in `markColor` (Volt on dark).
 * - `mark`: the struck p alone, used as the icon and below 24 px.
 */
import { computed, useId } from 'vue'

const props = withDefaults(
  defineProps<{
    kind?: 'wordmark' | 'mark'
    color?: string
    /** Colour of the tone marks; defaults to `color` (ink on light). */
    markColor?: string
    height?: number
  }>(),
  { kind: 'wordmark', color: 'var(--text-primary)', markColor: undefined, height: 24 },
)

// Masks are referenced by id: each instance needs its own.
const maskId = `pili-logo-${useId()}`

const viewBox = computed(() => (props.kind === 'mark' ? [0, 0, 100, 140] : [0, -62, 244, 204]))
const width = computed(
  () => Math.round(((props.height * viewBox.value[2]) / viewBox.value[3]) * 10) / 10,
)
const toneColor = computed(() => props.markColor ?? props.color)
// The bolt is a gap in the p, slightly wider on the small icon.
const gap = computed(() => (props.kind === 'mark' ? 11 : 10))
</script>

<template>
  <svg
    :viewBox="viewBox.join(' ')"
    :width="width"
    :height="height"
    class="pili-logo"
    role="img"
    aria-label="Pili"
  >
    <defs>
      <mask :id="maskId" maskUnits="userSpaceOnUse" x="-20" y="-60" width="300" height="260">
        <rect x="-20" y="-60" width="300" height="260" fill="#fff" />
        <path
          d="M84,-30 L42,60 L70,52 L24,170"
          fill="none"
          stroke="#000"
          :stroke-width="gap"
          stroke-linejoin="miter"
        />
      </mask>
    </defs>
    <!-- p, struck by the bolt -->
    <g :mask="`url(#${maskId})`" :fill="color">
      <rect x="0" y="0" width="22" height="140" />
      <circle cx="50" cy="50" r="39" fill="none" :stroke="color" stroke-width="22" />
    </g>
    <template v-if="kind === 'wordmark'">
      <!-- i, l, i -->
      <g :fill="color">
        <rect x="124" y="0" width="22" height="100" />
        <rect x="170" y="-45" width="22" height="145" />
        <rect x="216" y="0" width="22" height="100" />
      </g>
      <!-- tones of pī lì: flat, then falling -->
      <g :fill="toneColor">
        <rect x="116" y="-34" width="38" height="16" />
        <polygon points="208,-46 226,-46 244,-16 226,-16" />
      </g>
    </template>
  </svg>
</template>

<style scoped>
.pili-logo {
  display: block;
  overflow: visible;
  flex-shrink: 0;
}
</style>
