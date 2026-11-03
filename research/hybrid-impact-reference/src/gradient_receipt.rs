//! Evidence-binding receipt for branch-qualified contact-dynamics gradients.
//!
//! This module records the scope and provenance of a derivative result. It
//! does not compute gradients, authenticate a verifier, or prove equivalence.
//! In particular, it keeps branch-local, directional, set-valued, surrogate,
//! and unavailable results structurally distinct.

pub const GRADIENT_RECEIPT_SCHEMA: &str = "SYMTROPY_QUALIFIED_GRADIENT_RECEIPT.v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientKind {
    BranchLocal,
    DirectionalSet,
    SetValued,
    Surrogate,
    Unavailable,
}

impl GradientKind {
    fn tag(self) -> &'static str {
        match self {
            Self::BranchLocal => "BRANCH_LOCAL",
            Self::DirectionalSet => "DIRECTIONAL_SET",
            Self::SetValued => "SET_VALUED",
            Self::Surrogate => "SURROGATE",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstentionReason {
    Grazing,
    IllConditioned,
    UnresolvedEventOrder,
    DiscontinuousMap,
    NonDifferentiable,
    NotIdentifiable,
    MissingBranchEvidence,
    SolverNotConverged,
    InvalidForwardState,
}

impl AbstentionReason {
    fn tag(self) -> &'static str {
        match self {
            Self::Grazing => "GRAZING",
            Self::IllConditioned => "ILL_CONDITIONED",
            Self::UnresolvedEventOrder => "UNRESOLVED_EVENT_ORDER",
            Self::DiscontinuousMap => "DISCONTINUOUS_MAP",
            Self::NonDifferentiable => "NON_DIFFERENTIABLE",
            Self::NotIdentifiable => "NOT_IDENTIFIABLE",
            Self::MissingBranchEvidence => "MISSING_BRANCH_EVIDENCE",
            Self::SolverNotConverged => "SOLVER_NOT_CONVERGED",
            Self::InvalidForwardState => "INVALID_FORWARD_STATE",
        }
    }
}

/// One coordinate of a derivative vector, in the declared objective / model
/// coordinate system. Values must be finite.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientCoordinate {
    pub coordinate_id: String,
    pub value: f64,
}

/// A single branch-conditioned or direction-conditioned derivative response.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientResponse {
    /// Required for every available derivative. A branch identity is not a
    /// probability and must be sourced from branch evidence.
    pub branch_id: Option<String>,
    /// Required only for DIRECTIONAL_SET responses.
    pub direction_id: Option<String>,
    /// Required for exact branch/directional/set-valued responses. Surrogates
    /// may omit it when no certified local validity region exists.
    pub validity_region_digest: Option<String>,
    pub coordinates: Vec<GradientCoordinate>,
}

