<script setup lang="ts">
// On-screen keyboard for text fields (customer name, address, VAT
// number): the kiosk has no OS keyboard (V11-D19). Belgian AZERTY,
// upper case only (invoices are printed in capitals), digits, the
// accents of French names and the punctuation of addresses. The
// component owns no text: it returns the whole new value through
// v-model, so one keyboard can serve several fields.
import { ref } from 'vue'
import { useI18n } from '../i18n'

const {
  modelValue,
  maxLength = 80,
  caseToggle = false,
} = defineProps<{
  modelValue: string
  /** Longest text the field accepts. */
  maxLength?: number
  /** Offers a lower case key (menu names are not printed in capitals). */
  caseToggle?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  /** The enter key (moves to the next field). */
  submit: []
}>()

const { t } = useI18n()
const lower = ref(false)
const shown = (ch: string) => (lower.value ? ch.toLowerCase() : ch)

const DIGITS = '1234567890'.split('')
const ROWS = ['AZERTYUIOP', 'QSDFGHJKLM', 'WXCVBN'].map((r) => r.split(''))
const ACCENTS = ['É', 'È', 'Ê', 'À', 'Ç', 'Ù']
const PUNCTUATION = ['-', "'", '.', ',', '/', '&']

function press(ch: string) {
  if (modelValue.length >= maxLength) return
  emit('update:modelValue', modelValue + shown(ch))
}

function backspace() {
  emit('update:modelValue', modelValue.slice(0, -1))
}

function clear() {
  emit('update:modelValue', '')
}
</script>

<template>
  <div class="kbd" data-testid="text-keyboard">
    <div class="kbd-row">
      <button v-for="d in DIGITS" :key="d" class="key" @click="press(d)">{{ d }}</button>
    </div>
    <div v-for="(row, i) in ROWS" :key="i" class="kbd-row">
      <button v-for="ch in row" :key="ch" class="key" @click="press(ch)">{{ shown(ch) }}</button>
      <button
        v-if="i === ROWS.length - 1"
        class="key key-back"
        :aria-label="t('keyboard_backspace')"
        @click="backspace"
      >
        &#9003;
      </button>
    </div>
    <div class="kbd-row">
      <button v-for="a in ACCENTS" :key="a" class="key key-accent" @click="press(a)">
        {{ shown(a) }}
      </button>
      <button v-for="p in PUNCTUATION" :key="p" class="key key-punct" @click="press(p)">
        {{ p }}
      </button>
    </div>
    <div class="kbd-row">
      <button class="key key-clear" @click="clear">{{ t('clear') }}</button>
      <button
        v-if="caseToggle"
        class="key key-case"
        :class="{ on: lower }"
        :aria-pressed="lower"
        :aria-label="t('menu_ed_case')"
        data-testid="keyboard-case"
        @click="lower = !lower"
      >
        Aa
      </button>
      <button class="key key-space" :aria-label="t('keyboard_space')" @click="press(' ')">
        &#9251;
      </button>
      <button class="key key-enter" @click="emit('submit')">{{ t('keyboard_next') }}</button>
    </div>
  </div>
</template>

<style scoped>
.kbd {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  align-items: center;
}

.kbd-row {
  display: flex;
  gap: var(--space-sm);
  justify-content: center;
}

/* UX guide 4.1: touch keys 72x64, far above the 48 px floor. */
.key {
  width: 72px;
  height: 64px;
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-bold);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  color: var(--text-primary);
  transition: transform var(--tap-duration) ease;
}

.key:active {
  transform: scale(var(--tap-scale));
  background: var(--bg-pressed);
}

.key-accent,
.key-punct {
  color: var(--text-secondary);
}
.key-back {
  color: var(--accent-danger);
}
.key-clear {
  width: 152px;
  color: var(--accent-warning);
  font-size: var(--font-size-md);
}
.key-case {
  font-size: var(--font-size-md);
  color: var(--text-secondary);
}
.key-case.on {
  background: var(--selected-bg);
  color: var(--selected-fg);
}
.key-space {
  width: 440px;
}
.key-enter {
  width: 152px;
  font-size: var(--font-size-md);
}
</style>
