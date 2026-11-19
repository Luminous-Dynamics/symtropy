// SPDX-License-Identifier: AGPL-3.0-or-later
//! Calibrated marginal chance constraints for contact-aware locomotion.
//!
//! Five one-sided Gaussian predictive envelopes are combined with a
//! Bonferroni family-wise risk budget. This is a conditional one-step gate
//! under caller-supplied, calibrated Gaussian marginals; it is not a CBF,
//! forward-invariance proof, physical viability proof, or hardware safety case.

pub const RISK_SCALE_PPB: u64 = 1_000_000_000;
pub const CONSTRAINT_COUNT: usize = 5;
pub const MIN_FAMILYWISE_ALPHA_PPB: u64 = 50;
pub const MAX_FAMILYWISE_ALPHA_PPB: u64 = 500_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConstraintKind {
    Energy,
    SupportMargin,
    PeakNormalForce,
    ImpactImpulse,
    ActuatorLoadFraction,
}

impl ConstraintKind {
    fn index(self) -> usize {
        match self {
            Self::Energy => 0,
            Self::SupportMargin => 1,
            Self::PeakNormalForce => 2,
            Self::ImpactImpulse => 3,
            Self::ActuatorLoadFraction => 4,
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::Energy => "J",
            Self::SupportMargin => "m",
            Self::PeakNormalForce => "N",
            Self::ImpactImpulse => "N*s",
            Self::ActuatorLoadFraction => "ratio",
        }
    }

