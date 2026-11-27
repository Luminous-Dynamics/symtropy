#![deny(unsafe_code)]

pub const RECEIPT_SCHEMA: &str = "luminous.orbital-gravity-curvature-bound.receipt.v0.1";
pub const CURVATURE_SCALE: u128 = 1_000_000;
pub const SHA256_HEX_LEN: usize = 64;

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
pub enum CurvatureDisposition {
    BoundReady,
    ProviderFailed,
    Indeterminate,
    OutOfScope,
    InvalidInput,
    Overflow,
}

impl CurvatureDisposition {
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
pub struct Scope {
    pub scope_id: String,
    pub action_id: String,
    pub segment_id: String,
    pub target_id: String,
    pub chief_id: String,
    pub servicer_id: String,
    pub central_body_id: String,
    pub keepout_profile_id: String,
    pub keepout_geometry_revision: String,
    pub transform_digest: String,
    pub frame_id: String,
    pub frame_revision: String,
    pub time_basis_id: String,
    pub dynamics_profile_revision: String,
    pub start_tai_ns: u64,
    pub end_tai_ns: u64,
}

impl Scope {
    fn is_valid(&self) -> bool {
        [
            self.scope_id.as_str(),
            self.action_id.as_str(),
            self.segment_id.as_str(),
            self.target_id.as_str(),
            self.chief_id.as_str(),
            self.servicer_id.as_str(),
            self.central_body_id.as_str(),
            self.keepout_profile_id.as_str(),
            self.keepout_geometry_revision.as_str(),
            self.transform_digest.as_str(),
            self.frame_id.as_str(),
            self.frame_revision.as_str(),
            self.time_basis_id.as_str(),
            self.dynamics_profile_revision.as_str(),
        ]
        .iter()
        .all(|value| !value.trim().is_empty())
            && self.start_tai_ns < self.end_tai_ns
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
    pub mu: ProviderEvidence,
    pub chief_radius: ProviderEvidence,
    pub relative_separation: ProviderEvidence,
    pub differential_acceleration: ProviderEvidence,
    pub transform_norm: ProviderEvidence,
}

impl EvidenceSet {
    fn ordered(&self) -> [&ProviderEvidence; 5] {
        [
            &self.mu,
            &self.chief_radius,
            &self.relative_separation,
            &self.differential_acceleration,
            &self.transform_norm,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurvatureInput {
    pub case_id: String,
    pub scope: Scope,
    pub mu_upper_mm3_per_s2: u128,
    pub chief_radius_lower_mm: u64,
    pub relative_separation_upper_mm: u64,
    pub differential_acceleration_upper_mm_per_s2: u64,
    /// ceil(||L||_2 * 1e6 mm), where q = L r_rel is dimensionless.
    pub transform_operator_norm_upper_scaled: u64,
    pub transform_time_invariant: bool,
    pub perturbation_inventory_id: String,
    pub perturbation_inventory_complete: bool,
    pub evidence: EvidenceSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurvatureResult {
    pub case_id: String,
    pub scope: Scope,
    pub disposition: CurvatureDisposition,
    pub central_gravity_numerator: Option<u128>,
    pub radial_domain_denominator: Option<u128>,
    pub differential_acceleration_upper_mm_per_s2: Option<u128>,
    pub transform_operator_norm_upper_scaled: Option<u128>,
    pub scaled_central_numerator: Option<u128>,
    pub scaled_perturbation_term: Option<u128>,
    /// Outward upper bound on ||q''|| * 1e6 s^2.
    pub m_upper_scaled_per_s2: Option<u128>,
    pub m_scale: u128,
    pub m_unit: String,
    pub radial_slack_mm: Option<u64>,
    pub perturbation_inventory_id: String,
    pub perturbation_inventory_complete: bool,
    pub evidence: EvidenceSet,
    pub reason_codes: Vec<String>,
    pub bound_ready: bool,
    pub source_bytes_authenticated: bool,
    pub authority_granted: bool,
    pub physical_validation: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LEN && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn ceil_div_nonnegative(numerator: u128, denominator: u128) -> Option<u128> {
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

fn base_result(
    input: &CurvatureInput,
    disposition: CurvatureDisposition,
    reason: &str,
) -> CurvatureResult {
    CurvatureResult {
        case_id: input.case_id.clone(),
        scope: input.scope.clone(),
        disposition,
        central_gravity_numerator: None,
        radial_domain_denominator: None,
        differential_acceleration_upper_mm_per_s2: None,
        transform_operator_norm_upper_scaled: None,
        scaled_central_numerator: None,
        scaled_perturbation_term: None,
        m_upper_scaled_per_s2: None,
        m_scale: CURVATURE_SCALE,
        m_unit: "s^-2".to_owned(),
        radial_slack_mm: None,
        perturbation_inventory_id: input.perturbation_inventory_id.clone(),
        perturbation_inventory_complete: input.perturbation_inventory_complete,
        evidence: input.evidence.clone(),
        reason_codes: if reason.is_empty() {
            Vec::new()
        } else {
            vec![reason.to_owned()]
        },
        bound_ready: false,
        source_bytes_authenticated: false,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
    }
}

/// Derives an outward fixed-point bound for q'' under the declared
/// central-gravity and differential-acceleration assumptions.
///
/// For g(r) = -mu r / ||r||^3, the spectral norm of Dg is 2 mu / ||r||^3.
/// If ||r_chief|| >= R_c,min and ||r_deputy-r_chief|| <= rho_max, every point
/// on the joining line has radius at least R_min = R_c,min - rho_max.
/// Thus ||g(r_d)-g(r_c)|| <= 2 mu_max rho_max / R_min^3.
///
/// Add the declared upper bound on all differential non-central acceleration
/// channels, then apply the fixed, time-invariant transform q = L r_rel.
///
/// Units:
/// - mu: mm^3/s^2
/// - radii / separation: mm
/// - differential acceleration: mm/s^2
/// - transform norm: ceil(||L||_2 * 1e6 mm)
/// - output M: ceil(||q''|| * 1e6 s^2)
///
/// Every intermediate is checked u128. This verifies only the arithmetic
/// implication of the supplied bounds; it does not establish their physical
/// validity or authenticate their source bytes.
pub fn derive(input: &CurvatureInput) -> CurvatureResult {
    if input.case_id.trim().is_empty()
        || !input.scope.is_valid()
        || input.perturbation_inventory_id.trim().is_empty()
    {
        return base_result(
            input,
            CurvatureDisposition::InvalidInput,
            "CASE_SCOPE_OR_INVENTORY_INVALID",
        );
    }

    if !input.transform_time_invariant {
        return base_result(
            input,
            CurvatureDisposition::OutOfScope,
            "TIME_VARYING_TRANSFORM_UNSUPPORTED",
        );
    }

    if !input.perturbation_inventory_complete {
        return base_result(
            input,
            CurvatureDisposition::Indeterminate,
            "PERTURBATION_INVENTORY_INCOMPLETE",
        );
    }

    let evidence = input.evidence.ordered();
    if evidence.iter().any(|item| !item.shape_valid()) {
        return base_result(
            input,
            CurvatureDisposition::InvalidInput,
            "PROVIDER_EVIDENCE_SHAPE_INVALID",
        );
    }

    if evidence
        .iter()
        .any(|item| item.scope_id != input.scope.scope_id)
    {
        return base_result(
            input,
            CurvatureDisposition::OutOfScope,
            "PROVIDER_SCOPE_MISMATCH",
        );
    }

    if evidence
        .iter()
        .any(|item| item.state == ProviderState::Fail)
    {
        return base_result(
            input,
            CurvatureDisposition::ProviderFailed,
            "REQUIRED_BOUND_PROVIDER_FAILED",
        );
    }

    if evidence
        .iter()
        .any(|item| item.state != ProviderState::Pass)
    {
        return base_result(
            input,
            CurvatureDisposition::Indeterminate,
            "REQUIRED_BOUND_PROVIDER_NOT_CURRENT",
        );
    }

    if input.mu_upper_mm3_per_s2 == 0
        || input.chief_radius_lower_mm == 0
        || input.transform_operator_norm_upper_scaled == 0
    {
        return base_result(
            input,
            CurvatureDisposition::InvalidInput,
            "POSITIVE_PHYSICAL_BOUND_REQUIRED",
        );
    }

    let radial_slack = match input
        .chief_radius_lower_mm
        .checked_sub(input.relative_separation_upper_mm)
    {
        Some(value) if value > 0 => value,
        _ => {
            return base_result(
                input,
                CurvatureDisposition::OutOfScope,
                "RADIAL_DOMAIN_NOT_STRICTLY_POSITIVE",
            );
        }
    };

    let r_min = u128::from(radial_slack);
    let rho = u128::from(input.relative_separation_upper_mm);
    let mu = input.mu_upper_mm3_per_s2;
    let differential = u128::from(input.differential_acceleration_upper_mm_per_s2);
    let transform = u128::from(input.transform_operator_norm_upper_scaled);

    let denominator = match r_min.checked_mul(r_min).and_then(|v| v.checked_mul(r_min)) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                CurvatureDisposition::Overflow,
                "RADIAL_CUBE_OVERFLOW",
            );
        }
    };

    let central_numerator = match mu.checked_mul(rho).and_then(|v| v.checked_mul(2)) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                CurvatureDisposition::Overflow,
                "CENTRAL_GRAVITY_PRODUCT_OVERFLOW",
            );
        }
    };

