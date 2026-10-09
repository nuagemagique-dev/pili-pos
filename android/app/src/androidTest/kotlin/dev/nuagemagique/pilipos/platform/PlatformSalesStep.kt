package dev.nuagemagique.pilipos.platform

import android.util.Log
import kotlinx.coroutines.runBlocking
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.PosException

/**
 * The `sales` step: N takeaway sales of one product paid in cash, then the Z report, with the size of the
 * database and of the backups measured around them (the figure behind PLATFORM.md section 12).
 *
 * No amount is computed here: a cash part carries no amount, the engine settles what is left.
 */
internal object PlatformSalesStep {
    fun run(env: PlatformE2eEnv) {
        env.login()
        val before = env.dbBytes()
        val logicalBefore = env.dbLogical()
        var menuItemId: Long? = null
        repeat(env.sales) {
            val order = newOrder(env)
            val known = menuItemId
            if (known == null) {
                menuItemId = addFirstProductThatFits(env, order)
            } else {
                addLine(env, order, known)
            }
            pay(env, order)
        }
        val after = env.dbBytes()
        val logicalAfter = env.dbLogical()
        val perSale = (after - before) / env.sales
        PlatformE2eEnv.line(
            "PLATE2E_DBSIZE sales=${env.sales} bytes_before=$before bytes_after=$after bytes_per_sale=$perSale item=$menuItemId",
        )
        if (logicalBefore != null && logicalAfter != null) {
            val logicalPerSale = (logicalAfter.first - logicalBefore.first) / env.sales
            PlatformE2eEnv.line(
                "PLATE2E_DBSIZE_LOGICAL sales=${env.sales} bytes_before=${logicalBefore.first} bytes_after=${logicalAfter.first} " +
                    "bytes_per_sale=$logicalPerSale page_size=${logicalAfter.second} free_bytes=${logicalAfter.third}",
            )
        }
        multiLineSales(env, menuItemId ?: throw AssertionError("no product was sold"), logicalAfter?.first)
        val beforeZ = env.dbBytes()
        val z = engine(env) { it.generateZReport(env.insz) }
        if (!z.success) throw AssertionError("the Z report failed: ${z.error}")
        val afterZ = env.dbBytes()
        val (backupBytes, backupCount) = env.treeBytes(env.backups)
        PlatformE2eEnv.line("PLATE2E_DBSIZE_Z bytes_after_z=$afterZ z_delta=${afterZ - beforeZ} report_no=${z.reportNo}")
        PlatformE2eEnv.line("PLATE2E_BACKUPS after_z files=$backupCount bytes=$backupBytes")
    }

    /** A second phase of sales with several lines each: how much one more line costs. */
    private fun multiLineSales(
        env: PlatformE2eEnv,
        menuItemId: Long,
        logicalBefore: Long?,
    ) {
        val sales = env.salesMultiLine
        val lines = env.linesPerMultiLineSale
        if (sales <= 0) return
        repeat(sales) {
            val order = newOrder(env)
            repeat(lines) { addLine(env, order, menuItemId) }
            pay(env, order)
        }
        val logicalAfter = env.dbLogical()
        if (logicalBefore != null && logicalAfter != null) {
            PlatformE2eEnv.line(
                "PLATE2E_DBSIZE_LINES sales=$sales lines=$lines " +
                    "bytes_per_sale=${(logicalAfter.first - logicalBefore) / sales}",
            )
        }
    }

    private fun newOrder(env: PlatformE2eEnv): Long {
        val input = CreateOrderInput(OrderType.TAKEAWAY, null, env.insz)
        return engine(env) { it.createOrder(input) }.id
    }

    private fun addLine(
        env: PlatformE2eEnv,
        orderId: Long,
        menuItemId: Long,
    ) {
        engine(env) { it.addOrderLine(AddOrderLineInput(orderId, menuItemId, 1, null, null)) }
    }

    private fun pay(
        env: PlatformE2eEnv,
        orderId: Long,
    ) {
        val result = engine(env) { it.checkout(orderId, listOf(PaymentPart(PaymentMethod.CASH, null)), null) }
        if (!result.success) throw AssertionError("a sale must succeed: checkout answered ${result.error}")
    }

    private fun <T> engine(
        env: PlatformE2eEnv,
        block: suspend (PosEngineInterface) -> T,
    ): T = runBlocking { env.app.gateway().call(block) }

    /** Adds the first active, fixed-price product that the engine accepts to [orderId] and returns its id. */
    private fun addFirstProductThatFits(
        env: PlatformE2eEnv,
        orderId: Long,
    ): Long {
        val items = engine(env) { it.getMenuItems(null, "fr") }
        for (item in items.filter { it.active && !it.openPrice }) {
            try {
                addLine(env, orderId, item.id)
                return item.id
            } catch (e: PosException) {
                Log.i(PlatformE2eEnv.TAG, "product ${item.id} cannot be added plain: ${e.message}")
            }
        }
        throw AssertionError("no product of the menu can be added without choices")
    }
}
