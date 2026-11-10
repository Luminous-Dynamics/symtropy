#![forbid(unsafe_code)]

//! Deterministic offline evaluator for synthetic maritime engineering-health cases.
//! This crate qualifies evidence contracts only; it does not diagnose machinery,
//! model physical processes, authorize maintenance, or control equipment.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashSet};
use std::fmt;

const EXPECTED_CORPUS_ID: &str = "luminous.symtropy.maritime-engineering-health";
const EXPECTED_CORPUS_VERSION: &str = "0.1.0";
const MIN_CASES: usize = 30;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub corpus_id: String,
    pub corpus_version: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub case_class: CaseClass,
    pub description: String,
    pub asset: Asset,
    pub observations: Vec<Observation>,
    pub fault_hypotheses: Vec<String>,
    pub unknown_domain: bool,
    pub recovery: Recovery,
    pub expected: Expected,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseClass {
    Positive,
    Negative,
    Ambiguity,
    CommonCause,
    Recovery,
    Metamorphic,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub vessel_id: String,
    pub asset_id: String,
    pub component_id: String,
    pub config_revision: String,
    pub capability: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub observation_id: String,
    pub after_ms: u64,
    pub signal: String,
    pub source_role: SourceRole,
    pub quality: Quality,
    pub config_revision: String,
    pub dependency_group: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    MeasuredSensor,
    NativeEquipmentAlarm,
    OperatorReport,
    MaintenanceRecord,
    ModelResidual,
    ExternalDiagnostic,
    SimulatorOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Current,
    Stale,
    WrongConfiguration,
    WrongUnit,
    CalibrationExpired,
    Unauthenticated,
    Conflicted,
    SimulatorOnly,
    MissingLineage,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Recovery {
    pub attempted: bool,
    pub repair_record: bool,
    pub inspection: bool,
    pub proof_test: bool,
    pub authorization: bool,
    pub configuration_reconciled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub admitted_observations: usize,
    pub decision_evidence_observations: usize,
    pub distinct_declared_dependency_groups: usize,
    pub disposition: Disposition,
    pub capability_remains_restricted: bool,
    pub restoration_evidence_complete: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    RestrictCapability,
    ContinueWithWatch,
    UnknownEscalate,
    RestorationBlocked,
    RestorationEvidenceComplete,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub actual: Expected,
    pub expected: Expected,
    pub matched: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Report {
    pub corpus_sha256: String,
    pub corpus_version: String,
    pub total_cases: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub results: Vec<CaseResult>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EvalError {
    Parse(String),
    Invalid(String),
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(message) => write!(f, "corpus parse error: {message}"),
            Self::Invalid(message) => write!(f, "invalid corpus: {message}"),
        }
    }
}

impl std::error::Error for EvalError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Reports and maintenance records are admissible supporting evidence, but do
/// not become direct measurements. These roles cannot establish equipment state.
fn is_decision_evidence(role: SourceRole) -> bool {
    matches!(
        role,
        SourceRole::MeasuredSensor
            | SourceRole::NativeEquipmentAlarm
            | SourceRole::ModelResidual
            | SourceRole::ExternalDiagnostic
    )
}

fn is_admitted(observation: &Observation, asset: &Asset) -> bool {
    observation.quality == Quality::Current
        && observation.config_revision == asset.config_revision
        && observation.source_role != SourceRole::SimulatorOnly
}

pub fn validate_case(case: &Case) -> Result<(), EvalError> {
    if !nonempty(&case.case_id) || !nonempty(&case.description) {
        return Err(EvalError::Invalid("case identity/description is empty".into()));
    }
    for (field, value) in [
        ("vessel_id", case.asset.vessel_id.as_str()),
        ("asset_id", case.asset.asset_id.as_str()),
        ("component_id", case.asset.component_id.as_str()),
        ("config_revision", case.asset.config_revision.as_str()),
        ("capability", case.asset.capability.as_str()),
    ] {
        if !nonempty(value) {
            return Err(EvalError::Invalid(format!(
                "{} is empty in {}",
                field, case.case_id
            )));
        }
    }

    let mut ids = HashSet::new();
    let mut previous = None;
    for observation in &case.observations {
        if !nonempty(&observation.observation_id)
            || !nonempty(&observation.signal)
            || !nonempty(&observation.config_revision)
            || !nonempty(&observation.dependency_group)
        {
            return Err(EvalError::Invalid(format!(
                "observation identity/lineage is empty in {}",
                case.case_id
            )));
        }
        if !ids.insert(observation.observation_id.as_str()) {
            return Err(EvalError::Invalid(format!(
                "duplicate observation id {} in {}",
                observation.observation_id, case.case_id
            )));
        }
        if let Some(prior) = previous {
            if observation.after_ms < prior {
                return Err(EvalError::Invalid(format!(
                    "non-monotonic observation offsets in {}",
                    case.case_id
                )));
            }
        }
        previous = Some(observation.after_ms);
    }

    if case.fault_hypotheses.iter().any(|fault| !nonempty(fault)) {
        return Err(EvalError::Invalid(format!(
            "empty fault hypothesis in {}",
            case.case_id
        )));
    }
    Ok(())
}

pub fn validate_corpus(corpus: &Corpus) -> Result<(), EvalError> {
    if corpus.corpus_id != EXPECTED_CORPUS_ID {
        return Err(EvalError::Invalid("unexpected corpus id".into()));
    }
    if corpus.corpus_version != EXPECTED_CORPUS_VERSION {
        return Err(EvalError::Invalid("unsupported corpus version".into()));
    }
    if corpus.cases.len() < MIN_CASES {
        return Err(EvalError::Invalid(format!(
            "corpus must contain at least {MIN_CASES} cases"
        )));
    }
    if !nonempty(&corpus.claim_ceiling) {
        return Err(EvalError::Invalid("claim ceiling is empty".into()));
    }

    let mut case_ids = HashSet::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err(EvalError::Invalid(format!(
                "duplicate case id {}",
                case.case_id
            )));
        }
        validate_case(case)?;
    }
    Ok(())
}

