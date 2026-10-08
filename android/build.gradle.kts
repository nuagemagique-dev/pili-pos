buildscript {
    // A newer R8 than the one bundled with AGP 8.9.1, so release minification can read
    // Kotlin 2.2 metadata (DECISIONS section 5, copied from moyun-handheld).
    repositories { google() }
    dependencies { classpath(libs.r8) }
}

// Plugins are applied by the modules.
plugins {
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.binary.compatibility.validator) apply false
    alias(libs.plugins.ktlint) apply false
    alias(libs.plugins.detekt) apply false
}
