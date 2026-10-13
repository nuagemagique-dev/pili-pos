package dev.nuagemagique.pilipos.platform.policy

/** A [DevicePolicies] that records every call and fails the ones named in [failing]. */
class FakeDevicePolicies(
    var owner: Boolean = true,
    var present: Set<String> = emptySet(),
    val failing: MutableSet<String> = mutableSetOf(),
    var refuseBooleans: Set<String> = emptySet(),
    /** The packages of [PolicySet.HIDDEN_PACKAGES] that are on the device; the others answer false. */
    val installed: MutableSet<String> = mutableSetOf(),
    val hidden: MutableSet<String> = mutableSetOf(),
) : DevicePolicies {
    /** Every call, as `name` or `name(arg)`, in the order it happened. */
    val calls = mutableListOf<String>()

    private fun call(
        name: String,
        arg: String? = null,
    ) {
        calls += if (arg == null) name else "$name($arg)"
        if (name in failing || (arg != null && "$name($arg)" in failing)) {
            throw SecurityException("refused $name")
        }
    }

    override fun isDeviceOwner(): Boolean = owner

    override fun addUserRestriction(key: String) = call("addUserRestriction", key)

    override fun clearUserRestriction(key: String) = call("clearUserRestriction", key)

    override fun getUserRestrictions(): Set<String> {
        call("getUserRestrictions")
        return present
    }

    override fun setUninstallBlocked(
        packageName: String,
        blocked: Boolean,
    ) = call("setUninstallBlocked", "$packageName,$blocked")

    override fun setAutoTimeEnabled(enabled: Boolean) = call("setAutoTimeEnabled", enabled.toString())

    override fun setAutoTimeZoneEnabled(enabled: Boolean) = call("setAutoTimeZoneEnabled", enabled.toString())

    override fun setTimeZone(timeZoneId: String): Boolean {
        call("setTimeZone", timeZoneId)
        return "setTimeZone" !in refuseBooleans
    }

    override fun setGlobalSetting(
        name: String,
        value: String,
    ) = call("setGlobalSetting", "$name,$value")

    override fun setKeyguardDisabled(disabled: Boolean): Boolean {
        call("setKeyguardDisabled", disabled.toString())
        return "setKeyguardDisabled" !in refuseBooleans
    }

    override fun setStatusBarDisabled(disabled: Boolean): Boolean {
        call("setStatusBarDisabled", disabled.toString())
        return "setStatusBarDisabled" !in refuseBooleans
    }

    override fun isApplicationHidden(packageName: String): Boolean {
        call("isApplicationHidden", packageName)
        return packageName in hidden
    }

    override fun setApplicationHidden(
        packageName: String,
        hidden: Boolean,
    ): Boolean {
        call("setApplicationHidden", "$packageName,$hidden")
        if (packageName !in installed) return false
        if (hidden) this.hidden += packageName else this.hidden -= packageName
        return true
    }

    override fun setSystemUpdatePolicyWindowed(
        startMinute: Int,
        endMinute: Int,
    ) = call("setSystemUpdatePolicyWindowed", "$startMinute,$endMinute")

    override fun setLockTaskFeatures(features: Int) = call("setLockTaskFeatures", features.toString())
}
