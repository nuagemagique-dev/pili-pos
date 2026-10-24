package dev.nuagemagique.pilipos.platform.update

import org.json.JSONArray
import org.json.JSONObject
import java.io.IOException
import java.io.OutputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.CopyOnWriteArrayList
import kotlin.concurrent.thread

/** One request seen by a [TestServer]. */
data class SeenRequest(
    val method: String,
    val path: String,
    val query: String?,
    val authorization: String?,
    val accept: String?,
)

/** A request being answered; the handler replies once through [TestServer.reply]. */
class Exchange(
    val method: String,
    val path: String,
    val query: String?,
    private val headers: Map<String, String>,
    val out: OutputStream,
) {
    /** Extra response headers, such as `Location`. */
    val responseHeaders = mutableListOf<Pair<String, String>>()

    /** A request header by name (case-insensitive), or null. */
    fun header(name: String): String? = headers[name.lowercase()]

    /** What a test asserts on. */
    fun seen() = SeenRequest(method, path, query, header("Authorization"), header("Accept"))
}

/**
 * A minimal HTTP/1.1 server on 127.0.0.1 and a free port, one thread per connection, `Connection: close`.
 * It is the only network the tests use. (The JDK `HttpServer` lives in a module that the Kotlin test
 * compilation does not see, so this is plain sockets.)
 */
class TestServer {
    val seen = CopyOnWriteArrayList<SeenRequest>()
    private val server = ServerSocket(0, BACKLOG, InetAddress.getByName("127.0.0.1"))
    private val routes = mutableMapOf<String, (Exchange) -> Unit>()

    /** `http` plus the loopback address and the port: a loopback origin. */
    val baseUrl: String get() = "http://127.0.0.1:${server.localPort}"

    init {
        thread(isDaemon = true, name = "test-server-${server.localPort}") { acceptLoop() }
    }

    private fun acceptLoop() {
        while (!server.isClosed) {
            val socket = acceptOrNull() ?: return
            thread(isDaemon = true) { serve(socket) }
        }
    }

    @Suppress("SwallowedException")
    private fun acceptOrNull(): Socket? =
        try {
            server.accept()
        } catch (e: IOException) {
            null
        }

    @Suppress("SwallowedException")
    private fun serve(socket: Socket) {
        socket.use {
            try {
                val exchange = readRequest(socket) ?: return
                seen += exchange.seen()
                val handler = routes[exchange.path]
                if (handler == null) reply(exchange, 404, ByteArray(0)) else handler(exchange)
            } catch (e: IOException) {
                // The client gave up (a timeout, a size limit): nothing to answer.
            }
        }
    }

    private fun readRequest(socket: Socket): Exchange? {
        val reader = socket.getInputStream().bufferedReader(Charsets.ISO_8859_1)
        val parts = reader.readLine()?.split(" ") ?: return null
        val headers = mutableMapOf<String, String>()
        var line = reader.readLine()
        while (!line.isNullOrEmpty()) {
            headers[line.substringBefore(':').lowercase()] = line.substringAfter(':').trim()
            line = reader.readLine()
        }
        val target = parts[1]
        val query = target.substringAfter('?', "").ifEmpty { null }
        return Exchange(parts[0], target.substringBefore('?'), query, headers, socket.getOutputStream())
    }

    fun route(
        path: String,
        handler: (Exchange) -> Unit,
    ) {
        routes[path] = handler
    }

    fun serve(
        path: String,
        body: ByteArray,
        status: Int = 200,
    ) = route(path) { reply(it, status, body) }

    fun redirect(
        path: String,
        location: String,
        status: Int = 307,
    ) = route(path) {
        it.responseHeaders += "Location" to location
        reply(it, status, ByteArray(0))
    }

    fun stop() = server.close()

    companion object {
        private const val BACKLOG = 50

        fun reply(
            exchange: Exchange,
            status: Int,
            body: ByteArray,
        ) {
            val head =
                buildString {
                    append("HTTP/1.1 $status X\r\n")
                    append("Content-Length: ${body.size}\r\n")
                    append("Connection: close\r\n")
                    exchange.responseHeaders.forEach { (name, value) -> append("$name: $value\r\n") }
                    append("\r\n")
                }
            exchange.out.write(head.toByteArray(Charsets.ISO_8859_1))
            exchange.out.write(body)
            exchange.out.flush()
        }
    }
}

/**
 * A registry on 127.0.0.1 for one repository and tag: `/token`, the manifest and the blobs of the layers.
 * Blobs are served by this server, or through a redirect to [cdn] when set.
 */
class TestRegistry(
    private val repository: String = "nuagemagique-dev/pili-pos",
    private val tag: String = "declared-android",
    private val token: String = "test-token",
) {
    val server = TestServer()
    var cdn: TestServer? = null
    private val layers = mutableListOf<JSONObject>()

    val baseUrl: String get() = server.baseUrl

    init {
        server.route("/token") { exchange ->
            TestServer.reply(exchange, 200, JSONObject().put("token", token).toString().toByteArray())
        }
    }

    /**
     * Adds a layer titled [title]. The manifest announces [announcedDigestOf] (default: the real SHA-256) and
     * [announcedSize]; the blob served is [served] (default: [bytes]).
     */
    fun layer(
        title: String,
        bytes: ByteArray,
        served: ByteArray = bytes,
        announcedDigestOf: ByteArray = bytes,
        announcedSize: Long = bytes.size.toLong(),
    ) {
        val digest = "sha256:" + sha256Hex(announcedDigestOf)
        layers +=
            JSONObject()
                .put("mediaType", "application/octet-stream")
                .put("digest", digest)
                .put("size", announcedSize)
                .put("annotations", JSONObject().put("org.opencontainers.image.title", title))
        val path = "/v2/$repository/blobs/$digest"
        val target = cdn
        if (target == null) {
            server.serve(path, served)
        } else {
            target.serve("/blob/$title", served)
            server.redirect(path, target.baseUrl + "/blob/$title")
        }
    }

    /** Publishes the manifest of everything added so far. */
    fun publish() {
        val manifest =
            JSONObject()
                .put("schemaVersion", 2)
                .put("mediaType", "application/vnd.oci.image.manifest.v1+json")
                .put("layers", JSONArray(layers.map { it }))
        server.serve("/v2/$repository/manifests/$tag", manifest.toString().toByteArray())
    }

    fun stop() {
        server.stop()
        cdn?.stop()
    }
}
