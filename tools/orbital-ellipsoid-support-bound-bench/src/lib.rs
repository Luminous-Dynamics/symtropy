#![deny(unsafe_code)]

pub const RECEIPT_SCHEMA: &str = "luminous.orbital-ellipsoid-support-bound.receipt.v0.1";
pub const SUPPORT_SCALE_UM_PER_M: u128 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderState {
    Pass,
    Fail,
    Unknown,
    Stale,
    Revoked,
}

impl ProviderState {
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

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
            Self::Stale => "STALE",
            Self::Revoked => "REVOKED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportDisposition {
    BoundReady,
    ProviderFailed,
    Indeterminate,
    OutOfScope,
    InvalidInput,
    Overflow,
}

impl SupportDisposition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BoundReady => "BOUND_READY",
            Self::ProviderFailed => "PROVIDER_FAILED",
            Self::Indeterminate => "INDETERMINATE",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::InvalidInput => "INVALID_INPUT",
            Self::Overflow => "OVERFLOW",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportScope {
    pub scope_id: String,
    pub terminal_action_id: String,
    pub target_id: String,
    pub keepout_profile_id: String,
    pub keepout_geometry_revision: String,
    pub matrix_sha256: String,
    pub matrix_convention: String,
    pub frame_id: String,
    pub frame_revision: String,
    pub transform_digest: String,
    pub direction_sha256: String,
    pub direction_convention: String,
    pub length_unit: String,
    pub terminal_epoch_tai_ns: u64,
}

