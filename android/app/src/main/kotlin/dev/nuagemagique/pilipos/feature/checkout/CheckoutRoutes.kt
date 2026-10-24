package dev.nuagemagique.pilipos.feature.checkout

/** Route constants of the checkout feature. */
object CheckoutRoutes {
    /** Name of the Long argument carrying the order id. */
    const val ARG_ORDER_ID = "orderId"

    /** Pattern of the checkout screen. */
    const val CHECKOUT = "checkout/{orderId}"

    /** Route of the checkout screen for [orderId]. */
    fun checkout(orderId: Long): String = "checkout/$orderId"
}
