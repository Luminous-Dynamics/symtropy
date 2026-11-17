//! Dependence-aware multimodal tactile evidence fusion and conflict abstention.
//!
//! Four singleton contact-state masses plus an uncommitted/ignorance mass are
//! retained per modality. Pairwise Dempster-Shafer conflict K is reported
//! before any normalization. Evidence is admitted only when a pair-specific
//! independence attestation binds the exact source observations and the two
//! evidence paths are not obviously the same stream/readout/common-cause group.
//!
//! High conflict triggers abstention on the primary path. Unconditional
//! normalized Dempster fusion is exposed only as a research comparator.
//! This module is an evidence contract, not a slip detector.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

const CONTACT_STATE_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct MassAssignment {
    /// Singleton masses in the fixed order declared by ContactState.
    pub singleton: [f64; CONTACT_STATE_COUNT],
    /// Mass assigned to the whole frame of discernment (ignorance).
    pub ignorance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ContactState {
    NoContact,
    Stick,
    IncipientSlip,
    GrossSlip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum EvidenceIndependenceDisposition {
    IndependentlyVerified,
    SharedCommonCause,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ModalityEvidence {
    pub source_id: String,
    pub modality: String,
    pub interaction_event_id: String,
    pub raw_observation_digest: String,
    pub sample_identity: String,
    pub clock_domain: String,
    pub event_timestamp_ns: u64,
    pub timestamp_uncertainty_ns: u64,
    pub sensor_article_digest: String,
    pub readout_chain_digest: String,
    pub body_context_digest: String,
    pub operating_envelope_cell_id: String,
    pub causal_group_id: String,
    pub reference_ancestry_receipt_id: String,
    pub channel_qualified: bool,
    pub observable: bool,
    pub saturated: bool,
    pub mass: MassAssignment,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PairIndependenceAttestation {
    pub attestation_id: String,
    pub interaction_event_id: String,
    pub left_source_id: String,
    pub right_source_id: String,
    pub left_observation_digest: String,
    pub right_observation_digest: String,
    pub left_ancestry_receipt_id: String,
    pub right_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
    pub disposition: EvidenceIndependenceDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceFusionRequest {
    pub interaction_event_id: String,
    pub body_context_digest: String,
    pub modalities: Vec<ModalityEvidence>,
    pub pairwise_independence: Vec<PairIndependenceAttestation>,
    pub maximum_pairwise_time_skew_ns: u64,
    pub maximum_conflict: f64,
    pub normalization_epsilon: f64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct PairConflictReport {
    pub left_source_id: String,
    pub right_source_id: String,
    pub conflict_k: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum EvidenceFusionDisposition {
    SafelyCombined,
    ConflictTriggeredAbstention,
    DependenceUnresolved,
    TemporallyUnresolved,
    InvalidEvidence,
    NoEvidence,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct EvidenceFusionReport {
    pub disposition: EvidenceFusionDisposition,
    pub interaction_event_id: String,
    pub source_masses: Vec<ModalityEvidence>,
    pub pairwise_conflicts: Vec<PairConflictReport>,
    pub maximum_pairwise_conflict: Option<f64>,
    pub cumulative_dempster_conflict: Option<f64>,
    /// Primary path: only populated when independence, timing and conflict gates pass.
    pub safe_combined_mass: Option<MassAssignment>,
    /// Unconditional normalized Dempster result for comparison only.
    pub unconditional_dempster_comparator: Option<MassAssignment>,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

impl MassAssignment {
    pub fn validate(&self, tolerance: f64) -> Result<(), String> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("mass validation tolerance must be finite and positive".to_owned());
        }
        if self
            .singleton
            .iter()
            .chain(std::iter::once(&self.ignorance))
            .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
        {
            return Err("mass values must be finite and lie within [0,1]".to_owned());
        }
        let total = self.singleton.iter().sum::<f64>() + self.ignorance;
        if (total - 1.0).abs() > tolerance {
            return Err("singleton plus ignorance masses must sum to one".to_owned());
        }
        Ok(())
    }
}

fn pair_conflict(left: MassAssignment, right: MassAssignment) -> f64 {
    let mut conflict = 0.0;
    for left_state in 0..CONTACT_STATE_COUNT {
        for right_state in 0..CONTACT_STATE_COUNT {
            if left_state != right_state {
                conflict += left.singleton[left_state] * right.singleton[right_state];
            }
        }
    }
    conflict
}

fn dempster_combine(
    left: MassAssignment,
    right: MassAssignment,
    normalization_epsilon: f64,
) -> (f64, Option<MassAssignment>) {
    let conflict = pair_conflict(left, right);
    let denominator = 1.0 - conflict;
    if !denominator.is_finite() || denominator <= normalization_epsilon {
        return (conflict, None);
    }

    let mut singleton = [0.0; CONTACT_STATE_COUNT];
    for state in 0..CONTACT_STATE_COUNT {
        singleton[state] = left.singleton[state] * right.singleton[state]
            + left.singleton[state] * right.ignorance
            + left.ignorance * right.singleton[state];
        singleton[state] /= denominator;
    }
    let ignorance = left.ignorance * right.ignorance / denominator;
    (
        conflict,
        Some(MassAssignment {
            singleton,
            ignorance,
        }),
    )
}

fn ordered_pair<'a>(
    left: &'a ModalityEvidence,
    right: &'a ModalityEvidence,
) -> (&'a ModalityEvidence, &'a ModalityEvidence) {
    if left.source_id <= right.source_id {
        (left, right)
    } else {
        (right, left)
    }
}

fn validate_source(
    source: &ModalityEvidence,
    request: &EvidenceFusionRequest,
) -> Result<(), String> {
    for (name, value) in [
        ("source_id", source.source_id.as_str()),
        ("modality", source.modality.as_str()),
        ("interaction_event_id", source.interaction_event_id.as_str()),
        ("raw_observation_digest", source.raw_observation_digest.as_str()),
        ("sample_identity", source.sample_identity.as_str()),
        ("clock_domain", source.clock_domain.as_str()),
        ("sensor_article_digest", source.sensor_article_digest.as_str()),
        ("readout_chain_digest", source.readout_chain_digest.as_str()),
        ("body_context_digest", source.body_context_digest.as_str()),
        ("operating_envelope_cell_id", source.operating_envelope_cell_id.as_str()),
        ("causal_group_id", source.causal_group_id.as_str()),
        ("reference_ancestry_receipt_id", source.reference_ancestry_receipt_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("modality source field {name} is missing"));
        }
    }
    if source.interaction_event_id != request.interaction_event_id
        || source.body_context_digest != request.body_context_digest
    {
        return Err(format!("source {} does not bind the requested interaction/body context", source.source_id));
    }
    source.mass.validate(1.0e-9)?;
    if !source.channel_qualified || !source.observable || source.saturated {
        return Err(format!("source {} is unqualified, unobservable or saturated", source.source_id));
    }
    Ok(())
}

fn validate_pair_attestation(
    left: &ModalityEvidence,
    right: &ModalityEvidence,
    interaction_event_id: &str,
    attestations: &[PairIndependenceAttestation],
) -> Result<(), (EvidenceFusionDisposition, String)> {
    let (left, right) = ordered_pair(left, right);
    if left.raw_observation_digest == right.raw_observation_digest {
        return Err((
            EvidenceFusionDisposition::DependenceUnresolved,
            "modalities point to the same raw observation digest".to_owned(),
        ));
    }
    if left.readout_chain_digest == right.readout_chain_digest {
        return Err((
            EvidenceFusionDisposition::DependenceUnresolved,
            "modalities share the same readout chain".to_owned(),
        ));
    }
    if left.causal_group_id == right.causal_group_id {
        return Err((
            EvidenceFusionDisposition::DependenceUnresolved,
            "modalities share the same declared causal/common-cause group".to_owned(),
        ));
    }

    let matching: Vec<_> = attestations
        .iter()
        .filter(|attestation| {
            attestation.left_source_id == left.source_id
                && attestation.right_source_id == right.source_id
        })
        .collect();
    if matching.len() != 1 {
        return Err((
            EvidenceFusionDisposition::DependenceUnresolved,
            "exactly one pair-specific independence attestation is required".to_owned(),
        ));
    }
    let attestation = matching[0];
    for (name, value) in [
        ("attestation_id", attestation.attestation_id.as_str()),
        ("common_cause_analysis_receipt_id", attestation.common_cause_analysis_receipt_id.as_str()),
        ("verifier_receipt_id", attestation.verifier_receipt_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err((
                EvidenceFusionDisposition::DependenceUnresolved,
                format!("pair independence attestation field {name} is missing"),
            ));
        }
    }
    let bindings_match = attestation.interaction_event_id == interaction_event_id
        && attestation.left_observation_digest == left.raw_observation_digest
        && attestation.right_observation_digest == right.raw_observation_digest
        && attestation.left_ancestry_receipt_id == left.reference_ancestry_receipt_id
        && attestation.right_ancestry_receipt_id == right.reference_ancestry_receipt_id;
    if !bindings_match {
        return Err((
            EvidenceFusionDisposition::DependenceUnresolved,
            "pair independence attestation does not bind the exact source observations and ancestry".to_owned(),
        ));
    }
    if attestation.disposition != EvidenceIndependenceDisposition::IndependentlyVerified {
        return Err((
            EvidenceFusionDisposition::DependenceUnresolved,
            "pair independence/common-cause assessment is shared or unresolved".to_owned(),
        ));
    }
    Ok(())
}

fn base_report(
    request: &EvidenceFusionRequest,
    disposition: EvidenceFusionDisposition,
    reasons: Vec<String>,
) -> EvidenceFusionReport {
    EvidenceFusionReport {
        disposition,
        interaction_event_id: request.interaction_event_id.clone(),
        source_masses: request.modalities.clone(),
        pairwise_conflicts: Vec::new(),
        maximum_pairwise_conflict: None,
        cumulative_dempster_conflict: None,
        safe_combined_mass: None,
        unconditional_dempster_comparator: None,
        reasons,
        claim_class: "synthetic_dependence_aware_evidence_fusion_only",
    }
}

/// Admit, diagnose and combine a set of modality masses. Pairwise independence
/// is checked before conflict calculation; high conflict abstains before the
/// primary path applies Dempster normalization.
pub fn assess_evidence_fusion(request: &EvidenceFusionRequest) -> EvidenceFusionReport {
    if request.modalities.is_empty() {
        return base_report(
            request,
            EvidenceFusionDisposition::NoEvidence,
            vec!["no modality evidence was supplied".to_owned()],
        );
    }
    if request.modalities.len() < 2 {
        return base_report(
            request,
            EvidenceFusionDisposition::NoEvidence,
            vec!["at least two modalities are required for cross-modal fusion".to_owned()],
        );
    }
    if !request.maximum_conflict.is_finite()
        || !(0.0..=1.0).contains(&request.maximum_conflict)
        || !request.normalization_epsilon.is_finite()
        || request.normalization_epsilon <= 0.0
    {
        return base_report(
            request,
            EvidenceFusionDisposition::InvalidEvidence,
            vec!["conflict threshold and normalization epsilon are invalid".to_owned()],
        );
    }

    let mut source_ids = BTreeSet::new();
    for source in &request.modalities {
        if !source_ids.insert(&source.source_id) {
            return base_report(
                request,
                EvidenceFusionDisposition::InvalidEvidence,
                vec!["modality source IDs must be unique".to_owned()],
            );
        }
        if let Err(error) = validate_source(source, request) {
            return base_report(
                request,
                EvidenceFusionDisposition::InvalidEvidence,
                vec![error],
            );
        }
    }

    let mut pairwise_conflicts = Vec::new();
    let mut maximum_pairwise_conflict: f64 = 0.0;
    for left_index in 0..request.modalities.len() {
        for right_index in (left_index + 1)..request.modalities.len() {
            let left = &request.modalities[left_index];
            let right = &request.modalities[right_index];
            if let Err((disposition, reason)) = validate_pair_attestation(
                left,
                right,
                &request.interaction_event_id,
                &request.pairwise_independence,
            ) {
                return base_report(request, disposition, vec![reason]);
            }

            let time_separation = left.event_timestamp_ns.abs_diff(right.event_timestamp_ns);
            let combined_uncertainty = match left
                .timestamp_uncertainty_ns
                .checked_add(right.timestamp_uncertainty_ns)
            {
                Some(value) => value,
                None => {
                    return base_report(
                        request,
                        EvidenceFusionDisposition::TemporallyUnresolved,
                        vec!["combined modality timestamp uncertainty overflowed".to_owned()],
                    );
                }
            };
            let total_skew = match time_separation.checked_add(combined_uncertainty) {
                Some(value) => value,
                None => {
                    return base_report(
                        request,
                        EvidenceFusionDisposition::TemporallyUnresolved,
                        vec!["pairwise event-time skew overflowed".to_owned()],
                    );
                }
            };
            if total_skew > request.maximum_pairwise_time_skew_ns {
                return base_report(
                    request,
                    EvidenceFusionDisposition::TemporallyUnresolved,
                    vec![format!(
                        "modalities {} and {} exceed the permitted event-time skew",
                        left.source_id, right.source_id
                    )],
                );
            }

            let (ordered_left, ordered_right) = ordered_pair(left, right);
            let conflict = pair_conflict(ordered_left.mass, ordered_right.mass);
            maximum_pairwise_conflict = maximum_pairwise_conflict.max(conflict);
            pairwise_conflicts.push(PairConflictReport {
                left_source_id: ordered_left.source_id.clone(),
                right_source_id: ordered_right.source_id.clone(),
                conflict_k: conflict,
            });
        }
    }

    let mut unconditional = request.modalities[0].mass;
    let mut cumulative_conflict = 0.0;
    for source in request.modalities.iter().skip(1) {
        let (conflict, combined) = dempster_combine(
            unconditional,
            source.mass,
            request.normalization_epsilon,
        );
        cumulative_conflict = 1.0 - (1.0 - cumulative_conflict) * (1.0 - conflict);
        let Some(combined) = combined else {
            unconditional = MassAssignment {
                singleton: [0.0; CONTACT_STATE_COUNT],
                ignorance: 1.0,
            };
            break;
        };
        unconditional = combined;
    }

    let mut report = base_report(
        request,
        EvidenceFusionDisposition::SafelyCombined,
        Vec::new(),
    );
    report.pairwise_conflicts = pairwise_conflicts;
    report.maximum_pairwise_conflict = Some(maximum_pairwise_conflict);
    report.cumulative_dempster_conflict = Some(cumulative_conflict);
    report.unconditional_dempster_comparator = (1.0 - cumulative_conflict)
        .gt(&request.normalization_epsilon)
        .then_some(unconditional);

    if maximum_pairwise_conflict > request.maximum_conflict
        || cumulative_conflict > request.maximum_conflict
        || 1.0 - cumulative_conflict <= request.normalization_epsilon
    {
        report.disposition = EvidenceFusionDisposition::ConflictTriggeredAbstention;
        report.reasons.push(
            "pairwise or cumulative cross-modal conflict exceeds the admitted fusion threshold"
                .to_owned(),
        );
        report.safe_combined_mass = None;
        return report;
    }

    report.safe_combined_mass = Some(unconditional);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modality(
        source_id: &str,
        modality: &str,
        mass: MassAssignment,
        timestamp_ns: u64,
        causal_group_id: &str,
    ) -> ModalityEvidence {
        ModalityEvidence {
            source_id: source_id.to_owned(),
            modality: modality.to_owned(),
            interaction_event_id: "event:grasp-01".to_owned(),
            raw_observation_digest: format!("sha256:raw-{source_id}"),
            sample_identity: format!("sample-{source_id}"),
            clock_domain: format!("clock-{source_id}"),
            event_timestamp_ns: timestamp_ns,
            timestamp_uncertainty_ns: 100,
            sensor_article_digest: format!("sha256:article-{source_id}"),
            readout_chain_digest: format!("sha256:readout-{source_id}"),
            body_context_digest: "sha256:body-01".to_owned(),
            operating_envelope_cell_id: format!("cell-{source_id}"),
            causal_group_id: causal_group_id.to_owned(),
            reference_ancestry_receipt_id: format!("receipt:ancestry-{source_id}"),
            channel_qualified: true,
            observable: true,
            saturated: false,
            mass,
        }
    }

    fn attestation(
        left: &ModalityEvidence,
        right: &ModalityEvidence,
        disposition: EvidenceIndependenceDisposition,
    ) -> PairIndependenceAttestation {
        let (left, right) = ordered_pair(left, right);
        PairIndependenceAttestation {
            attestation_id: "attestation:pair-01".to_owned(),
            interaction_event_id: "event:grasp-01".to_owned(),
            left_source_id: left.source_id.clone(),
            right_source_id: right.source_id.clone(),
            left_observation_digest: left.raw_observation_digest.clone(),
            right_observation_digest: right.raw_observation_digest.clone(),
            left_ancestry_receipt_id: left.reference_ancestry_receipt_id.clone(),
            right_ancestry_receipt_id: right.reference_ancestry_receipt_id.clone(),
            common_cause_analysis_receipt_id: "receipt:pair-cca".to_owned(),
            verifier_receipt_id: "receipt:pair-verifier".to_owned(),
            disposition,
        }
    }

    fn request(
        left: ModalityEvidence,
        right: ModalityEvidence,
        disposition: EvidenceIndependenceDisposition,
    ) -> EvidenceFusionRequest {
        let proof = attestation(&left, &right, disposition);
        EvidenceFusionRequest {
            interaction_event_id: "event:grasp-01".to_owned(),
            body_context_digest: "sha256:body-01".to_owned(),
            modalities: vec![left, right],
            pairwise_independence: vec![proof],
            maximum_pairwise_time_skew_ns: 1_000_000,
            maximum_conflict: 0.25,
            normalization_epsilon: 1.0e-9,
        }
    }

    fn mass(state: usize, confidence: f64, ignorance: f64) -> MassAssignment {
        let mut singleton = [0.0; CONTACT_STATE_COUNT];
        singleton[state] = confidence;
        MassAssignment { singleton, ignorance }
    }

    #[test]
    fn independent_agreement_combines_and_preserves_ignorance() {
        let left = modality("pressure", "barometric-array", mass(1, 0.8, 0.2), 10_000, "physical-pressure");
        let right = modality("vibration", "imu", mass(1, 0.7, 0.3), 10_100, "mechanical-vibration");
        let report = assess_evidence_fusion(&request(
            left,
            right,
            EvidenceIndependenceDisposition::IndependentlyVerified,
        ));
        assert_eq!(report.disposition, EvidenceFusionDisposition::SafelyCombined);
        assert_eq!(report.maximum_pairwise_conflict, Some(0.0));
        let combined = report.safe_combined_mass.unwrap();
        assert!((combined.singleton[1] - 0.94).abs() < 1.0e-12);
        assert!((combined.ignorance - 0.06).abs() < 1.0e-12);
    }

    #[test]
    fn high_disagreement_abstains_before_normalized_dempster_output_is_admitted() {
        let left = modality("pressure", "barometric-array", mass(1, 0.95, 0.05), 10_000, "physical-pressure");
        let right = modality("vibration", "imu", mass(3, 0.95, 0.05), 10_100, "mechanical-vibration");
        let report = assess_evidence_fusion(&request(
            left,
            right,
            EvidenceIndependenceDisposition::IndependentlyVerified,
        ));
        assert_eq!(
            report.disposition,
            EvidenceFusionDisposition::ConflictTriggeredAbstention
        );
        assert!(report.maximum_pairwise_conflict.unwrap() > 0.9);
        assert!(report.safe_combined_mass.is_none());
        assert!(report.unconditional_dempster_comparator.is_some());
    }

    #[test]
    fn total_conflict_never_yields_a_primary_fused_state() {
        let left = modality("pressure", "barometric-array", mass(1, 1.0, 0.0), 10_000, "physical-pressure");
        let right = modality("vibration", "imu", mass(3, 1.0, 0.0), 10_100, "mechanical-vibration");
        let report = assess_evidence_fusion(&request(
            left,
            right,
            EvidenceIndependenceDisposition::IndependentlyVerified,
        ));
        assert_eq!(report.disposition, EvidenceFusionDisposition::ConflictTriggeredAbstention);
        assert_eq!(report.maximum_pairwise_conflict, Some(1.0));
        assert!(report.safe_combined_mass.is_none());
        assert!(report.unconditional_dempster_comparator.is_none());
    }

    #[test]
    fn shared_common_cause_or_same_readout_cannot_be_fused_as_independent() {
        let left = modality("pressure", "barometric-array", mass(1, 0.8, 0.2), 10_000, "shared-readout");
        let right = modality("vibration", "imu", mass(1, 0.8, 0.2), 10_100, "shared-readout");
        let report = assess_evidence_fusion(&request(
            left,
            right,
            EvidenceIndependenceDisposition::IndependentlyVerified,
        ));
        assert_eq!(report.disposition, EvidenceFusionDisposition::DependenceUnresolved);
        assert!(report.safe_combined_mass.is_none());
    }

    #[test]
    fn missing_or_unresolved_pair_attestation_fails_closed() {
        let left = modality("pressure", "barometric-array", mass(1, 0.8, 0.2), 10_000, "pressure");
        let right = modality("vibration", "imu", mass(1, 0.8, 0.2), 10_100, "vibration");
        let mut unresolved = request(
            left.clone(),
            right.clone(),
            EvidenceIndependenceDisposition::Unresolved,
        );
        assert_eq!(
            assess_evidence_fusion(&unresolved).disposition,
            EvidenceFusionDisposition::DependenceUnresolved
        );

        unresolved.pairwise_independence.clear();
        assert_eq!(
            assess_evidence_fusion(&unresolved).disposition,
            EvidenceFusionDisposition::DependenceUnresolved
        );
    }

    #[test]
    fn asynchronous_modalities_are_not_fused_without_event_time_alignment() {
        let left = modality("pressure", "barometric-array", mass(1, 0.8, 0.2), 10_000, "pressure");
        let right = modality("vibration", "imu", mass(1, 0.8, 0.2), 50_000_000, "vibration");
        let report = assess_evidence_fusion(&request(
            left,
            right,
            EvidenceIndependenceDisposition::IndependentlyVerified,
        ));
        assert_eq!(report.disposition, EvidenceFusionDisposition::TemporallyUnresolved);
        assert!(report.safe_combined_mass.is_none());
    }

    #[test]
    fn saturated_or_unqualified_modality_cannot_contribute_mass() {
        let left = modality("pressure", "barometric-array", mass(1, 0.8, 0.2), 10_000, "pressure");
        let mut right = modality("vibration", "imu", mass(1, 0.8, 0.2), 10_100, "vibration");
        right.saturated = true;
        let report = assess_evidence_fusion(&request(
            left,
            right,
            EvidenceIndependenceDisposition::IndependentlyVerified,
        ));
        assert_eq!(report.disposition, EvidenceFusionDisposition::InvalidEvidence);
    }

    #[test]
    fn malformed_mass_assignment_is_rejected() {
        let left = modality(
            "pressure",
            "barometric-array",
            MassAssignment {
                singleton: [0.8, 0.4, 0.0, 0.0],
                ignorance: 0.0,
            },
            10_000,
            "pressure",
        );
        let right = modality("vibration", "imu", mass(1, 0.8, 0.2), 10_100, "vibration");
        let report = assess_evidence_fusion(&request(
            left,
            right,
            EvidenceIndependenceDisposition::IndependentlyVerified,
        ));
        assert_eq!(report.disposition, EvidenceFusionDisposition::InvalidEvidence);
    }
}
