package dev.nuagemagique.pilipos.ui.components

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * C13 / R6 guard: the till never raises the system keyboard. No source of the app may draw a text field of Compose or an
 * `EditText`, bar the one declared exception (the Chinese name of a menu item, which the in-app keyboards cannot type):
 * a field is a `KeypadField` that opens an in-app keypad.
 */
class NoSystemKeyboardTest {
    private val root = File("src/main/kotlin")

    private val forbidden = Regex("""\b(BasicTextField|OutlinedTextField|TextField|BasicSecureTextField|EditText)\s*\(""")

    /** The file that keeps a text field, and how many. */
    private val allowed = mapOf("feature/settings/SettingsParts.kt" to 1)

    @Test
    fun theSourcesAreFound() {
        assertTrue("run from the app module: ${root.absolutePath}", root.isDirectory)
    }

    @Test
    fun onlyTheDeclaredExceptionDrawsATextField() {
        val found = mutableMapOf<String, Int>()
        root.walkTopDown().filter { it.isFile && it.extension == "kt" }.forEach { file ->
            val code = file.readLines().filterNot { it.trimStart().startsWith("*") || it.trimStart().startsWith("//") }
            val count = code.count { forbidden.containsMatchIn(it) }
            if (count > 0) found[file.relativeTo(File(root, "dev/nuagemagique/pilipos")).path] = count
        }
        assertEquals(allowed, found)
    }

    @Test
    fun theSystemKeyboardIsNeverAskedFor() {
        val offenders = mutableListOf<String>()
        root.walkTopDown().filter { it.isFile && it.extension == "kt" }.forEach { file ->
            if (file.readText().contains("LocalSoftwareKeyboardController") || file.readText().contains("showSoftInput")) {
                offenders += file.name
            }
        }
        assertEquals(emptyList<String>(), offenders)
    }
}