impl SupportScope {
    fn is_valid(&self) -> bool {
        [
            self.scope_id.as_str(),
            self.terminal_action_id.as_str(),
            self.target_id.as_str(),
            self.keepout_profile_id.as_str(),
            self.keepout_geometry_revision.as_str(),
            self.matrix_convention.as_str(),
            self.frame_id.as_str(),
            self.frame_revision.as_str(),
            self.transform_digest.as_str(),
            self.direction_convention.as_str(),
            self.length_unit.as_str(),
        ]
        .iter()
        .all(|value| !value.trim().is_empty())
            && self.matrix_convention == "body-fixed-K-inverse-m2-v1"
            && self.length_unit == "metre"
            && self.direction_convention == "unit-along-track-positive-y"
            && valid_sha256(&self.matrix_sha256)
            && valid_sha256(&self.direction_sha256)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderEvidence {
    pub provider_id: String,
    pub provider_revision: String,
    pub receipt_sha256: String,
    pub scope_id: String,
    pub state: ProviderState,
}

impl ProviderEvidence {
    fn shape_valid(&self) -> bool {
        !self.provider_id.trim().is_empty()
            && !self.provider_revision.trim().is_empty()
            && valid_sha256(&self.receipt_sha256)
            && !self.scope_id.trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSet {
    pub matrix_spd: ProviderEvidence,
    pub direction_unit: ProviderEvidence,
    pub quadratic_lower: ProviderEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportInput {
    pub case_id: String,
    pub scope: SupportScope,
    /// Positive rational lower bound q_lower = numerator / denominator in m^-2.
    /// It must be a verified lower enclosure of y_hat^T K y_hat.
    pub q_lower_numerator: u128,
    pub q_lower_denominator: u128,
    pub evidence: EvidenceSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportResult {
    pub case_id: String,
    pub scope: SupportScope,
    pub disposition: SupportDisposition,
    pub q_lower_numerator: u128,
    pub q_lower_denominator: u128,
    pub q_unit: String,
    pub rho_y_upper_um: Option<u128>,
    pub rho_unit: String,
    pub scale_um_per_m: u128,
    pub q_scaled_ceiling: Option<u128>,
    pub floor_sqrt: Option<u128>,
    pub perfect_square: Option<bool>,
    pub evidence: EvidenceSet,
    pub reason_codes: Vec<String>,
    pub bound_ready: bool,
    pub source_bytes_authenticated: bool,
    pub physical_geometry_validated: bool,
    pub authority_granted: bool,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn ceil_div(numerator: u128, denominator: u128) -> Option<u128> {
    if denominator == 0 {
        return None;
    }
    let quotient = numerator / denominator;
    if quotient.checked_mul(denominator) == Some(numerator) {
        Some(quotient)
    } else {
        quotient.checked_add(1)
    }
}

fn floor_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }

    let mut low = 1_u128;
    let mut high = 1_u128 << 64;
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if mid <= value / mid {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

fn ceil_sqrt(value: u128) -> (u128, u128, bool) {
    let floor = floor_sqrt(value);
    let perfect = floor.checked_mul(floor) == Some(value);
    let ceiling = if perfect {
        floor
    } else {
        floor + 1
    };
    (ceiling, floor, perfect)
}

fn base_result(input: &SupportInput, disposition: SupportDisposition, reason: &str) -> SupportResult {
    SupportResult {
        case_id: input.case_id.clone(),
        scope: input.scope.clone(),
        disposition,
        q_lower_numerator: input.q_lower_numerator,
        q_lower_denominator: input.q_lower_denominator,
        q_unit: "m^-2".to_owned(),
        rho_y_upper_um: None,
        rho_unit: "um".to_owned(),
        scale_um_per_m: SUPPORT_SCALE_UM_PER_M,
        q_scaled_ceiling: None,
        floor_sqrt: None,
        perfect_square: None,
        evidence: input.evidence.clone(),
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        bound_ready: false,
        source_bytes_authenticated: false,
        physical_geometry_validated: false,
        authority_granted: false,
    }
}

/// Outwardly bounds rho_y = (y_hat^T K y_hat)^(-1/2).
///
/// The function consumes a positive rational lower enclosure q_lower = n/d
/// in m^-2 and independently receipted claims that the exact K is SPD and the
/// direction is unit-normalized in the exact declared frame/convention.
///
/// The smallest integer R in micrometres satisfying
/// R^2 * n >= (1_000_000)^2 * d is returned. To avoid an unchecked square,
/// the implementation computes T = ceil((1_000_000)^2 * d / n), then
/// R = ceil(sqrt(T)) using division-based integer square-root comparison.
///
/// This validates only the fixed-point implication of the supplied q_lower
/// and evidence references. It does not authenticate source bytes or
/// independently establish that K and direction are physically correct.
pub fn derive(input: &SupportInput) -> SupportResult {
    if input.case_id.trim().is_empty() || !input.scope.is_valid() {
        return base_result(
            input,
            SupportDisposition::InvalidInput,
            "CASE_OR_SCOPE_INVALID",
        );
    }

    if input.q_lower_numerator == 0 || input.q_lower_denominator == 0 {
        return base_result(
            input,
            SupportDisposition::InvalidInput,
            "QUADRATIC_LOWER_BOUND_MUST_BE_STRICTLY_POSITIVE",
        );
    }

    let evidence = [
        &input.evidence.matrix_spd,
        &input.evidence.direction_unit,
        &input.evidence.quadratic_lower,
    ];
    if evidence.iter().any(|item| !item.shape_valid()) {
        return base_result(
            input,
            SupportDisposition::InvalidInput,
            "PROVIDER_EVIDENCE_SHAPE_INVALID",
        );
    }

    if evidence
        .iter()
        .any(|item| item.scope_id != input.scope.scope_id)
    {
        return base_result(
            input,
            SupportDisposition::OutOfScope,
            "PROVIDER_SCOPE_MISMATCH",
        );
    }

    if evidence.iter().any(|item| item.state == ProviderState::Fail) {
        return base_result(
            input,
            SupportDisposition::ProviderFailed,
            "REQUIRED_GEOMETRY_PROVIDER_FAILED",
        );
    }

    if evidence
        .iter()
        .any(|item| item.state != ProviderState::Pass)
    {
        return base_result(
            input,
            SupportDisposition::Indeterminate,
            "REQUIRED_GEOMETRY_PROVIDER_NOT_CURRENT",
        );
    }

    let scale_squared = match SUPPORT_SCALE_UM_PER_M.checked_mul(SUPPORT_SCALE_UM_PER_M) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                SupportDisposition::Overflow,
                "OUTPUT_SCALE_SQUARE_OVERFLOW",
            );
        }
    };
    let scaled_numerator = match scale_squared.checked_mul(input.q_lower_denominator) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                SupportDisposition::Overflow,
                "SCALED_QUADRATIC_NUMERATOR_OVERFLOW",
            );
        }
    };
    let q_ceiling = match ceil_div(scaled_numerator, input.q_lower_numerator) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                SupportDisposition::Overflow,
                "QUADRATIC_RATIO_OVERFLOW",
            );
        }
    };
    let (rho_upper, floor, perfect) = ceil_sqrt(q_ceiling);

    let mut result = base_result(input, SupportDisposition::BoundReady, "");
    result.rho_y_upper_um = Some(rho_upper);
    result.q_scaled_ceiling = Some(q_ceiling);
    result.floor_sqrt = Some(floor);
    result.perfect_square = Some(perfect);
    result.bound_ready = true;
    result
}

#[cfg(test)]
mod tests {
    use super::{
        EvidenceSet, ProviderEvidence, ProviderState, SupportDisposition, SupportInput,
        SupportScope, derive,
    };

    fn scope() -> SupportScope {
        SupportScope {
            scope_id: "support-scope-1".to_owned(),
            terminal_action_id: "terminal-action-1".to_owned(),
            target_id: "target-1".to_owned(),
            keepout_profile_id: "keepout-1".to_owned(),
            keepout_geometry_revision: "geometry-r1".to_owned(),
            matrix_sha256: "a".repeat(64),
            matrix_convention: "body-fixed-K-inverse-m2".to_owned(),
            frame_id: "LVLH".to_owned(),
            frame_revision: "frame-r1".to_owned(),
            transform_digest: "transform-r1".to_owned(),
            direction_sha256: "b".repeat(64),
            direction_convention: "unit-along-track-positive-y".to_owned(),
            length_unit: "metre".to_owned(),
            terminal_epoch_tai_ns: 100,
        }
    }