/// Independently recomputable residuals and conditioning quantities.
/// Missing values are unknown, never implicitly zero.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientMetrics {
    pub normalization_id: String,
    pub forward_state_error: Option<f64>,
    pub penetration_error: Option<f64>,
    pub complementarity_residual: Option<f64>,
    pub friction_residual: Option<f64>,
    pub kkt_residual: Option<f64>,
    /// Absolute transversality margin. Zero or absent does not qualify an
    /// event-time derivative.
    pub transversality_abs_margin: Option<f64>,
    pub condition_estimate: Option<f64>,
    pub directional_reference_error: Option<f64>,
    pub surrogate_gradient_bias: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedGradientReceipt {
    pub schema_revision: String,
    pub model_digest: String,
    pub trajectory_digest: String,
    pub objective_digest: String,
    pub solver_configuration_digest: String,
    pub solver_implementation_digest: String,
    pub gradient_source_digest: String,
    pub kind: GradientKind,
    /// Responses remain separate; no branch or directional averaging is
    /// performed by this receipt format.
    pub responses: Vec<GradientResponse>,
    /// Required for SURROGATE results.
    pub estimator_id: Option<String>,
    /// Required for SURROGATE results; points to independently measured bias.
    pub bias_evidence_reference: Option<String>,
    /// Required for every claimed exact/directional/set-valued qualification.
    /// It is paired with verification_receipt_digest and must be checked by an
    /// independent verifier.
    pub verifier_id: Option<String>,
    pub verification_receipt_digest: Option<String>,
    /// Required for UNAVAILABLE results.
    pub abstention_reason: Option<AbstentionReason>,
    /// True when this result differentiates an event-time / event-surface map.
    /// Exact/directional/set-valued results then require a strictly positive
    /// independently established lower bound on the absolute transversality.
    pub requires_transversality: bool,
    pub metrics: GradientMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GradientReceiptError {
    InvalidSchemaRevision,
    InvalidDigest,
    EmptyIdentifier,
    InvalidIdentifier,
    IncompleteVerifierBinding,
    MissingVerifierBinding,
    MissingResponses,
    UnexpectedResponses,
    InvalidResponseCount,
    MissingBranchIdentity,
    DuplicateBranchIdentity,
    MissingDirectionIdentity,
    DuplicateDirectionIdentity,
    UnexpectedDirectionIdentity,
    MissingValidityRegion,
    InvalidCoordinateSet,
    MismatchedCoordinateRoster,
    NonFiniteGradient,
    InvalidMetric,
    MissingNormalizationId,
    MissingEstimatorIdentity,
    MissingBiasEvidence,
    UnexpectedAbstentionReason,
    MissingAbstentionReason,
    SurrogateCannotClaimExactQualification,
    InvalidTransversalityMargin,
    UnexpectedVerifierBinding,
}

impl QualifiedGradientReceipt {
    /// Validate structural and evidence-binding rules.
    ///
    /// This checks what the record claims, not whether the source artifacts or
    /// verifier are authentic. An independent consumer must fetch and re-read
    /// each bound artifact and recompute all claimed metrics.
    pub fn validate(&self) -> Result<(), GradientReceiptError> {
        if self.schema_revision != GRADIENT_RECEIPT_SCHEMA {
            return Err(GradientReceiptError::InvalidSchemaRevision);
        }
        for digest in [
            self.model_digest.as_str(),
            self.trajectory_digest.as_str(),
            self.objective_digest.as_str(),
            self.solver_configuration_digest.as_str(),
            self.solver_implementation_digest.as_str(),
            self.gradient_source_digest.as_str(),
        ] {
            if !is_lower_hex_digest(digest) {
                return Err(GradientReceiptError::InvalidDigest);
            }
        }
        if self.metrics.normalization_id.trim().is_empty() {
            return Err(GradientReceiptError::MissingNormalizationId);
        }
        validate_optional_identifier(self.estimator_id.as_deref())?;
        validate_optional_identifier(self.verifier_id.as_deref())?;
        validate_optional_reference(self.bias_evidence_reference.as_deref())?;

        if self.verifier_id.is_some() != self.verification_receipt_digest.is_some() {
            return Err(GradientReceiptError::IncompleteVerifierBinding);
        }
        if self
            .verification_receipt_digest
            .as_deref()
            .is_some_and(|digest| !is_lower_hex_digest(digest))
        {
            return Err(GradientReceiptError::InvalidDigest);
        }

        validate_metrics(&self.metrics)?;

        match self.kind {
            GradientKind::BranchLocal => {
                if self.responses.len() != 1 {
                    return Err(GradientReceiptError::InvalidResponseCount);
                }
                if self.verifier_id.is_none() {
                    return Err(GradientReceiptError::MissingVerifierBinding);
                }
                if self.estimator_id.is_some() || self.bias_evidence_reference.is_some() {
                    return Err(GradientReceiptError::SurrogateCannotClaimExactQualification);
                }
                if self.abstention_reason.is_some() {
                    return Err(GradientReceiptError::UnexpectedAbstentionReason);
                }
                validate_branch_responses(&self.responses, true, true, false)?;
                validate_transversality(self.requires_transversality, &self.metrics)?;
            }
            GradientKind::DirectionalSet => {
                if self.responses.len() < 2 {
                    return Err(GradientReceiptError::InvalidResponseCount);
                }
                if self.verifier_id.is_none() {
                    return Err(GradientReceiptError::MissingVerifierBinding);
                }
                if self.estimator_id.is_some() || self.bias_evidence_reference.is_some() {
                    return Err(GradientReceiptError::SurrogateCannotClaimExactQualification);
                }
                if self.abstention_reason.is_some() {
                    return Err(GradientReceiptError::UnexpectedAbstentionReason);
                }
                validate_branch_responses(&self.responses, true, true, true)?;
                validate_transversality(self.requires_transversality, &self.metrics)?;
            }
            GradientKind::SetValued => {
                if self.responses.len() < 2 {
                    return Err(GradientReceiptError::InvalidResponseCount);
                }
                if self.verifier_id.is_none() {
                    return Err(GradientReceiptError::MissingVerifierBinding);
                }
                if self.estimator_id.is_some() || self.bias_evidence_reference.is_some() {
                    return Err(GradientReceiptError::SurrogateCannotClaimExactQualification);
                }
                if self.abstention_reason.is_some() {
                    return Err(GradientReceiptError::UnexpectedAbstentionReason);
                }
                validate_branch_responses(&self.responses, true, true, false)?;
                validate_transversality(self.requires_transversality, &self.metrics)?;
            }
            GradientKind::Surrogate => {
                if self.responses.len() != 1 {
                    return Err(GradientReceiptError::InvalidResponseCount);
                }
                if self.estimator_id.is_none() {
                    return Err(GradientReceiptError::MissingEstimatorIdentity);
                }
                if self.bias_evidence_reference.is_none() {
                    return Err(GradientReceiptError::MissingBiasEvidence);
                }
                if self.abstention_reason.is_some() {
                    return Err(GradientReceiptError::UnexpectedAbstentionReason);
                }
                validate_branch_responses(&self.responses, false, false, false)?;
            }
            GradientKind::Unavailable => {
                if !self.responses.is_empty() {
                    return Err(GradientReceiptError::UnexpectedResponses);
                }
                if self.abstention_reason.is_none() {
                    return Err(GradientReceiptError::MissingAbstentionReason);
                }
                if self.estimator_id.is_some() || self.bias_evidence_reference.is_some() {
                    return Err(GradientReceiptError::SurrogateCannotClaimExactQualification);
                }
                if self.verifier_id.is_some() {
                    return Err(GradientReceiptError::UnexpectedVerifierBinding);
                }
            }
        }

        Ok(())
    }

    /// Deterministic length-prefixed serialization of the receipt. It is not
    /// itself a digest; callers choose and record the digest algorithm.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut canonical = self.clone();
        canonical.responses.sort_by(|left, right| {
            (
                left.branch_id.as_deref().unwrap_or(""),
                left.direction_id.as_deref().unwrap_or(""),
            )
                .cmp(&(
                    right.branch_id.as_deref().unwrap_or(""),
                    right.direction_id.as_deref().unwrap_or(""),
                ))
        });
        for response in &mut canonical.responses {
            response
                .coordinates
                .sort_by(|left, right| left.coordinate_id.cmp(&right.coordinate_id));
        }

        let mut output = Vec::new();
        push_string(&mut output, &canonical.schema_revision);
        push_string(&mut output, &canonical.model_digest);
        push_string(&mut output, &canonical.trajectory_digest);
        push_string(&mut output, &canonical.objective_digest);
        push_string(&mut output, &canonical.solver_configuration_digest);
        push_string(&mut output, &canonical.solver_implementation_digest);
        push_string(&mut output, &canonical.gradient_source_digest);
        push_string(&mut output, canonical.kind.tag());

        push_len(&mut output, canonical.responses.len());
        for response in &canonical.responses {
            push_optional_string(&mut output, response.branch_id.as_deref());
            push_optional_string(&mut output, response.direction_id.as_deref());
            push_optional_string(
                &mut output,
                response.validity_region_digest.as_deref(),
            );
            push_len(&mut output, response.coordinates.len());
            for coordinate in &response.coordinates {
                push_string(&mut output, &coordinate.coordinate_id);
                output.extend_from_slice(&coordinate.value.to_bits().to_le_bytes());
            }
        }

        push_optional_string(&mut output, canonical.estimator_id.as_deref());
        push_optional_string(
            &mut output,
            canonical.bias_evidence_reference.as_deref(),
        );
        push_optional_string(&mut output, canonical.verifier_id.as_deref());
        push_optional_string(
            &mut output,
            canonical.verification_receipt_digest.as_deref(),
        );
        output.push(u8::from(canonical.requires_transversality));
        match canonical.abstention_reason {
            Some(reason) => {
                output.push(1);
                push_string(&mut output, reason.tag());
            }
            None => output.push(0),
        }

        push_string(&mut output, &canonical.metrics.normalization_id);
        for metric in [
            canonical.metrics.forward_state_error,
            canonical.metrics.penetration_error,
            canonical.metrics.complementarity_residual,
            canonical.metrics.friction_residual,
            canonical.metrics.kkt_residual,
            canonical.metrics.transversality_abs_margin,
            canonical.metrics.condition_estimate,
            canonical.metrics.directional_reference_error,
            canonical.metrics.surrogate_gradient_bias,
        ] {
            push_optional_f64(&mut output, metric);
        }
        output
    }
}

