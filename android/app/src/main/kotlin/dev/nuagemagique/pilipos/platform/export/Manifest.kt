package dev.nuagemagique.pilipos.platform.export

/** The `MANIFEST.sha256` file: `sha256sum` format, so `shasum -a 256 -c MANIFEST.sha256` checks an export. */
object Manifest {
    /** Name of the file; it is written last and does not list itself. */
    const val NAME = "MANIFEST.sha256"

    /** One `<sha256>  <relative path>` line per entry (two spaces), sorted by path, each ended by a newline. */
    fun render(hashes: Map<String, String>): String =
        hashes.entries
            .sortedBy { it.key }
            .joinToString(separator = "") { "${it.value}  ${it.key}\n" }
}
