// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Rust-first synthetic policy oracle for design-for-inspection and structural
// health sensing contracts. This is not an NDE/POD estimator, SHM classifier,
// structural solver, airworthiness determination, or maintenance-release tool.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

const CORPUS: &[u8] = include_bytes!("../inspection-observability-corpus-v0.1.json");

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    corpus_id: String,
    claim_ceiling: String,
    physical_probability_of_detection_claim: bool,
    aircraft_acceptance_claim: bool,
    cases: Vec<Case>,
    invariant_contract: InvariantContract,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvariantContract {
    #[serde(rename = "synthetic_POD_claim")]
    synthetic_pod_claim: String,
    #[serde(rename = "synthetic_sensor_reliability_probability")]
    synthetic_sensor_reliability_probability: String,
    maintenance_release: String,
    required_region_coverage: String,
    correction_must_not_clear_damage_hypothesis: bool,
    unknown_sensing_state_must_abstain: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    title: String,
    evidence_class: String,
    article: Article,
    inspection: Inspection,
    sensing: Sensing,
    expected_disposition: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Article {
    configuration_id: String,
    geometry_generation: String,
    material_family: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inspection {
    method: String,
    procedure_revision: String,
    procedure_current: bool,
    calibration_current: bool,
    coordinate_frame_match: bool,
    geometry_generation_match: bool,
    method_applicable: bool,
    surface_coverage_fraction: f64,
    required_regions: Vec<String>,
    observed_regions: Vec<String>,
    critical_regions_observed: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sensing {
    state_known: bool,
    structural_damage_candidate: bool,
    sensor_fault_candidate: bool,
    environment_shift: bool,
    correction_applied: bool,
    #[serde(default)]
    corrected_signal_looks_healthy: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Disposition {
    NoAnomalyInSyntheticFixture,
    UnresolvedIncompleteCriticalCoverage,
    RejectIdentityMismatch,
    RejectInvalidInspectionState,
    UnresolvedMethodOutOfDomain,
    RequireIndependentNde,
    SensorFaultSuspectedNoStructuralClearance,
    ReassessEnvironment,
    UnresolvedSensingState,
}

impl Disposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoAnomalyInSyntheticFixture => "NO_ANOMALY_IN_SYNTHETIC_FIXTURE",
            Self::UnresolvedIncompleteCriticalCoverage => "UNRESOLVED_INCOMPLETE_CRITICAL_COVERAGE",
            Self::RejectIdentityMismatch => "REJECT_IDENTITY_MISMATCH",
            Self::RejectInvalidInspectionState => "REJECT_INVALID_INSPECTION_STATE",
            Self::UnresolvedMethodOutOfDomain => "UNRESOLVED_METHOD_OUT_OF_DOMAIN",
            Self::RequireIndependentNde => "REQUIRE_INDEPENDENT_NDE",
            Self::SensorFaultSuspectedNoStructuralClearance => {
                "SENSOR_FAULT_SUSPECTED_NO_STRUCTURAL_CLEARANCE"
            }
            Self::ReassessEnvironment => "REASSESS_ENVIRONMENT",
            Self::UnresolvedSensingState => "UNRESOLVED_SENSING_STATE",
        }
    }
}

#[derive(Debug, Serialize)]
struct OracleResult {
    schema_version: &'static str,
    corpus_id: String,
    corpus_sha256: String,
    case_count: usize,
    synthetic_dispositions: BTreeMap<String, String>,
    physical_pod: &'static str,
    sensor_reliability_probability: &'static str,
    inspection_acceptance: &'static str,
    maintenance_release: &'static str,
    aircraft_status: &'static str,
    claim_ceiling: &'static str,
}

#[derive(Debug)]
struct ContractError(String);

impl std::fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ContractError {}

type Result<T> = std::result::Result<T, ContractError>;

fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(ContractError(message.into()))
}

fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition { Ok(()) } else { fail(message) }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut value = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut value, "{byte:02x}").expect("writing into String cannot fail");
    }
    value
}

fn decide(case: &Case) -> Disposition {
    let inspection = &case.inspection;
    let sensing = &case.sensing;

    // Identity errors invalidate every geometry/coverage conclusion downstream.
    if !inspection.geometry_generation_match || !inspection.coordinate_frame_match {
        return Disposition::RejectIdentityMismatch;
    }

    // Procedure and calibration are prerequisites, never quality metrics.
    if !inspection.procedure_current || !inspection.calibration_current {
        return Disposition::RejectInvalidInspectionState;
    }

    if !inspection.method_applicable {
        return Disposition::UnresolvedMethodOutOfDomain;
    }

    let required = inspection.required_regions.iter().collect::<BTreeSet<_>>();
    let observed = inspection.observed_regions.iter().collect::<BTreeSet<_>>();
    if !inspection.critical_regions_observed || !required.is_subset(&observed) {
        return Disposition::UnresolvedIncompleteCriticalCoverage;
    }

    if !sensing.state_known {
        return Disposition::UnresolvedSensingState;
    }

    // Conservative precedence: a damage hypothesis requires independent NDE,
    // whether or not sensor faults or signal correction are also present.
    // The corrected-signal appearance is deliberately not a clearance input.
    if sensing.structural_damage_candidate {
        return Disposition::RequireIndependentNde;
    }

    if sensing.sensor_fault_candidate {
        return Disposition::SensorFaultSuspectedNoStructuralClearance;
    }

    if sensing.environment_shift {
        return Disposition::ReassessEnvironment;
    }

    Disposition::NoAnomalyInSyntheticFixture
}

