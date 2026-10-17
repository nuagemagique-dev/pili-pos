package dev.nuagemagique.pilipos.feature.order

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.i18n.AppLanguage
import io.mockk.coEvery
import io.mockk.coVerify
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import uniffi.pos_ffi.AllergenEntry
import uniffi.pos_ffi.AllergenNames
import uniffi.pos_ffi.AllergenRef
import uniffi.pos_ffi.AllergenSheetView
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SheetProductView
import uniffi.pos_ffi.SlipOutcome

/** The allergen mode of the order screen: the legend, the sheet instead of an add, the print. */
@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class AllergenModeTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private val maki = menuItem(10, 1, "Maki saumon", 650, allergens = listOf("fish", "gluten"), traces = listOf("molluscs"))

    private val legend =
        listOf(
            AllergenEntry(
                "gluten",
                1,
                AllergenNames("Céréales contenant du gluten", "Glutenbevattende granen", "Cereals containing gluten", "含麸质的谷物"),
            ),
            AllergenEntry("molluscs", 14, AllergenNames("Mollusques", "Weekdieren", "Molluscs", "软体动物")),
        )

    private fun ref(
        code: String,
        number: Int,
        name: String,
    ) = AllergenRef(code, number, name)

    private fun sheet(
        lang: String,
        declared: Boolean = true,
    ): AllergenSheetView =
        AllergenSheetView(
            lang = lang,
            products =
                listOf(
                    SheetProductView(
                        id = 10,
                        name = "Maki saumon",
                        nameFr = "Maki saumon",
                        declared = declared,
                        contains = if (declared) listOf(ref("fish", 4, if (lang == "fr") "Poissons" else "Fish")) else emptyList(),
                        traces = if (declared) listOf(ref("molluscs", 14, if (lang == "fr") "Mollusques" else "Molluscs")) else emptyList(),
                        options = emptyList(),
                    ),
                ),
        )

    private suspend fun TestScope.ready(): OrderRig {
        val world = OrderWorld()
        val f = orderFixture(tmp.root, world)
        coEvery { f.engine.addOrderLine(any()) } returns orderLine(10)
        coEvery { f.engine.listAllergens() } returns legend
        coEvery { f.engine.getAllergenSheet(any(), "fr") } answers { sheet("fr") }
        coEvery { f.engine.getAllergenSheet(any(), "en") } answers { sheet("en") }
        coEvery { f.engine.printAllergenSheet(any(), any()) } returns SlipOutcome.PRINTED
        val r = startOrder(f, world, context)
        r.login()
        runCurrent()
        return r
    }

    @Test
    fun theModeStartsOffAndTheLegendIsReadWhenItIsTurnedOnForTheFirstTime() =
        runTest {
            val r = ready()
            assertFalse(r.state.menu.allergenMode)
            assertEquals(emptyList<AllergenEntry>(), r.state.menu.allergenLegend)
            coVerify(exactly = 0) { r.f.engine.listAllergens() }

            r.model.toggleAllergenMode()
            runCurrent()
            assertTrue(r.state.menu.allergenMode)
            assertEquals(legend, r.state.menu.allergenLegend)

            r.model.toggleAllergenMode()
            r.model.toggleAllergenMode()
            runCurrent()
            assertTrue(r.state.menu.allergenMode)
            coVerify(exactly = 1) { r.f.engine.listAllergens() }
        }

    @Test
    fun aFailingLegendReadLeavesTheModeOn() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.listAllergens() } throws PosException.Coded("internal_error", mapOf("detail" to "x"))

            r.model.toggleAllergenMode()
            runCurrent()

            assertTrue(r.state.menu.allergenMode)
            assertEquals(emptyList<AllergenEntry>(), r.state.menu.allergenLegend)
        }

    @Test
    fun inTheModeATapOpensTheSheetAndAddsNothing() =
        runTest {
            val r = ready()
            r.model.toggleAllergenMode()
            runCurrent()

            r.model.addItem(maki)
            runCurrent()

            val open = r.state.dialog as OrderDialog.AllergenSheet
            assertEquals(10L, open.item.id)
            assertEquals("Poissons", open.sheet?.contains?.single()?.name)
            assertNull("French sheet: no second read", open.sheetFr)
            coVerify(exactly = 0) { r.f.engine.addOrderLine(any()) }
            coVerify(exactly = 1) { r.f.engine.getAllergenSheet(10L, "fr") }
        }

    @Test
    fun outOfTheModeATapAddsTheProductAgain() =
        runTest {
            val r = ready()
            r.model.toggleAllergenMode()
            r.model.toggleAllergenMode()
            runCurrent()

            r.model.addItem(maki)
            runCurrent()

            assertEquals(OrderDialog.None, r.state.dialog)
            coVerify(exactly = 1) { r.f.engine.addOrderLine(any()) }
            coVerify(exactly = 0) { r.f.engine.getAllergenSheet(any(), any()) }
        }

    @Test
    fun outsideFrenchTheSheetIsReadInFrenchToo() =
        runTest {
            val r = ready()
            r.settings.setLanguage(AppLanguage.EN)
            runCurrent()
            r.model.toggleAllergenMode()
            runCurrent()

            r.model.addItem(maki)
            runCurrent()

            val open = r.state.dialog as OrderDialog.AllergenSheet
            assertEquals("Fish", open.sheet?.contains?.single()?.name)
            assertEquals("Poissons", open.sheetFr?.contains?.single()?.name)
            coVerify { r.f.engine.getAllergenSheet(10L, "en") }
            coVerify { r.f.engine.getAllergenSheet(10L, "fr") }
        }

    @Test
    fun aRefusedSheetShowsItsErrorInTheDialog() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.getAllergenSheet(any(), any()) } throws
                PosException.Coded("allergen_target_not_found", mapOf("id" to "10"))
            r.model.toggleAllergenMode()
            runCurrent()

            r.model.addItem(maki)
            runCurrent()

            val open = r.state.dialog as OrderDialog.AllergenSheet
            assertNull(open.sheet)
            assertNotNull(open.error)
        }

    @Test
    fun theSheetPrintsForTheProductOrForTheWholeMenu() =
        runTest {
            val r = ready()
            r.model.toggleAllergenMode()
            runCurrent()
            r.model.addItem(maki)
            runCurrent()

            r.model.printAllergenSheet(false)
            runCurrent()
            assertEquals(SlipOutcome.PRINTED, (r.state.dialog as OrderDialog.AllergenSheet).printResult)
            r.model.printAllergenSheet(true)
            runCurrent()

            coVerify { r.f.engine.printAllergenSheet(10L, "fr") }
            coVerify { r.f.engine.printAllergenSheet(null, "fr") }
            assertFalse((r.state.dialog as OrderDialog.AllergenSheet).busy)
        }

    @Test
    fun chineseSheetsPrintInFrench() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.getAllergenSheet(any(), "zh") } answers { sheet("zh") }
            r.settings.setLanguage(AppLanguage.ZH)
            runCurrent()
            r.model.toggleAllergenMode()
            runCurrent()
            r.model.addItem(maki)
            runCurrent()

            r.model.printAllergenSheet(false)
            runCurrent()

            coVerify { r.f.engine.printAllergenSheet(10L, "fr") }
        }

    @Test
    fun aPrinterProblemIsTheOutcomeAndARefusalIsTheError() =
        runTest {
            val r = ready()
            coEvery { r.f.engine.printAllergenSheet(any(), any()) } returns SlipOutcome.NO_PRINTER
            r.model.toggleAllergenMode()
            runCurrent()
            r.model.addItem(maki)
            runCurrent()

            r.model.printAllergenSheet(false)
            runCurrent()
            assertEquals(SlipOutcome.NO_PRINTER, (r.state.dialog as OrderDialog.AllergenSheet).printResult)

            coEvery { r.f.engine.printAllergenSheet(any(), any()) } throws PosException.Coded("not_logged_in", emptyMap())
            r.model.printAllergenSheet(true)
            runCurrent()
            val open = r.state.dialog as OrderDialog.AllergenSheet
            assertNull(open.printResult)
            assertNotNull(open.error)
        }

    @Test
    fun closingTheSheetClosesTheDialogAndTheModeStays() =
        runTest {
            val r = ready()
            r.model.toggleAllergenMode()
            runCurrent()
            r.model.addItem(maki)
            runCurrent()

            r.model.dismissDialog()

            assertEquals(OrderDialog.None, r.state.dialog)
            assertTrue(r.state.menu.allergenMode)
        }
}
