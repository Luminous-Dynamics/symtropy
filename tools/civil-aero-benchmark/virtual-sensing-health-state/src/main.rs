// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Tier-0 synthetic structure/sensor joint-state and damage-preservation oracle.
// This is not a damage detector, NDE procedure, SHM product, or aircraft safety tool.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const FIXTURE: &[u8] = include_bytes!("../data/hh-hf-dh-df-v0.1.json");
const CLAIM_CEILING: &str = "synthetic_joint_structure_sensor_state_and_abstention_only";

#[derive(Clone, Debug, Deserialize)]
struct Assumptions {
    observation_model: String,
    fault_model: String,
    structural_states: Vec<String>,
    channel_order: Vec<String>,
    response_units: String,
    operator_status: String,
    calibration_status: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Policy {
    maximum_faulty_channels: usize,
    minimum_consistent_channels: usize,
    absolute_residual_tolerance: f64,
    reconstruction_requires_unique_joint_hypothesis: bool,
    damage_clearance_from_reconstruction_prohibited: bool,
    fault_confirmation_requires_independent_witness: bool,
    raw_measurements_immutable: bool,
    reconstruction_is_not_measurement: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct Channel {
    sensor_id: String,
    quantity: String,
    unit: String,
    operator_sha256: String,
    operator_status: String,
    calibration_status: String,
    common_cause_group: String,
}

#[derive(Clone, Debug, Deserialize)]
struct SensorFaultWitness {
    sensor_id: String,
    status: String,
    evidence_kind: String,
    evidence_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct CaseExpected {
    disposition: String,
    structural_state: Option<String>,
    faulty_sensor_ids: Vec<String>,
    post_reconstruction_structural_state: Option<String>,
    damage_preserved: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
struct CaseInput {
    case_id: String,
    healthy_template: Vec<f64>,
    damaged_template: Vec<f64>,
    observed_values: Vec<Option<f64>>,
    sensor_fault_witnesses: Vec<SensorFaultWitness>,
    expected: CaseExpected,
}

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: String,
    model_id: String,
    model_scope: String,
    claim_ceiling: String,
    assumptions: Assumptions,
    policy: Policy,
    channels: Vec<Channel>,
    cases: Vec<CaseInput>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct CaseResult {
    case_id: String,
    disposition: String,
    structural_state: Option<String>,
    faulty_sensor_ids: Vec<String>,
    explanation_count: usize,
    consistent_channel_count: Option<usize>,
    maximum_consistent_residual: Option<f64>,
    observed_values: Vec<Option<f64>>,
    model_imputed_values: Option<Vec<Option<f64>>>,
    reconstructed_sensor_ids: Vec<String>,
    reconstruction_status: String,
    sensor_fault_confirmation: String,
    post_reconstruction_structural_state: Option<String>,
    damage_preserved: Option<bool>,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    model_id: String,
    source_sha256: String,
    source_bytes: usize,
    channel_count: usize,
    case_count: usize,
    disposition_counts: Vec<(String, usize)>,
    cases: Vec<CaseResult>,
    claim_ceiling: String,
}

#[derive(Clone, Debug)]
struct Explanation {
    structural_state: &'static str,
    template: Vec<f64>,
    faulty_indices: Vec<usize>,
    consistent_count: usize,
    maximum_consistent_residual: f64,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "aero-virtual-sensing-health-state/0.1.0" {
        return Err("unsupported virtual-sensing health-state schema".to_owned());
    }
    if fixture.model_id.trim().is_empty() {
        return Err("model_id must be non-empty".to_owned());
    }
    if fixture.model_scope != "synthetic_static_response_template_hypothesis_test" {
        return Err("model scope is outside the v0.1 synthetic contract".to_owned());
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        return Err("claim ceiling changed or was weakened".to_owned());
    }
    if fixture.assumptions.observation_model != "static_normalized_sensor_response"
        || fixture.assumptions.fault_model != "at_most_one_arbitrary_channel_residual"
        || fixture.assumptions.structural_states != vec!["HEALTHY".to_owned(), "DAMAGED".to_owned()]
        || fixture.assumptions.response_units != "normalized_response"
        || fixture.assumptions.operator_status != "ADMITTED"
        || fixture.assumptions.calibration_status != "VALID"
    {
        return Err("unsupported model assumptions".to_owned());
    }
    if fixture.channels.len() != 3 {
        return Err("v0.1 requires exactly three registered channels".to_owned());
    }
    let ids: Vec<String> = fixture
        .channels
        .iter()
        .map(|channel| channel.sensor_id.clone())
        .collect();
    if ids != fixture.assumptions.channel_order {
        return Err("channel registry order differs from declared channel order".to_owned());
    }
    let mut seen = BTreeSet::new();
    for channel in &fixture.channels {
        if channel.sensor_id.trim().is_empty() || !seen.insert(channel.sensor_id.as_str()) {
            return Err("channel IDs must be non-empty and unique".to_owned());
        }
        if channel.quantity.trim().is_empty()
            || channel.unit != "normalized_response"
            || channel.common_cause_group.trim().is_empty()
        {
            return Err(format!(
                "{}: incomplete channel contract",
                channel.sensor_id
            ));
        }
        if !is_sha256(&channel.operator_sha256) {
            return Err(format!("{}: invalid operator digest", channel.sensor_id));
        }
        if !matches!(channel.operator_status.as_str(), "ADMITTED" | "UNKNOWN") {
            return Err(format!(
                "{}: unsupported operator status",
                channel.sensor_id
            ));
        }
        if !matches!(
            channel.calibration_status.as_str(),
            "VALID" | "EXPIRED" | "UNKNOWN"
        ) {
            return Err(format!(
                "{}: unsupported calibration status",
                channel.sensor_id
            ));
        }
    }

    if fixture.policy.maximum_faulty_channels != 1
        || fixture.policy.minimum_consistent_channels < 2
        || fixture.policy.minimum_consistent_channels > fixture.channels.len()
        || !fixture.policy.absolute_residual_tolerance.is_finite()
        || fixture.policy.absolute_residual_tolerance <= 0.0
        || !fixture
            .policy
            .reconstruction_requires_unique_joint_hypothesis
        || !fixture
            .policy
            .damage_clearance_from_reconstruction_prohibited
        || !fixture
            .policy
            .fault_confirmation_requires_independent_witness
        || !fixture.policy.raw_measurements_immutable
        || !fixture.policy.reconstruction_is_not_measurement
    {
        return Err("v0.1 fault, support or damage-preservation policy was changed".to_owned());
    }

    if fixture.cases.is_empty() || fixture.cases.len() > 64 {
        return Err("case count must be in 1..=64".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    for case in &fixture.cases {
        if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.as_str()) {
            return Err("case IDs must be non-empty and unique".to_owned());
        }
        if case.healthy_template.len() != fixture.channels.len()
            || case.damaged_template.len() != fixture.channels.len()
            || case.observed_values.len() != fixture.channels.len()
        {
            return Err(format!(
                "{}: template/observation dimension mismatch",
                case.case_id
            ));
        }
        if case
            .healthy_template
            .iter()
            .chain(&case.damaged_template)
            .any(|v| !v.is_finite())
        {
            return Err(format!("{}: non-finite structural template", case.case_id));
        }
        if case
            .observed_values
            .iter()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(format!("{}: non-finite observation", case.case_id));
        }
        let mut witness_ids = BTreeSet::new();
        for witness in &case.sensor_fault_witnesses {
            if !ids.iter().any(|id| id == &witness.sensor_id)
                || !witness_ids.insert(witness.sensor_id.as_str())
            {
                return Err(format!(
                    "{}: unknown or duplicate sensor-fault witness",
                    case.case_id
                ));
            }
            if witness.evidence_kind.trim().is_empty() {
                return Err(format!("{}: witness evidence_kind is empty", case.case_id));
            }
            match witness.status.as_str() {
                "FAULT_CONFIRMED" => {
                    if !witness.evidence_sha256.as_deref().is_some_and(is_sha256) {
                        return Err(format!(
                            "{}: confirmed fault witness requires a SHA-256 receipt",
                            case.case_id
                        ));
                    }
                }
                "NOT_CONFIRMED" => {
                    if witness.evidence_sha256.is_some() {
                        return Err(format!(
                            "{}: unconfirmed witness cannot carry a confirmation digest",
                            case.case_id
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "{}: unsupported sensor-fault witness status",
                        case.case_id
                    ));
                }
            }
        }
        let expected_state = case.expected.structural_state.as_deref();
        if expected_state.is_some_and(|state| !matches!(state, "HEALTHY" | "DAMAGED")) {
            return Err(format!(
                "{}: unsupported expected structural state",
                case.case_id
            ));
        }
        if case.expected.post_reconstruction_structural_state != case.expected.structural_state {
            return Err(format!(
                "{}: expected reconstruction state must preserve the structural hypothesis",
                case.case_id
            ));
        }
        if case.expected.damage_preserved != expected_state.map(|state| state == "DAMAGED") {
            return Err(format!(
                "{}: expected damage-preservation marker contradicts structural state",
                case.case_id
            ));
        }
        let mut expected_fault_ids = BTreeSet::new();
        for sensor_id in &case.expected.faulty_sensor_ids {
            if !ids.iter().any(|id| id == sensor_id)
                || !expected_fault_ids.insert(sensor_id.as_str())
            {
                return Err(format!(
                    "{}: expected fault sensor IDs must be known and unique",
                    case.case_id
                ));
            }
        }

        if !matches!(
            case.expected.disposition.as_str(),
            "HEALTHY_WITHIN_DECLARED_SYNTHETIC_SUPPORT"
                | "SENSOR_FAULT_WITNESS_CONFIRMED"
                | "STRUCTURAL_DAMAGE_SUSPECTED"
                | "DAMAGE_AND_SENSOR_FAULT_WITNESS_CONFIRMED"
                | "ABSTAIN_AMBIGUOUS_HYPOTHESES"
                | "ABSTAIN_OUT_OF_MODEL"
                | "ABSTAIN_INSUFFICIENT_OBSERVABILITY"
                | "ABSTAIN_UNQUALIFIED_MEASUREMENT_CHAIN"
                | "ABSTAIN_SENSOR_FAULT_UNCONFIRMED"
        ) {
            return Err(format!(
                "{}: unsupported expected disposition",
                case.case_id
            ));
        }
    }
    Ok(())
}

fn explanation_for_template(
    fixture: &Fixture,
    case: &CaseInput,
    structural_state: &'static str,
    template: &[f64],
    observed_indices: &[usize],
) -> Option<Explanation> {
    let mut faulty_indices = Vec::new();
    let mut consistent_count = 0usize;
    let mut maximum_consistent_residual = 0.0_f64;

    for index in observed_indices {
        let residual = (case.observed_values[*index].expect("observed index is present")
            - template[*index])
            .abs();
        if residual > fixture.policy.absolute_residual_tolerance {
            faulty_indices.push(*index);
        } else {
            consistent_count += 1;
            maximum_consistent_residual = maximum_consistent_residual.max(residual);
        }
    }

    if faulty_indices.len() > fixture.policy.maximum_faulty_channels
        || consistent_count < fixture.policy.minimum_consistent_channels
    {
        return None;
    }
    Some(Explanation {
        structural_state,
        template: template.to_vec(),
        faulty_indices,
        consistent_count,
        maximum_consistent_residual,
    })
}

fn abstain(
    case: &CaseInput,
    disposition: &str,
    explanation_count: usize,
    sensor_fault_confirmation: &str,
) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        structural_state: None,
        faulty_sensor_ids: Vec::new(),
        explanation_count,
        consistent_channel_count: None,
        maximum_consistent_residual: None,
        observed_values: case.observed_values.clone(),
        model_imputed_values: None,
        reconstructed_sensor_ids: Vec::new(),
        reconstruction_status: "NOT_PERFORMED".to_owned(),
        sensor_fault_confirmation: sensor_fault_confirmation.to_owned(),
        post_reconstruction_structural_state: None,
        damage_preserved: None,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

fn evaluate_case(fixture: &Fixture, case: &CaseInput) -> CaseResult {
    let observed_indices: Vec<usize> = case
        .observed_values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.map(|_| index))
        .collect();

    if observed_indices.len() < fixture.policy.minimum_consistent_channels {
        return abstain(
            case,
            "ABSTAIN_INSUFFICIENT_OBSERVABILITY",
            0,
            "NOT_ASSESSED",
        );
    }

    // An unqualified channel must never be reconstructed into a seemingly valid one.
    let unqualified = observed_indices.iter().any(|index| {
        let channel = &fixture.channels[*index];
        channel.operator_status != "ADMITTED" || channel.calibration_status != "VALID"
    });
    if unqualified {
        return abstain(
            case,
            "ABSTAIN_UNQUALIFIED_MEASUREMENT_CHAIN",
            0,
            "NOT_ASSESSED",
        );
    }

    let mut explanations = Vec::new();
    if let Some(explanation) = explanation_for_template(
        fixture,
        case,
        "HEALTHY",
        &case.healthy_template,
        &observed_indices,
    ) {
        explanations.push(explanation);
    }
    if let Some(explanation) = explanation_for_template(
        fixture,
        case,
        "DAMAGED",
        &case.damaged_template,
        &observed_indices,
    ) {
        explanations.push(explanation);
    }

    if explanations.is_empty() {
        return abstain(case, "ABSTAIN_OUT_OF_MODEL", 0, "NOT_ASSESSED");
    }
    if explanations.len() > 1 {
        return abstain(
            case,
            "ABSTAIN_AMBIGUOUS_HYPOTHESES",
            explanations.len(),
            "NOT_ASSESSED",
        );
    }

    let explanation = explanations.pop().expect("exactly one explanation exists");
    let faulty_sensor_ids: Vec<String> = explanation
        .faulty_indices
        .iter()
        .map(|index| fixture.channels[*index].sensor_id.clone())
        .collect();

    if !faulty_sensor_ids.is_empty() {
        let all_faults_have_independent_witness = explanation.faulty_indices.iter().all(|index| {
            case.sensor_fault_witnesses.iter().any(|witness| {
                witness.sensor_id == fixture.channels[*index].sensor_id
                    && witness.status == "FAULT_CONFIRMED"
                    && witness.evidence_kind == "independent_builtin_self_test"
                    && witness.evidence_sha256.as_deref().is_some_and(is_sha256)
            })
        });
        if !all_faults_have_independent_witness {
            return abstain(case, "ABSTAIN_SENSOR_FAULT_UNCONFIRMED", 1, "NOT_CONFIRMED");
        }
    }

    let structural_state = explanation.structural_state;
    let disposition = match (structural_state, faulty_sensor_ids.is_empty()) {
        ("HEALTHY", true) => "HEALTHY_WITHIN_DECLARED_SYNTHETIC_SUPPORT",
        ("HEALTHY", false) => "SENSOR_FAULT_WITNESS_CONFIRMED",
        ("DAMAGED", true) => "STRUCTURAL_DAMAGE_SUSPECTED",
        ("DAMAGED", false) => "DAMAGE_AND_SENSOR_FAULT_WITNESS_CONFIRMED",
        _ => unreachable!("only the two admitted structural states exist"),
    };

    // Raw observations remain immutable. Model imputations have a separate field
    // and are never relabeled as measurements or fed back as independent evidence.
    let model_imputed_values = if faulty_sensor_ids.is_empty() {
        None
    } else {
        let mut estimates = vec![None; case.observed_values.len()];
        for index in &explanation.faulty_indices {
            estimates[*index] = Some(explanation.template[*index]);
        }
        Some(estimates)
    };
    let reconstruction_status = if faulty_sensor_ids.is_empty() {
        "NOT_APPLICABLE"
    } else {
        "CONDITIONAL_MODEL_ESTIMATE_NOT_MEASUREMENT"
    };

    CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        structural_state: Some(structural_state.to_owned()),
        faulty_sensor_ids: faulty_sensor_ids.clone(),
        explanation_count: 1,
        consistent_channel_count: Some(explanation.consistent_count),
        maximum_consistent_residual: Some(explanation.maximum_consistent_residual),
        observed_values: case.observed_values.clone(),
        model_imputed_values,
        reconstructed_sensor_ids: faulty_sensor_ids,
        reconstruction_status: reconstruction_status.to_owned(),
        sensor_fault_confirmation: if explanation.faulty_indices.is_empty() {
            "NO_SENSOR_FAULT_HYPOTHESIS".to_owned()
        } else {
            "INDEPENDENT_WITNESS_CONFIRMED".to_owned()
        },
        post_reconstruction_structural_state: Some(structural_state.to_owned()),
        damage_preserved: Some(structural_state == "DAMAGED"),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

fn evaluate(fixture: &Fixture, source: &[u8]) -> Result<Report, String> {
    validate_fixture(fixture)?;
    let cases: Vec<CaseResult> = fixture
        .cases
        .iter()
        .map(|case| evaluate_case(fixture, case))
        .collect();

    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for case in &cases {
        *counts.entry(case.disposition.clone()).or_default() += 1;
    }

    Ok(Report {
        schema_version: "aero-virtual-sensing-health-state-result/0.1.0".to_owned(),
        model_id: fixture.model_id.clone(),
        source_sha256: sha256_hex(source),
        source_bytes: source.len(),
        channel_count: fixture.channels.len(),
        case_count: cases.len(),
        disposition_counts: counts.into_iter().collect(),
        cases,
        claim_ceiling: CLAIM_CEILING.to_owned(),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Fixture = serde_json::from_slice(FIXTURE)?;
    let report = evaluate(&fixture, FIXTURE).map_err(std::io::Error::other)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_slice(FIXTURE).expect("frozen fixture parses")
    }

    #[test]
    fn all_joint_health_states_and_abstention_cases_match_frozen_answers() {
        let data = fixture();
        let report = evaluate(&data, FIXTURE).expect("fixture evaluates");
        assert_eq!(report.case_count, 8);
        for (actual, expected) in report.cases.iter().zip(&data.cases) {
            assert_eq!(
                actual.disposition, expected.expected.disposition,
                "{}",
                actual.case_id
            );
            assert_eq!(
                actual.structural_state, expected.expected.structural_state,
                "{}",
                actual.case_id
            );
            assert_eq!(
                actual.faulty_sensor_ids, expected.expected.faulty_sensor_ids,
                "{}",
                actual.case_id
            );
            assert_eq!(
                actual.post_reconstruction_structural_state,
                expected.expected.post_reconstruction_structural_state,
                "{}",
                actual.case_id
            );
            assert_eq!(
                actual.damage_preserved, expected.expected.damage_preserved,
                "{}",
                actual.case_id
            );
        }
    }

    #[test]
    fn simultaneous_damage_and_sensor_fault_never_clears_damage_after_correction() {
        let data = fixture();
        let report = evaluate(&data, FIXTURE).unwrap();
        let df = report
            .cases
            .iter()
            .find(|case| case.case_id == "DF_DAMAGED_STRUCTURE_FAULTY_SENSOR")
            .unwrap();
        assert_eq!(df.structural_state.as_deref(), Some("DAMAGED"));
        assert_eq!(df.faulty_sensor_ids, vec!["S2"]);
        assert_eq!(
            df.post_reconstruction_structural_state.as_deref(),
            Some("DAMAGED")
        );
        assert_eq!(df.damage_preserved, Some(true));
        assert_eq!(df.observed_values[1], Some(1.8));
        assert_eq!(df.model_imputed_values.as_ref().unwrap()[1], Some(1.0));
        assert_eq!(
            df.reconstruction_status,
            "CONDITIONAL_MODEL_ESTIMATE_NOT_MEASUREMENT"
        );
    }

    #[test]
    fn ambiguous_damage_versus_sensor_fault_abstains_without_correction() {
        let data = fixture();
        let report = evaluate(&data, FIXTURE).unwrap();
        let ambiguous = report
            .cases
            .iter()
            .find(|case| case.case_id == "AMBIGUOUS_DAMAGE_VS_SINGLE_SENSOR_FAULT")
            .unwrap();
        assert_eq!(ambiguous.disposition, "ABSTAIN_AMBIGUOUS_HYPOTHESES");
        assert_eq!(ambiguous.explanation_count, 2);
        assert!(ambiguous.model_imputed_values.is_none());
        assert!(ambiguous.structural_state.is_none());
    }

    #[test]
    fn unqualified_operator_and_expired_calibration_force_abstention() {
        let mut data = fixture();
        data.channels[1].operator_status = "UNKNOWN".to_owned();
        let report = evaluate(&data, FIXTURE).unwrap();
        assert!(report.cases.iter().all(|case| {
            case.disposition == "ABSTAIN_UNQUALIFIED_MEASUREMENT_CHAIN"
                || case.case_id == "INSUFFICIENT_OBSERVATIONS"
        }));

        let mut data = fixture();
        data.channels[1].calibration_status = "EXPIRED".to_owned();
        let report = evaluate(&data, FIXTURE).unwrap();
        let hf = report
            .cases
            .iter()
            .find(|case| case.case_id == "HF_HEALTHY_STRUCTURE_FAULTY_SENSOR")
            .unwrap();
        assert_eq!(hf.disposition, "ABSTAIN_UNQUALIFIED_MEASUREMENT_CHAIN");
        assert!(hf.model_imputed_values.is_none());
    }

    #[test]
    fn unconfirmed_single_channel_residual_abstains_without_reconstruction() {
        let data = fixture();
        let report = evaluate(&data, FIXTURE).unwrap();
        let case = report
            .cases
            .iter()
            .find(|case| case.case_id == "UNWITNESSED_LOCALIZED_ANOMALY")
            .unwrap();
        assert_eq!(case.disposition, "ABSTAIN_SENSOR_FAULT_UNCONFIRMED");
        assert_eq!(case.sensor_fault_confirmation, "NOT_CONFIRMED");
        assert_eq!(case.observed_values[0], Some(0.8));
        assert!(case.model_imputed_values.is_none());
        assert!(case.structural_state.is_none());
    }

    #[test]
    fn raw_measurement_is_never_overwritten_by_model_imputation() {
        let data = fixture();
        let report = evaluate(&data, FIXTURE).unwrap();
        let hf = report
            .cases
            .iter()
            .find(|case| case.case_id == "HF_HEALTHY_STRUCTURE_FAULTY_SENSOR")
            .unwrap();
        assert_eq!(hf.observed_values[1], Some(0.8));
        assert_eq!(hf.model_imputed_values.as_ref().unwrap()[1], Some(0.0));
        assert_eq!(hf.reconstructed_sensor_ids, vec!["S2"]);
        assert_eq!(
            hf.sensor_fault_confirmation,
            "INDEPENDENT_WITNESS_CONFIRMED"
        );
    }

    #[test]
    fn out_of_model_and_insufficient_observations_do_not_get_relabelled_as_damage() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        for (id, disposition) in [
            (
                "OUT_OF_MODEL_ENVIRONMENTAL_OR_UNKNOWN_SHIFT",
                "ABSTAIN_OUT_OF_MODEL",
            ),
            (
                "INSUFFICIENT_OBSERVATIONS",
                "ABSTAIN_INSUFFICIENT_OBSERVABILITY",
            ),
        ] {
            let case = report.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(case.disposition, disposition);
            assert!(case.structural_state.is_none());
            assert!(case.model_imputed_values.is_none());
        }
    }

    #[test]
    fn witness_must_bind_the_suspected_channel_and_independent_evidence_digest() {
        let mut data = fixture();
        let hf = data
            .cases
            .iter_mut()
            .find(|case| case.case_id == "HF_HEALTHY_STRUCTURE_FAULTY_SENSOR")
            .unwrap();
        hf.sensor_fault_witnesses[0].sensor_id = "S1".to_owned();
        let report = evaluate(&data, FIXTURE).unwrap();
        let result = report
            .cases
            .iter()
            .find(|case| case.case_id == "HF_HEALTHY_STRUCTURE_FAULTY_SENSOR")
            .unwrap();
        assert_eq!(result.disposition, "ABSTAIN_SENSOR_FAULT_UNCONFIRMED");
        assert!(result.model_imputed_values.is_none());

        let mut data = fixture();
        let hf = data
            .cases
            .iter_mut()
            .find(|case| case.case_id == "HF_HEALTHY_STRUCTURE_FAULTY_SENSOR")
            .unwrap();
        hf.sensor_fault_witnesses[0].evidence_sha256 = Some("bad".to_owned());
        assert!(validate_fixture(&data).is_err());
    }

    #[test]
    fn invalid_operator_digest_or_weakened_damage_guard_rejects_fixture() {
        let mut data = fixture();
        data.channels[0].operator_sha256 = "invalid".to_owned();
        assert!(validate_fixture(&data).is_err());

        let mut data = fixture();
        data.policy.damage_clearance_from_reconstruction_prohibited = false;
        assert!(validate_fixture(&data).is_err());
    }
}
