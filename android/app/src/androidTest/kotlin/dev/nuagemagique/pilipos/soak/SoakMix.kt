package dev.nuagemagique.pilipos.soak

import java.util.Random

/** The sale mix of one compressed day (P7D12): 70 percent takeaway, 20 percent table, 5 percent split, 5 percent refund. */
internal object SoakMix {
    private const val PERCENT = 100
    private const val TABLE_PERCENT = 20
    private const val RARE_PERCENT = 5

    /**
     * The kinds of the [sales] sales of a day in the order they are taken. At least one split and one refund; a
     * refund always comes after at least one sale, since it undoes an earlier sale of the same day.
     */
    fun plan(
        sales: Int,
        rng: Random,
    ): List<SaleKind> {
        val refunds = maxOf(1, sales * RARE_PERCENT / PERCENT)
        val splits = maxOf(1, sales * RARE_PERCENT / PERCENT)
        val tables = sales * TABLE_PERCENT / PERCENT
        val takeaways = sales - refunds - splits - tables
        val kinds = mutableListOf<SaleKind>()
        kinds += List(takeaways) { SaleKind.TAKEAWAY }
        kinds += List(tables) { SaleKind.TABLE }
        kinds += List(splits) { SaleKind.SPLIT }
        kinds += List(refunds) { SaleKind.REFUND }
        kinds.shuffle(rng)
        val firstSale = kinds.indexOfFirst { it != SaleKind.REFUND }
        if (kinds.first() == SaleKind.REFUND && firstSale > 0) {
            kinds[0] = kinds[firstSale].also { kinds[firstSale] = kinds[0] }
        }
        return kinds
    }
}
