//! Evidence-triggered tactile self-test and physical qualification disposition.
//!
//! Electrical loopback validates only the readout path it actually exercises.
//! It cannot establish that a physical 5 N contact is measured as 5 N.
//! Mechanical witness checks and externally referenced requalification are
//! separate evidence stages. Context changes suspend applicability until the
//! exact current sensorized-body/readout context is independently requalified.
//!
//! Receipt identifiers are references only. This contract does not resolve
//! signatures, reference traceability, physical witness procedures or safety.

#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ChannelSelfTestResult {
    Pass,
    Fail,
    NotTested,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ChannelSelfTest {
    pub channel_index: usize,
    pub result: ChannelSelfTestResult,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ElectricalLoopbackEvidence {
    pub evidence_id: String,
    pub sensorized_body_context_digest: String,
    pub readout_chain_digest: String,
    pub injection_point_digest: String,
    pub stimulus_digest: String,
    pub observation_digest: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
    pub channels: Vec<ChannelSelfTest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ReferenceIndependenceDisposition {
    IndependentlyCharacterized,
    SharedCommonCause,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct MechanicalWitnessEvidence {
    pub evidence_id: String,
    pub sensorized_body_context_digest: String,
    pub witness_article_digest: String,
    pub applied_stimulus_digest: String,
    pub observation_digest: String,
    pub reference_instrument_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
    pub reference_independence: ReferenceIndependenceDisposition,
    pub normalized_residual: f64,
    pub acceptance_tolerance: f64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ExternalRequalificationEvidence {
    pub evidence_id: String,
    pub calibration_record_id: String,
    pub sensorized_body_context_digest: String,
    pub readout_chain_digest: String,
    pub mechanical_witness_observation_digest: String,
    pub reference_instrument_receipt_id: String,
    pub reference_ancestry_receipt_id: String,
    pub common_cause_analysis_receipt_id: String,
    pub verifier_receipt_id: String,
    pub reference_independence: ReferenceIndependenceDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelfTestRequest {
    pub prior_qualified_context_digest: String,
    pub current_sensorized_body_context_digest: String,
    pub prior_readout_chain_digest: String,
    pub current_readout_chain_digest: String,
    pub expected_channel_count: usize,
    pub prior_measurement_qualification_active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelfTestEvidence {
    pub electrical_loopback: Option<ElectricalLoopbackEvidence>,
    pub mechanical_witness: Option<MechanicalWitnessEvidence>,
    pub external_requalification: Option<ExternalRequalificationEvidence>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum ElectricalDiagnosticDisposition {
    Pass,
    Fail,
    Incomplete,
    ContextMismatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum MechanicalWitnessDisposition {
    NotPerformed,
    Pass,
    TransferChanged,
    ReferenceChainUnresolved,
    ContextMismatch,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum MeasurementQualificationDisposition {
    PreviousQualificationRemainsApplicable,
    QualificationSuspended,
    ExternalRequalificationRequired,
    QualificationRestored,
    RemainsUnqualified,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SelfTestAssessment {
    pub electrical_disposition: ElectricalDiagnosticDisposition,
    pub mechanical_disposition: MechanicalWitnessDisposition,
    pub measurement_disposition: MeasurementQualificationDisposition,
    pub sensorized_body_context_changed: bool,
    pub readout_context_changed: bool,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

fn nonempty_fields(fields: &[(&str, &str)]) -> Vec<String> {
    fields
        .iter()
        .filter(|(_, value)| value.trim().is_empty())
        .map(|(name, _)| format!("required evidence field {name} is missing"))
        .collect()
}

fn evaluate_electrical(
    request: &SelfTestRequest,
    evidence: Option<&ElectricalLoopbackEvidence>,
) -> (ElectricalDiagnosticDisposition, Vec<String>) {
    let Some(evidence) = evidence else {
        return (
            ElectricalDiagnosticDisposition::Incomplete,
            vec!["electrical loopback evidence was not supplied".to_owned()],
        );
    };
    let mut reasons = nonempty_fields(&[
        ("evidence_id", &evidence.evidence_id),
        ("sensorized_body_context_digest", &evidence.sensorized_body_context_digest),
        ("readout_chain_digest", &evidence.readout_chain_digest),
        ("injection_point_digest", &evidence.injection_point_digest),
        ("stimulus_digest", &evidence.stimulus_digest),
        ("observation_digest", &evidence.observation_digest),
        ("reference_ancestry_receipt_id", &evidence.reference_ancestry_receipt_id),
        ("common_cause_analysis_receipt_id", &evidence.common_cause_analysis_receipt_id),
        ("verifier_receipt_id", &evidence.verifier_receipt_id),
    ]);
    if evidence.sensorized_body_context_digest
        != request.current_sensorized_body_context_digest
        || evidence.readout_chain_digest != request.current_readout_chain_digest
    {
        reasons.push("electrical loopback evidence does not bind the exact current physical/readout context".to_owned());
        return (ElectricalDiagnosticDisposition::ContextMismatch, reasons);
    }
    if evidence.channels.len() != request.expected_channel_count {
        reasons.push("electrical loopback does not cover the expected channel count".to_owned());
    }

    let mut seen = vec![false; request.expected_channel_count];
    let mut any_failed = false;
    let mut any_untested = false;
    for channel in &evidence.channels {
        if channel.channel_index >= request.expected_channel_count || seen[channel.channel_index] {
            reasons.push("electrical loopback channel indices are duplicate or out of range".to_owned());
            continue;
        }
        seen[channel.channel_index] = true;
        match channel.result {
            ChannelSelfTestResult::Pass => {}
            ChannelSelfTestResult::Fail => {
                any_failed = true;
                reasons.push(format!("electrical loopback failed on channel {}", channel.channel_index));
            }
            ChannelSelfTestResult::NotTested => {
                any_untested = true;
                reasons.push(format!("electrical loopback did not test channel {}", channel.channel_index));
            }
        }
    }
    if seen.iter().any(|present| !present) {
        reasons.push("electrical loopback omits one or more required channels".to_owned());
        any_untested = true;
    }
    if !reasons.is_empty() {
        if any_failed {
            return (ElectricalDiagnosticDisposition::Fail, reasons);
        }
        if any_untested || reasons.iter().any(|reason| reason.contains("missing")) {
            return (ElectricalDiagnosticDisposition::Incomplete, reasons);
        }
    }
    if any_failed {
        (ElectricalDiagnosticDisposition::Fail, reasons)
    } else if any_untested {
        (ElectricalDiagnosticDisposition::Incomplete, reasons)
    } else {
        (ElectricalDiagnosticDisposition::Pass, reasons)
    }
}

fn evaluate_mechanical(
    request: &SelfTestRequest,
    evidence: Option<&MechanicalWitnessEvidence>,
) -> (MechanicalWitnessDisposition, Vec<String>) {
    let Some(evidence) = evidence else {
        return (MechanicalWitnessDisposition::NotPerformed, Vec::new());
    };
    let mut reasons = nonempty_fields(&[
        ("evidence_id", &evidence.evidence_id),
        ("sensorized_body_context_digest", &evidence.sensorized_body_context_digest),
        ("witness_article_digest", &evidence.witness_article_digest),
        ("applied_stimulus_digest", &evidence.applied_stimulus_digest),
        ("observation_digest", &evidence.observation_digest),
        ("reference_instrument_receipt_id", &evidence.reference_instrument_receipt_id),
        ("reference_ancestry_receipt_id", &evidence.reference_ancestry_receipt_id),
        ("common_cause_analysis_receipt_id", &evidence.common_cause_analysis_receipt_id),
        ("verifier_receipt_id", &evidence.verifier_receipt_id),
    ]);
    if evidence.sensorized_body_context_digest
        != request.current_sensorized_body_context_digest
    {
        reasons.push("mechanical witness does not bind the exact current sensorized-body context".to_owned());
        return (MechanicalWitnessDisposition::ContextMismatch, reasons);
    }
    if !evidence.normalized_residual.is_finite()
        || !evidence.acceptance_tolerance.is_finite()
        || evidence.normalized_residual < 0.0
        || evidence.acceptance_tolerance <= 0.0
    {
        reasons.push("mechanical witness residual/tolerance is invalid".to_owned());
        return (MechanicalWitnessDisposition::Invalid, reasons);
    }
    if evidence.reference_independence != ReferenceIndependenceDisposition::IndependentlyCharacterized {
        reasons.push("mechanical witness reference ancestry/common-cause disposition is unresolved".to_owned());
        return (MechanicalWitnessDisposition::ReferenceChainUnresolved, reasons);
    }
    if !reasons.is_empty() {
        return (MechanicalWitnessDisposition::Invalid, reasons);
    }
    if evidence.normalized_residual > evidence.acceptance_tolerance {
        reasons.push("mechanical witness response differs from the qualified baseline".to_owned());
        (MechanicalWitnessDisposition::TransferChanged, reasons)
    } else {
        (MechanicalWitnessDisposition::Pass, reasons)
    }
}

fn validate_requalification(
    request: &SelfTestRequest,
    witness: &MechanicalWitnessEvidence,
    evidence: Option<&ExternalRequalificationEvidence>,
) -> Result<(), Vec<String>> {
    let Some(evidence) = evidence else {
        return Err(vec!["external requalification evidence was not supplied".to_owned()]);
    };
    let mut reasons = nonempty_fields(&[
        ("evidence_id", &evidence.evidence_id),
        ("calibration_record_id", &evidence.calibration_record_id),
        ("sensorized_body_context_digest", &evidence.sensorized_body_context_digest),
        ("readout_chain_digest", &evidence.readout_chain_digest),
        ("mechanical_witness_observation_digest", &evidence.mechanical_witness_observation_digest),
        ("reference_instrument_receipt_id", &evidence.reference_instrument_receipt_id),
        ("reference_ancestry_receipt_id", &evidence.reference_ancestry_receipt_id),
        ("common_cause_analysis_receipt_id", &evidence.common_cause_analysis_receipt_id),
        ("verifier_receipt_id", &evidence.verifier_receipt_id),
    ]);
    if evidence.sensorized_body_context_digest
        != request.current_sensorized_body_context_digest
    {
        reasons.push("external requalification does not bind current sensorized-body context".to_owned());
    }
    if evidence.readout_chain_digest != request.current_readout_chain_digest {
        reasons.push("external requalification does not bind current readout chain".to_owned());
    }
    if evidence.mechanical_witness_observation_digest != witness.observation_digest {
        reasons.push("external requalification does not bind the supplied mechanical witness observation".to_owned());
    }
    if evidence.reference_independence != ReferenceIndependenceDisposition::IndependentlyCharacterized {
        reasons.push("external requalification reference independence is unresolved".to_owned());
    }
    if reasons.is_empty() { Ok(()) } else { Err(reasons) }
}

/// Assess self-test evidence without allowing electrical diagnostics alone to
/// upgrade physical force/torque qualification.
pub fn assess_tactile_self_test(
    request: &SelfTestRequest,
    evidence: &SelfTestEvidence,
) -> SelfTestAssessment {
    let sensorized_body_context_changed = request.prior_qualified_context_digest
        != request.current_sensorized_body_context_digest;
    let readout_context_changed =
        request.prior_readout_chain_digest != request.current_readout_chain_digest;

    let (electrical_disposition, mut reasons) =
        evaluate_electrical(request, evidence.electrical_loopback.as_ref());
    let (mechanical_disposition, mechanical_reasons) =
        evaluate_mechanical(request, evidence.mechanical_witness.as_ref());
    reasons.extend(mechanical_reasons);

    if electrical_disposition == ElectricalDiagnosticDisposition::Fail {
        let measurement_disposition = if request.prior_measurement_qualification_active {
            MeasurementQualificationDisposition::QualificationSuspended
        } else {
            MeasurementQualificationDisposition::RemainsUnqualified
        };
        return SelfTestAssessment {
            electrical_disposition,
            mechanical_disposition,
            measurement_disposition,
            sensorized_body_context_changed,
            readout_context_changed,
            reasons,
            claim_class: "synthetic_self_test_disposition_only",
        };
    }

    let context_changed = sensorized_body_context_changed || readout_context_changed;
    let mechanical_problem = matches!(
        mechanical_disposition,
        MechanicalWitnessDisposition::TransferChanged
            | MechanicalWitnessDisposition::ReferenceChainUnresolved
            | MechanicalWitnessDisposition::ContextMismatch
            | MechanicalWitnessDisposition::Invalid
    );
    let requires_requalification = context_changed || mechanical_problem;

    if !request.prior_measurement_qualification_active {
        return SelfTestAssessment {
            electrical_disposition,
            mechanical_disposition,
            measurement_disposition: MeasurementQualificationDisposition::RemainsUnqualified,
            sensorized_body_context_changed,
            readout_context_changed,
            reasons,
            claim_class: "synthetic_self_test_disposition_only",
        };
    }

    if requires_requalification {
        if electrical_disposition != ElectricalDiagnosticDisposition::Pass {
            reasons.push("current readout path has not passed loopback; external qualification cannot be restored".to_owned());
            return SelfTestAssessment {
                electrical_disposition,
                mechanical_disposition,
                measurement_disposition: MeasurementQualificationDisposition::QualificationSuspended,
                sensorized_body_context_changed,
                readout_context_changed,
                reasons,
                claim_class: "synthetic_self_test_disposition_only",
            };
        }
        if mechanical_disposition != MechanicalWitnessDisposition::Pass {
            reasons.push("current sensorized-body mechanical witness has not passed; external qualification cannot be restored".to_owned());
            return SelfTestAssessment {
                electrical_disposition,
                mechanical_disposition,
                measurement_disposition: MeasurementQualificationDisposition::ExternalRequalificationRequired,
                sensorized_body_context_changed,
                readout_context_changed,
                reasons,
                claim_class: "synthetic_self_test_disposition_only",
            };
        }
        let Some(witness) = evidence.mechanical_witness.as_ref() else {
            reasons.push("mechanical witness evidence disappeared before requalification".to_owned());
            return SelfTestAssessment {
                electrical_disposition,
                mechanical_disposition,
                measurement_disposition: MeasurementQualificationDisposition::ExternalRequalificationRequired,
                sensorized_body_context_changed,
                readout_context_changed,
                reasons,
                claim_class: "synthetic_self_test_disposition_only",
            };
        };
        match validate_requalification(
            request,
            witness,
            evidence.external_requalification.as_ref(),
        ) {
            Ok(()) => {
                return SelfTestAssessment {
                    electrical_disposition,
                    mechanical_disposition,
                    measurement_disposition: MeasurementQualificationDisposition::QualificationRestored,
                    sensorized_body_context_changed,
                    readout_context_changed,
                    reasons,
                    claim_class: "synthetic_self_test_disposition_only",
                };
            }
            Err(requalification_reasons) => {
                reasons.extend(requalification_reasons);
                return SelfTestAssessment {
                    electrical_disposition,
                    mechanical_disposition,
                    measurement_disposition: MeasurementQualificationDisposition::ExternalRequalificationRequired,
                    sensorized_body_context_changed,
                    readout_context_changed,
                    reasons,
                    claim_class: "synthetic_self_test_disposition_only",
                };
            }
        }
    }

    if !request.prior_qualified_context_digest.trim().is_empty()
        && !request.prior_readout_chain_digest.trim().is_empty()
    {
        SelfTestAssessment {
            electrical_disposition,
            mechanical_disposition,
            measurement_disposition: MeasurementQualificationDisposition::PreviousQualificationRemainsApplicable,
            sensorized_body_context_changed,
            readout_context_changed,
            reasons,
            claim_class: "synthetic_self_test_disposition_only",
        }
    } else {
        SelfTestAssessment {
            electrical_disposition,
            mechanical_disposition,
            measurement_disposition: MeasurementQualificationDisposition::RemainsUnqualified,
            sensorized_body_context_changed,
            readout_context_changed,
            reasons,
            claim_class: "synthetic_self_test_disposition_only",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> SelfTestRequest {
        SelfTestRequest {
            prior_qualified_context_digest: "sha256:body-context".to_owned(),
            current_sensorized_body_context_digest: "sha256:body-context".to_owned(),
            prior_readout_chain_digest: "sha256:readout-chain".to_owned(),
            current_readout_chain_digest: "sha256:readout-chain".to_owned(),
            expected_channel_count: 3,
            prior_measurement_qualification_active: true,
        }
    }

    fn electrical() -> ElectricalLoopbackEvidence {
        ElectricalLoopbackEvidence {
            evidence_id: "evidence:loopback".to_owned(),
            sensorized_body_context_digest: "sha256:body-context".to_owned(),
            readout_chain_digest: "sha256:readout-chain".to_owned(),
            injection_point_digest: "sha256:injection-point".to_owned(),
            stimulus_digest: "sha256:stimulus".to_owned(),
            observation_digest: "sha256:loopback-observation".to_owned(),
            reference_ancestry_receipt_id: "receipt:loopback-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:loopback-cca".to_owned(),
            verifier_receipt_id: "receipt:loopback-verifier".to_owned(),
            channels: vec![
                ChannelSelfTest { channel_index: 0, result: ChannelSelfTestResult::Pass },
                ChannelSelfTest { channel_index: 1, result: ChannelSelfTestResult::Pass },
                ChannelSelfTest { channel_index: 2, result: ChannelSelfTestResult::Pass },
            ],
        }
    }

    fn mechanical() -> MechanicalWitnessEvidence {
        MechanicalWitnessEvidence {
            evidence_id: "evidence:mechanical-witness".to_owned(),
            sensorized_body_context_digest: "sha256:body-context".to_owned(),
            witness_article_digest: "sha256:witness-article".to_owned(),
            applied_stimulus_digest: "sha256:known-displacement".to_owned(),
            observation_digest: "sha256:mechanical-observation".to_owned(),
            reference_instrument_receipt_id: "receipt:force-reference".to_owned(),
            reference_ancestry_receipt_id: "receipt:force-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:force-cca".to_owned(),
            verifier_receipt_id: "receipt:force-verifier".to_owned(),
            reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
            normalized_residual: 0.01,
            acceptance_tolerance: 0.02,
        }
    }

    fn requalification() -> ExternalRequalificationEvidence {
        ExternalRequalificationEvidence {
            evidence_id: "evidence:external-requalification".to_owned(),
            calibration_record_id: "calibration:body-context".to_owned(),
            sensorized_body_context_digest: "sha256:new-body-context".to_owned(),
            readout_chain_digest: "sha256:new-readout-chain".to_owned(),
            mechanical_witness_observation_digest: "sha256:new-mechanical-observation".to_owned(),
            reference_instrument_receipt_id: "receipt:new-force-reference".to_owned(),
            reference_ancestry_receipt_id: "receipt:new-force-ancestry".to_owned(),
            common_cause_analysis_receipt_id: "receipt:new-force-cca".to_owned(),
            verifier_receipt_id: "receipt:new-force-verifier".to_owned(),
            reference_independence: ReferenceIndependenceDisposition::IndependentlyCharacterized,
        }
    }

    #[test]
    fn electrical_loopback_pass_does_not_promote_or_replace_physical_qualification() {
        let assessment = assess_tactile_self_test(
            &request(),
            &SelfTestEvidence {
                electrical_loopback: Some(electrical()),
                mechanical_witness: None,
                external_requalification: None,
            },
        );
        assert_eq!(assessment.electrical_disposition, ElectricalDiagnosticDisposition::Pass);
        assert_eq!(assessment.mechanical_disposition, MechanicalWitnessDisposition::NotPerformed);
        assert_eq!(
            assessment.measurement_disposition,
            MeasurementQualificationDisposition::PreviousQualificationRemainsApplicable
        );
    }

    #[test]
    fn failed_electrical_channel_suspends_measurement_qualification() {
        let mut loopback = electrical();
        loopback.channels[1].result = ChannelSelfTestResult::Fail;
        let assessment = assess_tactile_self_test(
            &request(),
            &SelfTestEvidence {
                electrical_loopback: Some(loopback),
                mechanical_witness: Some(mechanical()),
                external_requalification: None,
            },
        );
        assert_eq!(assessment.electrical_disposition, ElectricalDiagnosticDisposition::Fail);
        assert_eq!(
            assessment.measurement_disposition,
            MeasurementQualificationDisposition::QualificationSuspended
        );
    }

    #[test]
    fn readout_or_sensorized_body_change_requires_external_requalification() {
        let mut changed = request();
        changed.current_readout_chain_digest = "sha256:new-readout-chain".to_owned();
        changed.current_sensorized_body_context_digest = "sha256:new-body-context".to_owned();
        let mut loopback = electrical();
        loopback.readout_chain_digest = changed.current_readout_chain_digest.clone();
        loopback.sensorized_body_context_digest = changed.current_sensorized_body_context_digest.clone();
        let mut witness = mechanical();
        witness.sensorized_body_context_digest = changed.current_sensorized_body_context_digest.clone();
        witness.observation_digest = "sha256:new-mechanical-observation".to_owned();

        let assessment = assess_tactile_self_test(
            &changed,
            &SelfTestEvidence {
                electrical_loopback: Some(loopback),
                mechanical_witness: Some(witness),
                external_requalification: None,
            },
        );
        assert_eq!(
            assessment.measurement_disposition,
            MeasurementQualificationDisposition::ExternalRequalificationRequired
        );
    }

    #[test]
    fn independent_mechanical_witness_and_exact_external_receipt_can_restore_qualification() {
        let mut changed = request();
        changed.current_readout_chain_digest = "sha256:new-readout-chain".to_owned();
        changed.current_sensorized_body_context_digest = "sha256:new-body-context".to_owned();
        let mut loopback = electrical();
        loopback.readout_chain_digest = changed.current_readout_chain_digest.clone();
        loopback.sensorized_body_context_digest = changed.current_sensorized_body_context_digest.clone();
        let mut witness = mechanical();
        witness.sensorized_body_context_digest = changed.current_sensorized_body_context_digest.clone();
        witness.observation_digest = "sha256:new-mechanical-observation".to_owned();

        let mut receipt = requalification();
        receipt.readout_chain_digest = changed.current_readout_chain_digest.clone();
        receipt.sensorized_body_context_digest =
            changed.current_sensorized_body_context_digest.clone();

        let assessment = assess_tactile_self_test(
            &changed,
            &SelfTestEvidence {
                electrical_loopback: Some(loopback),
                mechanical_witness: Some(witness),
                external_requalification: Some(receipt),
            },
        );
        assert_eq!(
            assessment.measurement_disposition,
            MeasurementQualificationDisposition::QualificationRestored
        );
    }

    #[test]
    fn shared_common_cause_witness_never_restores_absolute_qualification() {
        let mut changed = request();
        changed.current_readout_chain_digest = "sha256:new-readout-chain".to_owned();
        changed.current_sensorized_body_context_digest = "sha256:new-body-context".to_owned();
        let mut loopback = electrical();
        loopback.readout_chain_digest = changed.current_readout_chain_digest.clone();
        loopback.sensorized_body_context_digest = changed.current_sensorized_body_context_digest.clone();
        let mut witness = mechanical();
        witness.sensorized_body_context_digest = changed.current_sensorized_body_context_digest.clone();
        witness.observation_digest = "sha256:new-mechanical-observation".to_owned();
        witness.reference_independence = ReferenceIndependenceDisposition::SharedCommonCause;
        let mut receipt = requalification();
        receipt.readout_chain_digest = changed.current_readout_chain_digest.clone();
        receipt.sensorized_body_context_digest =
            changed.current_sensorized_body_context_digest.clone();
        receipt.mechanical_witness_observation_digest = witness.observation_digest.clone();

        let assessment = assess_tactile_self_test(
            &changed,
            &SelfTestEvidence {
                electrical_loopback: Some(loopback),
                mechanical_witness: Some(witness),
                external_requalification: Some(receipt),
            },
        );
        assert_eq!(
            assessment.measurement_disposition,
            MeasurementQualificationDisposition::ExternalRequalificationRequired
        );
    }

    #[test]
    fn self_test_does_not_qualify_a_previously_unqualified_sensor() {
        let mut request = request();
        request.prior_measurement_qualification_active = false;
        let assessment = assess_tactile_self_test(
            &request,
            &SelfTestEvidence {
                electrical_loopback: Some(electrical()),
                mechanical_witness: Some(mechanical()),
                external_requalification: None,
            },
        );
        assert_eq!(
            assessment.measurement_disposition,
            MeasurementQualificationDisposition::RemainsUnqualified
        );
    }
}