    // Keep the central-gravity rational and the integer perturbation term
    // separate. Since transform and differential are integers,
    // ceil(T * (central / D + differential))
    //   = ceil(T * central / D) + T * differential.
    // This avoids materializing differential * D and the aggregate numerator,
    // both of which can overflow even when the final outward bound fits u128.
    let scaled_central_numerator = match transform.checked_mul(central_numerator) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                CurvatureDisposition::Overflow,
                "SCALED_CENTRAL_NUMERATOR_OVERFLOW",
            );
        }
    };

    let scaled_perturbation_term = match transform.checked_mul(differential) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                CurvatureDisposition::Overflow,
                "SCALED_PERTURBATION_TERM_OVERFLOW",
            );
        }
    };

    let central_m_upper = match ceil_div_nonnegative(scaled_central_numerator, denominator) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                CurvatureDisposition::Overflow,
                "CENTRAL_OUTWARD_DIVISION_FAILED",
            );
        }
    };

    let m_upper = match central_m_upper.checked_add(scaled_perturbation_term) {
        Some(value) => value,
        None => {
            return base_result(
                input,
                CurvatureDisposition::Overflow,
                "OUTWARD_BOUND_SUM_OVERFLOW",
            );
        }
    };

    let mut result = base_result(input, CurvatureDisposition::BoundReady, "");
    result.central_gravity_numerator = Some(central_numerator);
    result.radial_domain_denominator = Some(denominator);
    result.differential_acceleration_upper_mm_per_s2 = Some(differential);
    result.transform_operator_norm_upper_scaled = Some(transform);
    result.scaled_central_numerator = Some(scaled_central_numerator);
    result.scaled_perturbation_term = Some(scaled_perturbation_term);
    result.m_upper_scaled_per_s2 = Some(m_upper);
    result.radial_slack_mm = Some(radial_slack);
    result.bound_ready = true;
    result
}

