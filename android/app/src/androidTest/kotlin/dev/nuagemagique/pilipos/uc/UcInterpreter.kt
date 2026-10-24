package dev.nuagemagique.pilipos.uc

import dev.nuagemagique.pilipos.engine.EngineGateway
import kotlinx.coroutines.runBlocking
import org.json.JSONArray
import org.json.JSONObject
import uniffi.pos_ffi.AddOrderLineInput
import uniffi.pos_ffi.AddWithModifiersInput
import uniffi.pos_ffi.CreateOrderInput
import uniffi.pos_ffi.LeaveReason
import uniffi.pos_ffi.MoneyDirection
import uniffi.pos_ffi.OrderType
import uniffi.pos_ffi.PaymentMethod
import uniffi.pos_ffi.PaymentPart
import uniffi.pos_ffi.PosEngineInterface
import uniffi.pos_ffi.RefundItem
import uniffi.pos_ffi.RequestedMedium
import uniffi.pos_ffi.TicketDeliveryRequest

/**
 * The Kotlin interpreter of the UC vocabulary (P7 section 4.3, decision P7D6).
 *
 * A step is `{cmd, args}` where `args` is exactly what `ui/src/api/ipc.ts` passes to `invoke`. This class maps the
 * JSON arguments to the uniffi records, calls the engine through [EngineGateway] and converts the answer to the
 * JSON of the Rust type ([UcJson.toJson]). It computes no amount and holds no fiscal rule: a payment part carries
 * the amount the scenario wrote (or none), the engine settles the rest.
 */
