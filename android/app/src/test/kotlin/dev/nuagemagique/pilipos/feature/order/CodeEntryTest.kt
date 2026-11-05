package dev.nuagemagique.pilipos.feature.order

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** The grammar of the code pad, the same as `ui/src/utils/codeEntry.test.ts`. */
@Suppress("TooManyFunctions")
class CodeEntryTest {
    private val a12 = menuItem(1, 1, "Maki saumon", 450, code = "A12")
    private val b3 = menuItem(2, 1, "Nigiri", 300, code = "b3")
    private val noCode = menuItem(3, 1, "Thé")
    private val c1 = menuItem(4, 1, "Soupe", code = "C1")
    private val c1bis = menuItem(5, 1, "Soupe bis", code = "c1")
    private val items = listOf(a12, b3, noCode, c1, c1bis)

    @Test
    fun aPlainCodeIsOneUnitInAnyCaseAndWithBlanks() {
        assertEquals(CodeEntry.Add("A12", 1), parseCodeEntry("A12"))
        assertEquals(CodeEntry.Add("A12", 1), parseCodeEntry(" a 12 "))
        assertEquals(CodeEntry.Add("105", 1), parseCodeEntry("105"))
    }

    @Test
    fun theNumberBeforeTheMultiplySignIsTheQuantity() {
        assertEquals(CodeEntry.Add("A12", 3), parseCodeEntry("3*A12"))
        assertEquals(CodeEntry.Add("3", 12), parseCodeEntry("12*3"))
        assertEquals(CodeEntry.Add("A12", 3), parseCodeEntry("A12*3"))
    }

    @Test
    fun aLoneMultiplierWaits() {
        assertEquals(CodeEntry.Multiplier(4), parseCodeEntry("4*"))
        assertEquals(CodeEntry.Empty, parseCodeEntry("  "))
    }

    @Test
    fun zeroOrThreeDigitQuantitiesAndStraySignsAreInvalid() {
        listOf("0*A1", "100*A1", "A1*0", "-", "-A1", "*A1", "A1*B2").forEach {
            assertEquals(it, CodeEntry.Invalid, parseCodeEntry(it))
        }
    }

    @Test
    fun keysOnlyMakeValidTexts() {
        assertEquals("A", typeKey("", "a"))
        assertEquals("", typeKey("", "-"))
        assertEquals("A1", typeKey("A1", "-"))
        assertEquals("", typeKey("", TIMES))
        assertEquals("3*", typeKey("3", TIMES))
        assertEquals("3*", typeKey("3*", TIMES))
        assertEquals("A1*", typeKey("A1", TIMES))
        assertEquals("A", typeKey("A", "."))
        val full = "A".repeat(CODE_ENTRY_MAX)
        assertEquals(full, typeKey(full, "1"))
    }

    @Test
    fun findByCodeIsExact() {
        assertEquals(CodeMatch.Found(a12), findByCode(items, "a12"))
        assertEquals(CodeMatch.Found(b3), findByCode(items, "B3"))
        assertEquals(CodeMatch.Unknown, findByCode(items, "A1"))
        assertEquals(CodeMatch.Ambiguous, findByCode(items, "C1"))
    }

    @Test
    fun theResultsStartWithTheCodeShortestFirstInNaturalOrder() {
        val a1 = menuItem(6, 1, "Maki thon", code = "A1")
        val a2 = menuItem(7, 1, "Maki avocat", code = "a2")
        val a10 = menuItem(8, 1, "Maki crevette", code = "A10")
        val a9b = menuItem(9, 1, "Maki concombre", code = "A9B")
        val menu = listOf(a12, a10, a9b, a2, b3, noCode, a1)
        assertEquals(listOf(a1, a2, a9b, a10, a12), searchByCode(menu, "a"))
        assertEquals(listOf(a1, a10, a12), searchByCode(menu, "A1"))
        assertEquals(emptyList<Any>(), searchByCode(menu, ""))
        assertEquals(emptyList<Any>(), searchByCode(menu, "Z"))
    }

    @Test
    fun okTakesTheExactCodeElseTheOnlyProductStartingWithIt() {
        val a1 = menuItem(6, 1, "Maki thon", code = "A1")
        assertEquals(CodeMatch.Found(a1), pickByCode(items + a1, "A1"))
        assertEquals(CodeMatch.Found(a12), pickByCode(items, "A1"))
        assertEquals(CodeMatch.Unknown, pickByCode(items + a1, "A"))
        assertEquals(CodeMatch.Ambiguous, pickByCode(items, "C1"))
        assertEquals(CodeMatch.Unknown, pickByCode(items, "Z"))
    }

    @Test
    fun theLetterKeysAreTheLettersOfTheCodes() {
        assertEquals(listOf("A", "B", "C"), codeLetters(items))
    }

    @Test
    fun okAddsTheQuantityOfTheProduct() {
        assertEquals(CodeAction.Add(a12, 3), decideCode("3*a12", items, false))
    }

    @Test
    fun okRefusesUnknownSharedAndInvalidEntries() {
        assertEquals(CodeAction.Refuse(CodeMessage.Unknown("Z9")), decideCode("Z9", items, false))
        assertEquals(CodeAction.Refuse(CodeMessage.Ambiguous("C1")), decideCode("C1", items, false))
        assertEquals(CodeAction.Refuse(CodeMessage.Invalid), decideCode("A1*B2", items, false))
        assertEquals(CodeAction.Wait, decideCode("3*", items, false))
        assertEquals(CodeAction.Wait, decideCode("", items, false))
    }