fn validate_case(case: &Case) -> Result<()> {
    let id = &case.case_id;
    require(!id.trim().is_empty(), format!("{id}: empty case ID"))?;
    require(
        case.evidence_class == "synthetic",
        format!("{id}: case must remain labelled synthetic"),
    )?;

    for (label, value) in [
        ("configuration_id", &case.article.configuration_id),
        ("geometry_generation", &case.article.geometry_generation),
        ("material_family", &case.article.material_family),
        ("method", &case.inspection.method),
        ("procedure_revision", &case.inspection.procedure_revision),
    ] {
        require(!value.trim().is_empty(), format!("{id}: missing {label}"))?;
    }

    require(
        case.inspection.surface_coverage_fraction.is_finite()
            && (0.0..=1.0).contains(&case.inspection.surface_coverage_fraction),
        format!("{id}: invalid surface coverage fraction"),
    )?;
    require(
        !case.inspection.required_regions.is_empty(),
        format!("{id}: no required regions"),
    )?;
    require(
        case.inspection
            .required_regions
            .iter()
            .all(|region| !region.trim().is_empty()),
        format!("{id}: empty required region ID"),
    )?;
    require(
        case.inspection
            .observed_regions
            .iter()
            .all(|region| !region.trim().is_empty()),
        format!("{id}: empty observed region ID"),
    )?;

    let unique_required = case
        .inspection
        .required_regions
        .iter()
        .collect::<BTreeSet<_>>();
    let unique_observed = case
        .inspection
        .observed_regions
        .iter()
        .collect::<BTreeSet<_>>();
    require(
        unique_required.len() == case.inspection.required_regions.len(),
        format!("{id}: duplicate required region IDs"),
    )?;
    require(
        unique_observed.len() == case.inspection.observed_regions.len(),
        format!("{id}: duplicate observed region IDs"),
    )?;

    require(
        case.expected_disposition == decide(case).as_str(),
        format!(
            "{id}: expected {}, Rust policy returned {}",
            case.expected_disposition,
            decide(case).as_str()
        ),
    )
}

fn validate_corpus(corpus: &Corpus) -> Result<()> {
    require(
        corpus.schema_version == "aero-inspection-observability-corpus/0.1.0",
        "unsupported corpus schema version",
    )?;
    require(!corpus.corpus_id.trim().is_empty(), "empty corpus ID")?;
    require(
        corpus.claim_ceiling == "synthetic_contract_conformance_only",
        "corpus claim ceiling changed",
    )?;
    require(
        !corpus.physical_probability_of_detection_claim,
        "synthetic fixture cannot claim physical POD",
    )?;
    require(
        !corpus.aircraft_acceptance_claim,
        "synthetic fixture cannot claim aircraft acceptance",
    )?;

    let contract = &corpus.invariant_contract;
    require(
        contract.synthetic_pod_claim == "FORBIDDEN",
        "synthetic POD prohibition weakened",
    )?;
    require(
        contract.synthetic_sensor_reliability_probability == "FORBIDDEN",
        "synthetic sensor-reliability probability prohibition weakened",
    )?;
    require(
        contract.maintenance_release == "NOT_AUTHORIZED",
        "maintenance-release boundary weakened",
    )?;
    require(
        contract.required_region_coverage == "ALL_CRITICAL_REGIONS",
        "critical-region coverage requirement weakened",
    )?;
    require(
        contract.correction_must_not_clear_damage_hypothesis,
        "damage-correction protection missing",
    )?;
    require(
        contract.unknown_sensing_state_must_abstain,
        "unknown sensing-state abstention missing",
    )?;

    require(!corpus.cases.is_empty(), "corpus has no cases")?;
    let mut ids = BTreeSet::new();
    for case in &corpus.cases {
        require(ids.insert(case.case_id.as_str()), "duplicate case ID")?;
        validate_case(case)?;
    }
    Ok(())
}

fn evaluate(corpus_bytes: &[u8]) -> Result<OracleResult> {
    let corpus: Corpus = serde_json::from_slice(corpus_bytes)
        .map_err(|error| ContractError(format!("corpus JSON error: {error}")))?;
    validate_corpus(&corpus)?;

    let mut dispositions = BTreeMap::new();
    for case in &corpus.cases {
        dispositions.insert(case.case_id.clone(), decide(case).as_str().to_owned());
    }

    Ok(OracleResult {
        schema_version: "aero-inspection-observability-result/0.1.0",
        corpus_id: corpus.corpus_id,
        corpus_sha256: sha256_hex(corpus_bytes),
        case_count: corpus.cases.len(),
        synthetic_dispositions: dispositions,
        physical_pod: "NOT_ASSESSED",
        sensor_reliability_probability: "NOT_ASSESSED",
        inspection_acceptance: "NOT_A_REAL_INSPECTION",
        maintenance_release: "NOT_AUTHORIZED",
        aircraft_status: "NOT_ASSESSED",
        claim_ceiling: "synthetic_contract_conformance_only",
    })
}

