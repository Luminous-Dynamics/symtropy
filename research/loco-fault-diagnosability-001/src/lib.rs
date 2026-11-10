//! Action-conditioned fault diagnosability and ambiguity auditor.
//!
//! Consumes source-qualified observation envelopes and an upstream admitted
//! diagnostic-action/continuation receipt. It does not classify raw sensors,
//! identify physical faults, or compute viability.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSpec {
    pub feature_id: String,
    pub unit_id: String,
    /// Channels derived from the same encoder/bus/clock share this group.
    pub common_cause_group: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureInterval {
    pub feature_id: String,
    pub unit_id: String,
    pub lower: i64,
    pub upper: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeStatus {
    Bounded,
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HypothesisEnvelope {
    pub hypothesis_id: String,
    pub status: EnvelopeStatus,
    /// Must exactly match the declared feature schema for Bounded envelopes.
    /// Must be empty for Unknown envelopes.
    pub features: Vec<FeatureInterval>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamAdmissionStatus {
    Admitted,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticActionReceipt {
    pub receipt_id: String,
    pub action_id: String,
    pub action_duration_ms: u64,
    pub physical_fault_set_digest: String,
    pub current_state_digest: String,
    pub model_digest: String,
    pub actuator_digest: String,
    pub domain_digest: String,
    pub sensor_manifest_digest: String,
    pub calibration_digest: String,
    pub clock_map_digest: String,
    pub result_digest: String,
    /// Exact #4313/#4293 upstream receipt proving full diagnostic-action
    /// physical admission and a qualified post-action continuation.
    pub upstream_continuation_receipt_digest: String,
    pub admission_status: UpstreamAdmissionStatus,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationValue {
    pub feature_id: String,
    pub unit_id: String,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationSnapshot {
    pub observed_at_ns: u64,
    pub sensor_manifest_digest: String,
    pub calibration_digest: String,
    pub clock_map_digest: String,
    pub values: Vec<ObservationValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosabilityProblem {
    pub now_ns: u64,
    pub action_id: String,
    pub action_duration_ms: u64,
    pub physical_fault_set_digest: String,
    pub current_state_digest: String,
    pub model_digest: String,
    pub actuator_digest: String,
    pub domain_digest: String,
    pub sensor_manifest_digest: String,
    pub calibration_digest: String,
    pub clock_map_digest: String,
    /// Complete authoritative still-possible fault set from the upstream
    /// fault-state authority. Never replace with a classifier top-1 result.
    pub physically_possible_fault_ids: Vec<String>,
    pub feature_schema: Vec<FeatureSpec>,
    pub envelopes: Vec<HypothesisEnvelope>,
    pub receipt: DiagnosticActionReceipt,
    pub observation: Option<ObservationSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairStatus {
    Separable {
        feature_id: String,
        separation_margin: u128,
    },
    Overlapping,
    UnknownOverlapping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairRelation {
    pub left_hypothesis_id: String,
    pub right_hypothesis_id: String,
    pub status: PairStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationDisposition {
    NotObserved,
    UniqueModelConditional { hypothesis_id: String },
    Ambiguous { candidate_hypothesis_ids: Vec<String> },
    UnknownCandidate { candidate_hypothesis_ids: Vec<String> },
    OutOfModel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosabilityReport {
    pub action_id: String,
    pub action_duration_ms: u64,
    pub physical_fault_set_digest: String,
    pub upstream_continuation_receipt_digest: String,
    /// Pairs are in canonical lexicographic hypothesis order.
    pub pair_relations: Vec<PairRelation>,
    /// Connected components of the non-separable graph; a component of size
    /// one is only model-conditional separability for this action/context.
    pub ambiguity_components: Vec<Vec<String>>,
    pub observation_disposition: ObservationDisposition,
    pub candidate_hypothesis_ids: Vec<String>,
    pub feature_schema_ids: Vec<String>,
    pub common_cause_groups: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosabilityError {
    EmptyIdentifier,
    NoPhysicalFaultHypotheses,
    DuplicatePhysicalFaultHypothesis,
    NoFeatureSchema,
    DuplicateFeatureId,
    DuplicateHypothesisEnvelope,
    MissingHypothesisEnvelope,
    EnvelopeForImpossibleHypothesis,
    EmptyUnknownReason,
    UnknownEnvelopeHasFeatures,
    EnvelopeFeatureCardinalityMismatch,
    DuplicateEnvelopeFeature,
    EnvelopeFeatureOutsideSchema,
    EnvelopeUnitMismatch,
    InvertedInterval,
    ObservationSchemaMismatch,
    DuplicateObservationFeature,
    ObservationFeatureOutsideSchema,
    ObservationUnitMismatch,
    StaleReceipt,
    InvalidReceiptInterval,
    ReceiptContextMismatch,
    ReceiptDurationMismatch,
    InvalidActionDuration,
    DiagnosticActionRejected,
    DiagnosticActionUnknown,
    EvaluationBeforeObservation,
    ObservationOutsideReceiptWindow,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_identifiers(values: &[String]) -> Result<(), DiagnosabilityError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !nonempty(value) {
            return Err(DiagnosabilityError::EmptyIdentifier);
        }
        if !seen.insert(value.as_str()) {
            return Err(DiagnosabilityError::DuplicatePhysicalFaultHypothesis);
        }
    }
    Ok(())
}

fn validate_receipt(problem: &DiagnosabilityProblem) -> Result<(), DiagnosabilityError> {
    let receipt = &problem.receipt;
    let identifiers = [
        receipt.receipt_id.as_str(),
        receipt.action_id.as_str(),
        receipt.physical_fault_set_digest.as_str(),
        receipt.current_state_digest.as_str(),
        receipt.model_digest.as_str(),
        receipt.actuator_digest.as_str(),
        receipt.domain_digest.as_str(),
        receipt.sensor_manifest_digest.as_str(),
        receipt.calibration_digest.as_str(),
        receipt.clock_map_digest.as_str(),
        receipt.result_digest.as_str(),
        receipt.upstream_continuation_receipt_digest.as_str(),
    ];
    if identifiers.iter().any(|value| !nonempty(value)) {
        return Err(DiagnosabilityError::EmptyIdentifier);
    }
    if receipt.valid_from_ns > receipt.valid_until_ns {
        return Err(DiagnosabilityError::InvalidReceiptInterval);
    }
    if problem.now_ns < receipt.valid_from_ns || problem.now_ns > receipt.valid_until_ns {
        return Err(DiagnosabilityError::StaleReceipt);
    }
    if receipt.action_id != problem.action_id
        || receipt.physical_fault_set_digest != problem.physical_fault_set_digest
        || receipt.current_state_digest != problem.current_state_digest
        || receipt.model_digest != problem.model_digest
        || receipt.actuator_digest != problem.actuator_digest
        || receipt.domain_digest != problem.domain_digest
        || receipt.sensor_manifest_digest != problem.sensor_manifest_digest
        || receipt.calibration_digest != problem.calibration_digest
        || receipt.clock_map_digest != problem.clock_map_digest
    {
        return Err(DiagnosabilityError::ReceiptContextMismatch);
    }
    if receipt.action_duration_ms != problem.action_duration_ms {
        return Err(DiagnosabilityError::ReceiptDurationMismatch);
    }
    match receipt.admission_status {
        UpstreamAdmissionStatus::Admitted => {}
        UpstreamAdmissionStatus::Rejected => {
            return Err(DiagnosabilityError::DiagnosticActionRejected);
        }
        UpstreamAdmissionStatus::Unknown => {
            return Err(DiagnosabilityError::DiagnosticActionUnknown);
        }
    }
    Ok(())
}

fn canonicalize_and_validate(
    problem: &DiagnosabilityProblem,
) -> Result<(Vec<FeatureSpec>, Vec<HypothesisEnvelope>), DiagnosabilityError> {
    let root_ids = [
        problem.action_id.as_str(),
        problem.physical_fault_set_digest.as_str(),
        problem.current_state_digest.as_str(),
        problem.model_digest.as_str(),
        problem.actuator_digest.as_str(),
        problem.domain_digest.as_str(),
        problem.sensor_manifest_digest.as_str(),
        problem.calibration_digest.as_str(),
        problem.clock_map_digest.as_str(),
    ];
    if root_ids.iter().any(|value| !nonempty(value)) {
        return Err(DiagnosabilityError::EmptyIdentifier);
    }
    validate_receipt(problem)?;
    if problem.action_duration_ms == 0 {
        return Err(DiagnosabilityError::InvalidActionDuration);
    }
    if problem.physically_possible_fault_ids.is_empty() {
        return Err(DiagnosabilityError::NoPhysicalFaultHypotheses);
    }
    validate_identifiers(&problem.physically_possible_fault_ids)?;

    if problem.feature_schema.is_empty() {
        return Err(DiagnosabilityError::NoFeatureSchema);
    }
    let mut schema = problem.feature_schema.clone();
    schema.sort_by(|left, right| left.feature_id.cmp(&right.feature_id));
    let mut schema_ids = BTreeSet::new();
    for feature in &schema {
        if !nonempty(&feature.feature_id)
            || !nonempty(&feature.unit_id)
            || !nonempty(&feature.common_cause_group)
        {
            return Err(DiagnosabilityError::EmptyIdentifier);
        }
        if !schema_ids.insert(feature.feature_id.as_str()) {
            return Err(DiagnosabilityError::DuplicateFeatureId);
        }
    }

    let physical_ids: BTreeSet<&str> = problem
        .physically_possible_fault_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut envelope_map = BTreeMap::new();
    for envelope in &problem.envelopes {
        if !nonempty(&envelope.hypothesis_id) {
            return Err(DiagnosabilityError::EmptyIdentifier);
        }
        if !physical_ids.contains(envelope.hypothesis_id.as_str()) {
            return Err(DiagnosabilityError::EnvelopeForImpossibleHypothesis);
        }
        if envelope_map
            .insert(envelope.hypothesis_id.as_str(), envelope)
            .is_some()
        {
            return Err(DiagnosabilityError::DuplicateHypothesisEnvelope);
        }
        match &envelope.status {
            EnvelopeStatus::Unknown { reason } => {
                if !nonempty(reason) {
                    return Err(DiagnosabilityError::EmptyUnknownReason);
                }
                if !envelope.features.is_empty() {
                    return Err(DiagnosabilityError::UnknownEnvelopeHasFeatures);
                }
            }
            EnvelopeStatus::Bounded => {
                if envelope.features.len() != schema.len() {
                    return Err(DiagnosabilityError::EnvelopeFeatureCardinalityMismatch);
                }
                let mut feature_map = BTreeMap::new();
                for feature in &envelope.features {
                    if !nonempty(&feature.feature_id) || !nonempty(&feature.unit_id) {
                        return Err(DiagnosabilityError::EmptyIdentifier);
                    }
                    if feature.lower > feature.upper {
                        return Err(DiagnosabilityError::InvertedInterval);
                    }
                    if feature_map
                        .insert(feature.feature_id.as_str(), feature)
                        .is_some()
                    {
                        return Err(DiagnosabilityError::DuplicateEnvelopeFeature);
                    }
                }
                for spec in &schema {
                    let feature = feature_map
                        .get(spec.feature_id.as_str())
                        .ok_or(DiagnosabilityError::EnvelopeFeatureOutsideSchema)?;
                    if feature.unit_id != spec.unit_id {
                        return Err(DiagnosabilityError::EnvelopeUnitMismatch);
                    }
                }
                if feature_map.len() != schema.len() {
                    return Err(DiagnosabilityError::EnvelopeFeatureOutsideSchema);
                }
            }
        }
    }

    if envelope_map.len() != physical_ids.len()
        || physical_ids
            .iter()
            .any(|hypothesis_id| !envelope_map.contains_key(*hypothesis_id))
    {
        return Err(DiagnosabilityError::MissingHypothesisEnvelope);
    }

    let mut envelopes: Vec<HypothesisEnvelope> = problem.envelopes.clone();
    envelopes.sort_by(|left, right| left.hypothesis_id.cmp(&right.hypothesis_id));
    for envelope in &mut envelopes {
        envelope
            .features
            .sort_by(|left, right| left.feature_id.cmp(&right.feature_id));
    }

    Ok((schema, envelopes))
}

fn pair_relation(
    left: &HypothesisEnvelope,
    right: &HypothesisEnvelope,
) -> PairRelation {
    if matches!(&left.status, EnvelopeStatus::Unknown { .. })
        || matches!(&right.status, EnvelopeStatus::Unknown { .. })
    {
        return PairRelation {
            left_hypothesis_id: left.hypothesis_id.clone(),
            right_hypothesis_id: right.hypothesis_id.clone(),
            status: PairStatus::UnknownOverlapping,
        };
    }

    let mut best_separating_feature: Option<(String, u128)> = None;
    for (left_feature, right_feature) in left.features.iter().zip(&right.features) {
        if left_feature.upper < right_feature.lower {
            let margin = (i128::from(right_feature.lower) - i128::from(left_feature.upper))
                as u128;
            update_best_separation(
                &mut best_separating_feature,
                left_feature.feature_id.clone(),
                margin,
            );
        } else if right_feature.upper < left_feature.lower {
            let margin = (i128::from(left_feature.lower) - i128::from(right_feature.upper))
                as u128;
            update_best_separation(
                &mut best_separating_feature,
                left_feature.feature_id.clone(),
                margin,
            );
        }
    }

    let status = match best_separating_feature {
        Some((feature_id, separation_margin)) => PairStatus::Separable {
            feature_id,
            separation_margin,
        },
        None => PairStatus::Overlapping,
    };
    PairRelation {
        left_hypothesis_id: left.hypothesis_id.clone(),
        right_hypothesis_id: right.hypothesis_id.clone(),
        status,
    }
}

fn update_best_separation(
    best: &mut Option<(String, u128)>,
    feature_id: String,
    margin: u128,
) {
    match best {
        None => *best = Some((feature_id, margin)),
        Some((best_id, best_margin))
            if margin > *best_margin || (margin == *best_margin && feature_id < *best_id) =>
        {
            *best = Some((feature_id, margin));
        }
        Some(_) => {}
    }
}

fn build_pair_relations(envelopes: &[HypothesisEnvelope]) -> Vec<PairRelation> {
    let mut relations = Vec::new();
    for left_index in 0..envelopes.len() {
        for right_index in (left_index + 1)..envelopes.len() {
            relations.push(pair_relation(
                &envelopes[left_index],
                &envelopes[right_index],
            ));
        }
    }
    relations
}

fn build_ambiguity_components(
    envelopes: &[HypothesisEnvelope],
    relations: &[PairRelation],
) -> Vec<Vec<String>> {
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    for envelope in envelopes {
        adjacency.insert(envelope.hypothesis_id.clone(), BTreeSet::new());
    }
    for relation in relations {
        if !matches!(&relation.status, PairStatus::Separable { .. }) {
            if let Some(neighbors) = adjacency.get_mut(&relation.left_hypothesis_id) {
                neighbors.insert(relation.right_hypothesis_id.clone());
            }
            if let Some(neighbors) = adjacency.get_mut(&relation.right_hypothesis_id) {
                neighbors.insert(relation.left_hypothesis_id.clone());
            }
        }
    }

    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    for hypothesis_id in adjacency.keys() {
        if visited.contains(hypothesis_id) {
            continue;
        }
        let mut queue = VecDeque::from([hypothesis_id.clone()]);
        let mut component = BTreeSet::new();
        visited.insert(hypothesis_id.clone());
        while let Some(current) = queue.pop_front() {
            component.insert(current.clone());
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if visited.insert(neighbor.clone()) {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        components.push(component.into_iter().collect());
    }
    components.sort();
    components
}

fn evaluate_observation(
    observation: Option<&ObservationSnapshot>,
    schema: &[FeatureSpec],
    envelopes: &[HypothesisEnvelope],
    problem: &DiagnosabilityProblem,
) -> Result<(ObservationDisposition, Vec<String>), DiagnosabilityError> {
    let Some(observation) = observation else {
        return Ok((ObservationDisposition::NotObserved, Vec::new()));
    };
    if observation.observed_at_ns > problem.now_ns {
        return Err(DiagnosabilityError::EvaluationBeforeObservation);
    }
    if observation.observed_at_ns < problem.receipt.valid_from_ns
        || observation.observed_at_ns > problem.receipt.valid_until_ns
    {
        return Err(DiagnosabilityError::ObservationOutsideReceiptWindow);
    }
    if observation.sensor_manifest_digest != problem.sensor_manifest_digest
        || observation.calibration_digest != problem.calibration_digest
        || observation.clock_map_digest != problem.clock_map_digest
    {
        return Err(DiagnosabilityError::ReceiptContextMismatch);
    }

    if observation.values.len() != schema.len() {
        return Err(DiagnosabilityError::ObservationSchemaMismatch);
    }
    let mut values = BTreeMap::new();
    for value in &observation.values {
        if !nonempty(&value.feature_id) || !nonempty(&value.unit_id) {
            return Err(DiagnosabilityError::EmptyIdentifier);
        }
        if values.insert(value.feature_id.as_str(), value).is_some() {
            return Err(DiagnosabilityError::DuplicateObservationFeature);
        }
    }
    for feature in schema {
        let value = values
            .get(feature.feature_id.as_str())
            .ok_or(DiagnosabilityError::ObservationSchemaMismatch)?;
        if value.unit_id != feature.unit_id {
            return Err(DiagnosabilityError::ObservationUnitMismatch);
        }
    }
    if values.len() != schema.len() {
        return Err(DiagnosabilityError::ObservationFeatureOutsideSchema);
    }

    let mut candidates = Vec::new();
    let mut unknown_candidate = false;
    for envelope in envelopes {
        match &envelope.status {
            EnvelopeStatus::Unknown { .. } => {
                candidates.push(envelope.hypothesis_id.clone());
                unknown_candidate = true;
            }
            EnvelopeStatus::Bounded => {
                let contains = envelope.features.iter().all(|interval| {
                    values.get(interval.feature_id.as_str()).is_some_and(|value| {
                        value.value >= interval.lower && value.value <= interval.upper
                    })
                });
                if contains {
                    candidates.push(envelope.hypothesis_id.clone());
                }
            }
        }
    }
    candidates.sort();

    let disposition = match candidates.len() {
        0 => ObservationDisposition::OutOfModel,
        1 if unknown_candidate => {
            ObservationDisposition::UnknownCandidate {
                candidate_hypothesis_ids: candidates.clone(),
            }
        }
        1 => ObservationDisposition::UniqueModelConditional {
            hypothesis_id: candidates[0].clone(),
        },
        _ if unknown_candidate => ObservationDisposition::UnknownCandidate {
            candidate_hypothesis_ids: candidates.clone(),
        },
        _ => ObservationDisposition::Ambiguous {
            candidate_hypothesis_ids: candidates.clone(),
        },
    };
    Ok((disposition, candidates))
}

/// Audit finite action-conditioned observation envelopes over the full
/// authoritative physical fault hypothesis set. A singleton candidate is
/// model-conditional and never, by itself, authorizes a control action.
pub fn audit_diagnosability(
    problem: &DiagnosabilityProblem,
) -> Result<DiagnosabilityReport, DiagnosabilityError> {
    let (schema, envelopes) = canonicalize_and_validate(problem)?;
    let pair_relations = build_pair_relations(&envelopes);
    let ambiguity_components = build_ambiguity_components(&envelopes, &pair_relations);
    let (observation_disposition, candidate_hypothesis_ids) =
        evaluate_observation(problem.observation.as_ref(), &schema, &envelopes, problem)?;

    Ok(DiagnosabilityReport {
        action_id: problem.action_id.clone(),
        action_duration_ms: problem.action_duration_ms,
        physical_fault_set_digest: problem.physical_fault_set_digest.clone(),
        upstream_continuation_receipt_digest: problem
            .receipt
            .upstream_continuation_receipt_digest
            .clone(),
        pair_relations,
        ambiguity_components,
        observation_disposition,
        candidate_hypothesis_ids,
        feature_schema_ids: schema
            .iter()
            .map(|feature| feature.feature_id.clone())
            .collect(),
        common_cause_groups: schema
            .iter()
            .map(|feature| feature.common_cause_group.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Vec<FeatureSpec> {
        vec![
            FeatureSpec {
                feature_id: "joint_velocity_residual".to_owned(),
                unit_id: "mrad_per_s".to_owned(),
                common_cause_group: "joint-encoder-1".to_owned(),
            },
            FeatureSpec {
                feature_id: "motor_current_residual".to_owned(),
                unit_id: "milliamp".to_owned(),
                common_cause_group: "current-sensor-1".to_owned(),
            },
        ]
    }

    fn interval(feature_id: &str, unit_id: &str, lower: i64, upper: i64) -> FeatureInterval {
        FeatureInterval {
            feature_id: feature_id.to_owned(),
            unit_id: unit_id.to_owned(),
            lower,
            upper,
        }
    }

    fn bounded(hypothesis_id: &str, velocity: (i64, i64), current: (i64, i64)) -> HypothesisEnvelope {
        HypothesisEnvelope {
            hypothesis_id: hypothesis_id.to_owned(),
            status: EnvelopeStatus::Bounded,
            features: vec![
                interval(
                    "joint_velocity_residual",
                    "mrad_per_s",
                    velocity.0,
                    velocity.1,
                ),
                interval("motor_current_residual", "milliamp", current.0, current.1),
            ],
        }
    }

    fn receipt() -> DiagnosticActionReceipt {
        DiagnosticActionReceipt {
            receipt_id: "diagnostic-receipt-v1".to_owned(),
            action_id: "diagnostic-squat-sway-v1".to_owned(),
            action_duration_ms: 400,
            physical_fault_set_digest: "fault-set-v1".to_owned(),
            current_state_digest: "state-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            domain_digest: "domain-v1".to_owned(),
            sensor_manifest_digest: "sensors-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            clock_map_digest: "clock-v1".to_owned(),
            result_digest: "envelope-result-v1".to_owned(),
            upstream_continuation_receipt_digest: "viable-continuation-v1".to_owned(),
            admission_status: UpstreamAdmissionStatus::Admitted,
            valid_from_ns: 100,
            valid_until_ns: 1_000,
        }
    }

    fn problem(envelopes: Vec<HypothesisEnvelope>) -> DiagnosabilityProblem {
        DiagnosabilityProblem {
            now_ns: 500,
            action_id: "diagnostic-squat-sway-v1".to_owned(),
            action_duration_ms: 400,
            physical_fault_set_digest: "fault-set-v1".to_owned(),
            current_state_digest: "state-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            domain_digest: "domain-v1".to_owned(),
            sensor_manifest_digest: "sensors-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            clock_map_digest: "clock-v1".to_owned(),
            physically_possible_fault_ids: envelopes
                .iter()
                .map(|item| item.hypothesis_id.clone())
                .collect(),
            feature_schema: schema(),
            envelopes,
            receipt: receipt(),
            observation: None,
        }
    }

    fn observation(velocity: i64, current: i64) -> ObservationSnapshot {
        ObservationSnapshot {
            observed_at_ns: 600,
            sensor_manifest_digest: "sensors-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            clock_map_digest: "clock-v1".to_owned(),
            values: vec![
                ObservationValue {
                    feature_id: "joint_velocity_residual".to_owned(),
                    unit_id: "mrad_per_s".to_owned(),
                    value: velocity,
                },
                ObservationValue {
                    feature_id: "motor_current_residual".to_owned(),
                    unit_id: "milliamp".to_owned(),
                    value: current,
                },
            ],
        }
    }

    #[test]
    fn disjoint_observation_boxes_are_separable() {
        let input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            bounded("torque-derated", (40, 60), (10, 30)),
        ]);
        let report = audit_diagnosability(&input).expect("separable envelopes");
        assert_eq!(report.pair_relations.len(), 1);
        assert_eq!(
            report.pair_relations[0].status,
            PairStatus::Separable {
                feature_id: "joint_velocity_residual".to_owned(),
                separation_margin: 30,
            }
        );
        assert_eq!(report.ambiguity_components.len(), 2);
    }

    #[test]
    fn overlapping_low_load_signatures_remain_ambiguous() {
        let input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            bounded("friction-increase", (-5, 15), (10, 35)),
        ]);
        let report = audit_diagnosability(&input).expect("overlapping envelopes");
        assert_eq!(report.pair_relations[0].status, PairStatus::Overlapping);
        assert_eq!(
            report.ambiguity_components,
            vec![vec![
                "friction-increase".to_owned(),
                "joint-locked".to_owned()
            ]]
        );
    }

    #[test]
    fn touching_closed_intervals_overlap() {
        let input = problem(vec![
            bounded("fault-a", (-10, 10), (0, 20)),
            bounded("fault-b", (10, 30), (30, 40)),
        ]);
        let report = audit_diagnosability(&input).expect("touching boxes");
        assert_eq!(report.pair_relations[0].status, PairStatus::Overlapping);
    }

    #[test]
    fn unknown_envelope_overlaps_every_other_hypothesis() {
        let input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            HypothesisEnvelope {
                hypothesis_id: "unmodeled-fault".to_owned(),
                status: EnvelopeStatus::Unknown {
                    reason: "outside-qualified-fault-model".to_owned(),
                },
                features: Vec::new(),
            },
            bounded("torque-derated", (40, 60), (40, 60)),
        ]);
        let report = audit_diagnosability(&input).expect("unknown envelope");
        assert_eq!(report.pair_relations.len(), 3);
        assert_eq!(
            report.pair_relations[0].status,
            PairStatus::Separable {
                feature_id: "joint_velocity_residual".to_owned(),
                separation_margin: 30,
            }
        );
        assert_eq!(
            report.pair_relations[1].status,
            PairStatus::UnknownOverlapping
        );
        assert_eq!(
            report.ambiguity_components,
            vec![vec![
                "joint-locked".to_owned(),
                "torque-derated".to_owned(),
                "unmodeled-fault".to_owned()
            ]]
        );
    }

    #[test]
    fn actual_observation_selects_one_model_conditional_candidate() {
        let mut input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            bounded("torque-derated", (40, 60), (40, 60)),
        ]);
        input.observation = Some(observation(0, 10));
        let report = audit_diagnosability(&input).expect("unique observation");
        assert_eq!(
            report.observation_disposition,
            ObservationDisposition::UniqueModelConditional {
                hypothesis_id: "joint-locked".to_owned()
            }
        );
    }

    #[test]
    fn actual_observation_in_multiple_envelopes_returns_ambiguity_set() {
        let mut input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            bounded("friction-increase", (-5, 15), (10, 35)),
        ]);
        input.observation = Some(observation(5, 15));
        let report = audit_diagnosability(&input).expect("ambiguous observation");
        assert_eq!(
            report.observation_disposition,
            ObservationDisposition::Ambiguous {
                candidate_hypothesis_ids: vec![
                    "friction-increase".to_owned(),
                    "joint-locked".to_owned()
                ]
            }
        );
    }

    #[test]
    fn unknown_candidate_prevents_unique_isolation() {
        let mut input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            HypothesisEnvelope {
                hypothesis_id: "unknown-fault".to_owned(),
                status: EnvelopeStatus::Unknown {
                    reason: "unknown-response".to_owned(),
                },
                features: Vec::new(),
            },
        ]);
        input.observation = Some(observation(0, 10));
        let report = audit_diagnosability(&input).expect("unknown candidate");
        assert_eq!(
            report.observation_disposition,
            ObservationDisposition::UnknownCandidate {
                candidate_hypothesis_ids: vec![
                    "joint-locked".to_owned(),
                    "unknown-fault".to_owned()
                ]
            }
        );
    }

    #[test]
    fn observation_outside_every_envelope_is_ood_not_nearest_class() {
        let mut input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            bounded("torque-derated", (40, 60), (40, 60)),
        ]);
        input.observation = Some(observation(999, 999));
        let report = audit_diagnosability(&input).expect("ood observation");
        assert_eq!(
            report.observation_disposition,
            ObservationDisposition::OutOfModel
        );
        assert!(report.candidate_hypothesis_ids.is_empty());
    }

    #[test]
    fn missing_or_mismatched_observation_feature_is_rejected() {
        let mut input = problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        input.observation = Some(ObservationSnapshot {
            observed_at_ns: 600,
            sensor_manifest_digest: "sensors-v1".to_owned(),
            calibration_digest: "calibration-v1".to_owned(),
            clock_map_digest: "clock-v1".to_owned(),
            values: vec![ObservationValue {
                feature_id: "joint_velocity_residual".to_owned(),
                unit_id: "mrad_per_s".to_owned(),
                value: 0,
            }],
        });
        assert_eq!(
            audit_diagnosability(&input),
            Err(DiagnosabilityError::ObservationSchemaMismatch)
        );
    }

    #[test]
    fn stale_or_mismatched_receipts_prevent_identifiability_reuse() {
        let mut stale = problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        stale.receipt.valid_until_ns = 499;
        assert_eq!(
            audit_diagnosability(&stale),
            Err(DiagnosabilityError::StaleReceipt)
        );

        let mut wrong_fault_set =
            problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        wrong_fault_set.receipt.physical_fault_set_digest = "another-set".to_owned();
        assert_eq!(
            audit_diagnosability(&wrong_fault_set),
            Err(DiagnosabilityError::ReceiptContextMismatch)
        );
    }

    #[test]
    fn rejected_or_unknown_diagnostic_action_is_not_analysed_as_admitted() {
        let mut rejected = problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        rejected.receipt.admission_status = UpstreamAdmissionStatus::Rejected;
        assert_eq!(
            audit_diagnosability(&rejected),
            Err(DiagnosabilityError::DiagnosticActionRejected)
        );

        let mut unknown = problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        unknown.receipt.admission_status = UpstreamAdmissionStatus::Unknown;
        assert_eq!(
            audit_diagnosability(&unknown),
            Err(DiagnosabilityError::DiagnosticActionUnknown)
        );
    }

    #[test]
    fn receipt_reuse_after_calibration_or_action_duration_change_is_rejected() {
        let mut wrong_calibration =
            problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        wrong_calibration.calibration_digest = "calibration-v2".to_owned();
        assert_eq!(
            audit_diagnosability(&wrong_calibration),
            Err(DiagnosabilityError::ReceiptContextMismatch)
        );

        let mut wrong_duration =
            problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        wrong_duration.action_duration_ms = 600;
        assert_eq!(
            audit_diagnosability(&wrong_duration),
            Err(DiagnosabilityError::ReceiptDurationMismatch)
        );
    }

    #[test]
    fn observation_outside_receipt_time_window_is_rejected() {
        let mut input = problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        let mut item = observation(0, 10);
        item.observed_at_ns = 99;
        input.observation = Some(item);
        assert_eq!(
            audit_diagnosability(&input),
            Err(DiagnosabilityError::ObservationOutsideReceiptWindow)
        );
    }

    #[test]
    fn feature_id_and_unit_schema_must_match_exactly() {
        let mut input = problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        input.envelopes[0].features[0].unit_id = "degrees_per_s".to_owned();
        assert_eq!(
            audit_diagnosability(&input),
            Err(DiagnosabilityError::EnvelopeUnitMismatch)
        );
    }

    #[test]
    fn permutation_of_hypotheses_and_features_preserves_canonical_report() {
        let mut original_input = problem(vec![
            bounded("joint-locked", (-10, 10), (0, 20)),
            bounded("friction-increase", (-5, 15), (10, 35)),
            bounded("torque-derated", (40, 60), (40, 60)),
        ]);
        original_input.observation = Some(observation(0, 10));
        let original = audit_diagnosability(&original_input).expect("original");

        let mut permuted = original_input.clone();
        permuted.physically_possible_fault_ids.reverse();
        permuted.envelopes.reverse();
        permuted.feature_schema.reverse();
        for envelope in &mut permuted.envelopes {
            envelope.features.reverse();
        }
        permuted.observation.as_mut().unwrap().values.reverse();

        let reordered = audit_diagnosability(&permuted).expect("permuted");
        assert_eq!(original, reordered);
    }

    #[test]
    fn inverted_interval_and_empty_unknown_reason_are_rejected() {
        let mut inverted = problem(vec![bounded("joint-locked", (-10, 10), (0, 20))]);
        inverted.envelopes[0].features[0].lower = 1;
        inverted.envelopes[0].features[0].upper = 0;
        assert_eq!(
            audit_diagnosability(&inverted),
            Err(DiagnosabilityError::InvertedInterval)
        );

        let mut unknown = problem(vec![HypothesisEnvelope {
            hypothesis_id: "unknown".to_owned(),
            status: EnvelopeStatus::Unknown {
                reason: String::new(),
            },
            features: Vec::new(),
        }]);
        unknown.physically_possible_fault_ids = vec!["unknown".to_owned()];
        assert_eq!(
            audit_diagnosability(&unknown),
            Err(DiagnosabilityError::EmptyUnknownReason)
        );
    }

    #[test]
    fn common_cause_groups_are_preserved_in_canonical_report() {
        let report = audit_diagnosability(&problem(vec![bounded(
            "joint-locked",
            (-10, 10),
            (0, 20),
        )]))
        .expect("one hypothesis");
        assert_eq!(
            report.common_cause_groups,
            vec!["current-sensor-1".to_owned(), "joint-encoder-1".to_owned()]
        );
    }
}
