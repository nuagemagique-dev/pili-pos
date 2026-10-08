package dev.nuagemagique.pilipos.nav

import javax.inject.Inject
import javax.inject.Singleton

/**
 * The validated set of everything the features contribute to the shell.
 *
 * Construction fails with an [IllegalStateException] naming the problem when the contributions do not fit
 * together: a route registered by two entries, the start route registered by none or by several, two
 * settings rows with the same id, or a settings row pointing to a route no entry registers.
 */
@Singleton
class NavRegistry
    @Inject
    constructor(
        entries: Set<@JvmSuppressWildcards FeatureEntry>,
        settings: Set<@JvmSuppressWildcards SettingsEntry>,
        overlays: Set<@JvmSuppressWildcards ShellOverlay>,
    ) {
        /** Feature entries sorted by area. */
        val entries: List<FeatureEntry> = entries.sortedBy { it.area }

        /** Settings rows sorted by order, then id. */
        val settingsEntries: List<SettingsEntry> = settings.sortedWith(compareBy({ it.order }, { it.id }))

        /** Overlays sorted by order. */
        val overlays: List<ShellOverlay> = overlays.sortedBy { it.order }

        init {
            validateStart()
            validateRoutes()
            validateSettings()
        }

        private fun validateStart() {
            val starters = entries.filter { NavContract.START_ROUTE in it.routes }
            check(starters.isNotEmpty()) {
                "No entry registers the start route \"${NavContract.START_ROUTE}\""
            }
            check(starters.size == 1) {
                "The start route \"${NavContract.START_ROUTE}\" is registered by more than one entry: " +
                    starters.joinToString { it.area }
            }
        }

        private fun validateRoutes() {
            val owners = mutableMapOf<String, String>()
            for (entry in entries) {
                for (route in entry.routes) {
                    val previous = owners.put(route, entry.area)
                    check(previous == null) {
                        "The route \"$route\" is registered by two entries: \"$previous\" and \"${entry.area}\""
                    }
                }
            }
        }

        private fun validateSettings() {
            val routes = entries.flatMap { it.routes }.toSet()
            val ids = mutableSetOf<String>()
            for (row in settingsEntries) {
                check(ids.add(row.id)) { "Two settings entries share the id \"${row.id}\"" }
                check(row.route in routes) {
                    "The settings entry \"${row.id}\" points to \"${row.route}\", which no entry registers"
                }
            }
        }
    }
