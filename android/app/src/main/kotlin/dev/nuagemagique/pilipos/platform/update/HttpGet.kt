package dev.nuagemagique.pilipos.platform.update

import java.io.ByteArrayOutputStream
import java.io.IOException
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.MalformedURLException
import java.net.URL
import kotlin.coroutines.CoroutineContext
import kotlinx.coroutines.ensureActive as ensureContextActive

/** Connect and read timeouts of the registry calls. */
data class HttpTimeouts(
    val connectMs: Int = CONNECT_MS,
    val readMs: Int = READ_MS,
) {
    companion object {
        /** Connect timeout: 15 s. */
        const val CONNECT_MS = 15_000

        /** Read timeout: 60 s. */
        const val READ_MS = 60_000
    }
}

/**
 * Blocking GET over `HttpURLConnection` with the rules of the updater: every URL must pass [UpdateUrls.allowed],
 * redirects are followed by hand (at most [MAX_REDIRECTS]) and the bearer token is sent only to the origin of
 * [baseUrl]. Transport failures and HTTP errors are `network` refusals.
 */
internal class HttpGet(
    private val baseUrl: String,
    private val allowCleartext: Boolean,
    private val timeouts: HttpTimeouts,
) {
    private val baseOrigin: String by lazy { UpdateUrls.origin(url(baseUrl, null)) }

    /** GET [spec], following redirects, and hands the body of the final 2xx answer to [read]. */
    fun stream(
        spec: String,
        accept: String?,
        bearer: String?,
        read: (InputStream) -> Unit,
    ) {
        var current = url(spec, null)
        var hops = 0
        while (true) {
            current = attempt(current, accept, bearer, read) ?: return
            refuseUnless(++hops <= MAX_REDIRECTS, UpdateRefusalCode.NETWORK) { "more than $MAX_REDIRECTS redirects" }
        }
    }

    /** GET [spec] into memory; an answer above [maxBytes] is a `network` refusal. */
    fun bytes(
        spec: String,
        accept: String?,
        bearer: String?,
        maxBytes: Int,
        ctx: CoroutineContext,
    ): ByteArray {
        val out = ByteArrayOutputStream()
        stream(spec, accept, bearer) { input ->
            val buffer = ByteArray(BUFFER_SIZE)
            var read = input.read(buffer)
            while (read >= 0) {
                ctx.ensureContextActive()
                refuseUnless(out.size() + read <= maxBytes, UpdateRefusalCode.NETWORK) { "an answer is larger than $maxBytes bytes" }
                out.write(buffer, 0, read)
                read = input.read(buffer)
            }
        }
        return out.toByteArray()
    }

    /** One request; the redirect target, or null once the body was read. */
    private fun attempt(
        target: URL,
        accept: String?,
        bearer: String?,
        read: (InputStream) -> Unit,
    ): URL? {
        refuseUnless(UpdateUrls.allowed(target, allowCleartext), UpdateRefusalCode.NETWORK) {
            "refused: ${target.protocol} to ${target.host} is not allowed"
        }
        val connection = target.openConnection() as HttpURLConnection
        try {
            connection.instanceFollowRedirects = false
            connection.connectTimeout = timeouts.connectMs
            connection.readTimeout = timeouts.readMs
            connection.requestMethod = "GET"
            if (accept != null) connection.setRequestProperty("Accept", accept)
            if (bearer != null && UpdateUrls.origin(target) == baseOrigin) {
                connection.setRequestProperty("Authorization", "Bearer $bearer")
            }
            return answer(connection, target, read)
        } catch (e: IOException) {
            refuse(UpdateRefusalCode.NETWORK, "${e.javaClass.simpleName} from ${target.host}", e)
        } finally {
            connection.disconnect()
        }
    }

    private fun answer(
        connection: HttpURLConnection,
        target: URL,
        read: (InputStream) -> Unit,
    ): URL? {
        val status = connection.responseCode
        if (status in HTTP_OK) {
            connection.inputStream.use(read)
            return null
        }
        refuseUnless(status in REDIRECTS, UpdateRefusalCode.NETWORK) { "HTTP $status from ${target.host}" }
        val location = connection.getHeaderField("Location")
        refuseUnless(location != null, UpdateRefusalCode.NETWORK) { "redirect $status without a Location" }
        return url(location, target)
    }

    private fun url(
        spec: String,
        context: URL?,
    ): URL =
        try {
            URL(context, spec)
        } catch (e: MalformedURLException) {
            refuse(UpdateRefusalCode.NETWORK, "malformed URL", e)
        }

    companion object {
        /** Redirects followed before giving up. */
        const val MAX_REDIRECTS = 5

        private const val BUFFER_SIZE = 64 * 1024
        private val HTTP_OK = 200..299
        private val REDIRECTS = setOf(301, 302, 303, 307, 308)
    }
}
