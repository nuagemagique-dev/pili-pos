package dev.nuagemagique.pilipos.platform.update

import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import org.json.JSONException
import org.json.JSONObject
import timber.log.Timber
import java.io.ByteArrayOutputStream
import java.io.File
import java.io.IOException
import java.io.OutputStream
import java.net.URL
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.security.MessageDigest
import kotlin.coroutines.CoroutineContext
import kotlin.coroutines.coroutineContext
import kotlinx.coroutines.ensureActive as ensureContextActive

/** Where a URL may be fetched from: TLS everywhere, clear text only to the loopback of a debug build. */
internal object UpdateUrls {
    private const val HTTPS = "https"
    private const val CLEAR = "http"
    private val LOOPBACK = setOf("127.0.0.1", "localhost")

    /** True for the two hosts a host mock is reached on. */
    fun isLoopback(host: String?): Boolean = host != null && host in LOOPBACK

    /** True when [url] may be fetched: always for TLS, for clear text only on the loopback and [allowCleartext]. */
    fun allowed(
        url: URL,
        allowCleartext: Boolean,
    ): Boolean =
        when (url.protocol) {
            HTTPS -> true
            CLEAR -> allowCleartext && isLoopback(url.host)
            else -> false
        }

    /** Scheme, host and port of [url]; two URLs with the same origin may share the bearer token. */
    fun origin(url: URL): String = "${url.protocol}|${url.host.lowercase()}|${if (url.port < 0) url.defaultPort else url.port}"
}

/**
 * The declared release read from an OCI artifact of the public registry, anonymously (PD211).
 *
 * Order: a pull token from `{base}/token`, the manifest of [tag] from `{base}/v2/{repository}/manifests/{tag}`,
 * then the blobs `{base}/v2/{repository}/blobs/{digest}`. The layers are found by their
 * `org.opencontainers.image.title` annotation: `declared.json`, `declared.json.minisig` and the asset. Every
 * blob is checked against its `sha256:` digest while it streams.
 *
 * Redirects (the registry sends blobs to a CDN) are followed by hand, at most [HttpGet.MAX_REDIRECTS], and the
 * `Authorization` header goes only to the origin of the base URL, never to the target of a redirect. Clear
 * text is refused unless [allowCleartext] (debug builds) and the host is `127.0.0.1` or `localhost`.
 */
