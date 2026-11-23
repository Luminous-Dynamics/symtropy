//! Independent finite domain-grammar oracle for open-world action-universe falsification.
//!
//! This module verifies only a bounded synthetic grammar. A successful status never
//! establishes that the grammar itself describes the physical world.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-open-world-domain.corpus.v0.1";
const CLAIM_CEILING: &str = "independent-finite-domain-grammar-falsification-only";
const MAX_CASES: usize = 128;
const MAX_ROWS: usize = 512;
const MAX_ACTIONS: usize = 4096;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<DomainCase>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomainManifest {
    pub domain_id: String,
    pub grammar_version: String,
    pub snapshot_sha256: String,
    pub current_snapshot_sha256: String,
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    pub components: Vec<Component>,
    pub operations: Vec<Operation>,
    pub actors: Vec<Actor>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub component_id: String,
    pub capability_ids: Vec<String>,
    pub resource_capacity_milli: u64,
    pub active_start_ms: u64,
    pub active_end_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub operation_id: String,
    pub required_capability_id: String,
    pub required_role_id: String,
    pub resource_required_milli: u64,
    pub active_start_ms: u64,
    pub active_end_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub actor_id: String,
    pub role_ids: Vec<String>,
    pub active_start_ms: u64,
    pub active_end_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionWitness {
    pub action_id: String,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Enumerator {
    pub enumerator_id: String,
    pub method_id: String,
    pub source_id: String,
    pub snapshot_sha256: String,
    pub execution_receipt_id: String,
    pub action_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalChallenge {
    pub action_id: String,
    pub payload_sha256: String,
    pub in_scope_evidence_receipt_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DomainCase {
    pub case_id: String,
    pub domain: DomainManifest,
    pub registry_enumerator: Enumerator,
    pub model_enumerator: Enumerator,
    pub external_challenges: Vec<ExternalChallenge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumeratorReport {
    pub enumerator_id: String,
    pub method_id: String,
    pub source_id: String,
    pub action_count: usize,
    pub matched_generated_domain: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub domain_id: String,
    pub snapshot_sha256: String,
    pub generated_action_count: usize,
    pub registry_missing_action_ids: Vec<String>,
    pub model_missing_action_ids: Vec<String>,
    pub registry_extra_action_ids: Vec<String>,
    pub model_extra_action_ids: Vec<String>,
    pub jointly_omitted_action_ids: Vec<String>,
    pub external_challenge_action_ids: Vec<String>,
    pub enumerator_reports: Vec<EnumeratorReport>,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub closed_within_generated_domain_cases: usize,
    pub common_mode_omission_cases: usize,
    pub enumerator_disagreement_cases: usize,
    pub domain_model_inadequate_cases: usize,
    pub stale_snapshot_cases: usize,
    pub enumerator_outside_domain_cases: usize,
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
        .map_err(|error| format!("failed to serialize action ID inventory: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn payload_sha256(action_id: &str) -> String {
    sha256_hex(format!("orbital-domain-action:v0.1:{action_id}").as_bytes())
}

fn set_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn validate_ids(label: &str, ids: &[String]) -> Result<(), String> {
    if ids.len() > MAX_ACTIONS {
        return Err(format!("{label} exceeds the action bound"));
    }
    let mut seen = BTreeSet::new();
    for id in ids {
        if !valid_id(id) || !seen.insert(id) {
            return Err(format!("{label} contains invalid or duplicate IDs"));
        }
    }
    Ok(())
}

fn overlaps(
    domain_start: u64,
    domain_end: u64,
    component_start: u64,
    component_end: u64,
    operation_start: u64,
    operation_end: u64,
    actor_start: u64,
    actor_end: u64,
) -> bool {
    let start = domain_start
        .max(component_start)
        .max(operation_start)
        .max(actor_start);
    let end = domain_end
        .min(component_end)
        .min(operation_end)
        .min(actor_end);
    start < end
}

/// Enumerate the finite grammar by a direct Cartesian traversal and explicit predicates.
/// This is intentionally not implemented using either candidate enumerator's output.
fn generate_domain_actions(domain: &DomainManifest) -> Vec<ActionWitness> {
    let mut actions = Vec::new();
    for component in &domain.components {
        for operation in &domain.operations {
            if !component
                .capability_ids
                .iter()
                .any(|capability| capability == &operation.required_capability_id)
                || component.resource_capacity_milli < operation.resource_required_milli
            {
                continue;
            }
            for actor in &domain.actors {
                if !actor
                    .role_ids
                    .iter()
                    .any(|role| role == &operation.required_role_id)
                    || !overlaps(
                        domain.window_start_ms,
                        domain.window_end_ms,
                        component.active_start_ms,
                        component.active_end_ms,
                        operation.active_start_ms,
                        operation.active_end_ms,
                        actor.active_start_ms,
                        actor.active_end_ms,
                    )
                {
                    continue;
                }
                let action_id = format!(
                    "component:{}|operation:{}|actor:{}",
                    component.component_id, operation.operation_id, actor.actor_id
                );
                actions.push(ActionWitness {
                    payload_sha256: payload_sha256(&action_id),
                    action_id,
                });
            }
        }
    }
    actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    actions
}

fn validate_domain(domain: &DomainManifest) -> Result<(), String> {
    if !valid_id(&domain.domain_id)
        || !valid_id(&domain.grammar_version)
        || !valid_sha256(&domain.snapshot_sha256)
        || !valid_sha256(&domain.current_snapshot_sha256)
        || domain.window_start_ms >= domain.window_end_ms
        || domain.components.is_empty()
        || domain.operations.is_empty()
        || domain.actors.is_empty()
        || domain.components.len() > MAX_ROWS
        || domain.operations.len() > MAX_ROWS
        || domain.actors.len() > MAX_ROWS
    {
        return Err(format!("domain {} has invalid bounded manifest", domain.domain_id));
    }

    let mut component_ids = BTreeSet::new();
    for component in &domain.components {
        if !valid_id(&component.component_id)
            || !component_ids.insert(component.component_id.clone())
            || component.active_start_ms >= component.active_end_ms
        {
            return Err("component IDs and active intervals must be valid and unique".to_owned());
        }
        validate_ids("component capability inventory", &component.capability_ids)?;
    }

    let mut operation_ids = BTreeSet::new();
    for operation in &domain.operations {
        if !valid_id(&operation.operation_id)
            || !valid_id(&operation.required_capability_id)
            || !valid_id(&operation.required_role_id)
            || !operation_ids.insert(operation.operation_id.clone())
            || operation.active_start_ms >= operation.active_end_ms
        {
            return Err("operation IDs, requirements and active intervals must be valid".to_owned());
        }
    }

    let mut actor_ids = BTreeSet::new();
    for actor in &domain.actors {
        if !valid_id(&actor.actor_id)
            || !actor_ids.insert(actor.actor_id.clone())
            || actor.active_start_ms >= actor.active_end_ms
        {
            return Err("actor IDs and active intervals must be valid and unique".to_owned());
        }
        validate_ids("actor role inventory", &actor.role_ids)?;
    }
    Ok(())
}

fn validate_enumerator(enumerator: &Enumerator, domain: &DomainManifest) -> Result<(), String> {
    if !valid_id(&enumerator.enumerator_id)
        || !valid_id(&enumerator.method_id)
        || !valid_id(&enumerator.source_id)
        || !valid_sha256(&enumerator.snapshot_sha256)
        || !valid_id(&enumerator.execution_receipt_id)
    {
        return Err("enumerator receipt metadata is invalid".to_owned());
    }
    validate_ids("enumerator action inventory", &enumerator.action_ids)?;
    let _ = domain;
    Ok(())
}

fn evaluate_case(case: &DomainCase) -> Result<CaseResult, String> {
    if !valid_id(&case.case_id) {
        return Err("domain case ID is invalid".to_owned());
    }
    validate_domain(&case.domain)?;
    validate_enumerator(&case.registry_enumerator, &case.domain)?;
    validate_enumerator(&case.model_enumerator, &case.domain)?;
    if case.external_challenges.len() > MAX_ACTIONS {
        return Err("external challenge inventory exceeds bound".to_owned());
    }

    let generated = generate_domain_actions(&case.domain);
    let generated_ids = generated
        .iter()
        .map(|item| item.action_id.clone())
        .collect::<BTreeSet<_>>();
    let registry_ids = case
        .registry_enumerator
        .action_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let model_ids = case
        .model_enumerator
        .action_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    let registry_missing = set_difference(&generated_ids, &registry_ids);
    let model_missing = set_difference(&generated_ids, &model_ids);
    let registry_extra = set_difference(&registry_ids, &generated_ids);
    let model_extra = set_difference(&model_ids, &generated_ids);
    let jointly_omitted = registry_missing
        .iter()
        .filter(|id| model_missing.contains(id))
        .cloned()
        .collect::<Vec<_>>();

    let mut challenge_ids = BTreeSet::new();
    for challenge in &case.external_challenges {
        if !valid_id(&challenge.action_id)
            || !valid_sha256(&challenge.payload_sha256)
            || !valid_id(&challenge.in_scope_evidence_receipt_id)
            || challenge.payload_sha256 != payload_sha256(&challenge.action_id)
            || !challenge_ids.insert(challenge.action_id.clone())
        {
            return Err("external challenge witness is invalid or duplicated".to_owned());
        }
    }
    let external_challenge_action_ids = challenge_ids
        .difference(&generated_ids)
        .cloned()
        .collect::<Vec<_>>();

    let enumerator_snapshot_matches = case.registry_enumerator.snapshot_sha256
        == case.domain.snapshot_sha256
        && case.model_enumerator.snapshot_sha256 == case.domain.snapshot_sha256;

    let enumerator_reports = vec![
        EnumeratorReport {
            enumerator_id: case.registry_enumerator.enumerator_id.clone(),
            method_id: case.registry_enumerator.method_id.clone(),
            source_id: case.registry_enumerator.source_id.clone(),
            action_count: registry_ids.len(),
            matched_generated_domain: registry_ids == generated_ids,
        },
        EnumeratorReport {
            enumerator_id: case.model_enumerator.enumerator_id.clone(),
            method_id: case.model_enumerator.method_id.clone(),
            source_id: case.model_enumerator.source_id.clone(),
            action_count: model_ids.len(),
            matched_generated_domain: model_ids == generated_ids,
        },
    ];

    let status = if case.domain.snapshot_sha256 != case.domain.current_snapshot_sha256 {
        "DOMAIN_SNAPSHOT_DRIFT"
    } else if !enumerator_snapshot_matches {
        "STALE_ENUMERATION"
    } else if !external_challenge_action_ids.is_empty() {
        "DOMAIN_MODEL_INADEQUATE"
    } else if !registry_extra.is_empty() || !model_extra.is_empty() {
        "ENUMERATOR_OUTSIDE_DOMAIN"
    } else if !jointly_omitted.is_empty() {
        "COMMON_MODE_ACTION_OMISSION"
    } else if !registry_missing.is_empty() || !model_missing.is_empty() {
        "ENUMERATOR_DISAGREEMENT"
    } else {
        "CLOSED_WITHIN_GENERATED_DOMAIN"
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        domain_id: case.domain.domain_id.clone(),
        snapshot_sha256: case.domain.snapshot_sha256.clone(),
        generated_action_count: generated.len(),
        registry_missing_action_ids: registry_missing,
        model_missing_action_ids: model_missing,
        registry_extra_action_ids: registry_extra,
        model_extra_action_ids: model_extra,
        jointly_omitted_action_ids: jointly_omitted,
        external_challenge_action_ids,
        enumerator_reports,
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
        return Err("unsupported open-world domain corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("open-world domain case IDs must be unique".to_owned());
        }
        cases.push(evaluate_case(case)?);
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let count = |status: &str| cases.iter().filter(|case| case.status == status).count();
    let all_closed = cases
        .iter()
        .all(|case| case.status == "CLOSED_WITHIN_GENERATED_DOMAIN");
    Ok(Evaluation {
        result: if all_closed { "pass" } else { "adversarial-controls-detected" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary: Summary {
            case_count: cases.len(),
            closed_within_generated_domain_cases: count("CLOSED_WITHIN_GENERATED_DOMAIN"),
            common_mode_omission_cases: count("COMMON_MODE_ACTION_OMISSION"),
            enumerator_disagreement_cases: count("ENUMERATOR_DISAGREEMENT"),
            domain_model_inadequate_cases: count("DOMAIN_MODEL_INADEQUATE"),
            stale_snapshot_cases: count("DOMAIN_SNAPSHOT_DRIFT") + count("STALE_ENUMERATION"),
            enumerator_outside_domain_cases: count("ENUMERATOR_OUTSIDE_DOMAIN"),
        },
        cases,
        physical_qualification_granted: false,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Corpus {
        serde_json::from_str(include_str!("../fixtures/v0.1/open-world-domain-cases.json"))
            .expect("open-world domain corpus parses")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation
            .cases
            .iter()
            .find(|item| item.case_id == id)
            .expect("fixture control exists")
    }

    #[test]
    fn bounded_domain_closes_only_when_both_enumerators_match() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(case(&result, "bounded_closed").status, "CLOSED_WITHIN_GENERATED_DOMAIN");
        assert_eq!(case(&result, "bounded_closed").generated_action_count, 2);
    }

    #[test]
    fn independent_generator_detects_common_mode_omission() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let item = case(&result, "shared_omission");
        assert_eq!(item.status, "COMMON_MODE_ACTION_OMISSION");
        assert_eq!(
            item.jointly_omitted_action_ids,
            vec!["component:camera-1|operation:calibrate|actor:tech-1".to_owned()]
        );
    }

    #[test]
    fn independent_generator_detects_enumerator_disagreement() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(case(&result, "enumerator_disagreement").status, "ENUMERATOR_DISAGREEMENT");
    }

    #[test]
    fn external_in_scope_witness_falsifies_incomplete_grammar() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let item = case(&result, "grammar_inadequate");
        assert_eq!(item.status, "DOMAIN_MODEL_INADEQUATE");
        assert_eq!(
            item.external_challenge_action_ids,
            vec!["component:camera-1|operation:calibrate|actor:tech-1".to_owned()]
        );
    }

    #[test]
    fn stale_domain_and_enumerator_snapshots_block_closure() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(case(&result, "domain_snapshot_drift").status, "DOMAIN_SNAPSHOT_DRIFT");
        assert_eq!(case(&result, "stale_enumerator").status, "STALE_ENUMERATION");
    }

    #[test]
    fn source_enumerators_cannot_add_actions_outside_the_generated_domain() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(case(&result, "outside_domain_action").status, "ENUMERATOR_OUTSIDE_DOMAIN");
    }

    #[test]
    fn no_case_can_grant_physical_qualification_or_authority() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert!(!result.physical_qualification_granted);
        assert!(!result.authority_granted);
        assert!(
            result
                .cases
                .iter()
                .all(|item| !item.physical_qualification_granted && !item.authority_granted)
        );
    }
}
