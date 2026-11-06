use pos_fiscal::types::Mode;
use serde::{Deserialize, Serialize};

use pos_fiscal::fdm_client::{SignMutation, SignRequest};
use pos_fiscal::inputs::{DrawerInput, DrawerOpenInput, MoneyInOutInput, MoneyInOutLineInput};
use pos_fiscal::types::{EventLabel, InputMethod, MoneyInOutLineType, PaymentType};

use crate::commands::access::{self, Access};
use crate::commands::errors;
use crate::commands::signing::{self, ArchiveSpec, Begin, Kind, Op, PendingContext, Prepared};
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialResult {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoneyDirection {
    In,
    Out,
    /// The cash counted in the drawer (Basic Set M133): the amount is
    /// what is there, not a movement.
    Declaration,
}

/// Whether a cash movement also opens the drawer once it is signed.
///
/// On the wire (the Tauri command) it is the plain flag `openDrawer` of the
/// UI, read through `From<bool>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(from = "bool")]
pub enum DrawerAfter {
    /// Only the movement is recorded (the default of every other caller).
    Leave,
    /// The drawer opens in the same step, for the cash put in or taken out.
    Kick,
}

impl From<bool> for DrawerAfter {
    fn from(open: bool) -> Self {
        if open { Self::Kick } else { Self::Leave }
    }
}

/// Record a cash financial event (Event F): money put in or taken
/// out (cash float, withdrawal, ...), or the count of the drawer.
///
/// `DrawerAfter::Kick` kicks the drawer once, after the movement is signed
/// and archived, so the same tap records the movement and gives
/// access to the cash. It signs NO second event: the detailed
/// description (POS v2.2, `DRAWER_OPEN`, "Ouverture du tiroir-caisse en
/// dehors des ventes") and the producers' FAQ (Q405: `signDrawerOpen`
/// is used only when the drawer is opened without any other operation
/// recorded, and need not precede a `signMoneyInOut`) make the movement
/// itself the record of that opening; a `DRAWER_OPEN` beside it would
/// also inflate `drawersOpenCount` of the Z report ("sans
/// enregistrement de vente"). A count of the drawer (`Declaration`)
/// never kicks it, whatever the flag.
pub async fn do_money_in_out(
    state: &AppState,
    employee_insz: String,
    amount_cents: i64,
    direction: MoneyDirection,
    reason: String,
    drawer: DrawerAfter,
) -> Result<FinancialResult, String> {
    access::operator_named(state, &employee_insz, Access::Admin)?;
    let open_drawer =
        drawer == DrawerAfter::Kick && !matches!(direction, MoneyDirection::Declaration);
    // A retry of the same movement, answered late, is not made twice.
    // The flag is part of the identity only when set, so a movement
    // without it keeps the subject it always had.
    let drawer_mark = if open_drawer { "|drawer" } else { "" };
    let op = Op::new(
        Kind::Money,
        format!("{employee_insz}|{direction:?}|{amount_cents}|{reason}{drawer_mark}"),
    );
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        Ok(Begin::AlreadyDone(_)) => return Ok(FinancialResult::done()),
        Err(e) => return Ok(FinancialResult::failed(e)),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state, &guard)?;
    let label = EventLabel::F.in_mode(Mode::from(config.training_mode));
    let ticket_number = super::peek_ticket_number(state, &guard, label)?;

    let common = pos_fiscal::event::build_common_fields(
        &config,
        &state.identity,
        ticket_number,
        &employee_insz,
        &bp_id,
        // Drawer events are not printed.
        pos_fiscal::types::TicketMedium::None,
    );

    // A movement's direction is its sign: positive in, negative out.
    // A declaration is the counted amount, never negative.
    if amount_cents < 0 {
        return Err(errors::AMOUNT_NEGATIVE.into());
    }
    let (amount, amount_type) = match direction {
        MoneyDirection::In => (
            pos_core::money::eur(amount_cents),
            MoneyInOutLineType::MoneyInOut,
        ),
        MoneyDirection::Out => (
            -pos_core::money::eur(amount_cents),
            MoneyInOutLineType::MoneyInOut,
        ),
        MoneyDirection::Declaration => (
            pos_core::money::eur(amount_cents),
            MoneyInOutLineType::DrawerDeclaration,
        ),
    };

    let input = MoneyInOutInput {
        common,
        drawer: None,
        financials: vec![MoneyInOutLineInput {
            id: "MOV1".to_string(),
            name: reason.clone(),
            payment_type: PaymentType::Cash,
            provider: None,
            input_method: InputMethod::Manual,
            amount,
            amount_type,
            foreign_currency: None,
            reference: None,
            drawer: None,
        }],
    };

    let request = SignRequest::new(
        SignMutation::MoneyInOut,
        &input,
        Mode::from(config.training_mode),
    )
    .map_err(errors::internal)?;
    let mut prepared = prepared(op, request, label, ticket_number, &employee_insz, &input)?;
    if open_drawer {
        prepared.context = PendingContext::MoneyDrawer;
    }
    let context = prepared.context.clone();
    let committed = match signing::sign(state, &guard, &config, prepared).await {
        Ok(committed) => committed,
        Err(failure) => return Ok(FinancialResult::failed(failure.error)),
    };
    drop(guard);
    // Kick the physical drawer open, once, after the movement is safe.
    signing::follow_up(state, &config, &context, &committed, signing::Drawer::Kick).await;
    Ok(FinancialResult::done())
}

