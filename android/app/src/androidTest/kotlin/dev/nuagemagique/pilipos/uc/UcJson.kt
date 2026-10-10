package dev.nuagemagique.pilipos.uc

import org.json.JSONArray
import org.json.JSONObject
import uniffi.pos_ffi.PosException
import java.lang.reflect.Modifier

/** A scenario step that cannot run or whose arguments are wrong (not an engine refusal). */
internal class UcStepException(
    message: String,
    cause: Throwable? = null,
) : Exception(message, cause)

/**
 * The JSON side of the UC runner (P7 section 4): `$name.path` references, `expect` checks, the error code of a
 * refusal and the conversion of an engine answer to the JSON that the Rust types serialise to.
 *
 * Nothing here knows a fiscal rule or an amount: values are copied, compared for equality and printed.
 */
internal object UcJson {
    private val reference = Regex("^\\$([A-Za-z_][A-Za-z0-9_]*)((?:\\.[A-Za-z0-9_]+)*)$")

    /**
     * Rust structs that serialise with `rename_all = "camelCase"` (records of `android/rust/pos-ffi/src/records`).
     * Every other record keeps snake_case field names, as the Rust type does.
     */
    private val camelCaseRecords =
        setOf(
            "CopyResult", "CopyableTicket", "CreateUserInput", "DataItem", "DeliveryOutcome", "DeliveryView",
            "ExportStatus", "ExtensionItem", "FdmDevice", "FdmHealth", "FdmReference", "FdmStatusReport",
            "FinancialResult", "FiscalEventSummary", "LocationItem", "LockOutcome", "MailStatus", "MessageItem",
            "OpenShift", "PosSettings", "PresenceFailed", "RefundItem", "RefundableEvent", "ReportResult",
            "SessionLocked", "SignMessages", "SocialResult", "SupportRunning", "SupportStatus", "TableInfo",
            "TakeawayInfo", "TicketDeliveryRequest", "VatCalcItem", "ZReportSummary",
        )

    /** Enums whose variants serialise with their own name (`SCREAMING_SNAKE_CASE`, or none for `VatCode`). */
    private val upperCaseEnums =
        setOf("EventLabel", "FdmMessageDisplay", "FdmWarningCode", "MessageCategory", "TicketMedium", "VatLabel", "VatCode")

    /** The one record the engine answers as a JSON value in which an absent key means "not set". */
    private val omitNullRecords = setOf("FdmStatusReport")

    /** Replaces every string `$name.path` of [value] (an org.json tree) by the value saved under `name`. */
    fun resolve(
        value: Any?,
        saved: Map<String, Any?>,
    ): Any? =
        when (value) {
            is String -> resolveString(value, saved)
            is JSONObject -> {
                val copy = JSONObject()
                value.keys().forEach { key -> copy.put(key, resolve(value.get(key), saved) ?: JSONObject.NULL) }
                copy
            }
            is JSONArray -> {
                val copy = JSONArray()
                for (i in 0 until value.length()) copy.put(resolve(value.get(i), saved) ?: JSONObject.NULL)
                copy
            }
            else -> value
        }

    private fun resolveString(
        text: String,
        saved: Map<String, Any?>,
    ): Any? {
        val match = reference.matchEntire(text) ?: return text
        val name = match.groupValues[1]
        if (!saved.containsKey(name)) throw UcStepException("reference \$$name: no step saved a result under this name")
        val path = match.groupValues[2].removePrefix(".")
        return if (path.isEmpty()) saved[name] else lookup(saved[name], path, "\$$name")
    }

    /** The value at the dot separated [path] of [root]; numbers index arrays. */
    fun lookup(
        root: Any?,
        path: String,
        label: String = "result",
    ): Any? {
        var current: Any? = root
        for (token in path.split('.')) {
            current =
                when (current) {
                    is JSONObject -> if (current.has(token)) current.get(token) else refuse(label, "no field '$token' in $path")
                    is JSONArray -> current.get(arrayIndex(current, token, path, label))
                    else -> refuse(label, "'$token' of $path is looked up in a scalar")
                }
        }
        return current
    }

    private fun arrayIndex(
        array: JSONArray,
        token: String,
        path: String,
        label: String,
    ): Int {
        val index = token.toIntOrNull() ?: refuse(label, "'$token' is not an array index in $path")
        return if (index in 0 until array.length()) index else refuse(label, "index $index out of range in $path")
    }

    private fun refuse(
        label: String,
        reason: String,
    ): Nothing = throw UcStepException("$label: $reason")

    /** Equality of two JSON values; numbers compare by value, so 2 equals 2.0. */
    fun jsonEquals(
        actual: Any?,
        expected: Any?,
    ): Boolean {
        val a = if (actual === JSONObject.NULL) null else actual
        val e = if (expected === JSONObject.NULL) null else expected
        return when {
            a is Number && e is Number -> a.toDouble() == e.toDouble()
            a is JSONObject && e is JSONObject -> a.toString() == e.toString()
            a is JSONArray && e is JSONArray -> a.toString() == e.toString()
            else -> a == e
        }
    }

