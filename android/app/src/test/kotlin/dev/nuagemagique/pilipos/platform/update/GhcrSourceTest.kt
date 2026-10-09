package dev.nuagemagique.pilipos.platform.update

import kotlinx.coroutines.test.runTest
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.io.File

@RunWith(RobolectricTestRunner::class)
class GhcrSourceTest {
    @get:Rule
    val tmp = TemporaryFolder()

    private val stoppers = mutableListOf<() -> Unit>()

    @After
    fun stopServers() = stoppers.forEach { it() }

    private fun registry(withCdn: Boolean = false): TestRegistry =
        TestRegistry().also { registry ->
            if (withCdn) registry.cdn = TestServer()
            stoppers += { registry.stop() }
        }

    private fun source(
        registry: TestRegistry,
        allowCleartext: Boolean = true,
        readTimeoutMs: Int = 5_000,
        baseUrl: String = registry.baseUrl,
    ) = GhcrSource(baseUrl = baseUrl, allowCleartext = allowCleartext, timeouts = HttpTimeouts(readMs = readTimeoutMs))

    private fun full(registry: TestRegistry) {
        registry.layer("declared.json", UpdateFixtures.json)
        registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
        registry.layer(UpdateFixtures.ASSET_NAME, UpdateFixtures.ASSET_BYTES)
        registry.publish()
    }

    private suspend fun refusal(block: suspend () -> Unit): UpdateException {
        try {
            block()
        } catch (e: UpdateException) {
            return e
        }
        error("no refusal")
    }

    @Test
    fun theTokenManifestAndBlobFlowDeliversTheDescriptor() =
        runTest {
            val registry = registry()
            full(registry)
            val files = source(registry).fetchDescriptor()
            assertArrayEquals(UpdateFixtures.json, files.jsonBytes)
            assertEquals(UpdateFixtures.minisig, files.minisigText)

            val seen = registry.server.seen
            val token = seen.first { it.path == "/token" }
            assertEquals("repository:brunomoyse/moyun-pos:pull", token.query?.substringAfter("scope=")?.substringBefore("&"))
            assertEquals("service=ghcr.io", token.query?.substringAfter("&"))
            assertNull(token.authorization)
            val manifest = seen.first { it.path == "/v2/brunomoyse/moyun-pos/manifests/declared-android" }
            assertEquals("application/vnd.oci.image.manifest.v1+json", manifest.accept)
            assertEquals("Bearer test-token", manifest.authorization)
            val blobs = seen.filter { it.path.contains("/blobs/sha256:") }
            assertEquals(2, blobs.size)
            assertTrue(blobs.all { it.authorization == "Bearer test-token" })
            assertEquals(1, seen.count { it.path == "/token" })
            assertEquals(1, seen.count { it.path.contains("/manifests/") })
        }

    @Test
    fun theTokenAndTheManifestAreReusedForTheAsset() =
        runTest {
            val registry = registry()
            full(registry)
            val source = source(registry)
            source.fetchDescriptor()
            val dest = File(tmp.root, "update/${UpdateFixtures.ASSET_NAME}")
            source.fetchAsset(UpdateFixtures.ASSET_NAME, dest)
            assertArrayEquals(UpdateFixtures.ASSET_BYTES, dest.readBytes())
            assertEquals(1, registry.server.seen.count { it.path == "/token" })
            assertEquals(1, registry.server.seen.count { it.path.contains("/manifests/") })
            assertFalse(File(dest.path + ".part").exists())
        }

    @Test
    fun aRedirectToAnotherOriginCarriesNoAuthorizationHeader() =
        runTest {
            val registry = registry(withCdn = true)
            full(registry)
            val dest = File(tmp.root, "asset.apk")
            source(registry).fetchAsset(UpdateFixtures.ASSET_NAME, dest)
            assertArrayEquals(UpdateFixtures.ASSET_BYTES, dest.readBytes())

            val onRegistry = registry.server.seen.filter { it.path.contains("/blobs/") }
            assertEquals("Bearer test-token", onRegistry.single().authorization)
            val onCdn = checkNotNull(registry.cdn).seen
            assertEquals(1, onCdn.size)
            assertNull(onCdn.single().authorization)
        }