#[cfg(test)]
mod tests {
    use super::{
        CurvatureDisposition, CurvatureInput, EvidenceSet, ProviderEvidence, ProviderState, Scope,
        derive,
    };

    fn scope() -> Scope {
        Scope {
            scope_id: "scope-1".to_owned(),
            action_id: "action-1".to_owned(),
            segment_id: "segment-1".to_owned(),
            target_id: "target-1".to_owned(),
            chief_id: "chief-1".to_owned(),
            servicer_id: "servicer-1".to_owned(),
            central_body_id: "earth".to_owned(),
            keepout_profile_id: "keepout-1".to_owned(),
            keepout_geometry_revision: "geometry-r1".to_owned(),
            transform_digest: "transform-sha".to_owned(),
            frame_id: "INERTIAL".to_owned(),
            frame_revision: "frame-r1".to_owned(),
            time_basis_id: "TAI".to_owned(),
            dynamics_profile_revision: "central-plus-bounds-r1".to_owned(),
            start_tai_ns: 1,
            end_tai_ns: 1_000_000_001,
        }
    }

    fn provider(name: &str) -> ProviderEvidence {
        ProviderEvidence {
            provider_id: name.to_owned(),
            provider_revision: "r1".to_owned(),
            receipt_sha256: "a".repeat(64),
            scope_id: "scope-1".to_owned(),
            state: ProviderState::Pass,
        }
    }

