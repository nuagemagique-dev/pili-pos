// :app is the Pili POS Android application (Sunmi D3 Pro, API 33, arm64-v8a). P4-T2 lays the foundation:
// every dependency P4, P5 (UI) and P6 (platform) need, the full manifest and the stub classes the later
// cards fill, so the two lanes do not edit the same files. The engine is reached only through :pos-core.
// The release APK stays unsigned (app-release-unsigned.apk): scripts/android/sign_dev.sh signs a copy
// with a throwaway development key, the real release key is never in the build.
import com.android.build.api.variant.HostTestBuilder
import kotlinx.kover.gradle.plugin.dsl.CoverageUnit

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.ksp)
    alias(libs.plugins.hilt)
    alias(libs.plugins.ktlint)
    alias(libs.plugins.detekt)
    alias(libs.plugins.kover)
    alias(libs.plugins.roborazzi)
}

val repoRoot: File = rootDir.parentFile

// The version string has one source: the [package] version of the pos-ffi crate (DECISIONS section 2).
// MAJOR.MINOR.PATCH with an optional -dev suffix; versionCode = MAJOR * 10000 + MINOR * 100 + PATCH.
val ffiVersion: String =
    run {
        val manifest = File(repoRoot, "android/rust/pos-ffi/Cargo.toml")
        val text = manifest.readText()
        val packageSection = Regex("""(?ms)^\[package]\s*$(.*?)(?=^\[|\z)""").find(text)?.groupValues?.get(1).orEmpty()
        Regex("""(?m)^version\s*=\s*"([^"]*)"""").find(packageSection)?.groupValues?.get(1)
            ?: throw GradleException("No [package] version found in ${manifest.path}")
    }
val versionParts: List<Int> =
    Regex("""(\d+)\.(\d+)\.(\d+)(-dev)?""").matchEntire(ffiVersion)?.groupValues?.subList(1, 4)?.map { it.toInt() }
        ?: throw GradleException(
            "The pos-ffi version \"$ffiVersion\" must be MAJOR.MINOR.PATCH with an optional -dev suffix (android/rust/pos-ffi/Cargo.toml)",
        )
if (versionParts[1] >= 100 || versionParts[2] >= 100) {
    throw GradleException(
        "The pos-ffi version \"$ffiVersion\": MINOR and PATCH must be below 100 (versionCode scheme, DECISIONS section 2)",
    )
}
val computedVersionCode: Int = versionParts[0] * 10000 + versionParts[1] * 100 + versionParts[2]

// PD108: the provisioning public key (minisign) is a build-time constant. Debug uses the development key
// kept in the repository when present; release takes it from the property piliProvisioningPublicKey.
val devProvisioningKey: String =
    file("provisioning/dev-minisign.pub").takeIf { it.isFile }?.readText()?.trim().orEmpty()
val releaseProvisioningKey: String = (findProperty("piliProvisioningPublicKey") as String?)?.trim().orEmpty()

fun javaStringLiteral(value: String): String =
    buildString {
        append('"')
        for (c in value) {
            when {
                c == '\\' -> append("\\\\")
                c == '"' -> append("\\\"")
                c == '\n' -> append("\\n")
                c == '\r' -> append("\\r")
                c == '\t' -> append("\\t")
                c.code < 0x20 || c.code > 0x7e -> append("\\u%04x".format(c.code))
                else -> append(c)
            }
        }
        append('"')
    }

// Unit tests run on the debug variant only: the Compose test activity comes from ui-test-manifest, which
// stays out of the release APK, and the release variant differs only by R8 (and signing), which unit
// tests cannot see.
androidComponents {
    beforeVariants(selector().withBuildType("release")) { it.hostTests[HostTestBuilder.UNIT_TEST_TYPE]?.enable = false }
}

android {
    namespace = "dev.nuagemagique.pilipos"
    compileSdk = 36
    ndkVersion = "27.1.12297006"

    defaultConfig {
        applicationId = "dev.nuagemagique.pilipos"
        minSdk = 33
        targetSdk = 36
        versionName = ffiVersion
        versionCode = computedVersionCode
        ndk { abiFilters += "arm64-v8a" }
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
            buildConfigField("String", "PROVISIONING_PUBLIC_KEY", javaStringLiteral(devProvisioningKey))
            buildConfigField("boolean", "PROVISIONING_KEY_IS_DEV", "true")
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            buildConfigField("String", "PROVISIONING_PUBLIC_KEY", javaStringLiteral(releaseProvisioningKey))
            buildConfigField("boolean", "PROVISIONING_KEY_IS_DEV", "false")
            // No signing config: the APK stays app-release-unsigned.apk.
            vcsInfo.include = false
        }
    }

    // No Google dependency metadata blob in the APK or the bundle (reproducibility, S5).
    dependenciesInfo {
        includeInApk = false
        includeInBundle = false
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main").java.srcDirs("src/main/kotlin")
        getByName("test").java.srcDirs("src/test/kotlin")
        getByName("androidTest").java.srcDirs("src/androidTest/kotlin")
        getByName("debug").java.srcDirs("src/debug/kotlin")
    }

    packaging {
        // 16 KB pages: libraries stay page-aligned inside the APK instead of being extracted.
        jniLibs { useLegacyPackaging = false }
        resources.excludes +=
            setOf(
                "/META-INF/{AL2.0,LGPL2.1}",
                "/META-INF/DEPENDENCIES",
                "/META-INF/LICENSE*",
                "/META-INF/NOTICE*",
            )
    }

    testOptions {
        unitTests {
            // Robolectric reads the merged resources (strings, drawables).
            isIncludeAndroidResources = true
            isReturnDefaultValues = false
            all { test ->
                test.maxHeapSize = "2g"
                // Robolectric screens need real graphics for screenshots.
                test.systemProperty("robolectric.graphicsMode", "NATIVE")
                test.systemProperty("robolectric.pixelCopyRenderMode", "hardware")
                // The tests read the version from the pos-ffi manifest of this checkout.
                test.systemProperty("piliRepoRoot", rootDir.parentFile.absolutePath)
            }
        }
    }

    lint {
        abortOnError = true
        warningsAsErrors = false
        checkReleaseBuilds = true
        disable += setOf("MissingTranslation", "AndroidGradlePluginVersion", "GradleDependency")
    }
}

