package dev.nuagemagique.pilipos.nav

/**
 * Constants every feature agrees on.
 *
 * Route strings are lowercase `<area>` or `<area>/<sub>`, with `{arg}` segments for arguments. Each feature
 * owns its route constants in its own `<Feature>Routes` object; this contract holds only what the shell
 * itself needs to know.
 */
object NavContract {
    /** The route shown first: the login screen. Exactly one [FeatureEntry] registers it. */
    const val START_ROUTE = "login"
}
