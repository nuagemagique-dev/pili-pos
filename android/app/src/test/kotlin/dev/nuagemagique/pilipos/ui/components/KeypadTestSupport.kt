package dev.nuagemagique.pilipos.ui.components

import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsNodeInteractionsProvider
import androidx.compose.ui.test.onAllNodesWithTag
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick

/**
 * Typing in the tests goes through the in-app keypads, as the staff do (C13 / R6: no field takes the system keyboard).
 * Every helper taps the keys of an open keypad; `tag` is the prefix of its test tags (`keypad` for a field's text keypad,
 * `keyboard` for its AZERTY keyboard, `note`, `customer_name`... for a dialog that owns one).
 */

private fun SemanticsNodeInteractionsProvider.exists(tag: String): Boolean = onAllNodesWithTag(tag).fetchSemanticsNodes().isNotEmpty()

private fun SemanticsNodeInteractionsProvider.isOn(tag: String): Boolean =
    onNodeWithTag(tag).fetchSemanticsNode().config.getOrNull(SemanticsProperties.Selected) == true

/** Taps [text] on the open lower case [TextKeypad]: letters, digits, symbols and ' ' (space bar); a capital uses the shift key. */
fun SemanticsNodeInteractionsProvider.typeOnKeypad(
    text: String,
    tag: String = "keypad",
) {
    for (ch in text) {
        when {
            ch == ' ' -> onNodeWithTag("${tag}_space").performClick()
            else -> {
                if (ch.isLetter() && exists("${tag}_case") && isOn("${tag}_case") != ch.isUpperCase()) {
                    onNodeWithTag("${tag}_case").performClick()
                }
                onNodeWithTag("${tag}_key_$ch").performClick()
            }
        }
    }
}

/**
 * Taps [text] on the open AZERTY [TextKeyboard]: its keys are tagged with the upper case letter whatever the case shown,
 * and lower case letters use the "Aa" key when the keyboard has one.
 */
fun SemanticsNodeInteractionsProvider.typeOnKeyboard(
    text: String,
    tag: String = "keyboard",
) {
    for (ch in text) {
        if (ch == ' ') {
            onNodeWithTag("${tag}_space").performClick()
            continue
        }
        if (ch.isLetter() && exists("${tag}_case") && isOn("${tag}_case") != ch.isLowerCase()) {
            onNodeWithTag("${tag}_case").performClick()
        }
        onNodeWithTag("${tag}_key_${ch.uppercaseChar()}").performClick()
    }
}

/** Taps the digits of [digits] on the open number pad. */
fun SemanticsNodeInteractionsProvider.typeOnNumPad(digits: String) {
    for (d in digits) onNodeWithTag("numpad_key_$d").performClick()
}

/** Opens the text field tagged [fieldTag], clears what it holds, types [text] on its keypad and validates. */
fun SemanticsNodeInteractionsProvider.fillWithKeypad(
    fieldTag: String,
    text: String,
) {
    onNodeWithTag(fieldTag).performClick()
    onNodeWithTag("keypad_clear").performClick()
    typeOnKeypad(text)
    onNodeWithTag("keypad_confirm").performClick()
}

/** Opens the field tagged [fieldTag], clears what it holds, types [text] on its AZERTY keyboard and validates. */
fun SemanticsNodeInteractionsProvider.fillWithKeyboard(
    fieldTag: String,
    text: String,
) {
    onNodeWithTag(fieldTag).performClick()
    onNodeWithTag("keyboard_clear").performClick()
    typeOnKeyboard(text)
    onNodeWithTag("keyboard_confirm").performClick()
}

/** Opens the number field tagged [fieldTag], clears it, types [digits] on the pad and validates. */
fun SemanticsNodeInteractionsProvider.fillWithNumPad(
    fieldTag: String,
    digits: String,
) {
    onNodeWithTag(fieldTag).performClick()
    onNodeWithTag("numpad_clear").performClick()
    typeOnNumPad(digits)
    onNodeWithTag("field_number_ok").performClick()
}
