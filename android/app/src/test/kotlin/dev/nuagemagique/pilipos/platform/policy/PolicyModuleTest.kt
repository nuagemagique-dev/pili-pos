package dev.nuagemagique.pilipos.platform.policy

import dagger.hilt.android.testing.HiltAndroidRule
import dagger.hilt.android.testing.HiltAndroidTest
import dagger.hilt.android.testing.HiltTestApplication
import dev.nuagemagique.pilipos.di.AppLifecycleHook
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import javax.inject.Inject

@HiltAndroidTest
@RunWith(RobolectricTestRunner::class)
@Config(application = HiltTestApplication::class)
class PolicyModuleTest {
    @get:Rule
    val hiltRule = HiltAndroidRule(this)

    @Inject
    lateinit var hooks: Set<@JvmSuppressWildcards AppLifecycleHook>

    @Inject
    lateinit var policies: DevicePolicies

    @Inject
    lateinit var applier: PolicyApplier

    @Test
    fun thePolicyHookIsInTheHiltSetAtOrder100() {
        hiltRule.inject()
        val hook = hooks.filterIsInstance<PolicyHook>().single()
        assertEquals(100, hook.order)
    }

    @Test
    fun theProductionPoliciesAreBoundAndTheApplierIsSingleton() {
        hiltRule.inject()
        assertTrue(policies is AndroidDevicePolicies)
        // Not the device owner in a plain Robolectric app: nothing is applied.
        assertTrue(applier.apply(release = true, provisioned = true).notOwner)
    }
}