    @Test
    fun aMultiplierNeedsAPlainProductAndASoldOutOneIsRefused() {
        val withOptions = a12.copy(hasModifiers = true)
        val weighed = a12.copy(soldByWeight = true)
        assertEquals(CodeAction.Refuse(CodeMessage.NoMultiplier), decideCode("2*A12", listOf(withOptions), false))
        assertEquals(CodeAction.Refuse(CodeMessage.NoMultiplier), decideCode("2*A12", listOf(weighed), false))
        assertEquals(CodeAction.Add(withOptions, 1), decideCode("A12", listOf(withOptions), false))
        assertEquals(CodeAction.Refuse(CodeMessage.SoldOut), decideCode("A12", listOf(a12.copy(soldOut = true)), false))
    }

    @Test
    fun aModeThatTakesTheTapsGetsOnePlainTap() {
        val soldOut = a12.copy(soldOut = true)
        assertEquals(CodeAction.Add(soldOut, 1), decideCode("3*A12", listOf(soldOut), true))
    }

    @Test
    fun theQuantityOnThePadAppliesToTheTappedProduct() {
        assertEquals(CodeAction.Add(b3, 4), decideTap("4*", b3, false))
        assertEquals(CodeAction.Add(b3, 2), decideTap("2*B", b3, false))
        assertEquals(CodeAction.Add(b3, 1), decideTap("B", b3, false))
        assertNull(decideTap("", b3, false))
        assertNull(decideTap("A1*B2", b3, false))
    }

    @Test
    fun thePreviewNamesWhatOkWillDo() {
        assertEquals(CodeMessage.Hint, previewCode("", items))
        assertEquals(CodeMessage.Match(2, a12), previewCode("2*A12", items))
        assertEquals(CodeMessage.Match(1, a12), previewCode("A1", items))
        assertEquals(CodeMessage.Results(2), previewCode("C", items))
        assertEquals(CodeMessage.NoResult, previewCode("Z", items))
        assertEquals(CodeMessage.WaitingProduct(5), previewCode("5*", items))
        assertEquals(CodeMessage.Invalid, previewCode("0*", items))
    }

    // ── Digit after an add = quantity (C7), same cases as ui/src/utils/codeEntry.test.ts ──

    @Test
    fun theDigitWindowOpensForThreeSecondsOnTheUnitsTheAddGave() {
        val w = openDigitWindow(lineId = 5, itemId = 1, quantity = 1, now = 1_000)

        assertEquals(DigitWindow(5, 1, 1, "", 1_000 + DIGIT_WINDOW_MS), w)
        assertEquals(3_000L, DIGIT_WINDOW_MS)
    }

    @Test
    fun aDigitSetsHowManyThisAddGives() {
        val step = stepDigitWindow(openDigitWindow(5, 1, 1, 0), "3", 500) as DigitStep.Set

        assertEquals(3, step.window.quantity)
        assertEquals("3", step.window.digits)
        assertEquals(5L, step.window.lineId)
    }

    @Test
    fun aSecondDigitExtendsItAndTheDeadlineMoves() {
        val first = stepDigitWindow(openDigitWindow(5, 1, 1, 0), "1", 2_500) as DigitStep.Set
        // 3_400 is past the first deadline (3_000): only a deadline that moved still takes the digit.
        val second = stepDigitWindow(first.window, "2", 3_400) as DigitStep.Set

        assertEquals(12, second.window.quantity)
        assertEquals(3_400 + DIGIT_WINDOW_MS, second.window.expiresAt)
    }

    @Test
    fun aThirdDigitStartsACodeAndNinetyNineIsTheMaximum() {
        var w = openDigitWindow(5, 1, 1, 0)
        for (k in listOf("9", "9")) w = (stepDigitWindow(w, k, 100) as DigitStep.Set).window

        assertEquals(99, w.quantity)
        assertEquals(DigitStep.Pass, stepDigitWindow(w, "1", 200))
    }

    @Test
    fun aDigitAfterTheDeadlineStartsACodeFakeClock() {
        val w = openDigitWindow(5, 1, 1, 0)

        assertTrue(stepDigitWindow(w, "3", DIGIT_WINDOW_MS) is DigitStep.Set)
        assertEquals(DigitStep.Pass, stepDigitWindow(w, "3", DIGIT_WINDOW_MS + 1))
    }

    @Test
    fun aLetterTheMultiplySignOrAFirstZeroAreNotForTheWindow() {
        val w = openDigitWindow(5, 1, 1, 0)

        listOf("A", "*", "0", "Backspace", "Enter", "OK").forEach {
            assertEquals(it, DigitStep.Pass, stepDigitWindow(w, it, 10))
        }
    }

    @Test
    fun aZeroAfterAFirstDigitIsPartOfTheQuantity() {
        val first = stepDigitWindow(openDigitWindow(5, 1, 1, 0), "1", 10) as DigitStep.Set

        assertEquals(10, (stepDigitWindow(first.window, "0", 20) as DigitStep.Set).window.quantity)
    }

    @Test
    fun noWindowTakesNothing() {
        assertEquals(DigitStep.Pass, stepDigitWindow(null, "3", 0))
    }
}