    fn input() -> CurvatureInput {
        CurvatureInput {
            case_id: "CB-TEST".to_owned(),
            scope: scope(),
            mu_upper_mm3_per_s2: 398_600_000_000_000_000_000_000,
            chief_radius_lower_mm: 7_000_000_000,
            relative_separation_upper_mm: 1_000_000,
            differential_acceleration_upper_mm_per_s2: 0,
            transform_operator_norm_upper_scaled: 1_000,
            transform_time_invariant: true,
            perturbation_inventory_id: "forces-r1".to_owned(),
            perturbation_inventory_complete: true,
            evidence: EvidenceSet {
                mu: provider("mu"),
                chief_radius: provider("chief-radius"),
                relative_separation: provider("relative-separation"),
                differential_acceleration: provider("differential-acceleration"),
                transform_norm: provider("transform-norm"),
            },
        }
    }

    #[test]
    fn central_gravity_bound_is_positive_and_ready() {
        let result = derive(&input());
        assert_eq!(result.disposition, CurvatureDisposition::BoundReady);
        assert!(result.m_upper_scaled_per_s2.unwrap() > 0);
        assert!(result.bound_ready);
        assert!(!result.source_bytes_authenticated);
        assert!(!result.authority_granted);
    }

    #[test]
    fn valid_bound_does_not_overflow_from_avoidable_aggregate_products() {
        let mut value = input();
        value.mu_upper_mm3_per_s2 = 398_600_000_000_000_000_000_000;
        value.chief_radius_lower_mm = 7_000_000_000;
        value.relative_separation_upper_mm = 1_000_000;
        value.differential_acceleration_upper_mm_per_s2 = 1_000_000;
        value.transform_operator_norm_upper_scaled = 1_000;
        let result = derive(&value);
        assert_eq!(result.disposition, CurvatureDisposition::BoundReady);
        assert_eq!(result.scaled_perturbation_term, Some(1_000_000_000));
        assert!(result.m_upper_scaled_per_s2.unwrap() >= 1_000_000_000);
    }

