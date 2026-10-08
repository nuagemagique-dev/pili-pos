// :pos-core is the Android library around the frozen engine facade (android/rust/pos-ffi).
//   - Main sources: the android-flavour UniFFI bindings, generated in library mode from the
//     UNSTRIPPED arm64 cargo output (never from a copy inside an APK, DECISIONS 11.6) with
//     android/rust/android-uniffi.toml (PD60, PD77). Generated code is never committed.
//   - Native code: the arm64-v8a libpos_ffi.so built by cargo-ndk, packaged into the AAR. AGP strips the
//     packaged copy (android.ndkVersion), the unstripped one stays in target/android-rust.
//   - Host unit tests (debug only): the same bindings run against the host debug dylib with the plain
//     JNA jar (the generated cleaner code only touches android.* from API 34, so the JVM path works).
//     The mock FDM and printer processes come from the root workspace. mismatchTest loads a library
//     built with the abi-mismatch feature and must fail to load it.
import com.android.build.api.variant.HostTestBuilder
import javax.inject.Inject

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.binary.compatibility.validator)
    alias(libs.plugins.ktlint)
    alias(libs.plugins.detekt)
}

val repoRoot: File = rootDir.parentFile
val rustDir = File(repoRoot, "android/rust")
val rustTarget = File(repoRoot, "target/android-rust")
val mocksTarget = File(repoRoot, "target/android-mocks")
val mismatchTarget = File(repoRoot, "target/android-rust-mismatch")
val pinnedNdk = "27.1.12297006"

val isMac = System.getProperty("os.name").lowercase().contains("mac")
val dylibName = if (isMac) "libpos_ffi.dylib" else "libpos_ffi.so"
val hostLibDir = File(rustTarget, "debug")
val hostLib = File(hostLibDir, dylibName)
val mismatchLibDir = File(mismatchTarget, "debug")
val fdmMockBin = File(mocksTarget, "debug/pos-fdm-mock")
val printerMockBin = File(mocksTarget, "debug/pos-printer-mock")

// The arm64 build: cargo-ndk copies the unstripped library into <jniLibs>/arm64-v8a, the cargo output
// stays in target/android-rust and is the input of the bindgen.
val unstrippedSo = File(rustTarget, "aarch64-linux-android/android-release/libpos_ffi.so")
val cargoNdkJniLibs = layout.buildDirectory.dir("generated/cargo-ndk/jniLibs")
val uniffiAndroidDir = layout.buildDirectory.dir("generated/uniffi-android")

// DECISIONS 11.4: rustup's cargo must come first on PATH (Homebrew's has no Android std).
// RUSTFLAGS, CARGO_ENCODED_RUSTFLAGS, SOURCE_DATE_EPOCH and CFLAGS_* are never set here; whatever the
// caller exports reaches cargo unchanged (a later container build uses them).
fun Exec.cargoEnv(workDir: File) {
    workingDir = workDir
    environment("PATH", "${System.getProperty("user.home")}/.cargo/bin:${System.getenv("PATH")}")
    environment.remove("CARGO_TARGET_DIR")
}

// Everything cargo can read: the Android workspace, the engine crates it links, and the root manifests.
val rustInputs =
    fileTree(repoRoot) {
        include("android/rust/**/*.rs", "android/rust/**/Cargo.toml", "android/rust/Cargo.lock", "android/rust/**/*.toml")
        include("crates/**/*.rs", "crates/**/Cargo.toml", "crates/**/*.sql", "crates/**/*.crt")
        include("Cargo.toml", "Cargo.lock", "rust-toolchain.toml")
        exclude("**/target/**", "**/build/**", "**/.gradle/**")
    }