    fn is_lower_bounded(self) -> bool {
        self == Self::SupportMargin
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionFamily {
    GaussianMarginal,
    HeavyTailed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarianceAttribution {
    /// Optional sensor-noise contribution retained for audit only.
    pub sensor_variance: Option<f64>,
    /// Optional process-noise contribution retained for audit only.
    pub process_variance: Option<f64>,
    /// Optional model/discrepancy contribution retained for audit only.
    pub model_variance: Option<f64>,
    /// Optional signed cross-covariance correction. It is not folded into the
    /// total variance by this crate; the source predictor owns that calculation.
    pub covariance_adjustment: Option<f64>,
    pub provenance_digest: Option<String>,
}

impl VarianceAttribution {
    fn validate(&self) -> bool {
        let components = [
            self.sensor_variance,
            self.process_variance,
            self.model_variance,
            self.covariance_adjustment,
        ];
        let has_component = components.iter().any(Option::is_some);
        if !has_component {
            return true;
        }
        if self
            .provenance_digest
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return false;
        }
        if self
            .sensor_variance
            .into_iter()
            .chain(self.process_variance)
            .chain(self.model_variance)
            .any(|value| !value.is_finite() || value < 0.0)
        {
            return false;
        }
        self.covariance_adjustment.is_none_or(f64::is_finite)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintPrediction {
    pub action_id: String,
    pub kind: ConstraintKind,
    pub mean: f64,
    /// Total predictive marginal variance used by the chance bound. Optional
    /// component attribution is retained separately and is not blindly summed.
    pub total_variance: f64,
    pub unit: String,
    pub model_digest: String,
    pub operating_domain_digest: String,
    pub calibration_digest: String,
    pub predicted_at_ns: u64,
    pub valid_until_ns: u64,
    pub variance_attribution: VarianceAttribution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GaussianCalibrationReceipt {
    pub receipt_id: String,
    pub evidence_digest: String,
    pub model_digest: String,
    pub operating_domain_digest: String,
    pub calibration_digest: String,
    pub distribution_family: DistributionFamily,
    /// Upstream evidence claim that Gaussian marginal tails were checked for
    /// the declared operating stratum. This crate does not authenticate it.
    pub gaussian_tail_validation_passed: bool,
    /// Independent session/episode counts per constraint. Repeated ticks from
    /// one rollout do not qualify as independent units.
    pub independent_validation_units_by_constraint: [u64; CONSTRAINT_COUNT],
    /// Observed violations beyond each one-sided bound. V0 only admits zero.
    pub upper_tail_exceedances_by_constraint: [u64; CONSTRAINT_COUNT],
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionCandidate {
    pub action_id: String,
    /// Supplied by the separate contact-information estimator. The gate uses
    /// it only to rank candidates that have already passed every risk bound.
    pub expected_information_gain_nats: f64,
    pub predictions: Vec<ConstraintPrediction>,
    pub calibration_receipt: Option<GaussianCalibrationReceipt>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalLimits {
    pub energy_upper_joules: f64,
    pub support_margin_lower_metres: f64,
    pub peak_normal_force_upper_newtons: f64,
    pub impact_impulse_upper_newton_seconds: f64,
    pub actuator_load_fraction_upper: f64,
}

impl PhysicalLimits {
    pub fn validate(&self) -> Result<(), ChanceError> {
        let values = [
            self.energy_upper_joules,
            self.support_margin_lower_metres,
            self.peak_normal_force_upper_newtons,
            self.impact_impulse_upper_newton_seconds,
            self.actuator_load_fraction_upper,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ChanceError::NonFiniteLimit);
        }
        if self.energy_upper_joules <= 0.0
            || self.peak_normal_force_upper_newtons <= 0.0
            || self.impact_impulse_upper_newton_seconds <= 0.0
            || !(0.0..=1.0).contains(&self.actuator_load_fraction_upper)
        {
            return Err(ChanceError::InvalidLimit);
        }
        Ok(())
    }

    fn for_kind(&self, kind: ConstraintKind) -> f64 {
        match kind {
            ConstraintKind::Energy => self.energy_upper_joules,
            ConstraintKind::SupportMargin => self.support_margin_lower_metres,
            ConstraintKind::PeakNormalForce => self.peak_normal_force_upper_newtons,
            ConstraintKind::ImpactImpulse => self.impact_impulse_upper_newton_seconds,
            ConstraintKind::ActuatorLoadFraction => self.actuator_load_fraction_upper,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstentionReason {
    MissingCalibrationReceipt,
    InvalidCalibrationReceipt,
    UnverifiedCalibrationReceipt,
    ExpiredCalibrationReceipt,
    FutureCalibrationReceipt,
    UnsupportedDistributionFamily,
    GaussianTailCalibrationNotEstablished,
    NoIndependentValidationUnits,
    InsufficientIndependentValidationUnits,
    ObservedCalibrationTailExceedance,
    PredictionSetIncomplete,
    DuplicateConstraintPrediction,
    PredictionIdentityMismatch,
    OperatingDomainMismatch,
    CalibrationIdentityMismatch,
    PredictionNotYetAvailable,
    StalePrediction,
    InvalidPredictionValue,
    NegativePredictiveVariance,
    InvalidVarianceAttribution,
    UnitMismatch,
    InvalidInformationGain,
    InvalidRiskAllocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintViolation {
    EnergyUpperExceeded,
    SupportMarginLowerViolated,
    PeakNormalForceUpperExceeded,
    ImpactImpulseUpperExceeded,
    ActuatorLoadFractionUpperExceeded,
}

impl ConstraintViolation {
    fn from_kind(kind: ConstraintKind) -> Self {
        match kind {
            ConstraintKind::Energy => Self::EnergyUpperExceeded,
            ConstraintKind::SupportMargin => Self::SupportMarginLowerViolated,
            ConstraintKind::PeakNormalForce => Self::PeakNormalForceUpperExceeded,
            ConstraintKind::ImpactImpulse => Self::ImpactImpulseUpperExceeded,
            ConstraintKind::ActuatorLoadFraction => Self::ActuatorLoadFractionUpperExceeded,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateDisposition {
    Admitted,
    Rejected,
    Abstained,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintBound {
    pub kind: ConstraintKind,
    pub allocated_alpha_ppb: u64,
    pub upper_quantile: f64,
    pub mean: f64,
    pub standard_deviation: f64,
    /// Conservative upper bound for upper-bounded constraints and conservative
    /// lower bound for support margin.
    pub conservative_bound: f64,
    pub limit: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionAssessment {
    pub action_id: String,
    pub expected_information_gain_nats: f64,
    pub disposition: CandidateDisposition,
    pub abstention_reason: Option<AbstentionReason>,
    pub violations: Vec<ConstraintViolation>,
    pub bounds: Vec<ConstraintBound>,
    pub calibration_receipt_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChanceGateReport {
    pub familywise_alpha_ppb: u64,
    pub allocated_alpha_ppb: [u64; CONSTRAINT_COUNT],
    pub allocated_alpha_sum_ppb: u64,
    /// Only admitted actions appear here. Rejected and abstained candidates
    /// remain fully visible in assessments and can never be ranked by EIG.
    pub eligible_action_ids_by_information: Vec<String>,
    pub assessments: Vec<ActionAssessment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChanceError {
    EmptyDomainIdentity,
    InvalidFamilywiseAlpha,
    NonFiniteLimit,
    InvalidLimit,
    EmptyCandidateSet,
    EmptyActionId,
    DuplicateActionId,
}

fn allocate_bonferroni_alpha(
    familywise_alpha_ppb: u64,
) -> Result<[u64; CONSTRAINT_COUNT], ChanceError> {
    if !(MIN_FAMILYWISE_ALPHA_PPB..=MAX_FAMILYWISE_ALPHA_PPB).contains(&familywise_alpha_ppb) {
        return Err(ChanceError::InvalidFamilywiseAlpha);
    }
    let base = familywise_alpha_ppb / CONSTRAINT_COUNT as u64;
    let remainder = familywise_alpha_ppb % CONSTRAINT_COUNT as u64;
    let mut allocation = [base; CONSTRAINT_COUNT];
    for (index, value) in allocation.iter_mut().enumerate() {
        if (index as u64) < remainder {
            *value += 1;
        }
    }
    Ok(allocation)
}

/// Acklam's rational approximation for the inverse standard-normal CDF in the
/// lower tail. Input is represented in parts per billion to avoid cancellation
/// in 1 - alpha when calculating the upper one-sided quantile.
fn inverse_standard_normal_lower_tail(alpha_ppb: u64) -> Result<f64, AbstentionReason> {
    if alpha_ppb == 0 || alpha_ppb >= RISK_SCALE_PPB / 2 {
        return Err(AbstentionReason::InvalidRiskAllocation);
    }
    let p = alpha_ppb as f64 / RISK_SCALE_PPB as f64;
    if !p.is_finite() || p <= 0.0 || p >= 0.5 {
        return Err(AbstentionReason::InvalidCalibrationReceipt);
    }

    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];

    // Acklam uses a central rational approximation for p >= 0.02425 and a
    // logarithmic tail approximation below that split point.
    let value = if p < 0.024_25 {
        let q = (-2.0 * p.ln()).sqrt();
        let numerator = (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q) + C[5];
        let denominator = ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q) + 1.0;
        numerator / denominator
    } else {
        let q = p - 0.5;
        let r = q * q;
        let numerator = (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q;
        let denominator = ((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0;
        numerator / denominator
    };
    if value.is_finite() && value < 0.0 {
        Ok(value)
    } else {
        Err(AbstentionReason::InvalidCalibrationReceipt)
    }
}

/// Upper one-sided Gaussian quantile z = Phi^-1(1 - alpha), computed as
/// -Phi^-1(alpha) without subtracting a small alpha from one.
pub fn upper_normal_quantile(alpha_ppb: u64) -> Result<f64, AbstentionReason> {
    Ok(-inverse_standard_normal_lower_tail(alpha_ppb)?)
}

fn required_predictions(
    predictions: &[ConstraintPrediction],
) -> Result<[&ConstraintPrediction; CONSTRAINT_COUNT], AbstentionReason> {
    let mut ordered: [Option<&ConstraintPrediction>; CONSTRAINT_COUNT] = [None; CONSTRAINT_COUNT];
    for prediction in predictions {
        let index = prediction.kind.index();
        if ordered[index].is_some() {
            return Err(AbstentionReason::DuplicateConstraintPrediction);
        }
        ordered[index] = Some(prediction);
    }

    Ok([
        ordered[0].ok_or(AbstentionReason::PredictionSetIncomplete)?,
        ordered[1].ok_or(AbstentionReason::PredictionSetIncomplete)?,
        ordered[2].ok_or(AbstentionReason::PredictionSetIncomplete)?,
        ordered[3].ok_or(AbstentionReason::PredictionSetIncomplete)?,
        ordered[4].ok_or(AbstentionReason::PredictionSetIncomplete)?,
    ])
}

fn calibration_abstention_reason(
    candidate: &ActionCandidate,
    predictions: &[&ConstraintPrediction; CONSTRAINT_COUNT],
    expected_domain_digest: &str,
    allocation: &[u64; CONSTRAINT_COUNT],
    now_ns: u64,
) -> Result<String, AbstentionReason> {
    let receipt = candidate
        .calibration_receipt
        .as_ref()
        .ok_or(AbstentionReason::MissingCalibrationReceipt)?;

    if [
        receipt.receipt_id.as_str(),
        receipt.evidence_digest.as_str(),
        receipt.model_digest.as_str(),
        receipt.operating_domain_digest.as_str(),
        receipt.calibration_digest.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(AbstentionReason::InvalidCalibrationReceipt);
    }
    if !receipt.verified {
        return Err(AbstentionReason::UnverifiedCalibrationReceipt);
    }
    if receipt.valid_from_ns > receipt.valid_until_ns || now_ns > receipt.valid_until_ns {
        return Err(AbstentionReason::ExpiredCalibrationReceipt);
    }
    if now_ns < receipt.valid_from_ns {
        return Err(AbstentionReason::FutureCalibrationReceipt);
    }
    if receipt.distribution_family != DistributionFamily::GaussianMarginal {
        return Err(AbstentionReason::UnsupportedDistributionFamily);
    }
    if !receipt.gaussian_tail_validation_passed {
        return Err(AbstentionReason::GaussianTailCalibrationNotEstablished);
    }
    for (index, (units, exceedances)) in receipt
        .independent_validation_units_by_constraint
        .iter()
        .copied()
        .zip(receipt.upper_tail_exceedances_by_constraint.iter().copied())
        .enumerate()
    {
        if units == 0 {
            return Err(AbstentionReason::NoIndependentValidationUnits);
        }
        if exceedances > 0 {
            return Err(AbstentionReason::ObservedCalibrationTailExceedance);
        }
        // Exact one-sided 95% Clopper-Pearson upper bound for zero observed
        // Bernoulli tail events: 1 - 0.05^(1/n). exp_m1 avoids cancellation.
        let upper = -((0.05_f64.ln()) / units as f64).exp_m1();
        let allowed = allocation[index] as f64 / RISK_SCALE_PPB as f64;
        if !upper.is_finite() || upper > allowed {
            return Err(AbstentionReason::InsufficientIndependentValidationUnits);
        }
    }
    if receipt.operating_domain_digest != expected_domain_digest {
        return Err(AbstentionReason::OperatingDomainMismatch);
    }

    for prediction in predictions {
        if prediction.action_id != candidate.action_id
            || prediction.model_digest != receipt.model_digest
        {
            return Err(AbstentionReason::PredictionIdentityMismatch);
        }
        if prediction.operating_domain_digest != receipt.operating_domain_digest {
            return Err(AbstentionReason::OperatingDomainMismatch);
        }
        if prediction.calibration_digest != receipt.calibration_digest {
            return Err(AbstentionReason::CalibrationIdentityMismatch);
        }
        if prediction.predicted_at_ns > now_ns {
            return Err(AbstentionReason::PredictionNotYetAvailable);
        }
        if prediction.valid_until_ns < now_ns
            || prediction.predicted_at_ns > prediction.valid_until_ns
        {
            return Err(AbstentionReason::StalePrediction);
        }
        if !prediction.mean.is_finite() || !prediction.total_variance.is_finite() {
            return Err(AbstentionReason::InvalidPredictionValue);
        }
        if prediction.total_variance < 0.0 {
            return Err(AbstentionReason::NegativePredictiveVariance);
        }
        if prediction.unit != prediction.kind.unit() {
            return Err(AbstentionReason::UnitMismatch);
        }
        if !prediction.variance_attribution.validate() {
            return Err(AbstentionReason::InvalidVarianceAttribution);
        }
    }

    Ok(receipt.receipt_id.clone())
}

fn conservative_bound(
    prediction: &ConstraintPrediction,
    alpha_ppb: u64,
    limit: f64,
) -> Result<ConstraintBound, AbstentionReason> {
    let z = upper_normal_quantile(alpha_ppb)?;
    let standard_deviation = prediction.total_variance.sqrt();
    let offset = z * standard_deviation;
    let conservative_bound = if prediction.kind.is_lower_bounded() {
        prediction.mean - offset
    } else {
        prediction.mean + offset
    };
    if !standard_deviation.is_finite() || !conservative_bound.is_finite() {
        return Err(AbstentionReason::InvalidPredictionValue);
    }
    let passed = if prediction.kind.is_lower_bounded() {
        conservative_bound >= limit
    } else {
        conservative_bound <= limit
    };
    Ok(ConstraintBound {
        kind: prediction.kind,
        allocated_alpha_ppb: alpha_ppb,
        upper_quantile: z,
        mean: prediction.mean,
        standard_deviation,
        conservative_bound,
        limit,
        passed,
    })
}

fn assess_candidate(
    candidate: &ActionCandidate,
    limits: &PhysicalLimits,
    allocation: &[u64; CONSTRAINT_COUNT],
    expected_domain_digest: &str,
    now_ns: u64,
) -> ActionAssessment {
    let base = || ActionAssessment {
        action_id: candidate.action_id.clone(),
        expected_information_gain_nats: candidate.expected_information_gain_nats,
        disposition: CandidateDisposition::Abstained,
        abstention_reason: None,
        violations: Vec::new(),
        bounds: Vec::new(),
        calibration_receipt_id: None,
    };

    if !candidate.expected_information_gain_nats.is_finite()
        || candidate.expected_information_gain_nats < 0.0
    {
        let mut assessment = base();
        assessment.abstention_reason = Some(AbstentionReason::InvalidInformationGain);
        return assessment;
    }

    let ordered = match required_predictions(&candidate.predictions) {
        Ok(ordered) => ordered,
        Err(reason) => {
            let mut assessment = base();
            assessment.abstention_reason = Some(reason);
            return assessment;
        }
    };

    let receipt_id = match calibration_abstention_reason(
        candidate,
        &ordered,
        expected_domain_digest,
        allocation,
        now_ns,
    ) {
        Ok(receipt_id) => receipt_id,
        Err(reason) => {
            let mut assessment = base();
            assessment.abstention_reason = Some(reason);
            return assessment;
        }
    };

    let mut assessment = base();
    assessment.calibration_receipt_id = Some(receipt_id);
    for index in 0..CONSTRAINT_COUNT {
        match conservative_bound(
            ordered[index],
            allocation[index],
            limits.for_kind(ordered[index].kind),
        ) {
            Ok(bound) => {
                if !bound.passed {
                    assessment
                        .violations
                        .push(ConstraintViolation::from_kind(bound.kind));
                }
                assessment.bounds.push(bound);
            }
            Err(reason) => {
                assessment.disposition = CandidateDisposition::Abstained;
                assessment.abstention_reason = Some(reason);
                assessment.bounds.clear();
                assessment.violations.clear();
                return assessment;
            }
        }
    }

    if assessment.violations.is_empty() {
        assessment.disposition = CandidateDisposition::Admitted;
    } else {
        assessment.disposition = CandidateDisposition::Rejected;
    }
    assessment
}

/// Evaluate candidate actions under five one-sided marginal Gaussian bounds.
///
/// The Bonferroni allocation sums exactly to the supplied family-wise alpha.
/// No independence assumption is made between the five physical constraints:
/// if each marginal tail bound is valid, the union bound controls their union.
/// Only candidates that pass all five bounds are ranked by expected
/// information gain. Rejected/abstained candidates remain in the report.
/// Convenience API for a one-step family-wise budget. Allocates that budget
/// equally across the five constraints using deterministic integer ppb.
pub fn evaluate_actions(
    candidates: &[ActionCandidate],
    limits: &PhysicalLimits,
    familywise_alpha_ppb: u64,
    expected_domain_digest: &str,
    now_ns: u64,
) -> Result<ChanceGateReport, ChanceError> {
    let allocation = allocate_bonferroni_alpha(familywise_alpha_ppb)?;
    evaluate_actions_with_allocation(
        candidates,
        limits,
        allocation,
        expected_domain_digest,
        now_ns,
    )
}

/// Evaluate actions with an exact preallocated alpha for each constraint.
///
/// This is the composition API for the finite-horizon ledger in #4245:
/// callers pass the exact [step][constraint] allocation returned by the
/// immutable horizon risk plan. The function does not re-split, round or
/// redistribute those five values.
pub fn evaluate_actions_with_allocation(
    candidates: &[ActionCandidate],
    limits: &PhysicalLimits,
    allocation: [u64; CONSTRAINT_COUNT],
    expected_domain_digest: &str,
    now_ns: u64,
) -> Result<ChanceGateReport, ChanceError> {
    if candidates.is_empty() {
        return Err(ChanceError::EmptyCandidateSet);
    }
    if expected_domain_digest.trim().is_empty() {
        return Err(ChanceError::EmptyDomainIdentity);
    }
    limits.validate()?;
    if allocation.contains(&0) {
        return Err(ChanceError::InvalidFamilywiseAlpha);
    }
    let familywise_alpha_ppb = allocation
        .iter()
        .try_fold(0_u64, |sum, value| sum.checked_add(*value))
        .ok_or(ChanceError::InvalidFamilywiseAlpha)?;
    if !(MIN_FAMILYWISE_ALPHA_PPB..=MAX_FAMILYWISE_ALPHA_PPB).contains(&familywise_alpha_ppb) {
        return Err(ChanceError::InvalidFamilywiseAlpha);
    }

    let mut ids = std::collections::BTreeSet::new();
    for candidate in candidates {
        if candidate.action_id.trim().is_empty() {
            return Err(ChanceError::EmptyActionId);
        }
        if !ids.insert(candidate.action_id.clone()) {
            return Err(ChanceError::DuplicateActionId);
        }
    }

    let mut assessments = candidates
        .iter()
        .map(|candidate| {
            assess_candidate(
                candidate,
                limits,
                &allocation,
                expected_domain_digest,
                now_ns,
            )
        })
        .collect::<Vec<_>>();
    assessments.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let mut eligible = assessments
        .iter()
        .filter(|assessment| assessment.disposition == CandidateDisposition::Admitted)
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        right
            .expected_information_gain_nats
            .total_cmp(&left.expected_information_gain_nats)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });

    let allocated_alpha_sum_ppb = allocation.iter().sum();
    Ok(ChanceGateReport {
        familywise_alpha_ppb,
        allocated_alpha_ppb: allocation,
        allocated_alpha_sum_ppb,
        eligible_action_ids_by_information: eligible
            .iter()
            .map(|assessment| assessment.action_id.clone())
            .collect(),
        assessments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> PhysicalLimits {
        PhysicalLimits {
            energy_upper_joules: 10.0,
            support_margin_lower_metres: 0.0,
            peak_normal_force_upper_newtons: 100.0,
            impact_impulse_upper_newton_seconds: 5.0,
            actuator_load_fraction_upper: 0.9,
        }
    }

    fn prediction(
        action_id: &str,
        kind: ConstraintKind,
        mean: f64,
        variance: f64,
    ) -> ConstraintPrediction {
        ConstraintPrediction {
            action_id: action_id.to_owned(),
            kind,
            mean,
            total_variance: variance,
            unit: kind.unit().to_owned(),
            model_digest: "model-v1".to_owned(),
            operating_domain_digest: "rough-terrain-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            predicted_at_ns: 1_000,
            valid_until_ns: 5_000,
            variance_attribution: VarianceAttribution {
                sensor_variance: Some(variance * 0.4),
                process_variance: Some(variance * 0.3),
                model_variance: Some(variance * 0.2),
                covariance_adjustment: Some(variance * 0.1),
                provenance_digest: Some("variance-provenance-v1".to_owned()),
            },
        }
    }

    fn receipt() -> GaussianCalibrationReceipt {
        GaussianCalibrationReceipt {
            receipt_id: "gaussian-calibration-receipt-v1".to_owned(),
            evidence_digest: "heldout-evidence-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            operating_domain_digest: "rough-terrain-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            distribution_family: DistributionFamily::GaussianMarginal,
            gaussian_tail_validation_passed: true,
            independent_validation_units_by_constraint: [500; CONSTRAINT_COUNT],
            upper_tail_exceedances_by_constraint: [0; CONSTRAINT_COUNT],
            valid_from_ns: 500,
            valid_until_ns: 10_000,
            verified: true,
        }
    }

    fn candidate(action_id: &str, eig: f64) -> ActionCandidate {
        ActionCandidate {
            action_id: action_id.to_owned(),
            expected_information_gain_nats: eig,
            predictions: vec![
                prediction(action_id, ConstraintKind::Energy, 5.0, 1.0),
                prediction(action_id, ConstraintKind::SupportMargin, 2.0, 0.25),
                prediction(action_id, ConstraintKind::PeakNormalForce, 50.0, 25.0),
                prediction(action_id, ConstraintKind::ImpactImpulse, 2.0, 0.25),
                prediction(action_id, ConstraintKind::ActuatorLoadFraction, 0.5, 0.01),
            ],
            calibration_receipt: Some(receipt()),
        }
    }

    #[test]
    fn acklam_upper_quantiles_match_known_standard_normal_values() {
        assert!(
            (upper_normal_quantile(10_000_000).unwrap() - 2.326_347_874_040_840_8).abs() < 1.0e-8
        );
        assert!(
            (upper_normal_quantile(50_000_000).unwrap() - 1.644_853_626_951_472_2).abs() < 1.0e-8
        );
        assert!((upper_normal_quantile(1_000_000).unwrap() - 3.090_232_306_167_813).abs() < 1.0e-8);
        assert!((upper_normal_quantile(10).unwrap() - 5.612_001_244_174_789).abs() < 1.0e-7);
    }

    #[test]
    fn bonferroni_allocation_sums_exactly_to_familywise_alpha() {
        let allocation = allocate_bonferroni_alpha(50_000_003).unwrap();
        assert_eq!(
            allocation,
            [10_000_001, 10_000_001, 10_000_001, 10_000_000, 10_000_000]
        );
        assert_eq!(allocation.iter().sum::<u64>(), 50_000_003);
    }

    #[test]
    fn calibrated_safe_action_is_admitted_with_all_five_bounds() {
        let report = evaluate_actions(
            &[candidate("safe", 0.1)],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        assert_eq!(report.allocated_alpha_sum_ppb, 50_000_000);
        assert_eq!(report.eligible_action_ids_by_information, vec!["safe"]);
        assert_eq!(
            report.assessments[0].disposition,
            CandidateDisposition::Admitted
        );
        assert_eq!(report.assessments[0].bounds.len(), CONSTRAINT_COUNT);
        assert!(
            report.assessments[0]
                .bounds
                .iter()
                .all(|bound| bound.passed)
        );
    }

    #[test]
    fn isolated_violation_of_each_constraint_rejects_action() {
        let mutations: [(ConstraintKind, f64, f64); 5] = [
            (ConstraintKind::Energy, 9.0, 1.0),
            (ConstraintKind::SupportMargin, -1.0, 0.25),
            (ConstraintKind::PeakNormalForce, 110.0, 25.0),
            (ConstraintKind::ImpactImpulse, 6.0, 0.25),
            (ConstraintKind::ActuatorLoadFraction, 1.0, 0.01),
        ];
        for (kind, mean, variance) in mutations {
            let mut c = candidate("candidate", 1.0);
            let item = c
                .predictions
                .iter_mut()
                .find(|item| item.kind == kind)
                .unwrap();
            item.mean = mean;
            item.total_variance = variance;
            // Variance attribution is metadata, not a replacement total.
            let report =
                evaluate_actions(&[c], &limits(), 50_000_000, "rough-terrain-v1", 2_000).unwrap();
            assert_eq!(
                report.assessments[0].disposition,
                CandidateDisposition::Rejected
            );
            assert_eq!(
                report.assessments[0].violations,
                vec![ConstraintViolation::from_kind(kind)]
            );
            assert!(report.eligible_action_ids_by_information.is_empty());
        }
    }

    #[test]
    fn high_information_unsafe_probe_is_filtered_before_safe_action_ranking() {
        let mut unsafe_high_eig = candidate("unsafe-high-eig", 100.0);
        unsafe_high_eig
            .predictions
            .iter_mut()
            .find(|item| item.kind == ConstraintKind::PeakNormalForce)
            .unwrap()
            .mean = 200.0;
        let safe_low_eig = candidate("safe-low-eig", 0.01);
        let report = evaluate_actions(
            &[safe_low_eig, unsafe_high_eig],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        assert_eq!(
            report.eligible_action_ids_by_information,
            vec!["safe-low-eig".to_owned()]
        );
        let unsafe_assessment = report
            .assessments
            .iter()
            .find(|a| a.action_id == "unsafe-high-eig")
            .unwrap();
        assert_eq!(
            unsafe_assessment.disposition,
            CandidateDisposition::Rejected
        );
        assert_eq!(
            unsafe_assessment.violations,
            vec![ConstraintViolation::PeakNormalForceUpperExceeded]
        );
    }

    #[test]
    fn no_independence_assumption_is_required_between_physical_constraints() {
        let report = evaluate_actions(
            &[candidate("safe", 0.0)],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        // Alpha is split across five marginal events; their joint dependence is
        // unrestricted by the union bound.
        assert_eq!(report.allocated_alpha_ppb, [10_000_000; 5]);
        assert!(report.allocated_alpha_sum_ppb <= report.familywise_alpha_ppb);
    }

    #[test]
    fn zero_variance_is_a_deterministic_bound() {
        let mut c = candidate("deterministic", 0.0);
        for item in &mut c.predictions {
            item.total_variance = 0.0;
        }
        let report =
            evaluate_actions(&[c], &limits(), 50_000_000, "rough-terrain-v1", 2_000).unwrap();
        assert_eq!(
            report.assessments[0].disposition,
            CandidateDisposition::Admitted
        );
        assert!(
            report.assessments[0]
                .bounds
                .iter()
                .all(|bound| bound.standard_deviation == 0.0)
        );
    }

    #[test]
    fn missing_receipt_and_unverified_receipt_abstain() {
        let mut missing = candidate("missing", 1.0);
        missing.calibration_receipt = None;
        let mut unverified = candidate("unverified", 1.0);
        unverified.calibration_receipt.as_mut().unwrap().verified = false;
        let report = evaluate_actions(
            &[missing, unverified],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        for assessment in report.assessments {
            assert_eq!(assessment.disposition, CandidateDisposition::Abstained);
            assert!(assessment.bounds.is_empty());
        }
        assert!(report.eligible_action_ids_by_information.is_empty());
    }

    #[test]
    fn unsupported_heavy_tailed_or_unvalidated_gaussian_forecast_abstains() {
        let mut heavy = candidate("heavy", 1.0);
        heavy
            .calibration_receipt
            .as_mut()
            .unwrap()
            .distribution_family = DistributionFamily::HeavyTailed;
        let mut unvalidated = candidate("unvalidated", 1.0);
        unvalidated
            .calibration_receipt
            .as_mut()
            .unwrap()
            .gaussian_tail_validation_passed = false;
        let report = evaluate_actions(
            &[heavy, unvalidated],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        assert_eq!(
            report.assessments[0].abstention_reason,
            Some(AbstentionReason::UnsupportedDistributionFamily)
        );
        assert_eq!(
            report.assessments[1].abstention_reason,
            Some(AbstentionReason::GaussianTailCalibrationNotEstablished)
        );
        assert!(report.eligible_action_ids_by_information.is_empty());
    }

    #[test]
    fn stale_or_wrong_domain_calibration_abstains() {
        let mut stale = candidate("stale", 1.0);
        stale.calibration_receipt.as_mut().unwrap().valid_until_ns = 1_999;
        let wrong_domain = {
            let mut c = candidate("wrong-domain", 1.0);
            c.calibration_receipt
                .as_mut()
                .unwrap()
                .operating_domain_digest = "flat-floor-v1".to_owned();
            c
        };
        let report = evaluate_actions(
            &[stale, wrong_domain],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        assert_eq!(
            report.assessments[0].abstention_reason,
            Some(AbstentionReason::ExpiredCalibrationReceipt)
        );
        assert_eq!(
            report.assessments[1].abstention_reason,
            Some(AbstentionReason::OperatingDomainMismatch)
        );
    }

    #[test]
    fn prediction_identity_unit_and_variance_errors_abstain() {
        let mut wrong_unit = candidate("wrong-unit", 1.0);
        wrong_unit.predictions[0].unit = "kJ".to_owned();
        let mut wrong_action = candidate("wrong-action", 1.0);
        wrong_action.predictions[0].action_id = "other".to_owned();
        let mut negative_variance = candidate("negative", 1.0);
        negative_variance.predictions[0].total_variance = -1.0;
        let report = evaluate_actions(
            &[wrong_unit, wrong_action, negative_variance],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        let reason_for = |action_id: &str| {
            report
                .assessments
                .iter()
                .find(|assessment| assessment.action_id == action_id)
                .unwrap()
                .abstention_reason
        };
        assert_eq!(
            reason_for("wrong-unit"),
            Some(AbstentionReason::UnitMismatch)
        );
        assert_eq!(
            reason_for("wrong-action"),
            Some(AbstentionReason::PredictionIdentityMismatch)
        );
        assert_eq!(
            reason_for("negative"),
            Some(AbstentionReason::NegativePredictiveVariance)
        );
    }

    #[test]
    fn strict_alpha_cannot_admit_an_action_rejected_at_looser_alpha() {
        let mut marginal = candidate("marginal", 1.0);
        marginal
            .predictions
            .iter_mut()
            .find(|p| p.kind == ConstraintKind::Energy)
            .unwrap()
            .mean = 7.4;
        marginal
            .predictions
            .iter_mut()
            .find(|p| p.kind == ConstraintKind::Energy)
            .unwrap()
            .total_variance = 1.0;
        let loose = evaluate_actions(
            &[marginal.clone()],
            &limits(),
            100_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        let strict = evaluate_actions(
            &[marginal],
            &limits(),
            10_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        assert!(
            loose
                .eligible_action_ids_by_information
                .contains(&"marginal".to_owned())
        );
        assert!(
            !strict
                .eligible_action_ids_by_information
                .contains(&"marginal".to_owned())
        );
    }

    #[test]
    fn candidate_and_prediction_order_does_not_change_assessment() {
        let first = vec![candidate("a", 0.5), candidate("b", 0.9)];
        let mut second = vec![candidate("b", 0.9), candidate("a", 0.5)];
        for c in &mut second {
            c.predictions.reverse();
        }
        let one =
            evaluate_actions(&first, &limits(), 50_000_000, "rough-terrain-v1", 2_000).unwrap();
        let two =
            evaluate_actions(&second, &limits(), 50_000_000, "rough-terrain-v1", 2_000).unwrap();
        assert_eq!(one, two);
    }

    #[test]
    fn variance_components_are_preserved_but_not_blindly_reaggregated() {
        let mut c = candidate("variance-source", 0.1);
        let energy = c
            .predictions
            .iter_mut()
            .find(|p| p.kind == ConstraintKind::Energy)
            .unwrap();
        energy.variance_attribution.sensor_variance = Some(0.2);
        energy.variance_attribution.process_variance = Some(0.2);
        energy.variance_attribution.model_variance = Some(0.2);
        energy.variance_attribution.covariance_adjustment = Some(0.4);
        let report =
            evaluate_actions(&[c], &limits(), 50_000_000, "rough-terrain-v1", 2_000).unwrap();
        assert_eq!(
            report.assessments[0].disposition,
            CandidateDisposition::Admitted
        );
        assert_eq!(report.assessments[0].bounds[0].standard_deviation, 1.0);
    }

    #[test]
    fn global_risk_budget_and_physical_limits_fail_closed() {
        assert_eq!(
            evaluate_actions(
                &[candidate("a", 0.1)],
                &limits(),
                0,
                "rough-terrain-v1",
                2_000
            ),
            Err(ChanceError::InvalidFamilywiseAlpha)
        );
        assert_eq!(
            evaluate_actions(
                &[candidate("a", 0.1)],
                &limits(),
                500_000_001,
                "rough-terrain-v1",
                2_000
            ),
            Err(ChanceError::InvalidFamilywiseAlpha)
        );
        let mut invalid_limits = limits();
        invalid_limits.actuator_load_fraction_upper = 1.5;
        assert_eq!(
            evaluate_actions(
                &[candidate("a", 0.1)],
                &invalid_limits,
                50_000_000,
                "rough-terrain-v1",
                2_000
            ),
            Err(ChanceError::InvalidLimit)
        );
    }

    #[test]
    fn empty_prediction_set_abstains_without_panicking() {
        let mut incomplete = candidate("incomplete", 0.1);
        incomplete.predictions.clear();
        let report = evaluate_actions(
            &[incomplete],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        assert_eq!(
            report.assessments[0].disposition,
            CandidateDisposition::Abstained
        );
        assert_eq!(
            report.assessments[0].abstention_reason,
            Some(AbstentionReason::PredictionSetIncomplete)
        );
    }

    #[test]
    fn public_quantile_function_rejects_invalid_tail_probabilities() {
        assert_eq!(
            upper_normal_quantile(0),
            Err(AbstentionReason::InvalidRiskAllocation)
        );
        assert_eq!(
            upper_normal_quantile(RISK_SCALE_PPB / 2),
            Err(AbstentionReason::InvalidRiskAllocation)
        );
    }

    #[test]
    fn independent_zero_tail_validation_uses_exact_95_percent_upper_bound() {
        let mut underpowered = candidate("underpowered", 0.1);
        underpowered
            .calibration_receipt
            .as_mut()
            .unwrap()
            .independent_validation_units_by_constraint = [298; CONSTRAINT_COUNT];
        let mut adequately_powered = candidate("adequately-powered", 0.1);
        adequately_powered
            .calibration_receipt
            .as_mut()
            .unwrap()
            .independent_validation_units_by_constraint = [299; CONSTRAINT_COUNT];

        let report = evaluate_actions(
            &[underpowered, adequately_powered],
            &limits(),
            50_000_000,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();
        let underpowered = report
            .assessments
            .iter()
            .find(|assessment| assessment.action_id == "underpowered")
            .unwrap();
        let adequately_powered = report
            .assessments
            .iter()
            .find(|assessment| assessment.action_id == "adequately-powered")
            .unwrap();
        assert_eq!(
            underpowered.abstention_reason,
            Some(AbstentionReason::InsufficientIndependentValidationUnits)
        );
        assert_eq!(
            adequately_powered.disposition,
            CandidateDisposition::Admitted
        );
    }

    #[test]
    fn any_observed_validation_tail_event_abstains_in_zero_event_v0() {
        let mut c = candidate("tail-event", 1.0);
        c.calibration_receipt
            .as_mut()
            .unwrap()
            .upper_tail_exceedances_by_constraint[2] = 1;
        let report =
            evaluate_actions(&[c], &limits(), 50_000_000, "rough-terrain-v1", 2_000).unwrap();
        assert_eq!(
            report.assessments[0].abstention_reason,
            Some(AbstentionReason::ObservedCalibrationTailExceedance)
        );
        assert!(report.eligible_action_ids_by_information.is_empty());
    }

    #[test]
    fn exact_horizon_allocation_is_consumed_without_redistribution() {
        let mut c = candidate("horizon-step", 0.5);
        c.calibration_receipt
            .as_mut()
            .unwrap()
            .independent_validation_units_by_constraint = [1_000; CONSTRAINT_COUNT];
        let allocation = [3_333_334, 3_333_334, 3_333_333, 3_333_333, 3_333_333];
        let report = evaluate_actions_with_allocation(
            &[c],
            &limits(),
            allocation,
            "rough-terrain-v1",
            2_000,
        )
        .unwrap();

        assert_eq!(report.allocated_alpha_ppb, allocation);
        assert_eq!(report.allocated_alpha_sum_ppb, allocation.iter().sum());
        assert_eq!(report.familywise_alpha_ppb, 16_666_667);
        assert_eq!(
            report.assessments[0]
                .bounds
                .iter()
                .map(|bound| bound.allocated_alpha_ppb)
                .collect::<Vec<_>>(),
            allocation
        );
    }

    #[test]
    fn explicit_horizon_allocation_rejects_zero_or_out_of_range_budget() {
        let zero_event = [10, 10, 10, 10, 0];
        assert_eq!(
            evaluate_actions_with_allocation(
                &[candidate("zero-event", 0.1)],
                &limits(),
                zero_event,
                "rough-terrain-v1",
                2_000,
            ),
            Err(ChanceError::InvalidFamilywiseAlpha)
        );

        let too_small = [9, 9, 9, 9, 9];
        assert_eq!(
            evaluate_actions_with_allocation(
                &[candidate("too-small", 0.1)],
                &limits(),
                too_small,
                "rough-terrain-v1",
                2_000,
            ),
            Err(ChanceError::InvalidFamilywiseAlpha)
        );
    }

    #[test]
    fn candidate_duplicates_are_rejected() {
        assert_eq!(
            evaluate_actions(
                &[candidate("same", 0.1), candidate("same", 0.2)],
                &limits(),
                50_000_000,
                "rough-terrain-v1",
                2_000,
            ),
            Err(ChanceError::DuplicateActionId)
        );
    }
}
