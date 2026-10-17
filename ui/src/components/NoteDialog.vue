<script setup lang="ts">
import { computed } from "vue";
import TextKeypad from "./TextKeypad.vue";
import { useI18n } from "../i18n";

// The kitchen note of a cart line ("sans wasabi"): typed on the on-screen
// text keypad, at most 60 characters. An empty text clears the note. The
// hint says when it prints: now when the line already went to the
// kitchen, else with the order.

const props = defineProps<{
  /** The line's current note, if any. */
  initial?: string | null;
  /** The line already went to the kitchen: a change prints a note slip now. */
  sent: boolean;
}>();

const emit = defineEmits<{
  confirm: [note: string];
  cancel: [];
}>();

const { t } = useI18n();

/** The longest note (the engine refuses more: `note_too_long`). */
const NOTE_MAX = 60;
// Accents of French notes, and the usual punctuation, that the letter
// rows lack. Every one prints on the slip printer (CP858).
const NOTE_SYMBOLS = ["é", "è", "à", "ç", "ê", ",", ".", "-", "'", "/", "(", ")", "+", "%"];

const hint = computed(() => t(props.sent ? "note_hint_now" : "note_hint_with_order"));
</script>

<template>
  <TextKeypad
    :title="t('note_title')"
    :placeholder="t('note_placeholder')"
    :initial="props.initial ?? ''"
    :max-len="NOTE_MAX"
    :symbols="NOTE_SYMBOLS"
    :hint="hint"
    show-counter
    space
    @confirm="emit('confirm', $event)"
    @cancel="emit('cancel')"
  />
</template>