kotlin {
    jvmToolchain(17)
    compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) }
}

// Roborazzi goldens live next to the tests (record and verify run only on request).
roborazzi {
    outputDir.set(file("src/test/screenshots"))
}

dependencies {
    // The engine: only through the UniFFI facade packaged by :pos-core.
    implementation(project(":pos-core"))

    // Core
    implementation(libs.androidx.core)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.service)
    implementation(libs.androidx.lifecycle.process)
    implementation(libs.androidx.navigation.compose)
    implementation(libs.androidx.documentfile)

    // Compose
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.ui)
    implementation(libs.compose.foundation)
    implementation(libs.compose.material3)
    implementation(libs.compose.material.icons.extended)
    implementation(libs.compose.ui.tooling.preview)
    debugImplementation(libs.compose.ui.tooling)
    debugImplementation(libs.compose.ui.test.manifest)

    implementation(libs.kotlinx.coroutines.android)

    // DI
    implementation(libs.hilt.android)
    ksp(libs.hilt.compiler)
    implementation(libs.hilt.navigation.compose)

    implementation(libs.timber)

    // Host unit tests (Robolectric)
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.turbine)
    testImplementation(libs.mockk)
    testImplementation(libs.robolectric)
    testImplementation(libs.androidx.test.core)
    testImplementation(libs.androidx.test.ext.junit)
    testImplementation(platform(libs.compose.bom))
    testImplementation(libs.compose.ui.test.junit4)
    testImplementation(libs.hilt.android.testing)
    kspTest(libs.hilt.compiler)
    testImplementation(libs.roborazzi)
    testImplementation(libs.roborazzi.compose)
    testImplementation(libs.roborazzi.junit.rule)

    // Instrumented tests
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.rules)
    androidTestImplementation(libs.androidx.test.core)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(platform(libs.compose.bom))
    androidTestImplementation(libs.compose.ui.test.junit4)
    kspAndroidTest(libs.hilt.compiler)
}

ktlint {
    android.set(true)
    filter {
        exclude { it.file.absolutePath.contains("/generated/") || it.file.absolutePath.contains("/build/") }
    }
}

detekt {
    buildUponDefaultConfig = true
    allRules = false
    config.setFrom(File(rootDir, "config/detekt/detekt.yml"))
}

// Generated sources (Hilt, KSP, bindings) are never scanned.
tasks.withType<io.gitlab.arturbosch.detekt.Detekt>().configureEach { exclude("**/generated/**", "**/build/**") }

// Coverage (Kover): generated code and Compose previews are left out, every hand-written class counts.
kover {
    reports {
        filters {
            excludes {
                packages(
                    "dev.nuagemagique.pilipos.i18n.generated",
                    "hilt_aggregated_deps",
                    "dagger.hilt.internal.aggregatedroot.codegen",
                )
                classes(
                    "*.BuildConfig",
                    "*.R",
                    "*.R$*",
                    "*_Factory",
                    "*_Factory$*",
                    "*_MembersInjector",
                    "*_HiltModules*",
                    "*_GeneratedInjector",
                    "*Hilt_*",
                    "*.ComposableSingletons*",
                    "*_ComponentTreeDeps",
                    "*_Provide*Factory",
                    "*Module_*",
                )
                annotatedBy("androidx.compose.ui.tooling.preview.Preview")
            }
        }
        // The floor the gate holds (./gradlew :app:koverVerifyDebug), PD111.
        verify {
            rule("Coverage floor") {
                minBound(80)
                bound {
                    minValue.set(60)
                    coverageUnits.set(CoverageUnit.BRANCH)
                }
            }
        }
    }
}
