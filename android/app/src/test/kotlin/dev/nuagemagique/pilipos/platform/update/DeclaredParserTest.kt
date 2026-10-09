package dev.nuagemagique.pilipos.platform.update

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class DeclaredParserTest {
    private fun refusal(bytes: ByteArray): UpdateException = assertThrows(UpdateException::class.java) { DeclaredParser.parse(bytes) }

    private fun assertBad(vararg overrides: Pair<String, Any?>) {
        assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal(descriptorJson(*overrides)).code)
    }

    @Test
    fun theFixtureParsesWithEveryField() {
        val release = DeclaredParser.parse(UpdateFixtures.json)
        assertEquals("dev.nuagemagique.pilipos", release.packageName)
        assertEquals("1.2.0", release.version)
        assertEquals(102L, release.versionCode)
        assertEquals(UpdateFixtures.ASSET_NAME, release.asset)
        assertEquals(40, release.sha1.length)
        assertEquals(UpdateFixtures.SIGNER, release.signerSha256)
        assertEquals("2026-10-09T10:00:00Z", release.declaredAt)
    }

    @Test
    fun theHashesOfTheFixtureAreThoseOfTheTestPayload() {
        val release = DeclaredParser.parse(UpdateFixtures.json)
        assertEquals(sha256Hex(UpdateFixtures.ASSET_BYTES), release.sha256)
        assertEquals(
            java.security.MessageDigest
                .getInstance("SHA-1")
                .digest(UpdateFixtures.ASSET_BYTES)
                .joinToString("") { "%02x".format(it) },
            release.sha1,
        )
    }

    @Test
    fun unknownExtraKeysAreIgnored() {
        assertEquals(102L, DeclaredParser.parse(descriptorJson("note" to "x")).versionCode)
    }

    @Test
    fun anUnknownFormatIsRefused() {
        assertBad("format" to 2)
        assertBad("format" to "1")
        assertBad("format" to null)
    }

    @Test
    fun aPlatformOtherThanAndroidIsRefused() {
        assertBad("platform" to "linux")
        assertBad("platform" to "Android")
        assertBad("platform" to null)
    }

    @Test
    fun everyMissingFieldIsRefused() {
        listOf("package", "version", "versionCode", "asset", "sha1", "sha256", "signerSha256", "declared_at").forEach {
            assertBad(it to null)
        }
    }

    @Test
    fun aMistypedOrEmptyFieldIsRefused() {
        assertBad("package" to "")
        assertBad("package" to 5)
        assertBad("declared_at" to " ")
        assertBad("versionCode" to "102")
        assertBad("versionCode" to 1.5)
        assertBad("versionCode" to 0)
        assertBad("versionCode" to -3)
    }

    @Test
    fun aVersionCodeAboveIntRangeIsRead() {
        assertEquals(5_000_000_000L, DeclaredParser.parse(descriptorJson("versionCode" to 5_000_000_000L)).versionCode)
    }

    @Test
    fun hashesMustBeLowercaseHexOfTheRightLength() {
        val sha1 = "8fedc9b579c8784e5c9f8414b70224edee2c5867"
        val sha256 = "b82bdbbdfb2cedb7bed64cc2798aefd454f3fae2d20ee1ed83fb2efe7efaa0ed"
        assertBad("sha1" to sha1.uppercase())
        assertBad("sha1" to sha1.dropLast(1))
        assertBad("sha1" to sha1.dropLast(1) + "g")
        assertBad("sha1" to sha256)
        assertBad("sha256" to sha256.uppercase())
        assertBad("sha256" to sha256 + "0")
        assertBad("sha256" to sha1)
        assertBad("signerSha256" to sha256.dropLast(2))
        assertBad("signerSha256" to "z".repeat(64))
    }

    @Test
    fun theAssetNameIsFixedInShape() {
        assertBad("asset" to "pili-pos-android-1.2.0.apk.exe")
        assertBad("asset" to "../pili-pos-android-1.2.0.apk")
        assertBad("asset" to "dir/pili-pos-android-1.2.0.apk")
        assertBad("asset" to "pili-pos-android-1.2.apk")
        assertBad("asset" to "pili-pos-android-1.2.0-dev.apk")
        assertBad("asset" to "other-1.2.0.apk")
        assertBad("asset" to "pili-pos-android-1.2.0.apk\n")
    }

    @Test
    fun theAssetMustBelongToTheVersion() {
        assertBad("asset" to "pili-pos-android-1.2.1.apk")
        assertBad("version" to "1.2.1")
        assertBad("version" to "1.2.0-dev")
        assertBad("version" to "1.2")
    }

    @Test
    fun anythingButAJsonObjectIsRefused() {
        assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal("[]".toByteArray()).code)
        assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal("not json".toByteArray()).code)
        assertEquals(UpdateRefusalCode.BAD_DESCRIPTOR, refusal(ByteArray(0)).code)
    }

    @Test
    fun theSizeLimitIs64KiB() {
        val padded = descriptorJson("pad" to "x".repeat(DeclaredParser.MAX_BYTES))
        assertEquals(UpdateRefusalCode.TOO_LARGE, refusal(padded).code)
        val exactly = ByteArray(DeclaredParser.MAX_BYTES + 1) { ' '.code.toByte() }
        assertEquals(UpdateRefusalCode.TOO_LARGE, refusal(exactly).code)
        assertEquals(65536, DeclaredParser.MAX_BYTES)
    }
}