class GhcrSource(
    baseUrl: String = DEFAULT_BASE_URL,
    private val repository: String = DEFAULT_REPOSITORY,
    private val tag: String = DEFAULT_TAG,
    allowCleartext: Boolean = false,
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO,
    timeouts: HttpTimeouts = HttpTimeouts(),
) : UpdateSource {
    private val base: String = baseUrl.trim().trimEnd('/')
    private val http = HttpGet(base, allowCleartext, timeouts)
    private val tokenLock = Mutex()
    private val manifestLock = Mutex()
    private var token: String? = null
    private var layers: List<Layer>? = null

    override val label: String = "ghcr"

    override suspend fun fetchDescriptor(): DescriptorFiles {
        val ctx = coroutineContext
        val json = blobBytes(UpdateSource.DESCRIPTOR_NAME, ctx)
        val signature = blobBytes(UpdateSource.SIGNATURE_NAME, ctx)
        return DescriptorFiles(json, String(signature, Charsets.UTF_8))
    }

    override suspend fun fetchAsset(
        name: String,
        dest: File,
        maxBytes: Long,
    ) {
        val ctx = coroutineContext
        val layer = layer(name, ctx)
        refuseUnless(layer.size <= maxBytes, UpdateRefusalCode.TOO_LARGE) { "$name is ${layer.size} bytes" }
        val bearer = bearer(ctx)
        val partial = File(dest.parentFile, dest.name + ".part")
        withContext(ioDispatcher) {
            try {
                dest.parentFile?.mkdirs()
                dest.delete()
                partial.outputStream().use { out ->
                    download(layer, out, Limits(maxBytes, UpdateRefusalCode.SHA256_MISMATCH), bearer, ctx)
                }
                Files.move(partial.toPath(), dest.toPath(), StandardCopyOption.REPLACE_EXISTING)
            } finally {
                partial.delete()
            }
        }
    }

    private class Layer(
        val title: String,
        val digest: String,
        val size: Long,
    )

    /** How far a blob may grow and the refusal for a digest that is not the announced one. */
    private class Limits(
        val maxBytes: Long,
        val mismatch: UpdateRefusalCode,
    )

    private suspend fun blobBytes(
        name: String,
        ctx: CoroutineContext,
    ): ByteArray {
        val layer = layer(name, ctx)
        refuseUnless(layer.size <= DeclaredParser.MAX_BYTES, UpdateRefusalCode.TOO_LARGE) { "$name is ${layer.size} bytes" }
        val bearer = bearer(ctx)
        val limits = Limits(DeclaredParser.MAX_BYTES.toLong(), UpdateRefusalCode.BAD_DESCRIPTOR)
        return withContext(ioDispatcher) {
            val out = ByteArrayOutputStream()
            download(layer, out, limits, bearer, ctx)
            out.toByteArray()
        }
    }

    private suspend fun layer(
        title: String,
        ctx: CoroutineContext,
    ): Layer {
        val all = manifestLock.withLock { layers ?: loadManifest(ctx).also { layers = it } }
        val found = all.filter { it.title == title }
        refuseUnless(found.size == 1, UpdateRefusalCode.BAD_DESCRIPTOR) {
            if (found.isEmpty()) "the manifest has no layer called $title" else "the manifest has several layers called $title"
        }
        return found.single()
    }

    private suspend fun bearer(ctx: CoroutineContext): String = tokenLock.withLock { token ?: loadToken(ctx).also { token = it } }

    private suspend fun loadToken(ctx: CoroutineContext): String {
        val url = "$base/token?scope=repository:$repository:pull&service=ghcr.io"
        val body = withContext(ioDispatcher) { http.bytes(url, null, null, SMALL_BYTES, ctx) }
        val json = parseJson(body, "the token answer")
        val value = json.optString("token").ifEmpty { json.optString("access_token") }
        refuseUnless(value.isNotEmpty(), UpdateRefusalCode.NETWORK) { "the token answer holds no token" }
        return value
    }

    private suspend fun loadManifest(ctx: CoroutineContext): List<Layer> {
        val bearer = bearer(ctx)
        val url = "$base/v2/$repository/manifests/$tag"
        val body = withContext(ioDispatcher) { http.bytes(url, MANIFEST_TYPE, bearer, MANIFEST_BYTES, ctx) }
        val array =
            parseJson(body, "the manifest").optJSONArray("layers")
                ?: refuse(UpdateRefusalCode.BAD_DESCRIPTOR, "the manifest has no layers")
        return (0 until array.length()).mapNotNull { index -> array.optJSONObject(index)?.let(::layerOf) }
    }

    private fun layerOf(entry: JSONObject): Layer? {
        val title = entry.optJSONObject("annotations")?.optString(TITLE_ANNOTATION).orEmpty()
        val digest = entry.optString("digest")
        if (title.isEmpty() || digest.isEmpty() || !entry.has("size")) return null
        return Layer(title, digest, entry.optLong("size", -1L))
    }

    private fun parseJson(
        bytes: ByteArray,
        what: String,
    ): JSONObject =
        try {
            JSONObject(String(bytes, Charsets.UTF_8))
        } catch (e: JSONException) {
            refuse(UpdateRefusalCode.NETWORK, "$what is not JSON", e)
        }

    /** Streams the blob of [layer] into [out], hashing it; a wrong digest is refused with [Limits.mismatch]. */
    private fun download(
        layer: Layer,
        out: OutputStream,
        limits: Limits,
        bearer: String,
        ctx: CoroutineContext,
    ) {
        val expected = layer.digest.removePrefix(DIGEST_PREFIX)
        val supported = layer.digest.startsWith(DIGEST_PREFIX) && SHA256_HEX.matches(expected)
        refuseUnless(supported, UpdateRefusalCode.BAD_DESCRIPTOR) { "${layer.title}: unsupported digest" }
        val sha = MessageDigest.getInstance("SHA-256")
        http.stream("$base/v2/$repository/blobs/${layer.digest}", null, bearer) { input ->
            val buffer = ByteArray(BUFFER_SIZE)
            var total = 0L
            var read = input.read(buffer)
            while (read >= 0) {
                ctx.ensureContextActive()
                total += read
                refuseUnless(total <= limits.maxBytes, UpdateRefusalCode.TOO_LARGE) {
                    "${layer.title} is larger than ${limits.maxBytes} bytes"
                }
                sha.update(buffer, 0, read)
                out.write(buffer, 0, read)
                read = input.read(buffer)
            }
        }
        val actual = sha.digest().joinToString("") { "%02x".format(it) }
        refuseUnless(actual == expected, limits.mismatch) { "${layer.title}: digest $actual is not $expected" }
    }

    companion object {
        /** The public registry. */
        const val DEFAULT_BASE_URL = "https://ghcr.io"

        /** The public package shared with the Linux `:declared` tag. */
        const val DEFAULT_REPOSITORY = "nuagemagique-dev/pili-pos"

        /** The tag of the Android declaration. */
        const val DEFAULT_TAG = "declared-android"

        /** Debug-only override of the base URL, relative to `filesDir`. */
        const val OVERRIDE_PATH = "platform/registry.json"

        private const val MANIFEST_TYPE = "application/vnd.oci.image.manifest.v1+json"
        private const val TITLE_ANNOTATION = "org.opencontainers.image.title"
        private const val DIGEST_PREFIX = "sha256:"
        private val SHA256_HEX = Regex("^[0-9a-f]{64}$")
        private const val BUFFER_SIZE = 64 * 1024
        private const val SMALL_BYTES = 64 * 1024
        private const val MANIFEST_BYTES = 1024 * 1024

        /**
         * The base URL written in `filesDir/platform/registry.json` (`{"baseUrl":"..."}`), or null when the file
         * is absent or the URL is not acceptable (clear text is acceptable only to `127.0.0.1` or `localhost`).
         * Debug builds only: [forApp] never calls it in a release build.
         */
        fun overrideBaseUrl(filesDir: File): String? {
            val file = File(filesDir, OVERRIDE_PATH)
            if (!file.isFile) return null
            return try {
                val value = JSONObject(file.readText(Charsets.UTF_8)).optString("baseUrl").trim()
                val url = URL(value)
                if (UpdateUrls.allowed(url, allowCleartext = true)) value else null.also { Timber.w("GhcrSource: registry.json refused") }
            } catch (e: IOException) {
                Timber.w(e, "GhcrSource: registry.json is unreadable")
                null
            } catch (e: JSONException) {
                Timber.w(e, "GhcrSource: registry.json is not JSON")
                null
            }
        }

        /** The source of this build: the public registry, or in a debug build the override of [filesDir]. */
        fun forApp(
            filesDir: File,
            debug: Boolean,
            ioDispatcher: CoroutineDispatcher = Dispatchers.IO,
        ): GhcrSource =
            GhcrSource(
                baseUrl = if (debug) overrideBaseUrl(filesDir) ?: DEFAULT_BASE_URL else DEFAULT_BASE_URL,
                allowCleartext = debug,
                ioDispatcher = ioDispatcher,
            )
    }
}