fn main() {
    let corpus_path = std::env::args().nth(1).map(PathBuf::from);
    let bytes = match corpus_path {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!(
                    "inspection_observability_read_error={}: {error}",
                    path.display()
                );
                std::process::exit(2);
            }
        },
        None => CORPUS.to_vec(),
    };

    match evaluate(&bytes) {
        Ok(result) => match serde_json::to_string(&result) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("inspection_observability_serialize_error={error}");
                std::process::exit(2);
            }
        },
        Err(error) => {
            eprintln!("inspection_observability_error={error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_corpus() -> Corpus {
        let corpus: Corpus = serde_json::from_slice(CORPUS).expect("frozen corpus JSON");
        validate_corpus(&corpus).expect("frozen corpus contract");
        corpus
    }

    fn case_mut<'a>(id: &str, corpus: &'a mut Corpus) -> &'a mut Case {
        corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == id)
            .expect("known frozen case")
    }

    #[test]
    fn frozen_corpus_known_answers_match_rust_policy() {
        let corpus = load_corpus();
        assert_eq!(corpus.cases.len(), 12);
        for case in &corpus.cases {
            assert_eq!(case.expected_disposition, decide(case).as_str());
        }
    }

    #[test]
    fn stale_geometry_and_wrong_frame_are_rejected() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-001", &mut corpus);
        case.inspection.geometry_generation_match = false;
        assert_eq!(decide(case), Disposition::RejectIdentityMismatch);

        case.inspection.geometry_generation_match = true;
        case.inspection.coordinate_frame_match = false;
        assert_eq!(decide(case), Disposition::RejectIdentityMismatch);
    }

    #[test]
    fn expired_calibration_blocks_inspection_disposition() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-001", &mut corpus);
        case.inspection.calibration_current = false;
        assert_eq!(decide(case), Disposition::RejectInvalidInspectionState);
    }

    #[test]
    fn high_aggregate_coverage_cannot_hide_a_missing_critical_region() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-001", &mut corpus);
        case.inspection.surface_coverage_fraction = 0.99;
        case.inspection
            .observed_regions
            .retain(|region| region != "R-ROOT");
        case.inspection.critical_regions_observed = false;
        assert_eq!(
            decide(case),
            Disposition::UnresolvedIncompleteCriticalCoverage
        );
    }

    #[test]
    fn method_outside_domain_is_unresolved() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-001", &mut corpus);
        case.inspection.method_applicable = false;
        assert_eq!(decide(case), Disposition::UnresolvedMethodOutOfDomain);
    }

    #[test]
    fn unknown_sensor_state_abstains() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-001", &mut corpus);
        case.sensing.state_known = false;
        assert_eq!(decide(case), Disposition::UnresolvedSensingState);
    }

    #[test]
    fn correction_cannot_erase_a_damage_hypothesis() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-008", &mut corpus);
        case.sensing.structural_damage_candidate = true;
        case.sensing.sensor_fault_candidate = true;
        case.sensing.correction_applied = true;
        case.sensing.corrected_signal_looks_healthy = Some(true);
        assert_eq!(decide(case), Disposition::RequireIndependentNde);
    }

    #[test]
    fn sensor_fault_never_becomes_structural_clearance() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-001", &mut corpus);
        case.sensing.sensor_fault_candidate = true;
        case.sensing.correction_applied = true;
        assert_eq!(
            decide(case),
            Disposition::SensorFaultSuspectedNoStructuralClearance
        );
    }

    #[test]
    fn environmental_shift_requires_reassessment() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-001", &mut corpus);
        case.sensing.environment_shift = true;
        assert_eq!(decide(case), Disposition::ReassessEnvironment);
    }

    #[test]
    fn simultaneous_sensor_and_damage_hypotheses_require_nde() {
        let mut corpus = load_corpus();
        let case = case_mut("SYN-IO-009", &mut corpus);
        assert_eq!(decide(case), Disposition::RequireIndependentNde);
    }

    #[test]
    fn synthetic_result_keeps_physical_claims_unassessed() {
        let result = evaluate(CORPUS).expect("result");
        assert_eq!(result.case_count, 12);
        assert_eq!(result.physical_pod, "NOT_ASSESSED");
        assert_eq!(result.sensor_reliability_probability, "NOT_ASSESSED");
        assert_eq!(result.inspection_acceptance, "NOT_A_REAL_INSPECTION");
        assert_eq!(result.maintenance_release, "NOT_AUTHORIZED");
        assert_eq!(result.aircraft_status, "NOT_ASSESSED");
        assert_eq!(result.claim_ceiling, "synthetic_contract_conformance_only");
    }
}
