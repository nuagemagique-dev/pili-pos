package dev.nuagemagique.pilipos.feature.settings.menu

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import dev.nuagemagique.pilipos.feature.settings.SettingsEnv
import dev.nuagemagique.pilipos.feature.settings.admin
import dev.nuagemagique.pilipos.feature.settings.cashier
import dev.nuagemagique.pilipos.feature.settings.openEngine
import dev.nuagemagique.pilipos.feature.settings.settingsEnv
import dev.nuagemagique.pilipos.feature.settings.settingsFixture
import dev.nuagemagique.pilipos.ui.feedback.ToastVariant
import io.mockk.coEvery
import io.mockk.coVerify
import io.mockk.slot
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
import uniffi.pos_ffi.MenuSaved
import uniffi.pos_ffi.PosException
import uniffi.pos_ffi.SaveCategoryInput
import uniffi.pos_ffi.SaveModifierGroupInput
import uniffi.pos_ffi.SaveModifierInput
import uniffi.pos_ffi.SaveProductInput
import uniffi.pos_ffi.User

@OptIn(ExperimentalCoroutinesApi::class)
@Suppress("TooManyFunctions")
@RunWith(RobolectricTestRunner::class)
class MenuEditorViewModelTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val context: Context = ApplicationProvider.getApplicationContext()

    @After
    fun tearDown() {
        Dispatchers.resetMain()
    }

    private class Rig(
        val env: SettingsEnv,
        val model: MenuEditorViewModel,
    ) {
        val state get() = model.state.value
        val engine get() = env.f.engine
    }

    private suspend fun TestScope.start(who: User? = admin): Rig {
        val f = settingsFixture(tmp.root)
        coEvery { f.engine.menuAdminGet() } returns adminMenu()
        val env = settingsEnv(f, context)
        openEngine(env)
        if (who != null) with(env) { loginAs(who) }
        val model = MenuEditorViewModel(env.entry)
        runCurrent()
        return Rig(env, model)
    }

    private suspend fun Rig.opened(): Rig {
        model.load()
        return this
    }

    // ── Loading ───────────────────────────────────────────────────

    @Test
    fun aManagerReadsTheMenuAndTheFirstCategoryIsChosen() =
        runTest {
            val r = start().opened()

            assertTrue(r.state.loaded)
            assertEquals(2, r.state.categories.size)
            assertEquals(1L, r.state.selectedCategoryId)
            assertEquals(listOf(10L, 11L), r.state.products.map { it.id })
            assertFalse(r.state.locked)
            assertNull(r.state.error)
        }

    @Test
    fun anOperatorWhoIsNotAManagerNeverAsksTheEngine() =
        runTest {
            val r = start(who = cashier)

            r.model.load()

            assertNull(r.state.menu)
            coVerify(exactly = 0) { r.engine.menuAdminGet() }
        }

    @Test
    fun aRefusedReadShowsTheEngineTextAndKeepsNothing() =
        runTest {
            val r = start()
            coEvery { r.engine.menuAdminGet() } throws PosException.Coded("forbidden", emptyMap())

            r.model.load()

            assertTrue(r.state.loaded)
            assertNull(r.state.menu)
            assertEquals(r.env.text("err_forbidden"), r.state.error)
        }

    @Test
    fun aCatalogThatManagesTheMenuLocksTheEditor() =
        runTest {
            val r = start()
            coEvery { r.engine.menuAdminGet() } returns adminMenu(managedBy = "hubrise")
            r.model.load()

            r.model.selectCategory(1L)
            r.model.newCategory()
            r.model.editProduct(10L)
            r.model.newProduct()

            assertTrue(r.state.locked)
            assertNull(r.state.form)
            r.model.save()
            runCurrent()
            coVerify(exactly = 0) { r.engine.menuSaveCategory(any()) }
            coVerify(exactly = 0) { r.engine.menuSaveProduct(any()) }
        }

    // ── Choosing and opening forms ────────────────────────────────

    @Test
    fun aCategoryRowChoosesItAndOpensItsForm() =
        runTest {
            val r = start().opened()

            r.model.selectCategory(2L)

            assertEquals(2L, r.state.selectedCategoryId)
            assertEquals(FormKind.Category, r.state.form?.kind)
            assertEquals("Boissons", r.state.form?.names?.get("fr"))
            assertFalse(r.state.form?.active ?: true)
            assertTrue(r.state.products.isEmpty())
        }

    @Test
    fun aProductFormCarriesTheFieldsOfTheEngine() =
        runTest {
            val r = start().opened()

            r.model.editProduct(10L)

            val form = r.state.form!!
            assertEquals(FormKind.Product, form.kind)
            assertEquals(10L, form.id)
            assertEquals(650L, form.priceCents)
            assertEquals("B", form.vatCode)
            assertEquals("C", form.vatCodeTakeaway)
            assertEquals("A1", form.code)
            assertEquals(listOf(20L), r.state.groupsOf(10L).map { it.id })
            assertEquals(listOf(30L), r.state.optionsOf(20L).map { it.id })
        }

    @Test
    fun aNewProductStartsInTheChosenCategoryAfterTheLastOne() =
        runTest {
            val r = start().opened()

            r.model.newProduct()

            val form = r.state.form!!
            assertTrue(form.isNew)
            assertEquals(1L, form.parentId)
            assertEquals(3, form.sortOrder)
            assertEquals("B", form.vatCode)
            assertTrue(form.active)
        }

    @Test
    fun groupsAndOptionsOpenAndGoBackToTheirParent() =
        runTest {
            val r = start().opened()
            r.model.editProduct(10L)

            r.model.editGroup(20L)
            assertEquals(FormKind.Group, r.state.form?.kind)
            r.model.editOption(30L)
            assertEquals(FormKind.Option, r.state.form?.kind)

            r.model.backToParent()
            assertEquals(FormKind.Group, r.state.form?.kind)
            assertEquals(20L, r.state.form?.id)
            r.model.backToParent()
            assertEquals(FormKind.Product, r.state.form?.kind)
            assertEquals(10L, r.state.form?.id)
            r.model.backToParent()
            assertNull(r.state.form)
        }

    @Test
    fun cancelClosesTheFormWithoutCallingTheEngine() =
        runTest {
            val r = start().opened()
            r.model.newCategory()

            r.model.cancel()

            assertNull(r.state.form)
            coVerify(exactly = 0) { r.engine.menuSaveCategory(any()) }
        }

    // ── Editing ───────────────────────────────────────────────────

    @Test
    fun theFieldSettersChangeTheOpenForm() =
        runTest {
            val r = start().opened()
            r.model.editProduct(10L)

            r.model.setName("fr", "Maki thon")
            r.model.setName("nl", "Maki tonijn")
            r.model.setCode("B14")
            r.model.setPrice(720L)
            r.model.setVat("A")
            r.model.setVatTakeaway(null)
            r.model.setCategory(2L)
            r.model.toggleKitchen()
            r.model.toggleOpenPrice()
            r.model.toggleActive()
            r.model.stepPosition(2)

            val form = r.state.form!!
            assertEquals("Maki thon", form.names["fr"])
            assertEquals("Maki tonijn", form.names["nl"])
            assertEquals("zh", form.names.keys.first { it == "zh" })
            assertEquals("B14", form.code)
            assertEquals(720L, form.priceCents)
            assertEquals("A", form.vatCode)
            assertNull(form.vatCodeTakeaway)
            assertEquals(2L, form.parentId)
            assertFalse(form.kitchenPrinter)
            assertTrue(form.openPrice)
            assertFalse(form.active)
            assertEquals(3, form.sortOrder)
        }

    @Test
    fun namesAndCodesAreCutAtTheEnginesLimits() =
        runTest {
            val r = start().opened()
            r.model.editProduct(10L)

            r.model.setName("fr", "x".repeat(NAME_MAX + 10))
            r.model.setCode("y".repeat(CODE_MAX + 5))

            assertEquals(NAME_MAX, r.state.form?.names?.get("fr")?.length)
            assertEquals(CODE_MAX, r.state.form?.code?.length)
        }

    @Test
    fun thePositionNeverGoesBelowZero() =
        runTest {
            val r = start().opened()
            r.model.newCategory()

            r.model.stepPosition(-10)

            assertEquals(0, r.state.form?.sortOrder)
        }

    @Test
    fun theMinimumNeverPassesTheMaximumAndTheMaximumIsAtLeastOne() =
        runTest {
            val r = start().opened()
            r.model.newGroup(10L)
            assertEquals(0 to 1, r.state.form?.let { it.minSelect to it.maxSelect })

            r.model.stepMin(1)
            r.model.stepMin(1)
            assertEquals(2 to 2, r.state.form?.let { it.minSelect to it.maxSelect })

            r.model.stepMax(-1)
            assertEquals(1 to 1, r.state.form?.let { it.minSelect to it.maxSelect })
            r.model.stepMax(-1)
            assertEquals(1 to 1, r.state.form?.let { it.minSelect to it.maxSelect })
            r.model.stepMin(-5)
            assertEquals(0 to 1, r.state.form?.let { it.minSelect to it.maxSelect })
        }

    // ── Save ──────────────────────────────────────────────────────

    @Test
    fun savingAProductSendsTheFormAndReopensWhatWasSaved() =
        runTest {
            val r = start().opened()
            val input = slot<SaveProductInput>()
            val after = makiProduct.copy(priceCents = 720L, vatCode = "A")
            coEvery { r.engine.menuSaveProduct(capture(input)) } returns
                MenuSaved(10L, adminMenu(products = listOf(after, edamameProduct)))
            r.model.editProduct(10L)
            r.model.setPrice(720L)
            r.model.setVat("A")
            r.model.setVatTakeaway(null)
            r.model.setName("zh", "  ")

            r.model.save()
            runCurrent()

            assertEquals(10L, input.captured.id)
            assertEquals(1L, input.captured.categoryId)
            assertEquals(720L, input.captured.priceCents)
            assertEquals("A", input.captured.vatCode)
            assertNull(input.captured.vatCodeTakeaway)
            assertEquals(mapOf("fr" to "Maki saumon"), input.captured.names)
            assertEquals("A1", input.captured.code)
            assertEquals(FormKind.Product, r.state.form?.kind)
            assertEquals(720L, r.state.form?.priceCents)
            assertFalse(r.state.busy)
            assertEquals(ToastVariant.Success, r.env.lastToast().variant)
            assertEquals(r.env.text("menu_ed_saved"), r.env.lastToast().message)
        }

    @Test
    fun aNewCategoryIsSentWithNoIdAndBecomesTheChosenOne() =
        runTest {
            val r = start().opened()
            val input = slot<SaveCategoryInput>()
            val created = sushiCategory.copy(id = 3L, names = mapOf("fr" to "Desserts"))
            coEvery { r.engine.menuSaveCategory(capture(input)) } returns
                MenuSaved(3L, adminMenu(categories = listOf(sushiCategory, drinksCategory, created)))
            r.model.newCategory()
            r.model.setName("fr", "Desserts")
            r.model.setColor("#22C55E")

            r.model.save()
            runCurrent()

            assertNull(input.captured.id)
            assertEquals(3, input.captured.sortOrder)
            assertEquals("#22C55E", input.captured.color)
            assertEquals(3L, r.state.selectedCategoryId)
            assertEquals(3L, r.state.form?.id)
        }

    @Test
    fun groupsAndOptionsAreSavedUnderTheirParent() =
        runTest {
            val r = start().opened()
            val group = slot<SaveModifierGroupInput>()
            val option = slot<SaveModifierInput>()
            coEvery {
                r.engine.menuSaveModifierGroup(capture(group))
            } returns MenuSaved(21L, adminMenu(groups = listOf(sauceGroup, sauceGroup.copy(id = 21L))))
            coEvery { r.engine.menuSaveModifier(capture(option)) } returns MenuSaved(30L, adminMenu())
            r.model.newGroup(10L)
            r.model.setName("fr", "Piment")
            r.model.stepMax(1)

            r.model.save()
            runCurrent()

            assertEquals(10L, group.captured.menuItemId)
            assertNull(group.captured.id)
            assertEquals(2, group.captured.maxSelect)
            assertEquals(FormKind.Group, r.state.form?.kind)

            r.model.editOption(30L)
            r.model.setPrice(75L)
            r.model.save()
            runCurrent()

            assertEquals(30L, option.captured.id)
            assertEquals(20L, option.captured.groupId)
            assertEquals(75L, option.captured.priceCents)
        }

    @Test
    fun aRefusedSaveShowsTheEngineTextKeepsTheFormAndToasts() =
        runTest {
            val r = start().opened()
            coEvery { r.engine.menuSaveProduct(any()) } throws PosException.Coded("menu_invalid", mapOf("field" to "name"))
            r.model.newProduct()

            r.model.save()
            runCurrent()

            assertNotNull(r.state.form)
            assertTrue(r.state.form!!.isNew)
            assertFalse(r.state.busy)
            assertEquals(r.env.text("err_menu_invalid", mapOf("field" to "name")), r.state.error)
            assertEquals(ToastVariant.Error, r.env.lastToast().variant)
        }

    @Test
    fun aSecondSaveWhileOneRunsIsIgnored() =
        runTest {
            val r = start().opened()
            val gate = kotlinx.coroutines.CompletableDeferred<Unit>()
            coEvery { r.engine.menuSaveCategory(any()) } coAnswers
                {
                    gate.await()
                    MenuSaved(1L, adminMenu())
                }
            r.model.newCategory()
            r.model.setName("fr", "Entrées")

            r.model.save()
            runCurrent()
            r.model.save()
            runCurrent()
            gate.complete(Unit)
            runCurrent()

            coVerify(exactly = 1) { r.engine.menuSaveCategory(any()) }
        }

    @Test
    fun aCashierCannotSave() =
        runTest {
            val r = start(who = cashier)
            r.model.newCategory()

            r.model.save()
            runCurrent()

            coVerify(exactly = 0) { r.engine.menuSaveCategory(any()) }
        }

    // ── History ───────────────────────────────────────────────────

    @Test
    fun theHistoryAsksForTheLastHundredChangesAndCloses() =
        runTest {
            val r = start().opened()
            coEvery { r.engine.listMenuChanges(any()) } returns
                listOf(menuChange(2L), menuChange(1L, "category", "create", "Sushi", listOf("names")))
            r.model.editProduct(10L)

            r.model.openHistory()
            runCurrent()

            assertTrue(r.state.showHistory)
            assertNull(r.state.form)
            assertEquals(listOf(2L, 1L), r.state.changes.map { it.id })
            coVerify { r.engine.listMenuChanges(CHANGES_LIMIT) }
            r.model.closeHistory()
            assertFalse(r.state.showHistory)
        }

    @Test
    fun aRefusedHistoryShowsTheEngineText() =
        runTest {
            val r = start().opened()
            coEvery { r.engine.listMenuChanges(any()) } throws PosException.Coded("forbidden", emptyMap())

            r.model.openHistory()
            runCurrent()

            assertEquals(r.env.text("err_forbidden"), r.state.error)
        }

    // ── State helpers ─────────────────────────────────────────────

    @Test
    fun labelsFollowTheEstablishmentLanguageThenFrench() {
        val names = mapOf("fr" to "Sushi", "nl" to "Sushi NL", "zh" to "寿司")
        assertEquals("Sushi NL", labelOf(names, listOf("nl", "fr")))
        assertEquals("Sushi", labelOf(mapOf("fr" to "Sushi"), listOf("de", "fr")))
        assertEquals("寿司", labelOf(mapOf("zh" to "寿司"), listOf("fr")))
        assertEquals("", labelOf(emptyMap(), listOf("fr")))
        assertEquals("menu_ed_vat_x", vatKey("X"))
    }
}
