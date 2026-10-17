package dev.nuagemagique.pilipos.ui.state

import uniffi.pos_ffi.EftPayment

/** A payment of the terminal as the engine answers it; the card fields follow [status]. */
@Suppress("LongParameterList")
internal fun eftPayment(
    id: Long,
    status: String,
    amountCents: Long,
    orderId: Long = 7L,
    brand: String? = if (status == EFT_APPROVED) "Bancontact" else null,
    detail: String? = null,
    used: Boolean = false,
    mealVoucher: Boolean = false,
): EftPayment =
    EftPayment(
        id = id,
        orderId = orderId,
        amountCents = amountCents,
        status = status,
        cardType = if (status == EFT_APPROVED) "debit" else null,
        brand = brand,
        terminalReference = if (status == EFT_APPROVED) "T-$id" else null,
        authorisationCode = null,
        maskedPan = null,
        mealVoucher = mealVoucher,
        detail = detail,
        createdAt = "2026-10-12T10:00:00+02:00",
        used = used,
    )

/** The settings of a till with the Worldline terminal linked. */
internal fun linkedSettings(): uniffi.pos_ffi.PosSettings =
    uniffi.pos_ffi.PosSettings(
        trainingMode = false,
        printerAddress = null,
        fdmUrl = "http://fdm",
        posId = "POS1",
        terminalId = "T1",
        restaurantName = "Chez Pili",
        language = "fr",
        configProblems = emptyList(),
        posSwVersion = "pili-pos-android/1.2.3",
        paymentTerminal = "worldline_terminal_api",
    )