pub fn evaluate_case(case: &Case) -> Expected {
    let admitted: Vec<&Observation> = case
        .observations
        .iter()
        .filter(|observation| is_admitted(observation, &case.asset))
        .collect();

    let decision_evidence: Vec<&Observation> = admitted
        .iter()
        .copied()
        .filter(|observation| is_decision_evidence(observation.source_role))
        .collect();

    // A count of distinct declared dependency groups is not a claim of
    // statistical independence or proof that common causes are absent.
    let dependency_groups: BTreeSet<&str> = decision_evidence
        .iter()
        .filter_map(|observation| {
            let group = observation.dependency_group.trim();
            if group.is_empty() || group.eq_ignore_ascii_case("unknown") {
                None
            } else {
                Some(group)
            }
        })
        .collect();

    let restoration_complete = case.recovery.attempted
        && case.recovery.repair_record
        && case.recovery.inspection
        && case.recovery.proof_test
        && case.recovery.authorization
        && case.recovery.configuration_reconciled
        && !decision_evidence.is_empty()
        && !case.unknown_domain
        && case.fault_hypotheses.is_empty();

    let disposition = if case.recovery.attempted {
        if restoration_complete {
            Disposition::RestorationEvidenceComplete
        } else {
            Disposition::RestorationBlocked
        }
    } else if case.unknown_domain || decision_evidence.is_empty() {
        Disposition::UnknownEscalate
    } else if !case.fault_hypotheses.is_empty() {
        Disposition::RestrictCapability
    } else {
        Disposition::ContinueWithWatch
    };

    Expected {
        admitted_observations: admitted.len(),
        decision_evidence_observations: decision_evidence.len(),
        distinct_declared_dependency_groups: dependency_groups.len(),
        disposition,
        capability_remains_restricted: disposition != Disposition::ContinueWithWatch,
        restoration_evidence_complete: restoration_complete,
    }
}

