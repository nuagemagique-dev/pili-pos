package dev.nuagemagique.pilipos.ui.scaffold

/**
 * The screen a [PosScaffold] frames: it decides which header button is active, whether the FDM blocking veil
 * covers the content and whether leaving the screen must first register the open order.
 */
enum class HeaderArea(
    /** True for the sales screens (`SALES_ROUTES` of `FdmAlerts.vue`): the blocking veil covers them (PD209). */
    val isSales: Boolean,
    /** True for the screens where the current order is built or paid (`ORDER_ROUTES` of `router/index.ts`). */
    val holdsOrder: Boolean,
) {
    /** The table grid and the open takeaways. */
    TABLES(isSales = true, holdsOrder = false),

    /** The order being taken. */
    ORDER(isSales = true, holdsOrder = true),

    /** The payment of the order. */
    CHECKOUT(isSales = true, holdsOrder = true),

    /** The reports (managers only); reachable while the FDM is blocking. */
    REPORTS(isSales = false, holdsOrder = false),

    /** The settings (managers only); reachable while the FDM is blocking. */
    SETTINGS(isSales = false, holdsOrder = false),

    /** The online orders board. */
    WEBSHOP(isSales = false, holdsOrder = false),
}
