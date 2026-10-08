package dev.nuagemagique.pilipos.feature.order

/** Route constants of the order feature. */
object OrderRoutes {
    /** Name of the Long argument carrying the order id. */
    const val ARG_ORDER_ID = "orderId"

    /** Pattern of the order screen. */
    const val ORDER = "order/{orderId}"

    /** Route of the order screen for [orderId]. */
    fun order(orderId: Long): String = "order/$orderId"
}