internal class UcInterpreter(
    private val engine: EngineGateway,
    private val mocks: UcMocks,
) {
    /** Runs one command and returns the answer as org.json (JSONObject.NULL for a command with no answer). */
    fun execute(
        cmd: String,
        args: JSONObject,
    ): Any =
        when (cmd) {
            "login", "logout", "set_training_mode", "work_in", "work_out", "end_shift" -> session(cmd, args)
            "create_order",
            "add_order_line",
            "add_order_line_with_modifiers",
            "update_order_line_quantity",
            "remove_order_line",
            "set_active_order",
            "leave_order",
            "sign_table_order",
            "sign_pre_bill",
            -> orders(cmd, args)
            "checkout", "close_unpaid", "refund_checkout", "copy_ticket" -> sales(cmd, args)
            "money_in_out",
            "open_drawer_fiscal",
            "generate_x_report",
            "generate_z_report",
            "generate_user_x_report",
            -> cashAndReports(cmd, args)
            "check_fdm_status", "list_fiscal_events", "get_order" -> queries(cmd, args)
            "fdm_control", "fdm_control_reset", "printer_control", "sleep_ms" -> mock(cmd, args)
            else -> throw UcStepException("unknown command '$cmd'")
        }

    private fun session(
        cmd: String,
        args: JSONObject,
    ): Any =
        when (cmd) {
            "login" -> call { it.login(args.str("insz"), args.strOrNull("pin")) }
            "logout" -> call { it.logout() }
            "set_training_mode" -> call { it.setTrainingMode(args.bool("enabled")) }
            "work_in" -> call { it.workIn(args.str("employeeInsz")) }
            "work_out" -> call { it.workOut(args.str("employeeInsz")) }
            "end_shift" -> call { it.endShift(args.str("insz"), args.strOrNull("pin")) }
            else -> throw UcStepException("unknown session command '$cmd'")
        }

    private fun orders(
        cmd: String,
        args: JSONObject,
    ): Any =
        when (cmd) {
            "create_order" -> call { it.createOrder(createOrderInput(args.obj("input"))) }
            "add_order_line" -> call { it.addOrderLine(addOrderLineInput(args.obj("input"))) }
            "add_order_line_with_modifiers" -> call { it.addOrderLineWithModifiers(addWithModifiersInput(args.obj("input"))) }
            "update_order_line_quantity" -> call { it.updateOrderLineQuantity(args.long("lineId"), args.int("quantity")) }
            "remove_order_line" -> call { it.removeOrderLine(args.long("lineId")) }
            "set_active_order" -> call { it.setActiveOrder(args.longOrNull("orderId")) }
            "leave_order" -> call { it.leaveOrder(args.long("orderId"), args.enum<LeaveReason>("reason")) }
            "sign_table_order" -> call { it.signTableOrder(args.long("orderId")) }
            "sign_pre_bill" -> call { it.signPreBill(args.long("orderId")) }
            else -> throw UcStepException("unknown order command '$cmd'")
        }

    private fun sales(
        cmd: String,
        args: JSONObject,
    ): Any =
        when (cmd) {
            "checkout" -> call { it.checkout(args.long("orderId"), paymentParts(args.arr("payments")), delivery(args)) }
            "close_unpaid" -> call { it.closeUnpaid(args.long("orderId"), args.str("insz"), args.str("pin")) }
            "refund_checkout" -> call { refund(it, args) }
            "copy_ticket" -> call { it.copyTicket(args.long("fiscalEventId"), args.str("employeeInsz"), delivery(args)) }
            else -> throw UcStepException("unknown sale command '$cmd'")
        }

    private fun cashAndReports(
        cmd: String,
        args: JSONObject,
    ): Any =
        when (cmd) {
            "money_in_out" -> call { moneyInOut(it, args) }
            "open_drawer_fiscal" -> call { it.openDrawerFiscal(args.str("employeeInsz")) }
            "generate_x_report" -> call { it.generateXReport(args.str("employeeInsz")) }
            "generate_z_report" -> call { it.generateZReport(args.str("employeeInsz")) }
            "generate_user_x_report" -> call { it.generateUserXReport(args.str("employeeInsz")) }
            else -> throw UcStepException("unknown cash or report command '$cmd'")
        }

    private fun queries(
        cmd: String,
        args: JSONObject,
    ): Any =
        when (cmd) {
            "check_fdm_status" -> call { it.checkFdmStatus() }
            "list_fiscal_events" -> call { it.listFiscalEvents(args.longOrNull("limit")) }
            "get_order" -> call { it.getOrder(args.long("orderId")) }
            else -> throw UcStepException("unknown query command '$cmd'")
        }

    private fun mock(
        cmd: String,
        args: JSONObject,
    ): Any {
        when (cmd) {
            "fdm_control" -> mocks.fdmControl(args)
            "fdm_control_reset" -> mocks.fdmControlReset()
            "printer_control" -> mocks.printerControl(args)
            "sleep_ms" -> mocks.sleep(args.long("ms"))
            else -> throw UcStepException("unknown mock command '$cmd'")
        }
        return JSONObject.NULL
    }

    /** One engine call through the gateway, off the main thread; the answer is converted to JSON. */
    private fun <T> call(block: suspend (PosEngineInterface) -> T): Any = UcJson.toJson(runBlocking { engine.call(block) })

    private suspend fun refund(
        engine: PosEngineInterface,
        args: JSONObject,
    ) = engine.refundCheckout(
        args.long("originalFiscalEventId"),
        refundItems(args.arr("items")),
        args.str("employeeInsz"),
        delivery(args),
    )

    private suspend fun moneyInOut(
        engine: PosEngineInterface,
        args: JSONObject,
    ) = engine.moneyInOut(
        args.str("employeeInsz"),
        args.long("amountCents"),
        args.enum<MoneyDirection>("direction"),
        args.str("reason"),
    )

    private fun createOrderInput(input: JSONObject) =
        CreateOrderInput(input.enum<OrderType>("order_type"), input.intOrNull("table_number"), input.str("user_insz"))

    private fun addOrderLineInput(input: JSONObject) =
        AddOrderLineInput(
            input.long("order_id"),
            input.long("menu_item_id"),
            input.int("quantity"),
            input.strOrNull("modifiers_json"),
            input.longOrNull("unit_price_cents"),
        )

    private fun addWithModifiersInput(input: JSONObject): AddWithModifiersInput {
        val ids = input.arr("modifier_ids")
        return AddWithModifiersInput(
            input.long("order_id"),
            input.long("menu_item_id"),
            input.int("quantity"),
            List(ids.length()) { ids.getLong(it) },
        )
    }

    private fun paymentParts(parts: JSONArray): List<PaymentPart> =
        List(parts.length()) {
            val part = parts.getJSONObject(it)
            PaymentPart(part.enum<PaymentMethod>("method"), part.longOrNull("amount_cents"))
        }

    private fun refundItems(items: JSONArray): List<RefundItem> =
        List(items.length()) {
            val item = items.getJSONObject(it)
            RefundItem(item.long("menuItemId"), item.int("quantity"), item.longOrNull("lineId"))
        }

    private fun delivery(args: JSONObject): TicketDeliveryRequest? {
        if (args.isNull("delivery")) return null
        val request = args.getJSONObject("delivery")
        val medium = if (request.isNull("medium")) RequestedMedium.PAPER else request.enum<RequestedMedium>("medium")
        return TicketDeliveryRequest(medium, request.strOrNull("email"))
    }
}

private fun JSONObject.required(name: String) {
    if (isNull(name)) throw UcStepException("argument '$name' is missing or null")
}

private fun JSONObject.str(name: String): String {
    required(name)
    return getString(name)
}

private fun JSONObject.strOrNull(name: String): String? = if (isNull(name)) null else getString(name)

private fun JSONObject.long(name: String): Long {
    required(name)
    return getLong(name)
}

private fun JSONObject.longOrNull(name: String): Long? = if (isNull(name)) null else getLong(name)

private fun JSONObject.int(name: String): Int {
    required(name)
    return getInt(name)
}

private fun JSONObject.intOrNull(name: String): Int? = if (isNull(name)) null else getInt(name)

private fun JSONObject.bool(name: String): Boolean {
    required(name)
    return getBoolean(name)
}

private fun JSONObject.obj(name: String): JSONObject {
    required(name)
    return getJSONObject(name)
}

private fun JSONObject.arr(name: String): JSONArray {
    required(name)
    return getJSONArray(name)
}

/** The enum constant for the serde text of the field: `dine_in` is `DINE_IN`. */
private inline fun <reified T : Enum<T>> JSONObject.enum(name: String): T {
    val text = str(name)
    return try {
        enumValueOf<T>(text.uppercase())
    } catch (e: IllegalArgumentException) {
        throw UcStepException("argument '$name': '$text' is not a ${T::class.java.simpleName} (${e.message})", e)
    }
}
