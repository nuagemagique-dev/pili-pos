package dev.nuagemagique.pilipos.platform.about

/** A report with every field filled, for the screen, view model and golden tests. */
fun sampleReport(
    freeInternalBytes: Long = 12L * 1000 * 1000 * 1000,
    backupsCount: Int = 41,
    backupsBytes: Long = 820L * 1000 * 1000,
    posId: String? = "POS-01",
    soSha1: String? = "3f786850e387550fdab836ed7e6dc881de23001b",
): IdentityReport =
    IdentityReport(
        applicationId = "dev.nuagemagique.pilipos",
        versionName = "1.0.0",
        versionCode = 10000,
        posSwVersion = "pili-pos-android/1.0.0",
        engineRef = "pili-pos 1.0.0, linux-v1.0.0-rc3 (pending)",
        apkSha1 = "a9993e364706816aba3e25717850c26c9cd0d89d",
        apkSha256 = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        soSha1 = soSha1,
        soProblem = if (soSha1 == null) "lib/arm64-v8a/libpos_ffi.so is not in the APK" else null,
        signerSha256 = listOf("cb8379ac2098aa165029e3938a51da0bcecfc008fd6795f401178647f96c5b34"),
        deviceId = "SN-0001",
        posId = posId,
        terminalId = posId?.let { "TERM-01" },
        androidRelease = "13",
        sdkInt = 33,
        fingerprint = "SUNMI/D3_PRO/D3_PRO:13/TKQ1/eng:user/release-keys",
        manufacturer = "SUNMI",
        model = "D3 PRO",
        freeInternalBytes = freeInternalBytes,
        backupsCount = backupsCount,
        backupsBytes = backupsBytes,
        generatedAt = "2026-10-09T10:00:00.000+02:00",
    )