    fn provider(name: &str) -> ProviderEvidence {
        ProviderEvidence {
            provider_id: name.to_owned(),
            provider_revision: "r1".to_owned(),
            receipt_sha256: "c".repeat(64),
            scope_id: "support-scope-1".to_owned(),
            state: ProviderState::Pass,
        }
    }

    fn input(numerator: u128, denominator: u128) -> SupportInput {
        SupportInput {
            case_id: "SUPPORT-1".to_owned(),
            scope: scope(),
            q_lower_numerator: numerator,
            q_lower_denominator: denominator,
            evidence: EvidenceSet {
                matrix_spd: provider("matrix-spd"),
                direction_unit: provider("direction-unit"),
                quadratic_lower: provider("quadratic-lower"),
            },
        }
    }

    #[test]
    fn unit_sphere_support_is_one_metre() {
        let result = derive(&input(1, 1));
        assert_eq!(result.disposition, SupportDisposition::BoundReady);
        assert_eq!(result.rho_y_upper_um, Some(1_000_000));
        assert_eq!(result.perfect_square, Some(true));
    }

    #[test]
    fn four_inverse_square_metres_produce_half_metre_support() {
        let result = derive(&input(4, 1));
        assert_eq!(result.rho_y_upper_um, Some(500_000));
    }

    #[test]
    fn rational_support_rounds_outward() {
        let result = derive(&input(2, 1));
        assert_eq!(result.rho_y_upper_um, Some(707_107));
        assert_eq!(result.perfect_square, Some(false));
    }

    #[test]
    fn larger_quadratic_lower_bound_never_increases_support() {
        let low = derive(&input(2, 1)).rho_y_upper_um.unwrap();
        let high = derive(&input(8, 1)).rho_y_upper_um.unwrap();
        assert!(high <= low);
    }

    #[test]
    fn zero_numerator_or_denominator_is_invalid() {
        assert_eq!(derive(&input(0, 1)).disposition, SupportDisposition::InvalidInput);
        assert_eq!(derive(&input(1, 0)).disposition, SupportDisposition::InvalidInput);
    }

    #[test]
    fn scope_mismatch_is_out_of_scope() {
        let mut value = input(1, 1);
        value.evidence.matrix_spd.scope_id = "different".to_owned();
        assert_eq!(derive(&value).disposition, SupportDisposition::OutOfScope);
    }

    #[test]
    fn stale_and_unknown_geometry_evidence_is_indeterminate() {
        for state in [ProviderState::Unknown, ProviderState::Stale, ProviderState::Revoked] {
            let mut value = input(1, 1);
            value.evidence.direction_unit.state = state;
            assert_eq!(derive(&value).disposition, SupportDisposition::Indeterminate);
        }
    }

    #[test]
    fn failed_spd_provider_never_produces_support_bound() {
        let mut value = input(1, 1);
        value.evidence.matrix_spd.state = ProviderState::Fail;
        assert_eq!(derive(&value).disposition, SupportDisposition::ProviderFailed);
    }

    #[test]
    fn malformed_matrix_or_direction_digest_is_invalid() {
        let mut matrix = input(1, 1);
        matrix.scope.matrix_sha256 = "bad".to_owned();
        assert_eq!(derive(&matrix).disposition, SupportDisposition::InvalidInput);
        let mut direction = input(1, 1);
        direction.scope.direction_sha256 = "bad".to_owned();
        assert_eq!(derive(&direction).disposition, SupportDisposition::InvalidInput);
    }

    #[test]
    fn unsupported_matrix_convention_is_invalid() {
        let mut value = input(1, 1);
        value.scope.matrix_convention = "principal-axis-semiaxes-v0".to_owned();
        assert_eq!(
            derive(&value).disposition,
            SupportDisposition::InvalidInput
        );
    }

    #[test]
    fn unsupported_units_and_direction_conventions_are_invalid() {
        let mut wrong_unit = input(1, 1);
        wrong_unit.scope.length_unit = "millimetre".to_owned();
        assert_eq!(
            derive(&wrong_unit).disposition,
            SupportDisposition::InvalidInput
        );

        let mut wrong_direction = input(1, 1);
        wrong_direction.scope.direction_convention = "camera-x-forward".to_owned();
        assert_eq!(
            derive(&wrong_direction).disposition,
            SupportDisposition::InvalidInput
        );
    }

    #[test]
    fn upper_scale_product_overflow_is_nonpass() {
        let result = derive(&input(1, u128::MAX));
        assert_eq!(result.disposition, SupportDisposition::Overflow);
    }

    #[test]
    fn integer_square_root_handles_u128_max() {
        let value = u128::MAX;
        let result = super::floor_sqrt(value);
        assert_eq!(result, u64::MAX);
    }
}