    #[test]
    fn positive_differential_acceleration_increases_the_bound() {    #[test]
    fn positive_differential_acceleration_increases_the_bound() {
        let baseline = derive(&input());
        let mut higher = input();
        higher.differential_acceleration_upper_mm_per_s2 = 100;
        let increased = derive(&higher);
        assert!(increased.m_upper_scaled_per_s2.unwrap() > baseline.m_upper_scaled_per_s2.unwrap());
    }

    #[test]
    fn greater_transform_norm_does_not_reduce_the_bound() {
        let baseline = derive(&input());
        let mut higher = input();
        higher.transform_operator_norm_upper_scaled = 2_000;
        assert!(
            derive(&higher).m_upper_scaled_per_s2.unwrap()
                >= baseline.m_upper_scaled_per_s2.unwrap()
        );
    }

    #[test]
    fn larger_relative_separation_does_not_reduce_the_bound() {
        let baseline = derive(&input());
        let mut higher = input();
        higher.relative_separation_upper_mm = 2_000_000;
        assert!(
            derive(&higher).m_upper_scaled_per_s2.unwrap()
                >= baseline.m_upper_scaled_per_s2.unwrap()
        );
    }

    #[test]
    fn smaller_chief_radius_does_not_reduce_the_bound() {
        let baseline = derive(&input());
        let mut lower = input();
        lower.chief_radius_lower_mm = 6_900_000_000;
        assert!(
            derive(&lower).m_upper_scaled_per_s2.unwrap()
                >= baseline.m_upper_scaled_per_s2.unwrap()
        );
    }

    #[test]
    fn exact_nonintegral_rational_is_rounded_up() {
        let mut value = input();
        value.mu_upper_mm3_per_s2 = 1;
        value.chief_radius_lower_mm = 2;
        value.relative_separation_upper_mm = 1;
        value.transform_operator_norm_upper_scaled = 1;
        let result = derive(&value);
        assert_eq!(result.central_gravity_numerator, Some(2));
        assert_eq!(result.radial_domain_denominator, Some(1));
        assert_eq!(result.m_upper_scaled_per_s2, Some(2));
    }

    #[test]
    fn zero_relative_separation_retains_only_perturbation_term() {
        let mut value = input();
        value.relative_separation_upper_mm = 0;
        value.differential_acceleration_upper_mm_per_s2 = 3;
        let result = derive(&value);
        assert_eq!(result.central_gravity_numerator, Some(0));
        assert_eq!(
            result.differential_acceleration_numerator,
            Some(3 * result.radial_domain_denominator.unwrap())
        );
        assert_eq!(result.disposition, CurvatureDisposition::BoundReady);
    }

    #[test]
    fn nonpositive_radial_domain_is_out_of_scope() {
        let mut value = input();
        value.relative_separation_upper_mm = value.chief_radius_lower_mm;
        assert_eq!(derive(&value).disposition, CurvatureDisposition::OutOfScope);
    }

    #[test]
    fn incomplete_perturbation_inventory_is_indeterminate() {
        let mut value = input();
        value.perturbation_inventory_complete = false;
        assert_eq!(
            derive(&value).disposition,
            CurvatureDisposition::Indeterminate
        );
    }

    #[test]
    fn time_varying_transform_is_out_of_scope() {
        let mut value = input();
        value.transform_time_invariant = false;
        assert_eq!(derive(&value).disposition, CurvatureDisposition::OutOfScope);
    }

    #[test]
    fn stale_or_revoked_sources_never_produce_a_bound() {
        for state in [
            ProviderState::Unknown,
            ProviderState::Stale,
            ProviderState::Revoked,
        ] {
            let mut value = input();
            value.evidence.mu.state = state;
            assert_eq!(
                derive(&value).disposition,
                CurvatureDisposition::Indeterminate
            );
        }
    }

    #[test]
    fn failed_provider_is_not_a_bound() {
        let mut value = input();
        value.evidence.chief_radius.state = ProviderState::Fail;
        assert_eq!(
            derive(&value).disposition,
            CurvatureDisposition::ProviderFailed
        );
    }

    #[test]
    fn mismatched_provider_scope_is_out_of_scope() {
        let mut value = input();
        value.evidence.relative_separation.scope_id = "other-scope".to_owned();
        assert_eq!(derive(&value).disposition, CurvatureDisposition::OutOfScope);
    }

    #[test]
    fn malformed_digest_is_invalid() {
        let mut value = input();
        value.evidence.transform_norm.receipt_sha256 = "bad".to_owned();
        assert_eq!(
            derive(&value).disposition,
            CurvatureDisposition::InvalidInput
        );
    }

    #[test]
    fn u128_overflow_is_nonpass() {
        let mut value = input();
        value.mu_upper_mm3_per_s2 = u128::MAX;
        value.relative_separation_upper_mm = 0;
        value.chief_radius_lower_mm = u64::MAX;
        value.transform_operator_norm_upper_scaled = u64::MAX;
        assert_eq!(derive(&value).disposition, CurvatureDisposition::Overflow);
    }
}