impl FinancialResult {
    pub(crate) fn done() -> Self {
        Self {
            success: true,
            error: None,
        }
    }

    pub(crate) fn failed(error: String) -> Self {
        Self {
            success: false,
            error: Some(error),
        }
    }
}

fn prepared(
    op: Op,
    request: SignRequest,
    label: EventLabel,
    ticket_number: i64,
    employee_insz: &str,
    input: &impl Serialize,
) -> Result<Prepared, String> {
    let context = match op.kind {
        Kind::Drawer => PendingContext::Drawer,
        _ => PendingContext::Money,
    };
    Ok(Prepared {
        op,
        request,
        label,
        first_ticket_no: ticket_number,
        events: vec![ArchiveSpec {
            order_id: None,
            user_insz: employee_insz.to_string(),
            request_json: serde_json::to_string(input).map_err(|e| e.to_string())?,
            sent_entries: None,
        }],
        order_id: None,
        user_insz: employee_insz.to_string(),
        context,
    })
}

/// Record a fiscal drawer open event (Event F).
/// Tracks when the cash drawer is opened outside of a sale.
pub async fn do_open_drawer_fiscal(
    state: &AppState,
    employee_insz: String,
) -> Result<FinancialResult, String> {
    access::operator_named(state, &employee_insz, Access::Admin)?;
    let op = Op::new(Kind::Drawer, &employee_insz);
    let guard = match signing::begin(state, &op).await {
        Ok(Begin::Fresh(guard)) => guard,
        // Answered late: its follow-up opened the drawer.
        Ok(Begin::AlreadyDone(_)) => return Ok(FinancialResult::done()),
        Err(e) => return Ok(FinancialResult::failed(e)),
    };
    let config = state.config.lock().map_err(|e| e.to_string())?.clone();
    let bp_id = super::current_booking(state, &guard)?;
    let label = EventLabel::F.in_mode(Mode::from(config.training_mode));
    let ticket_number = super::peek_ticket_number(state, &guard, label)?;

    let common = pos_fiscal::event::build_common_fields(
        &config,
        &state.identity,
        ticket_number,
        &employee_insz,
        &bp_id,
        // Drawer events are not printed.
        pos_fiscal::types::TicketMedium::None,
    );

    let input = DrawerOpenInput {
        common,
        drawer: Some(DrawerInput {
            id: "DRAWER1".to_string(),
            name: "Tiroir principal".to_string(),
        }),
    };

    let request = SignRequest::new(
        SignMutation::DrawerOpen,
        &input,
        Mode::from(config.training_mode),
    )
    .map_err(errors::internal)?;
    let prepared = prepared(op, request, label, ticket_number, &employee_insz, &input)?;
    let context = prepared.context.clone();
    let committed = match signing::sign(state, &guard, &config, prepared).await {
        Ok(committed) => committed,
        Err(failure) => return Ok(FinancialResult::failed(failure.error)),
    };
    drop(guard);
    // Kick the physical drawer open.
    signing::follow_up(state, &config, &context, &committed, signing::Drawer::Kick).await;
    Ok(FinancialResult::done())
}