fn validate_branch_responses(
    responses: &[GradientResponse],
    require_branch_identity: bool,
    require_validity_region: bool,
    require_direction: bool,
) -> Result<(), GradientReceiptError> {
    let mut branch_ids = Vec::new();
    let mut direction_ids = Vec::new();
    let mut canonical_coordinate_ids: Option<Vec<&str>> = None;

    for response in responses {
        match response.branch_id.as_deref() {
            Some(branch_id) => {
                validate_identifier(branch_id)?;
                branch_ids.push(branch_id);
            }
            None if require_branch_identity => {
                return Err(GradientReceiptError::MissingBranchIdentity);
            }
            None => {}
        }

        if require_direction {
            let direction_id = response
                .direction_id
                .as_deref()
                .ok_or(GradientReceiptError::MissingDirectionIdentity)?;
            validate_identifier(direction_id)?;
            direction_ids.push(direction_id);
        } else if response.direction_id.is_some() {
            return Err(GradientReceiptError::UnexpectedDirectionIdentity);
        }

        if require_validity_region {
            let digest = response
                .validity_region_digest
                .as_deref()
                .ok_or(GradientReceiptError::MissingValidityRegion)?;
            if !is_lower_hex_digest(digest) {
                return Err(GradientReceiptError::InvalidDigest);
            }
        } else if response
            .validity_region_digest
            .as_deref()
            .is_some_and(|digest| !is_lower_hex_digest(digest))
        {
            return Err(GradientReceiptError::InvalidDigest);
        }

        if response.coordinates.is_empty() {
            return Err(GradientReceiptError::InvalidCoordinateSet);
        }
        let mut coordinates = response
            .coordinates
            .iter()
            .map(|coordinate| coordinate.coordinate_id.as_str())
            .collect::<Vec<_>>();
        for coordinate in &response.coordinates {
            validate_identifier(&coordinate.coordinate_id)?;
            if !coordinate.value.is_finite() {
                return Err(GradientReceiptError::NonFiniteGradient);
            }
        }
        coordinates.sort_unstable();
        if coordinates.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(GradientReceiptError::InvalidCoordinateSet);
        }
        if let Some(expected) = &canonical_coordinate_ids {
            if *expected != coordinates {
                return Err(GradientReceiptError::MismatchedCoordinateRoster);
            }
        } else {
            canonical_coordinate_ids = Some(coordinates);
        }
    }

    if !require_direction {
        branch_ids.sort_unstable();
        if branch_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(GradientReceiptError::DuplicateBranchIdentity);
        }
    }
    direction_ids.sort_unstable();
    if direction_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(GradientReceiptError::DuplicateDirectionIdentity);
    }
    Ok(())
}

