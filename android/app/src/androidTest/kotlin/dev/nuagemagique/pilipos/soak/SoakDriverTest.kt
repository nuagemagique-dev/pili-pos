package dev.nuagemagique.pilipos.soak

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Test
import org.junit.runner.RunWith

/**
 * P7-T5 (P7D12): the soak driver. One test that loops compressed service days against the real application and the
 * host mocks until `hours` have elapsed or `<externalFilesDir>/soak/stop` exists. `scripts/android/p7_soak.sh` starts
 * it with `am instrument` (without `-w`, so that it outlives an adb disconnect) and its sampler reads the result
 * lines (`SOAK_START`, `SOAK_BEAT`, `SOAK_SALE`, `SOAK_X`, `SOAK_Z`, `SOAK_LOCK`, tag `PiliSoak`) and
 * `<externalFilesDir>/soak/driver.jsonl`.
 *
 * Instrumentation arguments: `hours` (required), `dayMinutes` (60), `salesPerDay` (100), `managerInsz` / `managerPin`
 * (the seeded manager), `seed`, `idleGapMinutes` (6), `splitCardCents` (100).
 *
 * The driver forwards commands to the engine through the gateway and holds no fiscal rule and no amount arithmetic.
 */
@RunWith(AndroidJUnit4::class)
class SoakDriverTest {
    @Test
    fun soak() {
        SoakRunner(SoakEngine()).run()
    }
}