    /** The code of a refusal: what a scenario writes after `expect.error`. */
    fun errorCode(t: Throwable): String =
        when (t) {
            is PosException.Coded -> t.code
            is PosException.Fdm -> t.detail.substringBefore('|').trim()
            is PosException.Internal -> t.detail
            is PosException.EngineClosed -> "engine_closed"
            is UcStepException -> "uc_step: ${t.message}"
            else -> "${t.javaClass.simpleName}: ${t.message}"
        }

    /**
     * Checks one executed step against its `expect` (default `{}`). Returns null when the step met it, else
     * the reason. [failure] is the refusal the call raised, [result] the answer when it did not raise.
     */
    fun checkExpect(
        expect: JSONObject?,
        failure: Throwable?,
        result: Any?,
    ): String? {
        val wantedError = expect?.optString("error", "")?.takeIf { it.isNotEmpty() }
        val fields = expect?.optJSONObject("fields")
        val problem =
            when {
                failure != null && wantedError == null -> "the call failed: ${errorCode(failure)}"
                failure != null ->
                    errorCode(failure)
                        .takeUnless { it == wantedError || namesCode(errorCode(failure), wantedError.orEmpty()) }
                        ?.let { "error '$it', expected '$wantedError'" }
                wantedError != null -> errorOfResult(result, wantedError)
                else -> unsuccessful(result)
            }
        return problem ?: fieldProblem(fields, result)
    }

    private fun errorOfResult(
        result: Any?,
        wantedError: String,
    ): String? {
        val obj = result as? JSONObject
        val text = obj?.takeIf { it.has("success") && !it.optBoolean("success", true) }?.optString("error", "")
        return if (text != null && namesCode(text, wantedError)) null else "no error '$wantedError' (answer: $result)"
    }

    /**
     * Does [text] carry [code] as a whole word (no letter, digit or underscore around it)? An engine refusal is
     * `code|k=v`; an FDM refusal is free text that names the FDM code (`FDM error: GraphQL error: FDM_LOCKED: ...`),
     * as `names_code` of pos-uc-check reads it.
     */
    fun namesCode(
        text: String,
        code: String,
    ): Boolean {
        fun isWord(c: Char?) = c != null && (c == '_' || (c.code < 128 && c.isLetterOrDigit()))
        var at = text.indexOf(code)
        while (code.isNotEmpty() && at >= 0) {
            val bounded = !isWord(text.getOrNull(at - 1)) && !isWord(text.getOrNull(at + code.length))
            if (bounded) return true
            at = text.indexOf(code, at + 1)
        }
        return false
    }

    private fun unsuccessful(result: Any?): String? {
        val obj = result as? JSONObject ?: return null
        return if (obj.has("success") && !obj.optBoolean("success", false)) "success is false: ${obj.opt("error")}" else null
    }

    private fun fieldProblem(
        fields: JSONObject?,
        result: Any?,
    ): String? = fields?.keys()?.asSequence()?.mapNotNull { key -> fieldMismatch(key, fields.get(key), result) }?.firstOrNull()

    private fun fieldMismatch(
        key: String,
        expected: Any?,
        result: Any?,
    ): String? =
        try {
            val actual = lookup(result, key)
            if (jsonEquals(actual, expected)) null else "field $key is $actual, expected $expected"
        } catch (e: UcStepException) {
            "field $key: ${e.message}"
        }

    /** Converts an engine answer (uniffi records, lists, scalars) to the JSON of the Rust type. */
    fun toJson(value: Any?): Any =
        when (value) {
            null, Unit -> JSONObject.NULL
            is Boolean, is String, is Int, is Long, is Double -> value
            is Float -> value.toDouble()
            is Enum<*> -> enumName(value)
            is List<*> -> JSONArray().also { array -> value.forEach { array.put(toJson(it)) } }
            is Map<*, *> -> JSONObject().also { obj -> value.forEach { (k, v) -> obj.put(k.toString(), toJson(v)) } }
            else -> record(value)
        }

    private fun enumName(value: Enum<*>): String = if (value.javaClass.simpleName in upperCaseEnums) value.name else value.name.lowercase()

    private fun record(value: Any): JSONObject {
        val type = value.javaClass
        val camel = type.simpleName in camelCaseRecords
        val omitNull = type.simpleName in omitNullRecords
        val out = JSONObject()
        type.declaredFields.filterNot { Modifier.isStatic(it.modifiers) || it.isSynthetic }.forEach { field ->
            field.isAccessible = true
            val converted = toJson(field.get(value))
            if (!(omitNull && converted === JSONObject.NULL)) out.put(if (camel) field.name else snakeCase(field.name), converted)
        }
        return out
    }

    private fun snakeCase(name: String): String =
        buildString {
            for (c in name) {
                if (c.isUpperCase()) append('_').append(c.lowercaseChar()) else append(c)
            }
        }
}