// Environment values the caller may set; a change must rerun cargo.
val passThroughEnv = listOf("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "SOURCE_DATE_EPOCH")

// A directory copy whose output AGP wires as a generated source of one variant. The shared producer
// (cargoNdkBuild, uniffiBindgen) runs once; each variant gets its own copy so every output has a
// single producing task.
abstract class VariantCopy
    @Inject
    constructor(
        private val fs: FileSystemOperations,
    ) : DefaultTask() {
        @get:InputDirectory
        @get:PathSensitive(PathSensitivity.RELATIVE)
        abstract val source: DirectoryProperty

        @get:OutputDirectory
        abstract val target: DirectoryProperty

        @TaskAction
        fun copy() {
            fs.sync {
                from(source)
                into(target)
            }
        }
    }

// A single-file copy that AGP wires as a generated assets directory of the androidTest variant: the seed
// SQL of the instrumented tests comes from src-tauri/seed_spf_test.sql (read only) at build time and is
// never committed under src/androidTest.
abstract class SeedAssetCopy
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

// Host (macOS dylib or Linux so) build for the unit tests only; assemble tasks never depend on it.
val cargoBuildHost =
    tasks.register<Exec>("cargoBuildHost") {
        description = "Builds the host (macOS dylib or Linux so) of pos-ffi for the unit tests."
        cargoEnv(rustDir)
        inputs.files(rustInputs)
        outputs.file(hostLib)
        commandLine("cargo", "build", "--locked", "-p", "pos-ffi")
    }

val cargoNdkBuild =
    tasks.register<Exec>("cargoNdkBuild") {
        description = "Builds libpos_ffi.so for arm64-v8a (API 33) with cargo-ndk."
        cargoEnv(rustDir)
        inputs.files(rustInputs)
        passThroughEnv.forEach { inputs.property(it, providers.environmentVariable(it).orElse("")) }
        outputs.dir(cargoNdkJniLibs)
        outputs.file(unstrippedSo)
        val ndkDir = androidComponents.sdkComponents.ndkDirectory
        doFirst {
            environment("ANDROID_NDK_HOME", ndkDir.get().asFile.absolutePath)
            cargoNdkJniLibs.get().asFile.deleteRecursively()
        }
        commandLine(
            "cargo", "ndk", "-t", "arm64-v8a", "--platform", "33", "-o", cargoNdkJniLibs.get().asFile.absolutePath,
            "build", "--locked", "--profile", "android-release", "-p", "pos-ffi",
        )
    }

val uniffiBindgen =
    tasks.register<Exec>("uniffiBindgen") {
        description = "Generates the android-flavour Kotlin bindings from the unstripped arm64 library."
        cargoEnv(rustDir)
        dependsOn(cargoNdkBuild)
        inputs.file(unstrippedSo)
        inputs.file(File(rustDir, "android-uniffi.toml"))
        inputs.file(File(rustDir, "Cargo.lock"))
        outputs.dir(uniffiAndroidDir)
        doFirst { uniffiAndroidDir.get().asFile.deleteRecursively() }
        commandLine(
            "cargo", "run", "--locked", "-p", "uniffi-bindgen", "--",
            "generate", "--library", unstrippedSo.absolutePath, "--language", "kotlin",
            "--config", File(rustDir, "android-uniffi.toml").absolutePath,
            "--out-dir", uniffiAndroidDir.get().asFile.absolutePath, "--no-format",
        )
    }

val cargoBuildMismatch =
    tasks.register<Exec>("cargoBuildMismatch") {
        description = "Builds pos-ffi with the abi-mismatch feature (changed pos_sw_version signature)."
        cargoEnv(rustDir)
        inputs.files(rustInputs)
        outputs.file(File(mismatchLibDir, dylibName))
        commandLine(
            "cargo", "build", "--locked", "-p", "pos-ffi", "--features", "abi-mismatch",
            "--target-dir", mismatchTarget.absolutePath,
        )
    }

val cargoBuildMocks =
    tasks.register<Exec>("cargoBuildMocks") {
        description = "Builds the FDM and printer mocks from the root workspace (root Cargo.lock untouched)."
        cargoEnv(repoRoot)
        inputs.files(rustInputs)
        outputs.files(fdmMockBin, printerMockBin)
        commandLine(
            "cargo", "build", "--locked", "--manifest-path", File(repoRoot, "Cargo.toml").absolutePath,
            "-p", "pos-fdm-mock", "-p", "pos-printer-mock", "--target-dir", mocksTarget.absolutePath,
        )
    }

android {
    namespace = "dev.nuagemagique.pilipos.core"
    compileSdk = 36
    ndkVersion = pinnedNdk

    defaultConfig {
        minSdk = 33
        ndk { abiFilters += "arm64-v8a" }
        consumerProguardFiles("consumer-rules.pro")
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        // 16 KB pages: libraries stay page-aligned inside the APK instead of being extracted.
        jniLibs { useLegacyPackaging = false }
    }

    testOptions {
        animationsDisabled = true
    }

    lint {
        abortOnError = true
        warningsAsErrors = false
        checkReleaseBuilds = true
        lintConfig = file("lint.xml")
        disable += setOf("GradleDependency", "AndroidGradlePluginVersion", "NewerVersionAvailable")
    }
}

kotlin {
    jvmToolchain(17)
    compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17) }
}

