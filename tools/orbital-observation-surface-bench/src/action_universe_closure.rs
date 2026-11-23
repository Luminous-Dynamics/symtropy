//! Finite, source-bound action-universe closure checks.
//!
//! A successful result means only that the declared finite scope, planned partitions,
//! action inventory, enumerator outputs and witnesses agree structurally. It does not
//! prove that the declared scope captures the real world or authenticate an issuer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-action-closure.corpus.v0.1";
const CLAIM_CEILING: &str = "finite-source-bound-action-closure-structure-only";
const MAX_CASES: usize = 128;
const MAX_PARTITIONS: usize = 512;
const MAX_ACTIONS: usize = 4096;
const MAX_EXCLUSIONS: usize = 4096;
const MAX_ENUMERATORS: usize = 8;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<ClosureCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub scope_id: String,
    pub boundary_kind: String,
    pub system_config_version: String,
    pub time_window_start: String,
    pub time_window_end: String,
    pub component_inventory_sha256: String,
    pub actor_authority_boundary: String,
    pub operating_envelope_id: String,
    pub prerequisites_version: String,
    pub policy_version: String,
    pub snapshot_sha256: String,
    pub current_snapshot_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Partition {
    pub partition_id: String,
    pub classification: String,
    pub coverage_witness_receipt_id: String,
    pub action_ids: Vec<String>,
    pub exclusion_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionWitness {
    pub action_id: String,
    pub partition_id: String,
    pub payload_sha256: String,
    pub positive_witness_receipt_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExplicitExclusion {
    pub exclusion_id: String,
    pub partition_id: String,
    pub reason_code: String,
    pub evidence_receipt_id: String,
    pub negative_space_witness_receipt_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Enumerator {
    pub enumerator_id: String,
    pub method_id: String,
    pub algorithm_sha256: String,
    pub source_registry_id: String,
    pub scope_id: String,
    pub scope_snapshot_sha256: String,
    pub execution_receipt_id: String,
    pub dependency_inventory_receipt_id: String,
    pub dependency_ids_sha256: String,
    pub dependency_ids: Vec<String>,
    pub action_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClosureCase {
    pub case_id: String,
    pub scope: Scope,
    pub expected_partition_ids: Vec<String>,
    pub expected_partition_ids_sha256: String,
    pub partitions: Vec<Partition>,
    pub expected_action_ids: Vec<String>,
    pub expected_action_ids_sha256: String,
    /// Digest of sorted [action_id, payload_sha256] pairs.
    pub expected_action_payloads_sha256: String,
    pub actions: Vec<ActionWitness>,
    pub expected_exclusion_ids: Vec<String>,
    pub expected_exclusion_ids_sha256: String,
    pub exclusions: Vec<ExplicitExclusion>,
    pub enumerators: Vec<Enumerator>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumeratorReport {
    pub enumerator_id: String,
    pub method_id: String,
    pub source_registry_id: String,
    pub action_count: usize,
    pub matched_planned_inventory: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub scope_id: String,
    pub scope_snapshot_sha256: String,
    pub expected_partition_count: usize,
    pub observed_partition_count: usize,
    pub expected_action_count: usize,
    pub observed_action_count: usize,
    pub expected_exclusion_count: usize,
    pub observed_exclusion_count: usize,
    pub common_dependency_ids: Vec<String>,
    pub enumerator_reports: Vec<EnumeratorReport>,
    pub missing_partition_ids: Vec<String>,
    pub unwitnessed_exclusion_ids: Vec<String>,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub closed_within_declared_scope_cases: usize,
    pub enumerator_disagreement_cases: usize,
    pub partition_omitted_cases: usize,
    pub negative_space_unwitnessed_cases: usize,
    pub common_cause_unresolved_cases: usize,
    pub scope_drift_cases: usize,
    pub stale_snapshot_cases: usize,
    pub indeterminate_cases: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("failed to serialize canonical ID inventory: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn digest_action_payloads(actions: &[ActionWitness]) -> Result<String, String> {
    let mut pairs = actions
        .iter()
        .map(|action| (action.action_id.clone(), action.payload_sha256.clone()))
        .collect::<Vec<_>>();
    pairs.sort();
    let bytes = serde_json::to_vec(&pairs).map_err(|error| {
        format!("failed to serialize canonical action payload inventory: {error}")
    })?;
    Ok(sha256_hex(&bytes))
}

fn unique_ids(label: &str, ids: &[String], allow_empty: bool) -> Result<BTreeSet<String>, String> {
    if (!allow_empty && ids.is_empty()) || ids.len() > MAX_ACTIONS {
        return Err(format!("{label} inventory is empty or exceeds its bound"));
    }
    let mut result = BTreeSet::new();
    for id in ids {
        if !valid_id(id) || !result.insert(id.clone()) {
            return Err(format!("{label} IDs must be unique nonempty ASCII strings"));
        }
    }
    Ok(result)
}

fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn validate_case(case: &ClosureCase) -> Result<(), String> {
    if !valid_id(&case.case_id)
        || !valid_id(&case.scope.scope_id)
        || !valid_id(&case.scope.system_config_version)
        || !valid_id(&case.scope.time_window_start)
        || !valid_id(&case.scope.time_window_end)
        || !valid_sha256(&case.scope.component_inventory_sha256)
        || !valid_id(&case.scope.actor_authority_boundary)
        || !valid_id(&case.scope.operating_envelope_id)
        || !valid_id(&case.scope.prerequisites_version)
        || !valid_id(&case.scope.policy_version)
        || !valid_sha256(&case.scope.snapshot_sha256)
        || !valid_sha256(&case.scope.current_snapshot_sha256)
        || case.expected_partition_ids.is_empty()
        || case.expected_partition_ids.len() > MAX_PARTITIONS
        || case.partitions.len() > MAX_PARTITIONS
        || case.expected_action_ids.len() > MAX_ACTIONS
        || case.actions.len() > MAX_ACTIONS
        || case.expected_exclusion_ids.len() > MAX_EXCLUSIONS
        || case.exclusions.len() > MAX_EXCLUSIONS
        || case.enumerators.len() > MAX_ENUMERATORS
    {
        return Err(format!(
            "case {} has invalid scope identity or exceeds finite bounds",
            case.case_id
        ));
    }

    let expected_partitions = unique_ids("planned partition", &case.expected_partition_ids, false)?;
    if digest_ids(&case.expected_partition_ids)? != case.expected_partition_ids_sha256 {
        return Err(format!(
            "case {} planned partition digest mismatch",
            case.case_id
        ));
    }
    let expected_actions = unique_ids("planned action", &case.expected_action_ids, false)?;
    if digest_ids(&case.expected_action_ids)? != case.expected_action_ids_sha256 {
        return Err(format!(
            "case {} planned action digest mismatch",
            case.case_id
        ));
    }
    let expected_exclusions = unique_ids("planned exclusion", &case.expected_exclusion_ids, true)?;
    if digest_ids(&case.expected_exclusion_ids)? != case.expected_exclusion_ids_sha256 {
        return Err(format!(
            "case {} planned exclusion digest mismatch",
            case.case_id
        ));
    }

    if !valid_sha256(&case.expected_action_payloads_sha256) {
        return Err(format!(
            "case {} action payload digest is invalid",
            case.case_id
        ));
    }
    let mut action_ids = BTreeSet::new();
    for action in &case.actions {
        if !valid_id(&action.action_id)
            || !valid_id(&action.partition_id)
            || !valid_sha256(&action.payload_sha256)
            || !valid_id(&action.positive_witness_receipt_id)
            || !action_ids.insert(action.action_id.clone())
        {
            return Err(format!(
                "case {} contains an invalid or duplicate action witness",
                case.case_id
            ));
        }
    }

    let mut partition_ids = BTreeSet::new();
    for partition in &case.partitions {
        if !valid_id(&partition.partition_id)
            || !valid_id(&partition.coverage_witness_receipt_id)
            || !matches!(
                partition.classification.as_str(),
                "present" | "excluded" | "not_applicable" | "indeterminate"
            )
            || !partition_ids.insert(partition.partition_id.clone())
        {
            return Err(format!(
                "case {} contains an invalid or duplicate partition row",
                case.case_id
            ));
        }
        unique_ids("partition action", &partition.action_ids, true)?;
        unique_ids("partition exclusion", &partition.exclusion_ids, true)?;
    }

    let mut exclusion_ids = BTreeSet::new();
    for exclusion in &case.exclusions {
        if !valid_id(&exclusion.exclusion_id)
            || !valid_id(&exclusion.partition_id)
            || !exclusion_ids.insert(exclusion.exclusion_id.clone())
        {
            return Err(format!(
                "case {} contains an invalid or duplicate exclusion witness",
                case.case_id
            ));
        }
    }

    let mut enumerator_ids = BTreeSet::new();
    for enumerator in &case.enumerators {
        if !valid_id(&enumerator.enumerator_id)
            || !valid_id(&enumerator.method_id)
            || !valid_sha256(&enumerator.algorithm_sha256)
            || !valid_id(&enumerator.source_registry_id)
            || !valid_id(&enumerator.scope_id)
            || !valid_sha256(&enumerator.scope_snapshot_sha256)
            || !valid_id(&enumerator.execution_receipt_id)
            || !valid_sha256(&enumerator.dependency_ids_sha256)
            || digest_ids(&enumerator.dependency_ids)? != enumerator.dependency_ids_sha256
            || !enumerator_ids.insert(enumerator.enumerator_id.clone())
        {
            return Err(format!(
                "case {} contains an invalid or duplicate enumerator receipt",
                case.case_id
            ));
        }
        unique_ids("enumerator dependency", &enumerator.dependency_ids, true)?;
        unique_ids("enumerated action", &enumerator.action_ids, true)?;
    }

    // Recompute the payload digest even for negative controls. A stale digest is an input
    // integrity failure, distinct from an enumerator that legitimately disagrees.
    if digest_action_payloads(&case.actions)? != case.expected_action_payloads_sha256 {
        return Err(format!(
            "case {} action payload inventory digest mismatch",
            case.case_id
        ));
    }
    let _ = (expected_partitions, expected_actions, expected_exclusions);
    Ok(())
}

fn evaluate_case(case: &ClosureCase) -> Result<CaseResult, String> {
    validate_case(case)?;

    let expected_partitions = case
        .expected_partition_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed_partitions = case
        .partitions
        .iter()
        .map(|row| row.partition_id.clone())
        .collect::<BTreeSet<_>>();
    let missing_partition_ids = set_difference(&expected_partitions, &observed_partitions);
    let extra_partition_ids = set_difference(&observed_partitions, &expected_partitions);

    let expected_actions = case
        .expected_action_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed_actions = case
        .actions
        .iter()
        .map(|row| row.action_id.clone())
        .collect::<BTreeSet<_>>();
    let action_inventory_matches = expected_actions == observed_actions;

    let expected_exclusions = case
        .expected_exclusion_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let observed_exclusions = case
        .exclusions
        .iter()
        .map(|row| row.exclusion_id.clone())
        .collect::<BTreeSet<_>>();
    let exclusion_inventory_matches = expected_exclusions == observed_exclusions;

    let action_by_id = case
        .actions
        .iter()
        .map(|row| (row.action_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let exclusion_by_id = case
        .exclusions
        .iter()
        .map(|row| (row.exclusion_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let mut partition_semantics_valid = true;
    let mut unwitnessed_exclusion_ids = Vec::new();

    for partition in &case.partitions {
        if partition.classification == "indeterminate" {
            partition_semantics_valid = false;
        }
        let listed_actions = partition
            .action_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let witnessed_actions = case
            .actions
            .iter()
            .filter(|action| action.partition_id == partition.partition_id)
            .map(|action| action.action_id.clone())
            .collect::<BTreeSet<_>>();
        if listed_actions != witnessed_actions {
            partition_semantics_valid = false;
        }
        if partition.classification == "present" && witnessed_actions.is_empty() {
            partition_semantics_valid = false;
        }
        if matches!(
            partition.classification.as_str(),
            "excluded" | "not_applicable"
        ) {
            if partition.exclusion_ids.is_empty() {
                partition_semantics_valid = false;
                unwitnessed_exclusion_ids.extend(
                    case.exclusions
                        .iter()
                        .filter(|item| item.partition_id == partition.partition_id)
                        .map(|item| item.exclusion_id.clone()),
                );
            }
            for exclusion_id in &partition.exclusion_ids {
                match exclusion_by_id.get(exclusion_id.as_str()) {
                    Some(exclusion)
                        if exclusion.partition_id == partition.partition_id
                            && valid_id(&exclusion.reason_code)
                            && valid_id(&exclusion.evidence_receipt_id)
                            && valid_id(&exclusion.negative_space_witness_receipt_id) => {}
                    _ => unwitnessed_exclusion_ids.push(exclusion_id.clone()),
                }
            }
        } else if !partition.exclusion_ids.is_empty() {
            partition_semantics_valid = false;
        }
        for action_id in &partition.action_ids {
            if !action_by_id.contains_key(action_id.as_str()) {
                partition_semantics_valid = false;
            }
        }
    }

    let action_partition_ids = case
        .actions
        .iter()
        .map(|action| action.partition_id.as_str())
        .collect::<BTreeSet<_>>();
    if action_partition_ids
        .iter()
        .any(|partition_id| !observed_partitions.contains(*partition_id))
    {
        partition_semantics_valid = false;
    }

    if !exclusion_inventory_matches {
        unwitnessed_exclusion_ids.extend(set_difference(&expected_exclusions, &observed_exclusions));
        unwitnessed_exclusion_ids.extend(set_difference(&observed_exclusions, &expected_exclusions));
    }
    unwitnessed_exclusion_ids.sort();
    unwitnessed_exclusion_ids.dedup();

    let mut common_dependencies = BTreeSet::new();
    for left_index in 0..case.enumerators.len() {
        let left = &case.enumerators[left_index];
        for right in case.enumerators.iter().skip(left_index + 1) {
            let left_dependencies = left
                .dependency_ids
                .iter()
                .cloned()
                .chain(std::iter::once(format!(
                    "registry:{}",
                    left.source_registry_id
                )))
                .collect::<BTreeSet<_>>();
            let right_dependencies = right
                .dependency_ids
                .iter()
                .cloned()
                .chain(std::iter::once(format!(
                    "registry:{}",
                    right.source_registry_id
                )))
                .collect::<BTreeSet<_>>();
            common_dependencies
                .extend(left_dependencies.intersection(&right_dependencies).cloned());
        }
    }

    let mut enumerator_reports = Vec::new();
    let mut enumerator_sets = Vec::new();
    let mut methods = BTreeSet::new();
    let mut algorithms = BTreeSet::new();
    let mut registries = BTreeSet::new();
    let mut enumerator_scope_matches = true;
    for enumerator in &case.enumerators {
        let ids = enumerator
            .action_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let matched = ids == expected_actions;
        enumerator_sets.push(ids);
        methods.insert(enumerator.method_id.clone());
        algorithms.insert(enumerator.algorithm_sha256.clone());
        registries.insert(enumerator.source_registry_id.clone());
        if enumerator.scope_id != case.scope.scope_id {
            enumerator_scope_matches = false;
        }
        if enumerator.scope_snapshot_sha256 != case.scope.snapshot_sha256 {
            enumerator_scope_matches = false;
        }
        enumerator_reports.push(EnumeratorReport {
            enumerator_id: enumerator.enumerator_id.clone(),
            method_id: enumerator.method_id.clone(),
            source_registry_id: enumerator.source_registry_id.clone(),
            action_count: enumerator.action_ids.len(),
            matched_planned_inventory: matched,
        });
    }

    let enumerators_are_distinct = case.enumerators.len() >= 2
        && methods.len() == case.enumerators.len()
        && algorithms.len() == case.enumerators.len()
        && registries.len() == case.enumerators.len()
        && case.enumerators.iter().all(|enumerator| {
            valid_id(&enumerator.dependency_inventory_receipt_id)
                && !enumerator.dependency_ids.is_empty()
        })
        && common_dependencies.is_empty();

    let enumerator_agreement = !enumerator_sets.is_empty()
        && enumerator_sets.iter().all(|set| *set == expected_actions)
        && enumerator_sets.windows(2).all(|pair| pair[0] == pair[1]);

    let status = if case.scope.boundary_kind != "finite_bounded" {
        "CLOSURE_INDETERMINATE"
    } else if case.scope.snapshot_sha256 != case.scope.current_snapshot_sha256 {
        "SCOPE_DRIFT"
    } else if !enumerator_scope_matches {
        "STALE_SNAPSHOT"
    } else if !missing_partition_ids.is_empty() || !extra_partition_ids.is_empty() {
        "PARTITION_OMITTED"
    } else if !partition_semantics_valid || !unwitnessed_exclusion_ids.is_empty() {
        "NEGATIVE_SPACE_UNWITNESSED"
    } else if !enumerator_sets.is_empty() && !enumerators_are_distinct {
        "COMMON_CAUSE_UNRESOLVED"
    } else if !action_inventory_matches || !enumerator_agreement || case.enumerators.len() < 2 {
        "ENUMERATOR_DISAGREEMENT"
    } else {
        "CLOSED_WITHIN_DECLARED_SCOPE"
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        scope_id: case.scope.scope_id.clone(),
        scope_snapshot_sha256: case.scope.snapshot_sha256.clone(),
        expected_partition_count: case.expected_partition_ids.len(),
        observed_partition_count: case.partitions.len(),
        expected_action_count: case.expected_action_ids.len(),
        observed_action_count: case.actions.len(),
        expected_exclusion_count: case.expected_exclusion_ids.len(),
        observed_exclusion_count: case.exclusions.len(),
        common_dependency_ids: common_dependencies.into_iter().collect(),
        enumerator_reports,
        missing_partition_ids,
        unwitnessed_exclusion_ids,
        physical_qualification_granted: false,
        authority_granted: false,
    })
}

pub fn evaluate(corpus: &Corpus) -> Result<Evaluation, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported action-closure corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("action-closure case IDs must be unique".to_owned());
        }
        cases.push(evaluate_case(case)?);
    }

    let count = |status: &str| cases.iter().filter(|case| case.status == status).count();
    let summary = Summary {
        case_count: cases.len(),
        closed_within_declared_scope_cases: count("CLOSED_WITHIN_DECLARED_SCOPE"),
        enumerator_disagreement_cases: count("ENUMERATOR_DISAGREEMENT"),
        partition_omitted_cases: count("PARTITION_OMITTED"),
        negative_space_unwitnessed_cases: count("NEGATIVE_SPACE_UNWITNESSED"),
        common_cause_unresolved_cases: count("COMMON_CAUSE_UNRESOLVED"),
        scope_drift_cases: count("SCOPE_DRIFT"),
        stale_snapshot_cases: count("STALE_SNAPSHOT"),
        indeterminate_cases: count("CLOSURE_INDETERMINATE"),
    };
    let all_closed = summary.closed_within_declared_scope_cases == cases.len();
    Ok(Evaluation {
        result: if all_closed {
            "pass"
        } else {
            "adversarial-controls-detected"
        }
        .to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases,
        physical_qualification_granted: false,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Corpus {
        serde_json::from_str(include_str!(
            "../fixtures/v0.1/action-universe-closure-cases.json"
        ))
        .expect("action-closure fixture parses")
    }

    #[test]
    fn detects_shared_registry_common_cause() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let case = result
            .cases
            .iter()
            .find(|case| case.case_id == "shared_registry")
            .unwrap();
        assert_eq!(case.status, "COMMON_CAUSE_UNRESOLVED");
        assert!(
            case.common_dependency_ids
                .iter()
                .any(|item| item == "registry:catalogue-alpha")
        );
    }

    #[test]
    fn detects_enumerator_disagreement() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let case = result
            .cases
            .iter()
            .find(|case| case.case_id == "enumerator_disagreement")
            .unwrap();
        assert_eq!(case.status, "ENUMERATOR_DISAGREEMENT");
    }

    #[test]
    fn detects_planned_partition_omission() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let case = result
            .cases
            .iter()
            .find(|case| case.case_id == "partition_omitted")
            .unwrap();
        assert_eq!(case.status, "PARTITION_OMITTED");
        assert_eq!(case.missing_partition_ids, vec!["partition-c".to_owned()]);
    }

    #[test]
    fn detects_unwitnessed_negative_space() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let case = result
            .cases
            .iter()
            .find(|case| case.case_id == "unwitnessed_exclusion")
            .unwrap();
        assert_eq!(case.status, "NEGATIVE_SPACE_UNWITNESSED");
        assert_eq!(case.unwitnessed_exclusion_ids, vec!["exclude-c".to_owned()]);
    }

    #[test]
    fn detects_scope_drift_and_stale_enumerator_snapshots() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(
            result
                .cases
                .iter()
                .find(|case| case.case_id == "scope_drift")
                .unwrap()
                .status,
            "SCOPE_DRIFT"
        );
        assert_eq!(
            result
                .cases
                .iter()
                .find(|case| case.case_id == "stale_enumerator")
                .unwrap()
                .status,
            "STALE_SNAPSHOT"
        );
    }

    #[test]
    fn closes_only_a_bounded_fully_witnessed_scope() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let case = result
            .cases
            .iter()
            .find(|case| case.case_id == "bounded_closed")
            .unwrap();
        assert_eq!(case.status, "CLOSED_WITHIN_DECLARED_SCOPE");
        assert!(!case.physical_qualification_granted);
        assert!(!case.authority_granted);
    }

    #[test]
    fn never_grants_physical_qualification_or_authority() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert!(!result.physical_qualification_granted);
        assert!(!result.authority_granted);
        assert!(
            result
                .cases
                .iter()
                .all(|case| !case.physical_qualification_granted && !case.authority_granted)
        );
    }

    #[test]
    fn missing_dependency_inventory_evidence_blocks_independence() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let case = result
            .cases
            .iter()
            .find(|case| case.case_id == "dependency_evidence_missing")
            .expect("dependency evidence control exists");
        assert_eq!(case.status, "COMMON_CAUSE_UNRESOLVED");
    }

    #[test]
    fn extra_unplanned_exclusion_cannot_be_silently_accepted() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let case = result
            .cases
            .iter()
            .find(|case| case.case_id == "extra_exclusion")
            .expect("extra exclusion control exists");
        assert_eq!(case.status, "NEGATIVE_SPACE_UNWITNESSED");
        assert_eq!(case.unwitnessed_exclusion_ids, vec!["exclude-extra".to_owned()]);
    }
}
