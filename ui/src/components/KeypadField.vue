<script setup lang="ts">
// A field that never raises the OS keyboard (UX audit C13 / R6): it shows its value like an input
// and a tap opens the on-screen keypad, text on TextKeypad or whole numbers on the shared NumPad.
// The Cage kiosk has no virtual keyboard, so every field of the till is reached through this
// component (or an inline TextKeyboard). The value changes once, when the keypad is validated.
import { computed, ref } from 'vue'
import NumberEntryDialog from './NumberEntryDialog.vue'
import TextKeypad from './TextKeypad.vue'

const {
  modelValue,
  title,
  placeholder = '',
  maxLen = 60,
  symbols = [],
  space = false,
  caseToggle = false,
  numeric = false,
  maxDigits = 6,
  exactDigits = undefined,
  min = undefined,
  max = undefined,
  disabled = false,
  valid = () => true,
  invalidText = '',
  showCounter = false,
} = defineProps<{
  modelValue: string
  /** Title of the keypad that opens. */
  title: string
  placeholder?: string
  /** Text: the longest value. */
  maxLen?: number
  /** Text: extra symbol keys (accents, dashes). */
  symbols?: string[]
  /** Text: a space bar. */
  space?: boolean
  /** Text: a shift key (case-sensitive codes). */
  caseToggle?: boolean
  /** Whole numbers on the NumPad instead of text. */
  numeric?: boolean
  /** Numeric: the most digits. */
  maxDigits?: number
  /** Numeric: exactly this many digits, kept as typed (leading zeros stay, an INSZ). */
  exactDigits?: number
  /** Numeric: smallest and largest accepted number. */
  min?: number
  max?: number
  disabled?: boolean
  valid?: (value: string) => boolean
  invalidText?: string
  showCounter?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const open = ref(false)
const empty = computed(() => modelValue === '')

function onConfirm(value: string) {
  open.value = false
  if (value !== modelValue) emit('update:modelValue', value)
}
</script>

<template>
  <!-- One root so the caller's class and data-testid land on the button. -->
  <button
    type="button"
    class="keypad-field"
    :class="{ empty }"
    :disabled="disabled"
    @click="open = true"
  >
    {{ empty ? placeholder : modelValue }}
    <Teleport v-if="open" to="body">
      <NumberEntryDialog
        v-if="numeric"
        :title="title"
        :initial="modelValue"
        :max-digits="maxDigits"
        :exact-digits="exactDigits"
        :min="min"
        :max="max"
        @confirm="onConfirm"
        @cancel="open = false"
      />
      <TextKeypad
        v-else
        :title="title"
        :placeholder="placeholder"
        :initial="modelValue"
        :max-len="maxLen"
        :symbols="symbols"
        :space="space"
        :case-toggle="caseToggle"
        :valid="valid"
        :invalid-text="invalidText"
        :show-counter="showCounter"
        @confirm="onConfirm"
        @cancel="open = false"
      />
    </Teleport>
  </button>
</template>

<style scoped>
.keypad-field {
  display: block;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  transition: transform var(--tap-duration) ease;
}
.keypad-field:active:not(:disabled) {
  transform: scale(var(--tap-scale));
}
.keypad-field.empty {
  color: var(--text-secondary);
}
</style>