fn validate_transversality(
    required: bool,
    metrics: &GradientMetrics,
) -> Result<(), GradientReceiptError> {
    if required
        && metrics
            .transversality_abs_margin
            .is_none_or(|margin| margin <= 0.0)
    {
        return Err(GradientReceiptError::InvalidTransversalityMargin);
    }
    Ok(())
}

fn validate_metrics(metrics: &GradientMetrics) -> Result<(), GradientReceiptError> {
    for metric in [
        metrics.forward_state_error,
        metrics.penetration_error,
        metrics.complementarity_residual,
        metrics.friction_residual,
        metrics.kkt_residual,
        metrics.transversality_abs_margin,
        metrics.condition_estimate,
        metrics.directional_reference_error,
        metrics.surrogate_gradient_bias,
    ]
    .into_iter()
    .flatten()
    {
        if !metric.is_finite() || metric < 0.0 {
            return Err(GradientReceiptError::InvalidMetric);
        }
    }
    if metrics.condition_estimate == Some(0.0) {
        return Err(GradientReceiptError::InvalidMetric);
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), GradientReceiptError> {
    if value.trim().is_empty() {
        return Err(GradientReceiptError::EmptyIdentifier);
    }
    if value
        .chars()
        .any(|character| character.is_whitespace() || character == '/' || character == '\\')
    {
        return Err(GradientReceiptError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_optional_reference(value: Option<&str>) -> Result<(), GradientReceiptError> {
    if value.is_some_and(|reference| reference.trim().is_empty()) {
        return Err(GradientReceiptError::EmptyIdentifier);
    }
    Ok(())
}

fn validate_optional_identifier(value: Option<&str>) -> Result<(), GradientReceiptError> {
    if let Some(value) = value {
        validate_identifier(value)?;
    }
    Ok(())
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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

fn push_optional_f64(output: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(value) => {
            output.push(1);
            output.extend_from_slice(&value.to_bits().to_le_bytes());
        }
        None => output.push(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn metrics() -> GradientMetrics {
        GradientMetrics {
            normalization_id: "fixture-unit-normalization-v1".into(),
            forward_state_error: Some(1.0e-12),
            penetration_error: Some(0.0),
            complementarity_residual: Some(1.0e-10),
            friction_residual: Some(0.0),
            kkt_residual: Some(1.0e-12),
            transversality_abs_margin: Some(2.0),
            condition_estimate: Some(1.0),
            directional_reference_error: Some(1.0e-8),
            surrogate_gradient_bias: None,
        }
    }

    fn base_receipt(kind: GradientKind) -> QualifiedGradientReceipt {
        QualifiedGradientReceipt {
            schema_revision: GRADIENT_RECEIPT_SCHEMA.into(),
            model_digest: digest('a'),
            trajectory_digest: digest('b'),
            objective_digest: digest('c'),
            solver_configuration_digest: digest('d'),
            solver_implementation_digest: digest('e'),
            gradient_source_digest: digest('f'),
            kind,
            responses: Vec::new(),
            estimator_id: None,
            bias_evidence_reference: None,
            verifier_id: Some("independent-gradient-checker-v1".into()),
            verification_receipt_digest: Some(digest('9')),
            abstention_reason: None,
            requires_transversality: false,
            metrics: metrics(),
        }
    }

    fn coordinate(value: f64) -> GradientCoordinate {
        GradientCoordinate {
            coordinate_id: "initial-height".into(),
            value,
        }
    }

    fn branch_response(branch: &str, value: f64) -> GradientResponse {
        GradientResponse {
            branch_id: Some(branch.into()),
            direction_id: None,
            validity_region_digest: Some(digest('8')),
            coordinates: vec![coordinate(value)],
        }
    }

    #[test]
    fn branch_local_receipt_requires_branch_region_and_verifier_binding() {
        let mut receipt = base_receipt(GradientKind::BranchLocal);
        receipt.responses.push(branch_response("free-flight-rebound", 0.5));
        receipt.validate().expect("valid branch-local receipt");

        receipt.verifier_id = None;
        receipt.verification_receipt_digest = None;
        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::MissingVerifierBinding)
        );
    }

    #[test]
    fn directional_set_requires_distinct_explicit_directions() {
        let mut receipt = base_receipt(GradientKind::DirectionalSet);
        let mut positive = branch_response("impact", 1.5);
        positive.direction_id = Some("height-increase".into());
        let mut negative = branch_response("impact", 1.5);
        negative.direction_id = Some("height-decrease".into());
        receipt.responses = vec![positive, negative];

        receipt.validate().expect("valid directional set");
    }

    #[test]
    fn duplicate_direction_is_rejected() {
        let mut receipt = base_receipt(GradientKind::DirectionalSet);
        let mut first = branch_response("impact", 1.5);
        first.direction_id = Some("height-increase".into());
        let mut second = branch_response("impact", 1.6);
        second.direction_id = Some("height-increase".into());
        receipt.responses = vec![first, second];

        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::DuplicateDirectionIdentity)
        );
    }

    #[test]
    fn set_valued_receipt_preserves_distinct_branches() {
        let mut receipt = base_receipt(GradientKind::SetValued);
        receipt.responses = vec![
            branch_response("stick", 0.0),
            branch_response("slip-positive", 0.5),
        ];
        receipt.validate().expect("valid set-valued receipt");
    }

    #[test]
    fn set_valued_receipt_rejects_duplicate_branch_identity() {
        let mut receipt = base_receipt(GradientKind::SetValued);
        receipt.responses = vec![
            branch_response("stick", 0.0),
            branch_response("stick", 0.5),
        ];
        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::DuplicateBranchIdentity)
        );
    }

    #[test]
    fn surrogate_requires_estimator_and_bias_reference() {
        let mut receipt = base_receipt(GradientKind::Surrogate);
        receipt.responses.push(GradientResponse {
            branch_id: Some("pre-contact-estimator".into()),
            direction_id: None,
            validity_region_digest: None,
            coordinates: vec![coordinate(0.25)],
        });
        receipt.estimator_id = Some("contacts-from-distance-ste-v1".into());
        receipt.bias_evidence_reference = Some("artifact://gradient-bias-report".into());
        receipt.responses[0].branch_id = None;
        receipt.verifier_id = None;
        receipt.verification_receipt_digest = None;

        receipt.validate().expect("valid surrogate receipt");
    }

    #[test]
    fn unavailable_receipt_requires_typed_abstention_and_no_gradient() {
        let mut receipt = base_receipt(GradientKind::Unavailable);
        receipt.responses.clear();
        receipt.verifier_id = None;
        receipt.verification_receipt_digest = None;
        receipt.abstention_reason = Some(AbstentionReason::Grazing);

        receipt.validate().expect("valid abstention");
        receipt.responses.push(branch_response("grazing", 123.0));
        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::UnexpectedResponses)
        );
    }

    #[test]
    fn missing_metric_is_unknown_not_zero_and_canonical_bytes_bind_none() {
        let mut receipt = base_receipt(GradientKind::Unavailable);
        receipt.responses.clear();
        receipt.verifier_id = None;
        receipt.verification_receipt_digest = None;
        receipt.abstention_reason = Some(AbstentionReason::Grazing);
        receipt.metrics.transversality_abs_margin = None;

        let unknown = receipt.canonical_bytes();
        receipt.metrics.transversality_abs_margin = Some(0.0);
        assert_ne!(unknown, receipt.canonical_bytes());
    }

    #[test]
    fn non_finite_gradient_and_metric_values_are_rejected() {
        let mut receipt = base_receipt(GradientKind::BranchLocal);
        receipt.responses.push(branch_response("rebound", f64::INFINITY));
        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::NonFiniteGradient)
        );

        let mut metric_receipt = base_receipt(GradientKind::Unavailable);
        metric_receipt.responses.clear();
        metric_receipt.verifier_id = None;
        metric_receipt.verification_receipt_digest = None;
        metric_receipt.abstention_reason = Some(AbstentionReason::Grazing);
        metric_receipt.metrics.kkt_residual = Some(f64::NAN);
        assert_eq!(
            metric_receipt.validate(),
            Err(GradientReceiptError::InvalidMetric)
        );
    }

    #[test]
    fn repeated_coordinates_are_rejected() {
        let mut receipt = base_receipt(GradientKind::BranchLocal);
        let mut response = branch_response("rebound", 0.5);
        response.coordinates.push(coordinate(0.6));
        receipt.responses.push(response);

        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::InvalidCoordinateSet)
        );
    }

    #[test]
    fn canonical_bytes_are_response_and_coordinate_order_invariant() {
        let mut receipt = base_receipt(GradientKind::SetValued);
        let first = GradientResponse {
            branch_id: Some("branch-a".into()),
            direction_id: None,
            validity_region_digest: Some(digest('8')),
            coordinates: vec![
                GradientCoordinate {
                    coordinate_id: "velocity".into(),
                    value: 1.0,
                },
                GradientCoordinate {
                    coordinate_id: "height".into(),
                    value: 0.0,
                },
            ],
        };
        let second = GradientResponse {
            branch_id: Some("branch-b".into()),
            direction_id: None,
            validity_region_digest: Some(digest('7')),
            coordinates: vec![GradientCoordinate {
                coordinate_id: "height".into(),
                value: 0.5,
            }],
        };
        receipt.responses = vec![first.clone(), second.clone()];
        let canonical = receipt.canonical_bytes();

        receipt.responses = vec![second, first];
        assert_eq!(canonical, receipt.canonical_bytes());
    }
    #[test]
    fn event_sensitive_exact_gradient_requires_positive_transversality_margin() {
        let mut receipt = base_receipt(GradientKind::BranchLocal);
        receipt.responses.push(branch_response("impact", 0.5));
        receipt.requires_transversality = true;
        receipt.metrics.transversality_abs_margin = Some(0.0);

        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::InvalidTransversalityMargin)
        );

        receipt.metrics.transversality_abs_margin = Some(1.0e-6);
        receipt.validate().expect("strictly positive transversality bound");
    }


    #[test]
    fn branch_sets_require_identical_coordinate_rosters() {
        let mut receipt = base_receipt(GradientKind::SetValued);
        let height_only = GradientResponse {
            branch_id: Some("stick".into()),
            direction_id: None,
            validity_region_digest: Some(digest('8')),
            coordinates: vec![GradientCoordinate {
                coordinate_id: "height".into(),
                value: 0.0,
            }],
        };

        let two_coordinates = GradientResponse {
            branch_id: Some("slip".into()),
            direction_id: None,
            validity_region_digest: Some(digest('7')),
            coordinates: vec![
                GradientCoordinate {
                    coordinate_id: "height".into(),
                    value: 0.5,
                },
                GradientCoordinate {
                    coordinate_id: "velocity".into(),
                    value: 1.0,
                },
            ],
        };
        receipt.responses = vec![height_only, two_coordinates];

        assert_eq!(
            receipt.validate(),
            Err(GradientReceiptError::MismatchedCoordinateRoster)
        );
    }


}