androidComponents {
    // Host unit tests run on the debug variant only (the release variant would repeat them).
    beforeVariants(selector().withBuildType("release")) { it.hostTests[HostTestBuilder.UNIT_TEST_TYPE]?.enable = false }

    onVariants { variant ->
        val cap = variant.name.replaceFirstChar { it.uppercase() }
        val jniLibsCopy =
            tasks.register<VariantCopy>("copyJniLibs$cap") {
                dependsOn(cargoNdkBuild)
                source.set(cargoNdkJniLibs)
            }
        variant.sources.jniLibs?.addGeneratedSourceDirectory(jniLibsCopy, VariantCopy::target)
        val bindingsCopy =
            tasks.register<VariantCopy>("copyUniffiBindings$cap") {
                dependsOn(uniffiBindgen)
                source.set(uniffiAndroidDir)
            }
        variant.sources.java?.addGeneratedSourceDirectory(bindingsCopy, VariantCopy::target)
    }
}

androidComponents {
    // The instrumented tests (debug only) read the seed SQL as an asset of the test APK.
    onVariants(selector().withBuildType("debug")) { variant ->
        val testAssets =
            tasks.register<SeedAssetCopy>("copyAndroidTestSeedAssets") {
                seed.set(File(repoRoot, "src-tauri/seed_spf_test.sql"))
            }
        variant.androidTest?.sources?.assets?.addGeneratedSourceDirectory(testAssets, SeedAssetCopy::target)
    }
}

dependencies {
    // AAR on the device: it carries libjnidispatch.so per ABI. The unit tests add the plain jar below.
    implementation("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
    implementation(libs.kotlinx.coroutines.core)
    // The generated bindings use @RequiresApi (android_cleaner).
    implementation(libs.androidx.annotation)

    // Plain jar: it carries libjnidispatch for macOS and Linux (host unit tests only).
    testImplementation(libs.jna)
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)

    // Instrumented tests (connectedDebugAndroidTest, scripts/android/run_android_tests.sh).
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.rules)
    androidTestImplementation(libs.androidx.test.core)
    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.junit)
    androidTestImplementation(libs.kotlinx.coroutines.test)
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

// The UniFFI bindings are generated into build/ and are never scanned.
tasks.withType<io.gitlab.arturbosch.detekt.Detekt>().configureEach { exclude("**/build/**") }

val mismatchTestClass = "dev.nuagemagique.pilipos.core.ChecksumMismatchTest"

fun Test.commonTestConfig() {
    useJUnit()
    systemProperty("jna.debug_load", "false")
    // These tests start processes and load native libraries: never restore their result from a cache.
    outputs.cacheIf { false }
    testLogging {
        events("passed", "failed", "skipped")
        showStandardStreams = true
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}

tasks.withType<Test>().matching { it.name == "testDebugUnitTest" }.configureEach {
    commonTestConfig()
    dependsOn(cargoBuildHost, cargoBuildMocks)
    inputs.files(cargoBuildHost, cargoBuildMocks)
    systemProperty("jna.library.path", hostLibDir.absolutePath)
    systemProperty("pili.fdmMock", fdmMockBin.absolutePath)
    systemProperty("pili.printerMock", printerMockBin.absolutePath)
    systemProperty("pili.repoRoot", repoRoot.absolutePath)
    exclude("**/ChecksumMismatchTest*")
}

// The bindings come from the normal build, the library does not match: loading must fail.
val mismatchTest =
    tasks.register<Test>("mismatchTest") {
        description = "Loads the abi-mismatch library with the normal bindings; the load must fail."
        group = "verification"
        commonTestConfig()
        val unit = tasks.named<Test>("testDebugUnitTest")
        dependsOn(cargoBuildMismatch, "compileDebugUnitTestKotlin")
        inputs.files(cargoBuildMismatch)
        testClassesDirs = files(unit.map { it.testClassesDirs })
        classpath = files(unit.map { it.classpath })
        systemProperty("jna.library.path", mismatchLibDir.absolutePath)
        filter { includeTestsMatching(mismatchTestClass) }
    }

tasks.named("check") { dependsOn(mismatchTest) }