pub fn evaluate_bytes(bytes: &[u8]) -> Result<Report, EvalError> {
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| EvalError::Parse(error.to_string()))?;
    validate_corpus(&corpus)?;

    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        let actual = evaluate_case(case);
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            matched: actual == case.expected,
            actual,
            expected: case.expected.clone(),
        });
    }

    let passed_cases = results.iter().filter(|result| result.matched).count();
    let failed_cases = results.len() - passed_cases;
    let digest = Sha256::digest(bytes);
    let corpus_sha256 = digest.iter().map(|byte| format!("{byte:02x}")).collect();

    Ok(Report {
        corpus_sha256,
        corpus_version: corpus.corpus_version,
        total_cases: results.len(),
        passed_cases,
        failed_cases,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &[u8] = include_bytes!("../../maritime/engineering-health-corpus-v0.1.json");

    fn corpus() -> Corpus {
        serde_json::from_slice(CORPUS).expect("frozen corpus parses")
    }

    #[test]
    fn frozen_corpus_matches_all_declared_outcomes() {
        let report = evaluate_bytes(CORPUS).expect("corpus evaluates");
        assert_eq!(report.total_cases, 36);
        assert_eq!(report.passed_cases, 36);
        assert_eq!(report.failed_cases, 0);
        assert_eq!(report.corpus_sha256.len(), 64);
    }

    #[test]
    fn shared_dependency_does_not_become_multiple_independent_groups() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "common-cause-multiple-alarms")
            .expect("common-cause fixture exists");
        let actual = evaluate_case(case);
        assert_eq!(actual.decision_evidence_observations, 3);
        assert_eq!(actual.distinct_declared_dependency_groups, 1);
        assert_eq!(actual.disposition, Disposition::RestrictCapability);
    }

    #[test]
    fn supporting_reports_are_not_promoted_to_decision_evidence() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "ambiguity-maintenance-note-only")
            .expect("maintenance-note fixture exists");
        let actual = evaluate_case(case);
        assert_eq!(actual.admitted_observations, 1);
        assert_eq!(actual.decision_evidence_observations, 0);
        assert_eq!(actual.disposition, Disposition::UnknownEscalate);
    }

    #[test]
    fn simulator_only_never_becomes_admitted_evidence() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "negative-simulator-only")
            .expect("simulator fixture exists");
        let actual = evaluate_case(case);
        assert_eq!(actual.admitted_observations, 0);
        assert_eq!(actual.disposition, Disposition::UnknownEscalate);
    }

    #[test]
    fn all_invalid_quality_states_are_excluded() {
        let corpus = corpus();
        for id in [
            "negative-fuel-stale",
            "negative-wrong-unit",
            "negative-wrong-config",
            "negative-calibration-expired",
            "negative-unauthenticated",
            "negative-missing-lineage",
        ] {
            let case = corpus.cases.iter().find(|case| case.case_id == id).unwrap();
            assert_eq!(
                evaluate_case(case).decision_evidence_observations,
                0,
                "{id}"
            );
        }
    }

    #[test]
    fn unknown_domain_never_defaults_to_healthy() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "ambiguity-unknown-model-domain")
            .expect("unknown-domain fixture exists");
        let actual = evaluate_case(case);
        assert_eq!(actual.disposition, Disposition::UnknownEscalate);
        assert!(actual.capability_remains_restricted);
    }

    #[test]
    fn recovery_requires_every_record_and_never_clears_restriction() {
        let corpus = corpus();
        for id in [
            "recovery-missing-inspection",
            "recovery-no-authorization",
            "recovery-config-unreconciled",
            "recovery-stale-test-evidence",
            "recovery-active-fault-remains",
        ] {
            let case = corpus.cases.iter().find(|case| case.case_id == id).unwrap();
            let actual = evaluate_case(case);
            assert_eq!(actual.disposition, Disposition::RestorationBlocked, "{id}");
            assert!(actual.capability_remains_restricted, "{id}");
            assert!(!actual.restoration_evidence_complete, "{id}");
        }
        let valid = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "recovery-evidence-complete")
            .expect("complete recovery fixture exists");
        let actual = evaluate_case(valid);
        assert_eq!(actual.disposition, Disposition::RestorationEvidenceComplete);
        assert!(actual.capability_remains_restricted);
        assert!(actual.restoration_evidence_complete);
    }

    #[test]
    fn non_monotonic_observation_offsets_are_rejected() {
        let mut case = corpus()
            .cases
            .into_iter()
            .find(|case| case.case_id == "positive-three-separated-inputs")
            .expect("source fixture exists");
        case.observations[1].after_ms = 50;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn duplicate_observation_ids_are_rejected() {
        let mut case = corpus()
            .cases
            .into_iter()
            .find(|case| case.case_id == "positive-three-separated-inputs")
            .expect("source fixture exists");
        case.observations[1].observation_id = case.observations[0].observation_id.clone();
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let input = br#"{"corpus_id":"luminous.symtropy.maritime-engineering-health","corpus_version":"0.1.0","claim_ceiling":"x","unexpected":true,"cases":[]}"#;
        assert!(matches!(evaluate_bytes(input), Err(EvalError::Parse(_))));
    }
}