    @Test
    fun aRedirectToAnotherPortOfTheSameHostIsAnotherOrigin() =
        runTest {
            val registry = registry(withCdn = true)
            full(registry)
            val cdnPort = checkNotNull(registry.cdn).baseUrl.substringAfterLast(':').toInt()
            val registryPort = registry.baseUrl.substringAfterLast(':').toInt()
            assertTrue(cdnPort != registryPort)
            source(registry).fetchDescriptor()
            assertTrue(checkNotNull(registry.cdn).seen.all { it.authorization == null })
        }

    @Test
    fun aDigestMismatchOnTheAssetIsSha256MismatchAndLeavesNoFile() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", UpdateFixtures.json)
            registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
            registry.layer(UpdateFixtures.ASSET_NAME, UpdateFixtures.ASSET_BYTES, served = "tampered payload\n".toByteArray())
            registry.publish()
            val dest = File(tmp.root, "update/asset.apk")
            val error = refusal { source(registry).fetchAsset(UpdateFixtures.ASSET_NAME, dest) }
            assertEquals(UpdateRefusalCode.SHA256_MISMATCH, error.code)
            assertFalse(dest.exists())
            assertFalse(File(dest.path + ".part").exists())
        }

    @Test
    fun aDigestMismatchOnTheDescriptorIsBadDescriptor() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", UpdateFixtures.json, served = UpdateFixtures.tamperedJson)
            registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
            registry.publish()
            val error = refusal { source(registry).fetchDescriptor() }
            assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, error.code)
        }

    @Test
    fun aLayerLargerThanTheLimitIsRefusedBeforeItsBlobIsRequested() =
        runTest {
            val registry = registry()
            full(registry)
            val error = refusal { source(registry).fetchAsset(UpdateFixtures.ASSET_NAME, File(tmp.root, "a.apk"), maxBytes = 10) }
            assertEquals(UpdateRefusalCode.TOO_LARGE, error.code)
            assertTrue(registry.server.seen.none { it.path.contains("/blobs/") })
        }

    @Test
    fun aBlobLongerThanItsManifestSaysIsCutAtTheLimit() =
        runTest {
            val registry = registry()
            val big = ByteArray(5_000) { 7 }
            registry.layer(UpdateFixtures.ASSET_NAME, big, announcedSize = 100)
            registry.publish()
            val dest = File(tmp.root, "a.apk")
            val error = refusal { source(registry).fetchAsset(UpdateFixtures.ASSET_NAME, dest, maxBytes = 1_000) }
            assertEquals(UpdateRefusalCode.TOO_LARGE, error.code)
            assertFalse(dest.exists())
            assertFalse(File(dest.path + ".part").exists())
        }

    @Test
    fun aDescriptorAbove64KiBIsTooLarge() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", ByteArray(DeclaredParser.MAX_BYTES + 1))
            registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
            registry.publish()
            assertEquals(UpdateRefusalCode.TOO_LARGE, refusal { source(registry).fetchDescriptor() }.code)
        }

    @Test
    fun clearTextToAHostThatIsNotTheLoopbackIsRefusedWithoutAConnection() =
        runTest {
            val registry = registry()
            val error = refusal { source(registry, allowCleartext = true, baseUrl = "http://registry.example.org").fetchDescriptor() }
            assertEquals(UpdateRefusalCode.NETWORK, error.code)
            assertTrue(error.detail, error.detail.contains("not allowed"))
            assertTrue(registry.server.seen.isEmpty())
        }

    @Test
    fun clearTextToTheLoopbackIsRefusedOutsideDebugBuilds() =
        runTest {
            val registry = registry()
            full(registry)
            val error = refusal { source(registry, allowCleartext = false).fetchDescriptor() }
            assertEquals(UpdateRefusalCode.NETWORK, error.code)
            assertTrue(registry.server.seen.isEmpty())
        }

    @Test
    fun aRedirectToClearTextOnAnotherHostIsRefused() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", UpdateFixtures.json)
            registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
            registry.publish()
            val digest = "sha256:" + sha256Hex(UpdateFixtures.json)
            registry.server.redirect("/v2/brunomoyse/moyun-pos/blobs/$digest", "http://evil.example.org/blob")
            val error = refusal { source(registry).fetchDescriptor() }
            assertEquals(UpdateRefusalCode.NETWORK, error.code)
        }

    @Test
    fun moreThanFiveRedirectsAreRefused() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", UpdateFixtures.json)
            registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
            registry.publish()
            val digest = "sha256:" + sha256Hex(UpdateFixtures.json)
            registry.server.redirect("/v2/brunomoyse/moyun-pos/blobs/$digest", "/loop1")
            (1..9).forEach { registry.server.redirect("/loop$it", "/loop${it + 1}") }
            val error = refusal { source(registry).fetchDescriptor() }
            assertEquals(UpdateRefusalCode.NETWORK, error.code)
            assertTrue(error.detail, error.detail.contains("redirects"))
        }

    @Test
    fun aRelativeRedirectStaysOnTheRegistryAndKeepsTheToken() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", UpdateFixtures.json)
            registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
            registry.publish()
            val digest = "sha256:" + sha256Hex(UpdateFixtures.json)
            registry.server.redirect("/v2/brunomoyse/moyun-pos/blobs/$digest", "/elsewhere", status = 302)
            registry.server.serve("/elsewhere", UpdateFixtures.json)
            assertArrayEquals(UpdateFixtures.json, source(registry).fetchDescriptor().jsonBytes)
            assertEquals("Bearer test-token", registry.server.seen.last { it.path == "/elsewhere" }.authorization)
        }

    @Test
    fun anHttpErrorIsNetwork() =
        runTest {
            val registry = registry()
            registry.server.serve("/v2/brunomoyse/moyun-pos/manifests/declared-android", ByteArray(0), status = 404)
            val error = refusal { source(registry).fetchDescriptor() }
            assertEquals(UpdateRefusalCode.NETWORK, error.code)
            assertTrue(error.detail, error.detail.contains("404"))
        }

    @Test
    fun aMissingTokenIsNetwork() =
        runTest {
            val registry = registry()
            registry.server.serve("/token", "{}".toByteArray())
            assertEquals(UpdateRefusalCode.NETWORK, refusal { source(registry).fetchDescriptor() }.code)
        }

    @Test
    fun anAccessTokenFieldIsAccepted() =
        runTest {
            val registry = registry()
            full(registry)
            registry.server.serve("/token", "{\"access_token\":\"other\"}".toByteArray())
            source(registry).fetchDescriptor()
            assertEquals("Bearer other", registry.server.seen.first { it.path.contains("/manifests/") }.authorization)
        }

    @Test
    fun aMissingLayerIsBadDescriptor() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", UpdateFixtures.json)
            registry.publish()
            assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal { source(registry).fetchDescriptor() }.code)
        }

    @Test
    fun twoLayersWithTheSameTitleAreRefused() =
        runTest {
            val registry = registry()
            registry.layer("declared.json", UpdateFixtures.json)
            registry.layer("declared.json", UpdateFixtures.tamperedJson)
            registry.layer("declared.json.minisig", UpdateFixtures.minisig.toByteArray())
            registry.publish()
            assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal { source(registry).fetchDescriptor() }.code)
        }

    @Test
    fun aManifestWithoutLayersOrNotJsonIsRefused() =
        runTest {
            val registry = registry()
            registry.server.serve("/v2/brunomoyse/moyun-pos/manifests/declared-android", "{}".toByteArray())
            assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal { source(registry).fetchDescriptor() }.code)
            registry.server.serve("/v2/brunomoyse/moyun-pos/manifests/declared-android", "<html>".toByteArray())
            assertEquals(UpdateRefusalCode.NETWORK, refusal { source(registry).fetchDescriptor() }.code)
        }

    @Test
    fun aDigestOfAnotherAlgorithmIsRefused() =
        runTest {
            val registry = registry()
            val title = "org.opencontainers.image.title"

            fun layer(name: String) = "{\"digest\":\"sha512:abc\",\"size\":3,\"annotations\":{\"$title\":\"$name\"}}"
            val manifest = "{\"layers\":[${layer("declared.json")},${layer("declared.json.minisig")}]}"
            registry.server.serve("/v2/brunomoyse/moyun-pos/manifests/declared-android", manifest.toByteArray())
            assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal { source(registry).fetchDescriptor() }.code)
        }

    @Test
    fun aSlowServerTimesOutAsNetwork() =
        runTest {
            val registry = registry()
            registry.server.route("/token") {
                Thread.sleep(1_500)
                TestServer.reply(it, 200, "{\"token\":\"t\"}".toByteArray())
            }
            val error = refusal { source(registry, readTimeoutMs = 200).fetchDescriptor() }
            assertEquals(UpdateRefusalCode.NETWORK, error.code)
        }

    @Test
    fun anUnreachableServerIsNetwork() =
        runTest {
            val registry = registry()
            val base = registry.baseUrl
            registry.stop()
            assertEquals(UpdateRefusalCode.NETWORK, refusal { source(registry, baseUrl = base).fetchDescriptor() }.code)
        }

    @Test
    fun theDefaultsPointAtThePublicPackage() {
        assertEquals("https://ghcr.io", GhcrSource.DEFAULT_BASE_URL)
        assertEquals("brunomoyse/moyun-pos", GhcrSource.DEFAULT_REPOSITORY)
        assertEquals("declared-android", GhcrSource.DEFAULT_TAG)
        assertEquals("ghcr", GhcrSource().label)
    }

    private fun writeOverride(
        dir: File,
        content: String,
    ) {
        File(dir, "platform").mkdirs()
        File(dir, GhcrSource.OVERRIDE_PATH).writeText(content)
    }

    @Test
    fun theOverrideIsReadOnlyForALoopbackOrTlsUrl() {
        val dir = tmp.newFolder()
        assertNull(GhcrSource.overrideBaseUrl(dir))
        writeOverride(dir, "{\"baseUrl\":\"http://127.0.0.1:5000\"}")
        assertEquals("http://127.0.0.1:5000", GhcrSource.overrideBaseUrl(dir))
        writeOverride(dir, "{\"baseUrl\":\"http://localhost:5000\"}")
        assertEquals("http://localhost:5000", GhcrSource.overrideBaseUrl(dir))
        writeOverride(dir, "{\"baseUrl\":\"https://mirror.example.org\"}")
        assertEquals("https://mirror.example.org", GhcrSource.overrideBaseUrl(dir))
        writeOverride(dir, "{\"baseUrl\":\"http://mirror.example.org\"}")
        assertNull(GhcrSource.overrideBaseUrl(dir))
        writeOverride(dir, "{\"baseUrl\":\"ftp://127.0.0.1\"}")
        assertNull(GhcrSource.overrideBaseUrl(dir))
        writeOverride(dir, "{}")
        assertNull(GhcrSource.overrideBaseUrl(dir))
        writeOverride(dir, "not json")
        assertNull(GhcrSource.overrideBaseUrl(dir))
    }

    @Test
    fun aReleaseBuildIgnoresTheOverrideAndAReleaseBuildIsTlsOnly() =
        runTest {
            val dir = tmp.newFolder()
            writeOverride(dir, "{\"baseUrl\":\"http://127.0.0.1:5000\"}")
            val registry = registry()
            full(registry)
            // Release: the override is not read and clear text is refused whatever the host is.
            val release = GhcrSource.forApp(dir, debug = false)
            val error = refusal { GhcrSource(registry.baseUrl, allowCleartext = false).fetchDescriptor() }
            assertEquals(UpdateRefusalCode.NETWORK, error.code)
            assertEquals("ghcr", release.label)
            // Debug: the override is the base URL and clear text to the loopback is allowed.
            val debug = GhcrSource.forApp(dir.also { writeOverride(it, "{\"baseUrl\":\"${registry.baseUrl}\"}") }, debug = true)
            assertArrayEquals(UpdateFixtures.json, debug.fetchDescriptor().jsonBytes)
        }
}
