package dev.nuagemagique.pilipos.feature.settings.menu

import uniffi.pos_ffi.AdminAllergens
import uniffi.pos_ffi.AdminCategory
import uniffi.pos_ffi.AdminGroup
import uniffi.pos_ffi.AdminOption
import uniffi.pos_ffi.AdminProduct
import uniffi.pos_ffi.MenuAdmin
import uniffi.pos_ffi.MenuChange

internal val sushiCategory = AdminCategory(1L, mapOf("fr" to "Sushi", "zh" to "寿司"), 1, "#E94560", true)
internal val drinksCategory = AdminCategory(2L, mapOf("fr" to "Boissons"), 2, null, false)

internal val makiProduct =
    AdminProduct(
        id = 10L,
        categoryId = 1L,
        names = mapOf("fr" to "Maki saumon", "zh" to "三文鱼卷"),
        priceCents = 650L,
        vatCode = "B",
        vatCodeTakeaway = "C",
        kitchenPrinter = true,
        code = "A1",
        openPrice = false,
        sortOrder = 1,
        active = true,
    )

internal val edamameProduct =
    AdminProduct(11L, 1L, mapOf("fr" to "Edamame"), 450L, "B", null, false, null, false, 2, false)

internal val sauceGroup = AdminGroup(20L, 10L, mapOf("fr" to "Sauce"), 0, 2, 1, true)
internal val sojaOption = AdminOption(30L, 20L, mapOf("fr" to "Soja"), 50L, 1, true)

@Suppress("LongParameterList") // One defaulted parameter per list of MenuAdmin, the test builder of the record.
internal fun adminMenu(
    managedBy: String? = null,
    categories: List<AdminCategory> = listOf(sushiCategory, drinksCategory),
    products: List<AdminProduct> = listOf(makiProduct, edamameProduct),
    groups: List<AdminGroup> = listOf(sauceGroup),
    options: List<AdminOption> = listOf(sojaOption),
    allergens: List<AdminAllergens> = emptyList(),
): MenuAdmin =
    MenuAdmin(
        managedBy = managedBy,
        languages = listOf("fr", "nl", "en", "zh"),
        categories = categories,
        products = products,
        groups = groups,
        options = options,
        allergens = allergens,
    )

/** Maki saumon declared: fish and soy, traces of sesame. */
internal val makiAllergens = AdminAllergens(ALLERGEN_PRODUCT, 10L, listOf("fish", "soybeans"), listOf("sesame"))

internal fun menuChange(
    id: Long,
    entityType: String = "product",
    action: String = "update",
    label: String = "Maki saumon",
    fields: List<String> = listOf("price_cents", "vat_code"),
): MenuChange =
    MenuChange(
        id = id,
        entityType = entityType,
        entityId = 10L,
        action = action,
        label = label,
        changedFields = fields,
        beforeJson = if (action == "update") "{}" else null,
        afterJson = "{}",
        changedBy = "85010100123",
        changedByName = "Alice",
        changedAt = "2026-10-17 09:30:00",
    )
