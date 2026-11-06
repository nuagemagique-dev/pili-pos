package dev.nuagemagique.pilipos.feature.checkout

/** Route constants of the checkout feature. */
object CheckoutRoutes {
    /** Name of the Long argument carrying the order id. */
    const val ARG_ORDER_ID = "orderId"

    /** Name of the Boolean argument that marks an unpaid online order collected here (UX audit O2). */
    const val ARG_ONLINE = "online"

    /** Name of the Boolean argument that starts an online collection on the card method. */
    const val ARG_CARD = "card"

    /** Pattern of the checkout screen; the two optional arguments only matter for an online order. */
    const val CHECKOUT = "checkout/{orderId}?online={online}&card={card}"

    /** Route of the checkout screen for [orderId]. */
    fun checkout(orderId: Long): String = "checkout/$orderId"

    /** Route of the checkout of the unpaid online order [orderId], starting on the card method when [card]. */
    fun checkoutOnline(
        orderId: Long,
        card: Boolean,
    ): String = "checkout/$orderId?online=true&card=$card"
}
