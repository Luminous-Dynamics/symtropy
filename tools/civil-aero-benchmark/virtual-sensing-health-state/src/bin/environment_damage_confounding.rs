// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Tier-0 environment/damage identifiability benchmark.
// This synthetic hypothesis enumerator does not detect physical damage or qualify aircraft.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const FIXTURE: &[u8] = include_bytes!("../../data/environment-damage-confounding-v0.1.json");
const CLAIM_CEILING: &str = "synthetic_environment_damage_hypothesis_and_abstention_only";

#[derive(Clone, Debug, Deserialize)]
struct EnvironmentalProfile {
    profile_id: String,
    mean_temperature_delta_k: f64,
    span_gradient_delta_k: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct EnvironmentalModel {
    mean_temperature_effect_per_kelvin: [f64; 3],
    span_gradient_effect_per_kelvin: [f64; 3],
    profiles: Vec<EnvironmentalProfile>,
}

#[derive(Clone, Debug, Deserialize)]
struct FaultDomain {
    domain_type: String,
    domain_id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct StructuralFaultDomainRecord {
    sensor_id: String,
    fault_domains: Vec<FaultDomain>,
    topology_evidence_sha256: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Model {
    structural_channel_ids: Vec<String>,
    structural_channel_common_cause_groups: Vec<String>,
    structural_channel_fault_domains: Vec<StructuralFaultDomainRecord>,
    structural_sensor_response_units: String,
    healthy_signature: [f64; 3],
    damage_signature: [f64; 3],
    environmental_model: EnvironmentalModel,
}

#[derive(Clone, Debug, Deserialize)]
struct Policy {
    absolute_observation_tolerance: f64,
    environmental_profile_mean_tolerance_k: f64,
    environmental_profile_gradient_tolerance_k: f64,
    redundant_gradient_agreement_tolerance_k: f64,
    maximum_probe_standard_uncertainty_k: f64,
    required_environmental_probe_quantities: Vec<String>,
    evidence_digest_algorithm: String,
    witness_must_be_independent_of_structural_channels: bool,
    raw_structural_observations_immutable: bool,
    abstain_on_multiple_joint_hypotheses: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct EnvironmentalProbe {
    sensor_id: String,
    quantity: String,
    reading_k: f64,
    standard_uncertainty_k: f64,
    calibration_status: String,
    operator_status: String,
    evidence_sha256: String,
    topology_evidence_sha256: String,
    fault_domains: Vec<FaultDomain>,
    common_cause_group: String,
    acquisition_window_id: String,
    spatial_support: String,
    independent_of_structural_channels: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct EnvironmentalWitness {
    witness_id: String,
    probes: Vec<EnvironmentalProbe>,
}

#[derive(Clone, Debug, Deserialize)]
struct Expected {
    disposition: String,
    structural_state: Option<String>,
    environmental_state: Option<String>,
    candidate_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct CaseInput {
    case_id: String,
    acquisition_window_id: String,
    observed_values: [f64; 3],
    environmental_witnesses: Vec<EnvironmentalWitness>,
    expected: Expected,
}

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: String,
    problem_id: String,
    claim_ceiling: String,
    model: Model,
    policy: Policy,
    cases: Vec<CaseInput>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct JointHypothesis {
    structural_state: String,
    environmental_state: String,
    predicted_values: [f64; 3],
    maximum_absolute_residual: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct CaseResult {
    case_id: String,
    disposition: String,
    structural_state: Option<String>,
    environmental_state: Option<String>,
    witness_status: String,
    observed_values: [f64; 3],
    candidate_hypotheses: Vec<JointHypothesis>,
    claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    problem_id: String,
    source_sha256: String,
    source_bytes: usize,
    case_count: usize,
    disposition_counts: Vec<(String, usize)>,
    cases: Vec<CaseResult>,
    claim_ceiling: String,
}

#[derive(Clone, Debug)]
struct AdmittedEnvironment {
    profile: EnvironmentalProfile,
    witness_count: usize,
}

fn fail<T>(message: impl Into<String>) -> Result<T, String> {
    Err(message.into())
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

fn close(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance
}

const REQUIRED_FAULT_DOMAIN_TYPES: [&str; 5] = [
    "power",
    "daq",
    "clock",
    "calibration_chain",
    "wiring_harness",
];

fn validated_domain_keys(domains: &[FaultDomain]) -> Result<BTreeSet<(String, String)>, String> {
    if domains.len() != REQUIRED_FAULT_DOMAIN_TYPES.len() {
        return fail("fault-domain manifest must name power, daq, clock, calibration_chain and wiring_harness");
    }
    let mut types = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for domain in domains {
        if !REQUIRED_FAULT_DOMAIN_TYPES.contains(&domain.domain_type.as_str())
            || domain.domain_id.trim().is_empty()
            || !types.insert(domain.domain_type.as_str())
            || !keys.insert((domain.domain_type.clone(), domain.domain_id.clone()))
        {
            return fail("fault-domain manifest contains an unknown type, empty ID or duplicate type");
        }
    }
    if types.len() != REQUIRED_FAULT_DOMAIN_TYPES.len() {
        return fail("fault-domain manifest is incomplete");
    }
    Ok(keys)
}

fn structural_fault_domain_keys(fixture: &Fixture) -> Result<BTreeSet<(String, String)>, String> {
    let mut keys = BTreeSet::new();
    for record in &fixture.model.structural_channel_fault_domains {
        if !is_sha256(&record.topology_evidence_sha256) {
            return fail("structural topology evidence digest is invalid");
        }
        let domains = validated_domain_keys(&record.fault_domains)?;
        keys.extend(domains);
    }
    Ok(keys)
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "aero-causal-environment-damage-confounding/0.1.0" {
        return fail("unsupported environment/damage-confounding schema");
    }
    if fixture.problem_id.trim().is_empty() || fixture.claim_ceiling != CLAIM_CEILING {
        return fail("problem identity or claim ceiling invalid");
    }
    if fixture.model.structural_channel_ids.len() != 3
        || fixture.model.structural_channel_common_cause_groups.len() != 3
        || fixture.model.structural_sensor_response_units != "normalized_response"
    {
        return fail("v0.1 requires three registered normalized structural channels");
    }
    if fixture.model.structural_channel_fault_domains.len() != 3 {
        return fail("v0.1 requires topology fault-domain records for all three structural channels");
    }
    let mut topology_sensor_ids = BTreeSet::new();
    for record in &fixture.model.structural_channel_fault_domains {
        if !topology_sensor_ids.insert(record.sensor_id.as_str())
            || !fixture.model.structural_channel_ids.iter().any(|id| id == &record.sensor_id)
        {
            return fail("structural topology manifest has an unknown or duplicate sensor ID");
        }
        if !is_sha256(&record.topology_evidence_sha256) {
            return fail("structural topology evidence digest is invalid");
        }
        validated_domain_keys(&record.fault_domains)?;
    }
    structural_fault_domain_keys(fixture)?;
    let mut structural_ids = BTreeSet::new();
    let mut structural_groups = BTreeSet::new();
    for (id, group) in fixture
        .model
        .structural_channel_ids
        .iter()
        .zip(&fixture.model.structural_channel_common_cause_groups)
    {
        if id.trim().is_empty()
            || group.trim().is_empty()
            || !structural_ids.insert(id.as_str())
            || !structural_groups.insert(group.as_str())
        {
            return fail("structural channel IDs and common-cause groups must be unique");
        }
    }
    if fixture
        .model
        .healthy_signature
        .iter()
        .chain(&fixture.model.damage_signature)
        .chain(&fixture.model.environmental_model.mean_temperature_effect_per_kelvin)
        .chain(&fixture.model.environmental_model.span_gradient_effect_per_kelvin)
        .any(|value| !value.is_finite())
    {
        return fail("structural or environmental response coefficients must be finite");
    }
    let profiles = &fixture.model.environmental_model.profiles;
    if profiles.len() != 4 {
        return fail("v0.1 requires exactly four frozen environmental profiles");
    }
    let mut profile_ids = BTreeSet::new();
    for profile in profiles {
        if profile.profile_id.trim().is_empty()
            || !profile_ids.insert(profile.profile_id.as_str())
            || !profile.mean_temperature_delta_k.is_finite()
            || !profile.span_gradient_delta_k.is_finite()
        {
            return fail("environmental profile registry is invalid");
        }
    }
    let policy = &fixture.policy;
    for tolerance in [
        policy.absolute_observation_tolerance,
        policy.environmental_profile_mean_tolerance_k,
        policy.environmental_profile_gradient_tolerance_k,
        policy.redundant_gradient_agreement_tolerance_k,
        policy.maximum_probe_standard_uncertainty_k,
    ] {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return fail("all environmental and observation tolerances must be finite and positive");
        }
    }
    if policy.evidence_digest_algorithm != "sha256"
        || !policy.witness_must_be_independent_of_structural_channels
        || !policy.raw_structural_observations_immutable
        || !policy.abstain_on_multiple_joint_hypotheses
        || policy.required_environmental_probe_quantities
            != vec![
                "wing_mean_temperature_delta".to_owned(),
                "wing_span_temperature_gradient".to_owned(),
                "wing_span_temperature_gradient".to_owned(),
            ]
    {
        return fail("environmental witness or abstention policy was changed");
    }

    if fixture.cases.is_empty() || fixture.cases.len() > 128 {
        return fail("case count must be in 1..=128");
    }
    let mut case_ids = BTreeSet::new();
    for case in &fixture.cases {
        if case.case_id.trim().is_empty()
            || case.acquisition_window_id.trim().is_empty()
            || !case_ids.insert(case.case_id.as_str())
            || case.observed_values.iter().any(|value| !value.is_finite())
        {
            return fail("case identity, acquisition window or observed values invalid");
        }
        if case.expected.candidate_count > profiles.len() * 2 {
            return fail(format!("{}: expected candidate count outside model", case.case_id));
        }
        if let Some(state) = case.expected.structural_state.as_deref()
            && !matches!(state, "HEALTHY" | "DAMAGED")
        {
            return fail(format!("{}: unsupported expected structural state", case.case_id));
        }
        if let Some(environment) = case.expected.environmental_state.as_deref()
            && !profile_ids.contains(environment)
        {
            return fail(format!("{}: expected environmental state is unknown", case.case_id));
        }
    }
    Ok(())
}

fn admit_witness(
    fixture: &Fixture,
    case: &CaseInput,
    witness: &EnvironmentalWitness,
) -> Result<EnvironmentalProfile, String> {
    if witness.witness_id.trim().is_empty() {
        return fail("environmental witness ID must be non-empty");
    }
    if witness.probes.len() != 3 {
        return fail("ABSTAIN_INSUFFICIENT_ENVIRONMENTAL_OBSERVABILITY");
    }

    let structural_domains = structural_fault_domain_keys(fixture)?;
    let mut seen_ids = BTreeSet::new();
    let mut seen_groups = BTreeSet::new();
    let mut seen_failure_domains = BTreeSet::new();
    let mut mean_probe: Option<&EnvironmentalProbe> = None;
    let mut gradient_probes: Vec<&EnvironmentalProbe> = Vec::new();
    for probe in &witness.probes {
        if probe.sensor_id.trim().is_empty()
            || !seen_ids.insert(probe.sensor_id.as_str())
            || probe.common_cause_group.trim().is_empty()
            || !seen_groups.insert(probe.common_cause_group.as_str())
        {
            return fail("ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS");
        }
        if fixture
            .model
            .structural_channel_ids
            .iter()
            .any(|id| id == &probe.sensor_id)
            || fixture
                .model
                .structural_channel_common_cause_groups
                .iter()
                .any(|group| group == &probe.common_cause_group)
        {
            return fail("ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT");
        }
        if !is_sha256(&probe.topology_evidence_sha256) {
            return fail("ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS");
        }
        let domain_keys = validated_domain_keys(&probe.fault_domains)
            .map_err(|_| "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS".to_owned())?;
        if !domain_keys.is_disjoint(&structural_domains)
            || domain_keys
                .iter()
                .any(|key| !seen_failure_domains.insert(key.clone()))
        {
            return fail("ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT");
        }
        if !probe.independent_of_structural_channels
            || probe.operator_status != "ADMITTED"
            || probe.calibration_status != "VALID"
            || !is_sha256(&probe.evidence_sha256)
            || !probe.reading_k.is_finite()
            || !probe.standard_uncertainty_k.is_finite()
            || probe.standard_uncertainty_k > fixture.policy.maximum_probe_standard_uncertainty_k
            || probe.standard_uncertainty_k < 0.0
        {
            return fail("ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS");
        }
        if probe.acquisition_window_id != case.acquisition_window_id
            || probe.spatial_support != "WING-THERMAL-CONTEXT-01"
        {
            return fail("ABSTAIN_ENVIRONMENT_WINDOW_MISMATCH");
        }

        match probe.quantity.as_str() {
            "wing_mean_temperature_delta" => {
                if mean_probe.replace(probe).is_some() {
                    return fail("ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS");
                }
            }
            "wing_span_temperature_gradient" => gradient_probes.push(probe),
            _ => return fail("ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS"),
        }
    }
    let Some(mean) = mean_probe else {
        return fail("ABSTAIN_INSUFFICIENT_ENVIRONMENTAL_OBSERVABILITY");
    };
    if gradient_probes.len() != 2 {
        return fail("ABSTAIN_INSUFFICIENT_ENVIRONMENTAL_OBSERVABILITY");
    }
    if !close(
        gradient_probes[0].reading_k,
        gradient_probes[1].reading_k,
        fixture.policy.redundant_gradient_agreement_tolerance_k,
    ) {
        return fail("ABSTAIN_CONFLICTING_ENVIRONMENTAL_WITNESSES");
    }
    let gradient = 0.5 * (gradient_probes[0].reading_k + gradient_probes[1].reading_k);
    let matches: Vec<&EnvironmentalProfile> = fixture
        .model
        .environmental_model
        .profiles
        .iter()
        .filter(|profile| {
            close(
                mean.reading_k,
                profile.mean_temperature_delta_k,
                fixture.policy.environmental_profile_mean_tolerance_k,
            ) && close(
                gradient,
                profile.span_gradient_delta_k,
                fixture.policy.environmental_profile_gradient_tolerance_k,
            )
        })
        .collect();
    if matches.len() != 1 {
        return fail("ABSTAIN_ENVIRONMENT_OUT_OF_MODEL");
    }
    Ok(matches[0].clone())
}

fn environmental_effect(fixture: &Fixture, profile: &EnvironmentalProfile) -> [f64; 3] {
    let model = &fixture.model.environmental_model;
    std::array::from_fn(|index| {
        profile.mean_temperature_delta_k * model.mean_temperature_effect_per_kelvin[index]
            + profile.span_gradient_delta_k * model.span_gradient_effect_per_kelvin[index]
    })
}

fn joint_candidates(
    fixture: &Fixture,
    observed: [f64; 3],
    profiles: &[EnvironmentalProfile],
) -> Vec<JointHypothesis> {
    let mut candidates = Vec::new();
    for (state, signature) in [
        ("HEALTHY", fixture.model.healthy_signature),
        ("DAMAGED", fixture.model.damage_signature),
    ] {
        for profile in profiles {
            let environment = environmental_effect(fixture, profile);
            let predicted_values =
                std::array::from_fn(|index| signature[index] + environment[index]);
            let maximum_absolute_residual = (0..3)
                .map(|index| (observed[index] - predicted_values[index]).abs())
                .fold(0.0_f64, f64::max);
            if (0..3).all(|index| {
                (observed[index] - predicted_values[index]).abs()
                    <= fixture.policy.absolute_observation_tolerance
            }) {
                candidates.push(JointHypothesis {
                    structural_state: state.to_owned(),
                    environmental_state: profile.profile_id.clone(),
                    predicted_values,
                    maximum_absolute_residual,
                });
            }
        }
    }
    candidates
}

fn abstain(case: &CaseInput, disposition: &str, witness_status: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        structural_state: None,
        environmental_state: None,
        witness_status: witness_status.to_owned(),
        observed_values: case.observed_values,
        candidate_hypotheses: Vec::new(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

fn evaluate_case(fixture: &Fixture, case: &CaseInput) -> CaseResult {
    let mut admitted_profiles = Vec::<EnvironmentalProfile>::new();
    if case.environmental_witnesses.is_empty() {
        admitted_profiles.extend(fixture.model.environmental_model.profiles.clone());
    } else {
        let mut witness_ids = BTreeSet::new();
        let mut probe_ids = BTreeSet::new();
        let mut common_cause_groups = BTreeSet::new();
        let mut failure_domain_keys = BTreeSet::new();
        for witness in &case.environmental_witnesses {
            if !witness_ids.insert(witness.witness_id.as_str()) {
                return abstain(
                    case,
                    "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
                    "REJECTED",
                );
            }
            for probe in &witness.probes {
                let domains = match validated_domain_keys(&probe.fault_domains) {
                    Ok(value) => value,
                    Err(_) => {
                        return abstain(
                            case,
                            "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS",
                            "REJECTED",
                        );
                    }
                };
                if !probe_ids.insert(probe.sensor_id.as_str())
                    || !common_cause_groups.insert(probe.common_cause_group.as_str())
                    || domains.iter().any(|domain| !failure_domain_keys.insert(domain.clone()))
                {
                    return abstain(
                        case,
                        "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT",
                        "REJECTED",
                    );
                }
            }
            match admit_witness(fixture, case, witness) {
                Ok(profile) => {
                    if admitted_profiles
                        .iter()
                        .any(|existing| existing.profile_id != profile.profile_id)
                    {
                        return abstain(
                            case,
                            "ABSTAIN_CONFLICTING_ENVIRONMENTAL_WITNESSES",
                            "CONFLICTING",
                        );
                    }
                    if !admitted_profiles
                        .iter()
                        .any(|existing| existing.profile_id == profile.profile_id)
                    {
                        admitted_profiles.push(profile);
                    }
                }
                Err(disposition) => {
                    let status = if disposition == "ABSTAIN_ENVIRONMENT_OUT_OF_MODEL" {
                        "OUT_OF_MODEL"
                    } else {
                        "REJECTED"
                    };
                    return abstain(case, &disposition, status);
                }
            }
        }
    }

    let mut candidates = joint_candidates(fixture, case.observed_values, &admitted_profiles);
    if candidates.is_empty() {
        return abstain(
            case,
            "ABSTAIN_STRUCTURAL_ENVIRONMENT_MODEL_MISMATCH",
            if case.environmental_witnesses.is_empty() {
                "NOT_AVAILABLE"
            } else {
                "ADMITTED"
            },
        );
    }
    if candidates.len() > 1 {
        return CaseResult {
            case_id: case.case_id.clone(),
            disposition: "ABSTAIN_ENVIRONMENT_DAMAGE_AMBIGUOUS".to_owned(),
            structural_state: None,
            environmental_state: None,
            witness_status: if case.environmental_witnesses.is_empty() {
                "NOT_AVAILABLE".to_owned()
            } else {
                "ADMITTED".to_owned()
            },
            observed_values: case.observed_values,
            candidate_hypotheses: candidates,
            claim_ceiling: CLAIM_CEILING.to_owned(),
        };
    }

    let selected = candidates.pop().expect("one joint hypothesis exists");
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: if selected.structural_state == "DAMAGED" {
            "STRUCTURAL_DAMAGE_SUSPECTED".to_owned()
        } else {
            "HEALTHY_WITHIN_DECLARED_SUPPORT".to_owned()
        },
        structural_state: Some(selected.structural_state),
        environmental_state: Some(selected.environmental_state),
        witness_status: if case.environmental_witnesses.is_empty() {
            "NOT_AVAILABLE".to_owned()
        } else {
            "ADMITTED".to_owned()
        },
        observed_values: case.observed_values,
        candidate_hypotheses: vec![selected],
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
        schema_version: "aero-causal-environment-damage-confounding-result/0.1.0".to_owned(),
        problem_id: fixture.problem_id.clone(),
        source_sha256: sha256_hex(source),
        source_bytes: source.len(),
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
        serde_json::from_slice(FIXTURE).expect("environment/damage fixture parses")
    }

    #[test]
    fn frozen_joint_environment_damage_cases_match_expected_labels() {
        let data = fixture();
        let report = evaluate(&data, FIXTURE).expect("environment/damage fixture evaluates");
        assert_eq!(report.case_count, 17);
        for (actual, expected) in report.cases.iter().zip(&data.cases) {
            assert_eq!(actual.disposition, expected.expected.disposition, "{}", actual.case_id);
            assert_eq!(actual.structural_state, expected.expected.structural_state, "{}", actual.case_id);
            assert_eq!(actual.environmental_state, expected.expected.environmental_state, "{}", actual.case_id);
            assert_eq!(actual.candidate_hypotheses.len(), expected.expected.candidate_count, "{}", actual.case_id);
        }
    }

    #[test]
    fn environment_witness_resolves_damage_signature_collision() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        let case = report.cases.iter()
            .find(|item| item.case_id == "APPARENT_DAMAGE_EXPLAINED_BY_THERMAL_GRADIENT")
            .unwrap();
        assert_eq!(case.structural_state.as_deref(), Some("HEALTHY"));
        assert_eq!(case.environmental_state.as_deref(), Some("THERMAL_GRADIENT"));
        assert_eq!(case.witness_status, "ADMITTED");
    }

    #[test]
    fn missing_environment_witness_preserves_both_joint_explanations() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        let case = report.cases.iter()
            .find(|item| item.case_id == "AMBIGUOUS_DAMAGE_VS_THERMAL_GRADIENT_NO_WITNESS")
            .unwrap();
        assert_eq!(case.disposition, "ABSTAIN_ENVIRONMENT_DAMAGE_AMBIGUOUS");
        assert_eq!(case.candidate_hypotheses.len(), 2);
        assert_eq!(
            case.candidate_hypotheses.iter().map(|item| item.structural_state.as_str()).collect::<Vec<_>>(),
            vec!["HEALTHY", "DAMAGED"]
        );
    }

    #[test]
    fn separate_witnesses_sharing_a_common_cause_group_are_not_independent() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        let result = report
            .cases
            .iter()
            .find(|item| item.case_id == "DUPLICATE_COMMON_CAUSE_ENVIRONMENTAL_WITNESSES")
            .unwrap();
        assert_eq!(
            result.disposition,
            "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT"
        );
        assert!(result.structural_state.is_none());
    }

    #[test]
    fn environmental_probe_sharing_structural_daq_domain_is_rejected() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        let result = report
            .cases
            .iter()
            .find(|item| item.case_id == "ENVIRONMENTAL_WITNESS_SHARES_DAQ_WITH_STRUCTURAL_SENSOR")
            .unwrap();
        assert_eq!(
            result.disposition,
            "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT"
        );
        assert!(result.structural_state.is_none());
    }

    #[test]
    fn malformed_topology_receipt_or_incomplete_fault_domains_abstains() {
        let mut data = fixture();
        let case_index = data
            .cases
            .iter()
            .position(|item| item.case_id == "DH_NOMINAL_ENVIRONMENT")
            .unwrap();
        data.cases[case_index].environmental_witnesses[0].probes[0]
            .topology_evidence_sha256 = "bad".to_owned();
        let case = data.cases[case_index].clone();
        let result = evaluate_case(&data, &case);
        assert_eq!(
            result.disposition,
            "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS"
        );

        let mut data = fixture();
        let case_index = data
            .cases
            .iter()
            .position(|item| item.case_id == "DH_NOMINAL_ENVIRONMENT")
            .unwrap();
        data.cases[case_index].environmental_witnesses[0].probes[0]
            .fault_domains
            .pop();
        let case = data.cases[case_index].clone();
        let result = evaluate_case(&data, &case);
        assert_eq!(
            result.disposition,
            "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS"
        );
    }

    #[test]
    fn expired_or_conflicting_environmental_probes_never_authorize_a_damage_call() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        for (case_id, disposition) in [
            ("CONFLICTING_REDUNDANT_GRADIENT_PROBES", "ABSTAIN_CONFLICTING_ENVIRONMENTAL_WITNESSES"),
            ("EXPIRED_ENVIRONMENTAL_PROBE", "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS"),
            ("ENVIRONMENTAL_PROBE_WINDOW_MISMATCH", "ABSTAIN_ENVIRONMENT_WINDOW_MISMATCH"),
            ("ENVIRONMENTAL_PROBE_UNCERTAINTY_TOO_LARGE", "ABSTAIN_UNQUALIFIED_ENVIRONMENTAL_WITNESS"),
            ("ENVIRONMENTAL_OUTSIDE_DECLARED_PROFILES", "ABSTAIN_ENVIRONMENT_OUT_OF_MODEL"),
        ] {
            let result = report.cases.iter().find(|item| item.case_id == case_id).unwrap();
            assert_eq!(result.disposition, disposition);
            assert!(result.structural_state.is_none());
            assert!(result.candidate_hypotheses.is_empty());
        }
    }

    #[test]
    fn environment_witness_must_be_independent_of_structural_sensing_common_causes() {
        let mut data = fixture();
        let case_index = data
            .cases
            .iter()
            .position(|item| item.case_id == "DH_NOMINAL_ENVIRONMENT")
            .unwrap();
        data.cases[case_index].environmental_witnesses[0].probes[0].common_cause_group =
            "STRAIN-BUS-A".to_owned();
        let case = data.cases[case_index].clone();
        let result = evaluate_case(&data, &case);
        assert_eq!(result.disposition, "ABSTAIN_ENVIRONMENTAL_WITNESS_NOT_INDEPENDENT");
        assert!(result.structural_state.is_none());
    }

    #[test]
    fn unknown_or_mismatched_environmental_response_abstains() {
        let report = evaluate(&fixture(), FIXTURE).unwrap();
        let mismatch = report.cases.iter()
            .find(|item| item.case_id == "STRUCTURE_ENVIRONMENT_MODEL_MISMATCH")
            .unwrap();
        assert_eq!(mismatch.disposition, "ABSTAIN_STRUCTURAL_ENVIRONMENT_MODEL_MISMATCH");
        assert!(mismatch.structural_state.is_none());
        let insufficient = report.cases.iter()
            .find(|item| item.case_id == "INSUFFICIENT_ENVIRONMENTAL_PROBES")
            .unwrap();
        assert_eq!(insufficient.disposition, "ABSTAIN_INSUFFICIENT_ENVIRONMENTAL_OBSERVABILITY");
    }
}
