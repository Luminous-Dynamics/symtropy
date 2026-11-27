#![deny(unsafe_code)]

use orbital_inspection_horizon_composition_bench::{HorizonDisposition, HorizonScope};

pub const RECEIPT_SCHEMA: &str = "luminous.orbital-passive-safety-terminal.receipt.v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundState {
    Pass,
    Fail,
    Unknown,
    Stale,
    Revoked,
}

impl BoundState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
            Self::Stale => "STALE",
            Self::Revoked => "REVOKED",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "PASS" => Some(Self::Pass),
            "FAIL" => Some(Self::Fail),
            "UNKNOWN" => Some(Self::Unknown),
            "STALE" => Some(Self::Stale),
            "REVOKED" => Some(Self::Revoked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalDisposition {
    SecularOutboundPassiveSafe,
    PeriodicRelativeMotionOnly,
    PeriodicPassiveSafe,
    NotPassiveSafe,
    Indeterminate,
    OutOfScope,
    InvalidInput,
}

impl TerminalDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SecularOutboundPassiveSafe => "SECULAR_OUTBOUND_PASSIVE_SAFE",
            Self::PeriodicRelativeMotionOnly => "PERIODIC_RELATIVE_MOTION_ONLY",
            Self::PeriodicPassiveSafe => "PERIODIC_PASSIVE_SAFE",
            Self::NotPassiveSafe => "NOT_PASSIVE_SAFE",
            Self::Indeterminate => "INDETERMINATE",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::InvalidInput => "INVALID_INPUT",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundEnvelope {
    pub delta_a_lower_scaled: i64,
    pub delta_a_upper_scaled: i64,
    pub y_lower_scaled: i64,
    pub y_upper_scaled: i64,
    pub rho_y_upper_scaled: i64,
    pub provider_id: String,
    pub provider_revision: String,
    pub receipt_sha256: String,
    pub state: BoundState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodWitness {
    pub horizon_id: String,
    pub scope: HorizonScope,
    pub start_tai_ns: u64,
    pub end_tai_ns: u64,
    pub relative_period_ns: u64,
    pub complete_time_coverage: bool,
    pub disposition: HorizonDisposition,
    pub segment_count: usize,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCase {
    pub case_id: String,
    pub scope: HorizonScope,
    pub terminal_epoch_tai_ns: u64,
    pub bounds: BoundEnvelope,
    pub period_witness: Option<PeriodWitness>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalResult {
    pub case_id: String,
    pub scope: HorizonScope,
    pub terminal_epoch_tai_ns: u64,
    pub bounds_provider_id: String,
    pub bounds_provider_revision: String,
    pub bounds_receipt_sha256: String,
    pub bounds_state: BoundState,
    pub period_witness_id: Option<String>,
    pub period_witness_receipt_sha256: Option<String>,
    pub disposition: TerminalDisposition,
    pub delta_a_branch: String,
    pub margin_scaled: Option<i128>,
    pub reason_codes: Vec<String>,
    pub period_witness_consumed: bool,
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn scope_valid(scope: &HorizonScope) -> bool {
    [
        scope.action_id.as_str(),
        scope.target_id.as_str(),
        scope.keepout_profile_id.as_str(),
        scope.keepout_geometry_revision.as_str(),
        scope.transform_digest.as_str(),
        scope.frame_id.as_str(),
        scope.frame_revision.as_str(),
        scope.time_basis_id.as_str(),
        scope.dynamics_profile_revision.as_str(),
    ]
    .iter()
    .all(|value| valid_text(value))
}

fn result(
    case: &TerminalCase,
    disposition: TerminalDisposition,
    branch: &str,
    margin_scaled: Option<i128>,
    reason: &str,
    period_witness_consumed: bool,
) -> TerminalResult {
    TerminalResult {
        case_id: case.case_id.clone(),
        scope: case.scope.clone(),
        terminal_epoch_tai_ns: case.terminal_epoch_tai_ns,
        bounds_provider_id: case.bounds.provider_id.clone(),
        bounds_provider_revision: case.bounds.provider_revision.clone(),
        bounds_receipt_sha256: case.bounds.receipt_sha256.clone(),
        bounds_state: case.bounds.state,
        period_witness_id: case.period_witness.as_ref().map(|witness| witness.horizon_id.clone()),
        period_witness_receipt_sha256: case
            .period_witness
            .as_ref()
            .map(|witness| witness.receipt_sha256.clone()),
        disposition,
        delta_a_branch: branch.to_owned(),
        margin_scaled,
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        period_witness_consumed,
    }
}

fn validate_period_witness(
    case: &TerminalCase,
    witness: &PeriodWitness,
) -> Result<(), (TerminalDisposition, &'static str)> {
    if !valid_text(&witness.horizon_id)
        || !valid_sha256(&witness.receipt_sha256)
        || witness.relative_period_ns == 0
        || witness.segment_count == 0
        || witness.start_tai_ns >= witness.end_tai_ns
    {
        return Err((
            TerminalDisposition::InvalidInput,
            "PERIOD_WITNESS_SHAPE_INVALID",
        ));
    }

    if witness.scope != case.scope {
        return Err((
            TerminalDisposition::OutOfScope,
            "PERIOD_WITNESS_SCOPE_MISMATCH",
        ));
    }

    if witness.start_tai_ns != case.terminal_epoch_tai_ns {
        return Err((
            TerminalDisposition::OutOfScope,
            "PERIOD_WITNESS_DOES_NOT_START_AT_TERMINAL_EPOCH",
        ));
    }

    let covered_duration = witness
        .end_tai_ns
        .checked_sub(witness.start_tai_ns)
        .ok_or((
            TerminalDisposition::InvalidInput,
            "PERIOD_WITNESS_INTERVAL_INVALID",
        ))?;

    if covered_duration < witness.relative_period_ns {
        return Err((
            TerminalDisposition::Indeterminate,
            "PERIOD_WITNESS_SHORTER_THAN_ONE_RELATIVE_PERIOD",
        ));
    }

    if !witness.complete_time_coverage
        || witness.disposition != HorizonDisposition::CompleteConditionalClear
    {
        return Err((
            TerminalDisposition::Indeterminate,
            "PERIOD_WITNESS_NOT_COMPLETE_AND_CLEAR",
        ));
    }

    Ok(())
}

/// Checks the source-specific first-order eccentric passive-safety algebra.
///
/// The function consumes outward-enclosed delta-a, y_lower, y_upper and
/// keep-out support inputs. It does not construct these enclosures. A negative
/// delta-a branch requires the complete lower y bound to lie strictly beyond
/// the positive keep-out support. A positive delta-a branch requires the
/// complete upper y bound to lie strictly beyond the negative support.
///
/// Exact delta-a == 0 establishes periodic relative motion only. The stronger
/// PERIODIC_PASSIVE_SAFE disposition additionally requires a complete,
/// scope-identical, post-terminal interval covering at least one full relative
/// period and independently classified clear by the horizon composer.
pub fn evaluate(case: &TerminalCase) -> TerminalResult {
    if !valid_text(&case.case_id) || !scope_valid(&case.scope) {
        return result(
            case,
            TerminalDisposition::InvalidInput,
            "UNCLASSIFIED",
            None,
            "TERMINAL_CASE_OR_SCOPE_INVALID",
            false,
        );
    }

    let bounds = &case.bounds;
    if !valid_text(&bounds.provider_id)
        || !valid_text(&bounds.provider_revision)
        || !valid_sha256(&bounds.receipt_sha256)
        || bounds.delta_a_lower_scaled > bounds.delta_a_upper_scaled
        || bounds.y_lower_scaled > bounds.y_upper_scaled
        || bounds.rho_y_upper_scaled <= 0
    {
        return result(
            case,
            TerminalDisposition::InvalidInput,
            "UNCLASSIFIED",
            None,
            "BOUND_ENVELOPE_INVALID",
            false,
        );
    }

    match bounds.state {
        BoundState::Fail => {
            return result(
                case,
                TerminalDisposition::NotPassiveSafe,
                "UPSTREAM_BOUND_FAIL",
                None,
                "UPSTREAM_BOUND_PROVIDER_FAILED",
                false,
            );
        }
        BoundState::Unknown | BoundState::Stale | BoundState::Revoked => {
            return result(
                case,
                TerminalDisposition::Indeterminate,
                "UPSTREAM_BOUND_UNRESOLVED",
                None,
                "UPSTREAM_BOUND_PROVIDER_NOT_CURRENT",
                false,
            );
        }
        BoundState::Pass => {}
    }

    let delta_lo = i128::from(bounds.delta_a_lower_scaled);
    let delta_hi = i128::from(bounds.delta_a_upper_scaled);
    let rho = i128::from(bounds.rho_y_upper_scaled);
    let y_lower = i128::from(bounds.y_lower_scaled);
    let y_upper = i128::from(bounds.y_upper_scaled);

    if delta_lo == 0 && delta_hi == 0 {
        let Some(witness) = &case.period_witness else {
            return result(
                case,
                TerminalDisposition::PeriodicRelativeMotionOnly,
                "EXACT_ZERO_DELTA_A",
                None,
                "PERIODIC_MOTION_REQUIRES_COMPLETE_PERIOD_CLEARANCE",
                false,
            );
        };

        if let Err((disposition, reason)) = validate_period_witness(case, witness) {
            return result(case, disposition, "EXACT_ZERO_DELTA_A", None, reason, true);
        }

        return result(
            case,
            TerminalDisposition::PeriodicPassiveSafe,
            "EXACT_ZERO_DELTA_A",
            None,
            "",
            true,
        );
    }

    if case.period_witness.is_some() {
        return result(
            case,
            TerminalDisposition::InvalidInput,
            "NONZERO_DELTA_A",
            None,
            "PERIODIC_WITNESS_PROVIDED_FOR_NONPERIODIC_BRANCH",
            false,
        );
    }

    if delta_hi < 0 {
        let margin = y_lower.checked_sub(rho);
        return match margin {
            Some(value) if value > 0 => result(
                case,
                TerminalDisposition::SecularOutboundPassiveSafe,
                "NEGATIVE_DELTA_A_MOVES_POSITIVE_Y",
                Some(value),
                "",
                false,
            ),
            Some(value) => result(
                case,
                TerminalDisposition::NotPassiveSafe,
                "NEGATIVE_DELTA_A_MOVES_POSITIVE_Y",
                Some(value),
                "LOWER_Y_BOUND_DOES_NOT_STRICTLY_EXCEED_KEEP_OUT_SUPPORT",
                false,
            ),
            None => result(
                case,
                TerminalDisposition::Indeterminate,
                "NEGATIVE_DELTA_A_MOVES_POSITIVE_Y",
                None,
                "CLEARANCE_MARGIN_OVERFLOW",
                false,
            ),
        };
    }

    if delta_lo > 0 {
        let negative_rho = rho.checked_neg();
        let margin = negative_rho.and_then(|limit| limit.checked_sub(y_upper));
        return match margin {
            Some(value) if value > 0 => result(
                case,
                TerminalDisposition::SecularOutboundPassiveSafe,
                "POSITIVE_DELTA_A_MOVES_NEGATIVE_Y",
                Some(value),
                "",
                false,
            ),
            Some(value) => result(
                case,
                TerminalDisposition::NotPassiveSafe,
                "POSITIVE_DELTA_A_MOVES_NEGATIVE_Y",
                Some(value),
                "UPPER_Y_BOUND_DOES_NOT_STRICTLY_EXCEED_NEGATIVE_KEEP_OUT_SUPPORT",
                false,
            ),
            None => result(
                case,
                TerminalDisposition::Indeterminate,
                "POSITIVE_DELTA_A_MOVES_NEGATIVE_Y",
                None,
                "CLEARANCE_MARGIN_OVERFLOW",
                false,
            ),
        };
    }

    result(
        case,
        TerminalDisposition::Indeterminate,
        "DELTA_A_SIGN_UNRESOLVED",
        None,
        "DELTA_A_INTERVAL_TOUCHES_OR_CROSSES_ZERO",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        BoundEnvelope, BoundState, PeriodWitness, TerminalCase, TerminalDisposition, evaluate,
    };
    use orbital_inspection_horizon_composition_bench::{HorizonDisposition, HorizonScope};

    fn scope() -> HorizonScope {
        HorizonScope {
            action_id: "action-1".to_owned(),
            target_id: "target-1".to_owned(),
            keepout_profile_id: "keepout-1".to_owned(),
            keepout_geometry_revision: "geometry-r1".to_owned(),
            transform_digest: "transform-r1".to_owned(),
            frame_id: "LVLH".to_owned(),
            frame_revision: "frame-r1".to_owned(),
            time_basis_id: "TAI".to_owned(),
            dynamics_profile_revision: "eccentric-first-order-r1".to_owned(),
        }
    }

    fn bounds(delta_lo: i64, delta_hi: i64, y_lo: i64, y_hi: i64, rho: i64) -> BoundEnvelope {
        BoundEnvelope {
            delta_a_lower_scaled: delta_lo,
            delta_a_upper_scaled: delta_hi,
            y_lower_scaled: y_lo,
            y_upper_scaled: y_hi,
            rho_y_upper_scaled: rho,
            provider_id: "relative-element-enclosure".to_owned(),
            provider_revision: "r1".to_owned(),
            receipt_sha256: "a".repeat(64),
            state: BoundState::Pass,
        }
    }

    fn case(value: BoundEnvelope) -> TerminalCase {
        TerminalCase {
            case_id: "case-1".to_owned(),
            scope: scope(),
            terminal_epoch_tai_ns: 100,
            bounds: value,
            period_witness: None,
        }
    }

    fn period_witness() -> PeriodWitness {
        PeriodWitness {
            horizon_id: "period-horizon-1".to_owned(),
            scope: scope(),
            start_tai_ns: 100,
            end_tai_ns: 1_000_000_100,
            relative_period_ns: 1_000_000_000,
            complete_time_coverage: true,
            disposition: HorizonDisposition::CompleteConditionalClear,
            segment_count: 3,
            receipt_sha256: "b".repeat(64),
        }
    }

    #[test]
    fn negative_delta_a_with_positive_y_clearance_passes() {
        let result = evaluate(&case(bounds(-100, -90, 1_500_000, 2_000_000, 1_000_000)));
        assert_eq!(
            result.disposition,
            TerminalDisposition::SecularOutboundPassiveSafe
        );
        assert_eq!(result.margin_scaled, Some(500_000));
    }

    #[test]
    fn positive_delta_a_with_negative_y_clearance_passes() {
        let result = evaluate(&case(bounds(90, 100, -2_000_000, -1_500_000, 1_000_000)));
        assert_eq!(
            result.disposition,
            TerminalDisposition::SecularOutboundPassiveSafe
        );
        assert_eq!(result.margin_scaled, Some(500_000));
    }

    #[test]
    fn negative_delta_a_equality_is_not_a_strict_clearance_pass() {
        let result = evaluate(&case(bounds(-10, -1, 1_000_000, 2_000_000, 1_000_000)));
        assert_eq!(result.disposition, TerminalDisposition::NotPassiveSafe);
        assert_eq!(result.margin_scaled, Some(0));
    }

    #[test]
    fn positive_delta_a_equality_is_not_a_strict_clearance_pass() {
        let result = evaluate(&case(bounds(1, 10, -2_000_000, -1_000_000, 1_000_000)));
        assert_eq!(result.disposition, TerminalDisposition::NotPassiveSafe);
        assert_eq!(result.margin_scaled, Some(0));
    }

    #[test]
    fn exact_zero_delta_a_without_period_witness_is_only_periodic() {
        let result = evaluate(&case(bounds(0, 0, -9_000_000, 9_000_000, 1_000_000)));
        assert_eq!(
            result.disposition,
            TerminalDisposition::PeriodicRelativeMotionOnly
        );
        assert!(!result.period_witness_consumed);
    }

    #[test]
    fn exact_zero_delta_a_with_complete_period_clearance_passes() {
        let mut value = case(bounds(0, 0, -9_000_000, 9_000_000, 1_000_000));
        value.period_witness = Some(period_witness());
        let result = evaluate(&value);
        assert_eq!(result.disposition, TerminalDisposition::PeriodicPassiveSafe);
        assert!(result.period_witness_consumed);
    }

    #[test]
    fn period_witness_must_begin_at_terminal_epoch() {
        let mut value = case(bounds(0, 0, -9_000_000, 9_000_000, 1_000_000));
        let mut witness = period_witness();
        witness.start_tai_ns = 99;
        value.period_witness = Some(witness);
        assert_eq!(
            evaluate(&value).disposition,
            TerminalDisposition::OutOfScope
        );
    }

    #[test]
    fn period_witness_must_cover_at_least_one_relative_period() {
        let mut value = case(bounds(0, 0, -9_000_000, 9_000_000, 1_000_000));
        let mut witness = period_witness();
        witness.end_tai_ns = 1_000_000_099;
        value.period_witness = Some(witness);
        assert_eq!(
            evaluate(&value).disposition,
            TerminalDisposition::Indeterminate
        );
    }

    #[test]
    fn incomplete_or_not_clear_period_witness_is_indeterminate() {
        let mut value = case(bounds(0, 0, -9_000_000, 9_000_000, 1_000_000));
        let mut witness = period_witness();
        witness.complete_time_coverage = false;
        value.period_witness = Some(witness);
        assert_eq!(
            evaluate(&value).disposition,
            TerminalDisposition::Indeterminate
        );
    }

    #[test]
    fn period_witness_scope_mismatch_is_out_of_scope() {
        let mut value = case(bounds(0, 0, -9_000_000, 9_000_000, 1_000_000));
        let mut witness = period_witness();
        witness.scope.frame_revision = "frame-r2".to_owned();
        value.period_witness = Some(witness);
        assert_eq!(
            evaluate(&value).disposition,
            TerminalDisposition::OutOfScope
        );
    }

    #[test]
    fn a_delta_interval_crossing_zero_is_indeterminate() {
        let result = evaluate(&case(bounds(-1, 1, 2_000_000, 3_000_000, 1_000_000)));
        assert_eq!(result.disposition, TerminalDisposition::Indeterminate);
    }

    #[test]
    fn a_delta_interval_touching_zero_is_indeterminate() {
        assert_eq!(
            evaluate(&case(bounds(-1, 0, 2_000_000, 3_000_000, 1_000_000))).disposition,
            TerminalDisposition::Indeterminate
        );
        assert_eq!(
            evaluate(&case(bounds(0, 1, -3_000_000, -2_000_000, 1_000_000))).disposition,
            TerminalDisposition::Indeterminate
        );
    }

    #[test]
    fn wrong_secular_direction_does_not_pass() {
        assert_eq!(
            evaluate(&case(bounds(-10, -1, -3_000_000, -2_000_000, 1_000_000))).disposition,
            TerminalDisposition::NotPassiveSafe
        );
        assert_eq!(
            evaluate(&case(bounds(1, 10, 2_000_000, 3_000_000, 1_000_000))).disposition,
            TerminalDisposition::NotPassiveSafe
        );
    }

    #[test]
    fn uncertain_or_stale_bounds_do_not_pass() {
        let mut unknown = case(bounds(-10, -1, 2_000_000, 3_000_000, 1_000_000));
        unknown.bounds.state = BoundState::Unknown;
        assert_eq!(
            evaluate(&unknown).disposition,
            TerminalDisposition::Indeterminate
        );
        let mut stale = unknown.clone();
        stale.bounds.state = BoundState::Stale;
        assert_eq!(
            evaluate(&stale).disposition,
            TerminalDisposition::Indeterminate
        );
    }

    #[test]
    fn failed_bound_provider_does_not_pass() {
        let mut value = case(bounds(-10, -1, 2_000_000, 3_000_000, 1_000_000));
        value.bounds.state = BoundState::Fail;
        assert_eq!(
            evaluate(&value).disposition,
            TerminalDisposition::NotPassiveSafe
        );
    }

    #[test]
    fn bound_envelope_requires_ordered_values_and_positive_support() {
        assert_eq!(
            evaluate(&case(bounds(10, -10, 1, 2, 1))).disposition,
            TerminalDisposition::InvalidInput
        );
        assert_eq!(
            evaluate(&case(bounds(-10, -1, 2, 1, 1))).disposition,
            TerminalDisposition::InvalidInput
        );
        assert_eq!(
            evaluate(&case(bounds(-10, -1, 2, 3, 0))).disposition,
            TerminalDisposition::InvalidInput
        );
    }

    #[test]
    fn a_period_witness_cannot_promote_a_secular_branch() {
        let mut value = case(bounds(-10, -1, 2_000_000, 3_000_000, 1_000_000));
        value.period_witness = Some(period_witness());
        assert_eq!(
            evaluate(&value).disposition,
            TerminalDisposition::InvalidInput
        );
    }
}
