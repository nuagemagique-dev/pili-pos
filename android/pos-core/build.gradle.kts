// :pos-core in P2: a Kotlin/JVM library whose main sources are the UniFFI bindings generated
// from the host build of pos-ffi (PD60). P3.1 turns it into an Android library and keeps these
// tests as host unit tests. Generated code is never committed.
plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.binary.compatibility.validator)
}

val repoRoot: File = rootDir.parentFile
val rustDir = File(repoRoot, "android/rust")
val rustTarget = File(repoRoot, "target/android-rust")
val mocksTarget = File(repoRoot, "target/android-mocks")
val mismatchTarget = File(repoRoot, "target/android-rust-mismatch")

val isMac = System.getProperty("os.name").lowercase().contains("mac")
val dylibName = if (isMac) "libpos_ffi.dylib" else "libpos_ffi.so"
val hostLibDir = File(rustTarget, "debug")
val hostLib = File(hostLibDir, dylibName)
val mismatchLibDir = File(mismatchTarget, "debug")
val fdmMockBin = File(mocksTarget, "debug/pos-fdm-mock")
val printerMockBin = File(mocksTarget, "debug/pos-printer-mock")
val generatedDir = layout.buildDirectory.dir("generated/uniffi")

// DECISIONS 11.4: rustup's cargo must come first on PATH (Homebrew's has no Android std).
fun Exec.cargoEnv(workDir: File) {
    workingDir = workDir
    environment("PATH", "${System.getProperty("user.home")}/.cargo/bin:${System.getenv("PATH")}")
    environment.remove("CARGO_TARGET_DIR")
}

val rustInputs = fileTree(repoRoot) {
    include("android/rust/**/*.rs", "android/rust/**/Cargo.toml", "android/rust/Cargo.lock")
    include("android/rust/**/*.toml", "crates/**/*.rs", "crates/**/Cargo.toml", "Cargo.lock")
    exclude("**/target/**", "**/build/**", "**/.gradle/**")
}

val cargoBuildHost = tasks.register<Exec>("cargoBuildHost") {
    description = "Builds the host (macOS dylib or Linux so) of pos-ffi."
    cargoEnv(rustDir)
    inputs.files(rustInputs)
    outputs.file(hostLib)
    commandLine("cargo", "build", "--locked", "-p", "pos-ffi")
}

val uniffiBindgen = tasks.register<Exec>("uniffiBindgen") {
    description = "Generates the Kotlin bindings from the host library (library mode)."
    cargoEnv(rustDir)
    dependsOn(cargoBuildHost)
    inputs.file(hostLib)
    inputs.file(File(rustDir, "jvm-uniffi.toml"))
    outputs.dir(generatedDir)
    doFirst { generatedDir.get().asFile.deleteRecursively() }
    commandLine(
        "cargo", "run", "--locked", "-p", "uniffi-bindgen", "--",
        "generate", "--library", hostLib.absolutePath, "--language", "kotlin",
        "--config", File(rustDir, "jvm-uniffi.toml").absolutePath,
        "--out-dir", generatedDir.get().asFile.absolutePath, "--no-format",
    )
}

val cargoBuildMismatch = tasks.register<Exec>("cargoBuildMismatch") {
    description = "Builds pos-ffi with the abi-mismatch feature (changed pos_sw_version signature)."
    cargoEnv(rustDir)
    inputs.files(rustInputs)
    outputs.file(File(mismatchLibDir, dylibName))
    commandLine(
        "cargo", "build", "--locked", "-p", "pos-ffi", "--features", "abi-mismatch",
        "--target-dir", mismatchTarget.absolutePath,
    )
}

val cargoBuildMocks = tasks.register<Exec>("cargoBuildMocks") {
    description = "Builds the FDM and printer mocks from the root workspace (root Cargo.lock untouched)."
    cargoEnv(repoRoot)
    inputs.files(rustInputs)
    outputs.files(fdmMockBin, printerMockBin)
    commandLine(
        "cargo", "build", "--locked", "--manifest-path", File(repoRoot, "Cargo.toml").absolutePath,
        "-p", "pos-fdm-mock", "-p", "pos-printer-mock", "--target-dir", mocksTarget.absolutePath,
    )
}

kotlin {
    jvmToolchain(17)
    sourceSets.named("main") { kotlin.srcDir(generatedDir) }
}

tasks.named("compileKotlin") { dependsOn(uniffiBindgen) }

dependencies {
    // Plain jar here (not the AAR): it carries libjnidispatch for macOS and Linux.
    implementation(libs.jna)
    implementation(libs.kotlinx.coroutines.core)
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
}

val mismatchTestClass = "dev.nuagemagique.pilipos.core.ChecksumMismatchTest"

fun Test.commonTestConfig() {
    useJUnit()
    systemProperty("jna.debug_load", "false")
    testLogging {
        events("passed", "failed", "skipped")
        showStandardStreams = true
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}

tasks.test {
    commonTestConfig()
    dependsOn(cargoBuildHost, cargoBuildMocks)
    systemProperty("jna.library.path", hostLibDir.absolutePath)
    systemProperty("pili.fdmMock", fdmMockBin.absolutePath)
    systemProperty("pili.printerMock", printerMockBin.absolutePath)
    systemProperty("pili.repoRoot", repoRoot.absolutePath)
    exclude("**/ChecksumMismatchTest*")
}

// The bindings come from the normal build, the library does not match: loading must fail.
val mismatchTest = tasks.register<Test>("mismatchTest") {
    description = "Loads the abi-mismatch library with the normal bindings; the load must fail."
    group = "verification"
    commonTestConfig()
    dependsOn(cargoBuildMismatch)
    val testSet = sourceSets.named("test").get()
    testClassesDirs = testSet.output.classesDirs
    classpath = testSet.runtimeClasspath
    systemProperty("jna.library.path", mismatchLibDir.absolutePath)
    filter { includeTestsMatching(mismatchTestClass) }
}

tasks.check { dependsOn(mismatchTest) }
