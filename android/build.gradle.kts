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
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.ksp) apply false
    alias(libs.plugins.hilt) apply false
    alias(libs.plugins.kover) apply false
    alias(libs.plugins.roborazzi) apply false
}

// S5 determinism: archives (AAR, JAR, APK inputs) carry no timestamps and a stable entry order.
allprojects {
    tasks.withType<AbstractArchiveTask>().configureEach {
        isPreserveFileTimestamps = false
        isReproducibleFileOrder = true
    }
}

// PD93: detekt also scans the instrumented tests of every module that applies it (the default source
// set only covers src/main/java, src/main/kotlin, src/test/* and src/debug).
subprojects {
    pluginManager.withPlugin("io.gitlab.arturbosch.detekt") {
        extensions.configure<io.gitlab.arturbosch.detekt.extensions.DetektExtension> {
            source.setFrom(
                files(
                    "src/main/kotlin",
                    "src/main/java",
                    "src/test/kotlin",
                    "src/test/java",
                    "src/debug/kotlin",
                    "src/androidTest/kotlin",
                    "src/androidTest/java",
                ),
            )
        }
    }
}
