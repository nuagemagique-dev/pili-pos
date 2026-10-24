// :smoke is a never-shipped test app (PD80). It consumes :pos-core in a minified, unsigned release
// build and runs a scripted engine scenario on the emulator, which proves the consumer R8 rules of
// :pos-core: its own proguard-rules.pro adds no keep rule. The seed SQL of the scenario is copied
// from src-tauri/seed_spf_test.sql (read only) into a generated assets directory.
import javax.inject.Inject

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.ktlint)
    alias(libs.plugins.detekt)
}

val repoRoot: File = rootDir.parentFile

// A single-file copy that AGP wires as a generated assets directory of one variant.
abstract class SmokeAssetsCopy
    @Inject
    constructor(
        private val fs: FileSystemOperations,
    ) : DefaultTask() {
        @get:InputFile
        @get:PathSensitive(PathSensitivity.NONE)
        abstract val seed: RegularFileProperty

        @get:OutputDirectory
        abstract val target: DirectoryProperty

        @TaskAction
        fun copy() {
            fs.sync {
                from(seed)
                into(target)
            }
        }
    }

android {
    namespace = "dev.nuagemagique.pilipos.smoke"
    compileSdk = 36
    ndkVersion = "27.1.12297006"

    defaultConfig {
        applicationId = "dev.nuagemagique.pilipos.smoke"
        minSdk = 33
        targetSdk = 36
        versionCode = 1
        versionName = "0.0.0-smoke"
        ndk { abiFilters += "arm64-v8a" }
    }

    buildTypes {
        release {
            // The release APK stays unsigned (smoke-release-unsigned.apk): scripts/android/sign_dev.sh
            // signs a copy with the development key. No signing config, no key in the build.
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            vcsInfo.include = false
        }
    }

    // No Google dependency metadata blob in the APK or the bundle (reproducibility, S5).
    dependenciesInfo {
        includeInApk = false
        includeInBundle = false
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        // 16 KB pages: libraries stay page-aligned inside the APK instead of being extracted.
        jniLibs { useLegacyPackaging = false }
    }

    lint {
        abortOnError = true
        warningsAsErrors = false
        checkReleaseBuilds = true
        disable += setOf("GradleDependency", "AndroidGradlePluginVersion", "NewerVersionAvailable")
    }
}

kotlin {
    jvmToolchain(17)
    compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) }
}

androidComponents {
    onVariants { variant ->
        val cap = variant.name.replaceFirstChar { it.uppercase() }
        val assets =
            tasks.register<SmokeAssetsCopy>("copySmokeAssets$cap") {
                seed.set(File(repoRoot, "src-tauri/seed_spf_test.sql"))
            }
        variant.sources.assets?.addGeneratedSourceDirectory(assets, SmokeAssetsCopy::target)
    }
}

dependencies {
    implementation(project(":pos-core"))
    implementation(libs.kotlinx.coroutines.android)
}

ktlint {
    android.set(true)
    filter {
        exclude { it.file.absolutePath.contains("/build/") }
    }
}

detekt {
    buildUponDefaultConfig = true
    allRules = false
    config.setFrom(File(rootDir, "config/detekt/detekt.yml"))
}

tasks.withType<io.gitlab.arturbosch.detekt.Detekt>().configureEach { exclude("**/build/**") }
