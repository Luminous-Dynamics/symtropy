//! Bias-aware routing among gradient estimators for contact-rich optimization.
//!
//! This module is an estimator-selection baseline, not a gradient implementation.
//! It deliberately selects one compatible estimator per update; it never
//! averages gradients from different objectives, branches, coordinate systems,
//! or estimand definitions. Multi-branch, grazing, and unresolved cases abstain
//! rather than forcing a single Jacobian.

pub const GRADIENT_ROUTING_SCHEMA: &str = "SYMTROPY_GRADIENT_ROUTING.v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimatorKind {
    /// Derivative valid only on an exact identified contact/event branch.
    BranchLocalExact,
    /// Backward-only mode-bridging derivative (e.g. central-path sensitivity).
    ModeBridgingSurrogate,
    /// Score-function estimator such as REINFORCE.
    ScoreFunction,
    /// Finite-difference or other zeroth-order estimate.
    ZerothOrder,
}

impl EstimatorKind {
    fn tag(self) -> &'static str {
        match self {
            Self::BranchLocalExact => "BRANCH_LOCAL_EXACT",
            Self::ModeBridgingSurrogate => "MODE_BRIDGING_SURROGATE",
            Self::ScoreFunction => "SCORE_FUNCTION",
            Self::ZerothOrder => "ZEROTH_ORDER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonsmoothReason {
    ActiveSetChange,
    StickSlipTransition,
    ContactMakeOrBreak,
    EventSurfaceCrossing,
    EventOrderAmbiguity,
    EmpiricallyDetectedDiscontinuity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingularRegimeReason {
    Grazing,
    IllConditioned,
    UnresolvedEventOrder,
    NonUniqueContactResponse,
    MissingBranchEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactRegime {
    /// A single branch is currently certified and the proposed update remains
    /// within its declared validity region.
    SmoothSingleBranch {
        branch_id: String,
        branch_evidence_digest: String,
        validity_region_digest: String,
    },
    /// A detected nonsmooth boundary where a branch-local derivative must not
    /// be treated as a derivative toward a neighboring mode.
    Nonsmooth {
        reason: NonsmoothReason,
    },
    /// More than one physically admissible branch is retained. This v0 router
    /// does not collapse a set-valued response to one vector.
    MultiBranch {
        branch_ids: Vec<String>,
    },
    /// No single-gradient update is admitted at a grazing, singular, or
    /// unresolved event/contact state.
    SingularOrUnresolved {
        reason: SingularRegimeReason,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientEstimatorCandidate {
    pub estimator_id: String,
    pub kind: EstimatorKind,
    /// Objective/estimand identity. Mixing objectives is invalid even if the
    /// gradient coordinates happen to look compatible.
    pub objective_digest: String,
    /// Coordinates and vector values use this exact declared order.
    pub coordinate_ids: Vec<String>,
    pub gradient: Vec<f64>,
    /// Per-coordinate variance estimates from the estimator's sampling process.
    pub variance: Vec<f64>,
    /// Digest of the independently reproducible variance calculation / samples.
    pub variance_evidence_digest: String,
    /// Per-coordinate absolute-bias upper bounds. Unknown bias is not zero.
    pub absolute_bias_bound: Vec<f64>,
    /// Digest of the independent bound/calibration procedure for bias.
    pub bias_bound_evidence_digest: String,
    /// Comparable caller-declared cost units (e.g. rollout-equivalents).
    pub cost_units: f64,
    /// Required only for BranchLocalExact.
    pub branch_id: Option<String>,
    pub branch_evidence_digest: Option<String>,
    pub validity_region_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EstimatorRouterConfig {
    pub max_abs_bias_bound: f64,
    pub max_mse_upper_bound: f64,
    /// Adds cost penalty to the risk score. Zero means select by risk only.
    pub cost_weight: f64,
    pub allow_mode_bridging_surrogates: bool,
    pub allow_score_function: bool,
    pub allow_zeroth_order: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstimatorRoutingContext {
    pub objective_digest: String,
    pub coordinate_ids: Vec<String>,
    pub regime: ContactRegime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedEstimator {
    pub estimator_id: String,
    pub kind: EstimatorKind,
    pub objective_digest: String,
    pub coordinate_ids: Vec<String>,
    /// The actual selected gradient, in exactly coordinate_ids order.
    pub gradient: Vec<f64>,
    pub variance: Vec<f64>,
    pub absolute_bias_bound: Vec<f64>,
    pub branch_id: Option<String>,
    pub branch_evidence_digest: Option<String>,
    pub validity_region_digest: Option<String>,
    /// Regime evidence that governed admission of the selected estimator.
    pub regime: ContactRegime,
    /// Exact router thresholds and admissibility switches used for this decision.
    pub routing_configuration: EstimatorRouterConfig,
    pub variance_evidence_digest: String,
    pub bias_bound_evidence_digest: String,
    /// Mean per-coordinate upper bound: bias_bound^2 + variance.
    pub mean_mse_upper_bound: f64,
    /// MSE upper bound plus declared compute penalty.
    pub routing_score: f64,
    pub cost_units: f64,
}

impl SelectedEstimator {
    /// Deterministic, length-prefixed bytes for auditing or hashing the exact
    /// selected estimate and the configuration that admitted it.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();
        push_string(&mut output, GRADIENT_ROUTING_SCHEMA);
        push_string(&mut output, &self.estimator_id);
        push_string(&mut output, self.kind.tag());
        push_string(&mut output, &self.objective_digest);

        push_len(&mut output, self.coordinate_ids.len());
        for (index, coordinate_id) in self.coordinate_ids.iter().enumerate() {
            push_string(&mut output, coordinate_id);
            push_f64(&mut output, self.gradient[index]);
            push_f64(&mut output, self.variance[index]);
            push_f64(&mut output, self.absolute_bias_bound[index]);
        }

        push_optional_string(&mut output, self.branch_id.as_deref());
        push_optional_string(&mut output, self.branch_evidence_digest.as_deref());
        push_optional_string(&mut output, self.validity_region_digest.as_deref());
        push_string(&mut output, self.variance_evidence_digest.as_str());
        push_string(&mut output, self.bias_bound_evidence_digest.as_str());
        encode_regime(&mut output, &self.regime);

        push_f64(&mut output, self.routing_configuration.max_abs_bias_bound);
        push_f64(&mut output, self.routing_configuration.max_mse_upper_bound);
        push_f64(&mut output, self.routing_configuration.cost_weight);
        output.push(u8::from(self.routing_configuration.allow_mode_bridging_surrogates));
        output.push(u8::from(self.routing_configuration.allow_score_function));
        output.push(u8::from(self.routing_configuration.allow_zeroth_order));

        push_f64(&mut output, self.mean_mse_upper_bound);
        push_f64(&mut output, self.routing_score);
        push_f64(&mut output, self.cost_units);
        output
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingAbstention {
    NoAdmissibleEstimator,
    MultiBranchRequiresSetValuedUpdate,
    GrazingOrSingular,
    UnresolvedEventOrder,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EstimatorDecision {
    Selected(SelectedEstimator),
    Abstained(RoutingAbstention),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimatorRoutingError {
    InvalidSchemaRevision,
    InvalidConfiguration,
    InvalidObjectiveDigest,
    InvalidEstimatorIdentity,
    InvalidBranchIdentity,
    InvalidCoordinateRoster,
    InvalidCandidateShape,
    InvalidGradientValue,
    InvalidVariance,
    InvalidVarianceEvidenceDigest,
    UnknownBiasBound,
    InvalidBiasBound,
    InvalidBiasEvidenceDigest,
    InvalidCost,
    DuplicateEstimatorIdentity,
    MixedObjective,
    MissingBranchIdentity,
    InvalidBranchEvidenceDigest,
    InvalidValidityRegionDigest,
    UnexpectedBranchBinding,
    InvalidRegimeBranchSet,
    NumericalOverflow,
}

impl EstimatorRouterConfig {
    pub fn validate(&self) -> Result<(), EstimatorRoutingError> {
        if !self.max_abs_bias_bound.is_finite()
            || self.max_abs_bias_bound < 0.0
            || !self.max_mse_upper_bound.is_finite()
            || self.max_mse_upper_bound < 0.0
            || !self.cost_weight.is_finite()
            || self.cost_weight < 0.0
        {
            return Err(EstimatorRoutingError::InvalidConfiguration);
        }
        Ok(())
    }
}

impl GradientEstimatorCandidate {
    pub fn validate(
        &self,
        context: &EstimatorRoutingContext,
    ) -> Result<(), EstimatorRoutingError> {
        if !is_identifier(&self.estimator_id) {
            return Err(EstimatorRoutingError::InvalidEstimatorIdentity);
        }
        if !is_lower_hex_digest(&self.objective_digest) {
            return Err(EstimatorRoutingError::InvalidObjectiveDigest);
        }
        if self.objective_digest != context.objective_digest {
            return Err(EstimatorRoutingError::MixedObjective);
        }
        if self.coordinate_ids.is_empty()
            || self.coordinate_ids != context.coordinate_ids
            || has_duplicate_ids(&self.coordinate_ids)
            || self.coordinate_ids.iter().any(|id| !is_identifier(id))
        {
            return Err(EstimatorRoutingError::InvalidCoordinateRoster);
        }
        let dimensions = self.coordinate_ids.len();
        if self.gradient.len() != dimensions
            || self.variance.len() != dimensions
            || self.absolute_bias_bound.len() != dimensions
        {
            return Err(EstimatorRoutingError::InvalidCandidateShape);
        }
        if self.gradient.iter().any(|value| !value.is_finite()) {
            return Err(EstimatorRoutingError::InvalidGradientValue);
        }
        if self
            .variance
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(EstimatorRoutingError::InvalidVariance);
        }
        if !is_lower_hex_digest(&self.variance_evidence_digest) {
            return Err(EstimatorRoutingError::InvalidVarianceEvidenceDigest);
        }
        if self
            .absolute_bias_bound
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(EstimatorRoutingError::InvalidBiasBound);
        }
        if !is_lower_hex_digest(&self.bias_bound_evidence_digest) {
            return Err(EstimatorRoutingError::InvalidBiasEvidenceDigest);
        }
        if !self.cost_units.is_finite() || self.cost_units < 0.0 {
            return Err(EstimatorRoutingError::InvalidCost);
        }

        match self.kind {
            EstimatorKind::BranchLocalExact => {
                let branch_id = self
                    .branch_id
                    .as_deref()
                    .ok_or(EstimatorRoutingError::MissingBranchIdentity)?;
                if !is_identifier(branch_id) {
                    return Err(EstimatorRoutingError::InvalidBranchIdentity);
                }
                if !self
                    .branch_evidence_digest
                    .as_deref()
                    .is_some_and(is_lower_hex_digest)
                {
                    return Err(EstimatorRoutingError::InvalidBranchEvidenceDigest);
                }
                if !self
                    .validity_region_digest
                    .as_deref()
                    .is_some_and(is_lower_hex_digest)
                {
                    return Err(EstimatorRoutingError::InvalidValidityRegionDigest);
                }
            }
            _ => {
                if self.branch_id.is_some()
                    || self.branch_evidence_digest.is_some()
                    || self.validity_region_digest.is_some()
                {
                    return Err(EstimatorRoutingError::UnexpectedBranchBinding);
                }
            }
        }
        Ok(())
    }

    fn mean_mse_upper_bound(&self) -> Result<f64, EstimatorRoutingError> {
        let total = self
            .absolute_bias_bound
            .iter()
            .zip(self.variance.iter())
            .try_fold(0.0, |accumulator, (bias, variance)| {
                let bias_squared = bias * bias;
                let term = bias_squared + variance;
                let next = accumulator + term;
                next.is_finite()
                    .then_some(next)
                    .ok_or(EstimatorRoutingError::NumericalOverflow)
            })?;
        let result = total / self.coordinate_ids.len() as f64;
        if result.is_finite() {
            Ok(result)
        } else {
            Err(EstimatorRoutingError::NumericalOverflow)
        }
    }
}

/// Select one estimator subject to bias, variance and branch-validity limits.
///
/// This v0 implementation is deliberately a selector, not a fusion engine.
/// IVW-style fusion is deferred until estimator covariance is available and
/// independently bound; independent marginal variances alone are insufficient
/// to justify combining correlated estimates.
impl EstimatorKind {
    fn tag(self) -> &'static str {
        match self {
            Self::BranchLocalExact => "BRANCH_LOCAL_EXACT",
            Self::ModeBridgingSurrogate => "MODE_BRIDGING_SURROGATE",
            Self::ScoreFunction => "SCORE_FUNCTION",
            Self::ZerothOrder => "ZEROTH_ORDER",
        }
    }
}

impl NonsmoothReason {
    fn tag(self) -> &'static str {
        match self {
            Self::ActiveSetChange => "ACTIVE_SET_CHANGE",
            Self::StickSlipTransition => "STICK_SLIP_TRANSITION",
            Self::ContactMakeOrBreak => "CONTACT_MAKE_OR_BREAK",
            Self::EventSurfaceCrossing => "EVENT_SURFACE_CROSSING",
            Self::EventOrderAmbiguity => "EVENT_ORDER_AMBIGUITY",
            Self::EmpiricallyDetectedDiscontinuity => "EMPIRICALLY_DETECTED_DISCONTINUITY",
        }
    }
}

impl SingularRegimeReason {
    fn tag(self) -> &'static str {
        match self {
            Self::Grazing => "GRAZING",
            Self::IllConditioned => "ILL_CONDITIONED",
            Self::UnresolvedEventOrder => "UNRESOLVED_EVENT_ORDER",
            Self::NonUniqueContactResponse => "NON_UNIQUE_CONTACT_RESPONSE",
            Self::MissingBranchEvidence => "MISSING_BRANCH_EVIDENCE",
        }
    }
}

fn encode_regime(output: &mut Vec<u8>, regime: &ContactRegime) {
    match regime {
        ContactRegime::SmoothSingleBranch {
            branch_id,
            branch_evidence_digest,
            validity_region_digest,
        } => {
            output.push(0);
            push_string(output, branch_id);
            push_string(output, branch_evidence_digest);
            push_string(output, validity_region_digest);
        }
        ContactRegime::Nonsmooth { reason } => {
            output.push(1);
            push_string(output, reason.tag());
        }
        ContactRegime::MultiBranch { branch_ids } => {
            output.push(2);
            push_len(output, branch_ids.len());
            for branch_id in branch_ids {
                push_string(output, branch_id);
            }
        }
        ContactRegime::SingularOrUnresolved { reason } => {
            output.push(3);
            push_string(output, reason.tag());
        }
    }
}

fn push_len(output: &mut Vec<u8>, length: usize) {
    output.extend_from_slice(&(length as u64).to_le_bytes());
}

fn push_string(output: &mut Vec<u8>, value: &str) {
    push_len(output, value.len());
    output.extend_from_slice(value.as_bytes());
}

fn push_optional_string(output: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            output.push(1);
            push_string(output, value);
        }
        None => output.push(0),
    }
}

fn push_f64(output: &mut Vec<u8>, value: f64) {
    let bits = if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    };
    output.extend_from_slice(&bits.to_le_bytes());
}

pub fn route_gradient_estimator(
    context: &EstimatorRoutingContext,
    candidates: &[GradientEstimatorCandidate],
    config: &EstimatorRouterConfig,
) -> Result<EstimatorDecision, EstimatorRoutingError> {
    config.validate()?;
    validate_context(context)?;
    if has_duplicate_by(candidates, |candidate| candidate.estimator_id.as_str()) {
        return Err(EstimatorRoutingError::DuplicateEstimatorIdentity);
    }

    match &context.regime {
        ContactRegime::MultiBranch { branch_ids } => {
            validate_branch_set(branch_ids)?;
            return Ok(EstimatorDecision::Abstained(
                RoutingAbstention::MultiBranchRequiresSetValuedUpdate,
            ));
        }
        ContactRegime::SingularOrUnresolved { reason } => {
            let abstention = match reason {
                SingularRegimeReason::UnresolvedEventOrder => {
                    RoutingAbstention::UnresolvedEventOrder
                }
                _ => RoutingAbstention::GrazingOrSingular,
            };
            return Ok(EstimatorDecision::Abstained(abstention));
        }
        _ => {}
    }

    let mut best: Option<SelectedEstimator> = None;

    for candidate in candidates {
        candidate.validate(context)?;
        if candidate
            .absolute_bias_bound
            .iter()
            .any(|bound| *bound > config.max_abs_bias_bound)
        {
            continue;
        }

        let admissible = match candidate.kind {
            EstimatorKind::BranchLocalExact => match &context.regime {
                ContactRegime::SmoothSingleBranch {
                    branch_id,
                    branch_evidence_digest,
                    validity_region_digest,
                } => {
                    candidate.branch_id.as_deref() == Some(branch_id.as_str())
                        && candidate.branch_evidence_digest.as_deref()
                            == Some(branch_evidence_digest.as_str())
                        && candidate.validity_region_digest.as_deref()
                            == Some(validity_region_digest.as_str())
                }
                ContactRegime::Nonsmooth { .. } => false,
                ContactRegime::MultiBranch { .. }
                | ContactRegime::SingularOrUnresolved { .. } => false,
            },
            EstimatorKind::ModeBridgingSurrogate => {
                config.allow_mode_bridging_surrogates
            }
            EstimatorKind::ScoreFunction => config.allow_score_function,
            EstimatorKind::ZerothOrder => config.allow_zeroth_order,
        };
        if !admissible {
            continue;
        }

        let mse = candidate.mean_mse_upper_bound()?;
        if mse > config.max_mse_upper_bound {
            continue;
        }
        let risk_penalty = config.cost_weight * candidate.cost_units;
        let routing_score = mse + risk_penalty;
        if !routing_score.is_finite() {
            return Err(EstimatorRoutingError::NumericalOverflow);
        }

        let selected = SelectedEstimator {
            estimator_id: candidate.estimator_id.clone(),
            kind: candidate.kind,
            objective_digest: candidate.objective_digest.clone(),
            coordinate_ids: candidate.coordinate_ids.clone(),
            gradient: candidate.gradient.clone(),
            variance: candidate.variance.clone(),
            absolute_bias_bound: candidate.absolute_bias_bound.clone(),
            branch_id: candidate.branch_id.clone(),
            branch_evidence_digest: candidate.branch_evidence_digest.clone(),
            validity_region_digest: candidate.validity_region_digest.clone(),
            regime: context.regime.clone(),
            routing_configuration: config.clone(),
            variance_evidence_digest: candidate.variance_evidence_digest.clone(),
            bias_bound_evidence_digest: candidate.bias_bound_evidence_digest.clone(),
            mean_mse_upper_bound: mse,
            routing_score,
            cost_units: candidate.cost_units,
        };
        let replace = best.as_ref().is_none_or(|current| {
            selected.routing_score < current.routing_score
                || (selected.routing_score == current.routing_score
                    && selected.estimator_id < current.estimator_id)
        });
        if replace {
            best = Some(selected);
        }
    }

    Ok(match best {
        Some(selected) => EstimatorDecision::Selected(selected),
        None => EstimatorDecision::Abstained(RoutingAbstention::NoAdmissibleEstimator),
    })
}

fn validate_context(
    context: &EstimatorRoutingContext,
) -> Result<(), EstimatorRoutingError> {
    if !is_lower_hex_digest(&context.objective_digest) {
        return Err(EstimatorRoutingError::InvalidObjectiveDigest);
    }
    if context.coordinate_ids.is_empty()
        || has_duplicate_ids(&context.coordinate_ids)
        || context.coordinate_ids.iter().any(|id| !is_identifier(id))
    {
        return Err(EstimatorRoutingError::InvalidCoordinateRoster);
    }
    if let ContactRegime::SmoothSingleBranch {
        branch_id,
        branch_evidence_digest,
        validity_region_digest,
    } = &context.regime
    {
        if !is_identifier(branch_id) {
            return Err(EstimatorRoutingError::InvalidBranchIdentity);
        }
        if !is_lower_hex_digest(branch_evidence_digest) {
            return Err(EstimatorRoutingError::InvalidBranchEvidenceDigest);
        }
        if !is_lower_hex_digest(validity_region_digest) {
            return Err(EstimatorRoutingError::InvalidValidityRegionDigest);
        }
    }
    if let ContactRegime::MultiBranch { branch_ids } = &context.regime {
        validate_branch_set(branch_ids)?;
    }
    Ok(())
}

fn validate_branch_set(branch_ids: &[String]) -> Result<(), EstimatorRoutingError> {
    if branch_ids.len() < 2
        || has_duplicate_ids(branch_ids)
        || branch_ids.iter().any(|id| !is_identifier(id))
    {
        return Err(EstimatorRoutingError::InvalidRegimeBranchSet);
    }
    Ok(())
}

fn is_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && !value
            .chars()
            .any(|character| character.is_whitespace() || character == '/' || character == '\\')
}

fn has_duplicate_ids(values: &[String]) -> bool {
    let mut sorted = values.iter().map(String::as_str).collect::<Vec<_>>();
    sorted.sort_unstable();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn has_duplicate_by<'a, T, F>(values: &'a [T], key: F) -> bool
where
    F: Fn(&'a T) -> &'a str,
{
    let mut keys = values.iter().map(key).collect::<Vec<_>>();
    keys.sort_unstable();
    keys.windows(2).any(|pair| pair[0] == pair[1])
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn context(regime: ContactRegime) -> EstimatorRoutingContext {
        EstimatorRoutingContext {
            objective_digest: digest('a'),
            coordinate_ids: vec!["joint-1".into(), "joint-2".into()],
            regime,
        }
    }

    fn config() -> EstimatorRouterConfig {
        EstimatorRouterConfig {
            max_abs_bias_bound: 0.05,
            max_mse_upper_bound: 0.01,
            cost_weight: 0.0,
            allow_mode_bridging_surrogates: true,
            allow_score_function: true,
            allow_zeroth_order: true,
        }
    }

    fn candidate(estimator_id: &str, kind: EstimatorKind, bias: f64, variance: f64) -> GradientEstimatorCandidate {
        GradientEstimatorCandidate {
            estimator_id: estimator_id.into(),
            kind,
            objective_digest: digest('a'),
            coordinate_ids: vec!["joint-1".into(), "joint-2".into()],
            gradient: vec![0.1, -0.2],
            variance: vec![variance, variance],
            variance_evidence_digest: digest('d'),
            absolute_bias_bound: vec![bias, bias],
            bias_bound_evidence_digest: digest('e'),
            cost_units: 1.0,
            branch_id: None,
            branch_evidence_digest: None,
            validity_region_digest: None,
        }
    }

    fn exact_candidate() -> GradientEstimatorCandidate {
        let mut candidate = candidate("exact-branch", EstimatorKind::BranchLocalExact, 0.0, 1.0e-5);
        candidate.branch_id = Some("stick".into());
        candidate.branch_evidence_digest = Some(digest('b'));
        candidate.validity_region_digest = Some(digest('c'));
        candidate
    }

    #[test]
    fn smooth_single_branch_selects_exact_matching_branch_derivative() {
        let context = context(ContactRegime::SmoothSingleBranch {
            branch_id: "stick".into(),
            branch_evidence_digest: digest('b'),
            validity_region_digest: digest('c'),
        });
        let candidates = vec![
            exact_candidate(),
            candidate("surrogate", EstimatorKind::ModeBridgingSurrogate, 0.01, 1.0e-4),
        ];
        let result = route_gradient_estimator(&context, &candidates, &config()).unwrap();
        assert!(matches!(
            result,
            EstimatorDecision::Selected(SelectedEstimator {
                estimator_id,
                kind: EstimatorKind::BranchLocalExact,
                ..
            }) if estimator_id == "exact-branch"
        ));
    }

    #[test]
    fn nonsmooth_region_switches_away_from_invalid_local_branch_gradient() {
        let context = context(ContactRegime::Nonsmooth {
            reason: NonsmoothReason::StickSlipTransition,
        });
        let candidates = vec![
            exact_candidate(),
            candidate("diffcontact", EstimatorKind::ModeBridgingSurrogate, 0.02, 2.0e-4),
            candidate("score", EstimatorKind::ScoreFunction, 0.0, 2.0e-3),
        ];
        let result = route_gradient_estimator(&context, &candidates, &config()).unwrap();
        assert!(matches!(
            result,
            EstimatorDecision::Selected(SelectedEstimator {
                estimator_id,
                kind: EstimatorKind::ModeBridgingSurrogate,
                ..
            }) if estimator_id == "diffcontact"
        ));
    }

    #[test]
    fn low_variance_does_not_launder_an_excessive_bias_bound() {
        let context = context(ContactRegime::Nonsmooth {
            reason: NonsmoothReason::ActiveSetChange,
        });
        let mut config = config();
        config.max_abs_bias_bound = 0.01;

        let candidates = vec![
            candidate("biased-low-variance", EstimatorKind::ModeBridgingSurrogate, 0.5, 0.0),
            candidate("score-safe", EstimatorKind::ScoreFunction, 0.0, 1.0e-3),
        ];
        let result = route_gradient_estimator(&context, &candidates, &config()).unwrap();
        assert!(matches!(
            result,
            EstimatorDecision::Selected(SelectedEstimator { estimator_id, .. })
                if estimator_id == "score-safe"
        ));
    }

    #[test]
    fn multi_branch_context_abstains_instead_of_averaging() {
        let context = context(ContactRegime::MultiBranch {
            branch_ids: vec!["stick".into(), "slip".into()],
        });
        let candidates = vec![
            candidate("score", EstimatorKind::ScoreFunction, 0.0, 1.0e-3),
            candidate("zo", EstimatorKind::ZerothOrder, 0.0, 2.0e-3),
        ];
        let result = route_gradient_estimator(&context, &candidates, &config()).unwrap();
        assert_eq!(
            result,
            EstimatorDecision::Abstained(
                RoutingAbstention::MultiBranchRequiresSetValuedUpdate
            )
        );
    }

    #[test]
    fn grazing_and_unresolved_events_abstain() {
        let grazing = context(ContactRegime::SingularOrUnresolved {
            reason: SingularRegimeReason::Grazing,
        });
        let unresolved = context(ContactRegime::SingularOrUnresolved {
            reason: SingularRegimeReason::UnresolvedEventOrder,
        });

        assert_eq!(
            route_gradient_estimator(&grazing, &[], &config()).unwrap(),
            EstimatorDecision::Abstained(RoutingAbstention::GrazingOrSingular)
        );
        assert_eq!(
            route_gradient_estimator(&unresolved, &[], &config()).unwrap(),
            EstimatorDecision::Abstained(RoutingAbstention::UnresolvedEventOrder)
        );
    }

    #[test]
    fn objective_mismatch_is_not_silently_ignored() {
        let context = context(ContactRegime::Nonsmooth {
            reason: NonsmoothReason::EventSurfaceCrossing,
        });
        let mut invalid = candidate("stale-objective", EstimatorKind::ScoreFunction, 0.0, 1.0e-3);
        invalid.objective_digest = digest('z');

        assert_eq!(
            route_gradient_estimator(&context, &[invalid], &config()),
            Err(EstimatorRoutingError::MixedObjective)
        );
    }

    #[test]
    fn coordinate_order_mismatch_is_rejected() {
        let context = context(ContactRegime::Nonsmooth {
            reason: NonsmoothReason::ActiveSetChange,
        });
        let mut invalid = candidate("different-order", EstimatorKind::ScoreFunction, 0.0, 1.0e-3);
        invalid.coordinate_ids.reverse();

        assert_eq!(
            route_gradient_estimator(&context, &[invalid], &config()),
            Err(EstimatorRoutingError::InvalidCoordinateRoster)
        );
    }

    #[test]
    fn invalid_branch_certificate_or_region_is_rejected() {
        let context = context(ContactRegime::SmoothSingleBranch {
            branch_id: "stick".into(),
            branch_evidence_digest: digest('b'),
            validity_region_digest: digest('c'),
        });
        let mut invalid = exact_candidate();
        invalid.branch_evidence_digest = Some("missing-proof".into());

        assert_eq!(
            route_gradient_estimator(&context, &[invalid], &config()),
            Err(EstimatorRoutingError::InvalidBranchEvidenceDigest)
        );
    }

    #[test]
    fn candidate_identity_duplicates_are_rejected() {
        let context = context(ContactRegime::Nonsmooth {
            reason: NonsmoothReason::StickSlipTransition,
        });
        let candidates = vec![
            candidate("same", EstimatorKind::ScoreFunction, 0.0, 1.0e-3),
            candidate("same", EstimatorKind::ZerothOrder, 0.0, 2.0e-3),
        ];
        assert_eq!(
            route_gradient_estimator(&context, &candidates, &config()),
            Err(EstimatorRoutingError::DuplicateEstimatorIdentity)
        );
    }

    #[test]
    fn inverse_variance_is_not_used_without_covariance_evidence() {
        let context = context(ContactRegime::Nonsmooth {
            reason: NonsmoothReason::StickSlipTransition,
        });
        let candidates = vec![
            candidate("score-a", EstimatorKind::ScoreFunction, 0.0, 1.0e-3),
            candidate("score-b", EstimatorKind::ZerothOrder, 0.0, 1.0e-3),
        ];
        let result = route_gradient_estimator(&context, &candidates, &config()).unwrap();
        assert!(matches!(result, EstimatorDecision::Selected(_)));
        // The router returns one chosen estimate. It does not construct a
        // weighted average without estimator covariance evidence.
    }
}
