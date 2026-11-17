//! E7 source-aware evidence conflict, dependence and bitemporal replay gate.
//!
//! This synthetic-only verifier deliberately does not average contradictory
//! observations or calculate a generic evidence-confidence score. It separates
//! scope compatibility, conflict detection, dependence, explicit independence
//! attestations and as-of admissibility. A clean result is not a safety pass.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::process;

const CORPUS_BYTES: &[u8] = include_bytes!(
    "../../../fixtures/protect-evidence-adjudication-e7-v0.1/adjudication-corpus.json"
);
const SCHEMA_TEXT: &str = include_str!(
    "../../../schemas/protect-evidence-adjudication-e7-v0.1.schema.json"
);
const CLAIM_CEILING: &str = "EVIDENCE_CONFLICT_AND_REPLAY_ONLY";
const RULE_VERSION: &str = "protect-evidence-adjudication-e7/0.1";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct Claim {
    claim_id: String,
    subject_id: String,
    configuration_digest: String,
    quantity: String,
    unit: String,
    method_id: String,
    absolute_tolerance: f64,
    valid_from: String,
    valid_to: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct EvidenceArtifact {
    evidence_id: String,
    source_identity: String,
    source_digest: String,
    state: String,
    recorded_at: String,
    rights_status: String,
    permitted_use: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct Assertion {
    assertion_id: String,
    claim_id: String,
    evidence_id: String,
    subject_id: String,
    configuration_digest: String,
    quantity: String,
    value: f64,
    unit: String,
    method_id: String,
    valid_from: String,
    valid_to: String,
    observed_at: String,
    recorded_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct Dependency {
    dependent_evidence_id: String,
    upstream_evidence_id: String,
    relation: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct IndependenceAttestation {
    attestation_id: String,
    claim_id: String,
    configuration_digest: String,
    quantity: String,
    unit: String,
    method_id: String,
    evidence_ids: Vec<String>,
    status: String,
    basis: String,
    attested_by: String,
    recorded_at: String,
    valid_from: String,
    valid_to: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct LifecycleEvent {
    event_id: String,
    event_type: String,
    evidence_id: String,
    effective_at: String,
    recorded_at: String,
    reason_code: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct Corpus {
    schema_version: String,
    evidence_class: String,
    claim_ceiling: String,
    decision_time_default: String,
    valid_at_default: String,
    claim: Claim,
    evidence_artifacts: Vec<EvidenceArtifact>,
    assertions: Vec<Assertion>,
    dependencies: Vec<Dependency>,
    independence_attestations: Vec<IndependenceAttestation>,
    lifecycle_events: Vec<LifecycleEvent>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SourceReceipt {
    fixture_sha256: String,
    fixture_byte_length: usize,
    evidence_class: &'static str,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ExcludedAssertion {
    assertion_id: String,
    evidence_id: String,
    reason_code: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct IncomparableFinding {
    assertion_id: String,
    evidence_id: String,
    reason_code: String,
    detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ConflictPair {
    left_assertion_id: String,
    right_assertion_id: String,
    left_value: f64,
    right_value: f64,
    absolute_difference: f64,
    unit: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ConflictSet {
    conflict_id: String,
    claim_id: String,
    subject_id: String,
    configuration_digest: String,
    quantity: String,
    unit: String,
    method_id: String,
    valid_at: String,
    assertion_ids: Vec<String>,
    evidence_ids: Vec<String>,
    reason_code: &'static str,
    pairwise_disagreements: Vec<ConflictPair>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct IndependenceRejection {
    attestation_id: String,
    reason_code: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ResolutionAction {
    action_type: &'static str,
    reason: &'static str,
    acceptance_criteria: Vec<&'static str>,
    expected_discriminatory_value: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct AdjudicationReport {
    schema_version: &'static str,
    rule_version: &'static str,
    claim_ceiling: &'static str,
    decision_time: String,
    valid_at: String,
    adjudication_status: &'static str,
    admitted_assertion_ids: Vec<String>,
    excluded_assertions: Vec<ExcludedAssertion>,
    incomparable_assertions: Vec<IncomparableFinding>,
    conflict_sets: Vec<ConflictSet>,
    verified_independence_attestation_ids: Vec<String>,
    verified_independent_evidence_ids: Vec<String>,
    rejected_independence_attestations: Vec<IndependenceRejection>,
    dependence_edge_count: usize,
    unknown_dependence_edge_count: usize,
    resolution_action: Option<ResolutionAction>,
    numeric_aggregate_emitted: bool,
    source_receipt: SourceReceipt,
}

fn parse_corpus() -> Result<Corpus, String> {
    let value: Value = serde_json::from_slice(CORPUS_BYTES)
        .map_err(|error| format!("E7 fixture JSON is invalid: {error}"))?;
    let schema: Value = serde_json::from_str(SCHEMA_TEXT)
        .map_err(|error| format!("E7 fixture schema JSON is invalid: {error}"))?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("E7 fixture schema could not be built: {error}"))?;
    validator
        .validate(&value)
        .map_err(|error| format!("E7 fixture schema validation failed: {error}"))?;
    serde_json::from_value(value).map_err(|error| format!("E7 fixture model is invalid: {error}"))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn canonical_utc(value: &str, path: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err(format!("{path} must be a canonical UTC timestamp (YYYY-MM-DDTHH:MM:SSZ)"));
    }
    Ok(())
}

fn validate_timestamps(corpus: &Corpus) -> Result<(), String> {
    canonical_utc(&corpus.decision_time_default, "decision_time_default")?;
    canonical_utc(&corpus.valid_at_default, "valid_at_default")?;
    canonical_utc(&corpus.claim.valid_from, "claim.valid_from")?;
    canonical_utc(&corpus.claim.valid_to, "claim.valid_to")?;
    for evidence in &corpus.evidence_artifacts {
        canonical_utc(&evidence.recorded_at, &format!("{}.recorded_at", evidence.evidence_id))?;
    }
    for assertion in &corpus.assertions {
        for (name, value) in [
            ("valid_from", &assertion.valid_from),
            ("valid_to", &assertion.valid_to),
            ("observed_at", &assertion.observed_at),
            ("recorded_at", &assertion.recorded_at),
        ] {
            canonical_utc(value, &format!("{}.{}", assertion.assertion_id, name))?;
        }
        if assertion.observed_at > assertion.recorded_at {
            return Err(format!(
                "{} observed_at is later than recorded_at",
                assertion.assertion_id
            ));
        }
    }
    for attestation in &corpus.independence_attestations {
        for (name, value) in [
            ("recorded_at", &attestation.recorded_at),
            ("valid_from", &attestation.valid_from),
            ("valid_to", &attestation.valid_to),
        ] {
            canonical_utc(value, &format!("{}.{}", attestation.attestation_id, name))?;
        }
    }
    for event in &corpus.lifecycle_events {
        canonical_utc(&event.effective_at, &format!("{}.effective_at", event.event_id))?;
        canonical_utc(&event.recorded_at, &format!("{}.recorded_at", event.event_id))?;
    }
    Ok(())
}

fn unique_ids<'a>(
    values: impl Iterator<Item = &'a str>,
    label: &str,
) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for value in values {
        if !ids.insert(value.to_owned()) {
            return Err(format!("duplicate {label} identity: {value}"));
        }
    }
    Ok(ids)
}

fn validate_structure(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_version != RULE_VERSION
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
    {
        return Err("E7 corpus must remain synthetic and conflict/replay-only".to_owned());
    }
    if corpus.claim.absolute_tolerance <= 0.0
        || !corpus.claim.absolute_tolerance.is_finite()
        || corpus.claim.valid_from >= corpus.claim.valid_to
    {
        return Err("E7 claim tolerance or validity interval is invalid".to_owned());
    }
    let evidence_ids = unique_ids(
        corpus.evidence_artifacts.iter().map(|item| item.evidence_id.as_str()),
        "evidence",
    )?;
    unique_ids(
        corpus.assertions.iter().map(|item| item.assertion_id.as_str()),
        "assertion",
    )?;
    unique_ids(
        corpus.independence_attestations.iter().map(|item| item.attestation_id.as_str()),
        "independence attestation",
    )?;
    unique_ids(
        corpus.lifecycle_events.iter().map(|item| item.event_id.as_str()),
        "lifecycle event",
    )?;
    let evidence_by_id: BTreeMap<&str, &EvidenceArtifact> = corpus
        .evidence_artifacts
        .iter()
        .map(|item| (item.evidence_id.as_str(), item))
        .collect();
    for evidence in &corpus.evidence_artifacts {
        if evidence.source_digest.len() != 64
            || !evidence.source_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!("{}.source_digest is not a SHA-256 hex digest", evidence.evidence_id));
        }
        if evidence.rights_status != "SYNTHETIC_OPEN_FIXTURE"
            || evidence.permitted_use != "SYNTHETIC_TEST_ONLY"
        {
            return Err(format!("{}.rights/use state exceeds the synthetic fixture ceiling", evidence.evidence_id));
        }
    }
    for assertion in &corpus.assertions {
        if !evidence_ids.contains(&assertion.evidence_id) {
            return Err(format!(
                "{} references unknown evidence {}",
                assertion.assertion_id, assertion.evidence_id
            ));
        }
        if assertion.valid_from >= assertion.valid_to || !assertion.value.is_finite() {
            return Err(format!("{}.validity/value is invalid", assertion.assertion_id));
        }
    }
    for edge in &corpus.dependencies {
        if edge.dependent_evidence_id == edge.upstream_evidence_id {
            return Err("self-dependency is invalid".to_owned());
        }
        if !evidence_by_id.contains_key(edge.dependent_evidence_id.as_str())
            || !evidence_by_id.contains_key(edge.upstream_evidence_id.as_str())
        {
            return Err(format!(
                "dependency edge references unknown evidence: {} -> {}",
                edge.dependent_evidence_id, edge.upstream_evidence_id
            ));
        }
    }
    for attestation in &corpus.independence_attestations {
        let unique = attestation.evidence_ids.iter().collect::<BTreeSet<_>>();
        if unique.len() != attestation.evidence_ids.len() || attestation.evidence_ids.len() < 2 {
            return Err(format!("{}.evidence_ids must contain at least two unique sources", attestation.attestation_id));
        }
        if attestation.evidence_ids.iter().any(|id| !evidence_ids.contains(id)) {
            return Err(format!("{}.evidence_ids references unknown evidence", attestation.attestation_id));
        }
    }
    for event in &corpus.lifecycle_events {
        if !evidence_ids.contains(&event.evidence_id) {
            return Err(format!(
                "{} references unknown evidence {}",
                event.event_id, event.evidence_id
            ));
        }
        if event.recorded_at < event.effective_at && event.event_type == "REVOKE" {
            // A revocation may be back-dated; preserve both clocks rather than rejecting it.
        }
    }
    Ok(())
}

fn dependency_graph(corpus: &Corpus) -> BTreeMap<String, BTreeSet<String>> {
    let mut graph: BTreeMap<String, BTreeSet<String>> = corpus
        .evidence_artifacts
        .iter()
        .map(|item| (item.evidence_id.clone(), BTreeSet::new()))
        .collect();
    for edge in &corpus.dependencies {
        graph
            .entry(edge.dependent_evidence_id.clone())
            .or_default()
            .insert(edge.upstream_evidence_id.clone());
        graph
            .entry(edge.upstream_evidence_id.clone())
            .or_default()
            .insert(edge.dependent_evidence_id.clone());
    }
    graph
}

fn dependency_path(
    graph: &BTreeMap<String, BTreeSet<String>>,
    start: &str,
    end: &str,
) -> bool {
    let mut pending = VecDeque::from([start.to_owned()]);
    let mut visited = BTreeSet::from([start.to_owned()]);
    while let Some(node) = pending.pop_front() {
        if node == end {
            return true;
        }
        if let Some(neighbours) = graph.get(&node) {
            for neighbour in neighbours {
                if visited.insert(neighbour.clone()) {
                    pending.push_back(neighbour.clone());
                }
            }
        }
    }
    false
}

fn revocation_reason<'a>(
    evidence_id: &str,
    corpus: &'a Corpus,
    decision_time: &str,
    valid_at: &str,
) -> Option<&'a str> {
    corpus
        .lifecycle_events
        .iter()
        .filter(|event| {
            event.evidence_id == evidence_id
                && event.recorded_at.as_str() <= decision_time
                && event.effective_at.as_str() <= valid_at
        })
        .max_by(|left, right| left.recorded_at.cmp(&right.recorded_at))
        .map(|event| match event.event_type.as_str() {
            "REVOKE" => "REVOKED_EFFECTIVE",
            "EXPIRE" => "EXPIRED_EFFECTIVE",
            "SUPERSEDE" => "SUPERSEDED_EFFECTIVE",
            _ => "UNKNOWN_LIFECYCLE_EVENT",
        })
}

fn evidence_admissibility_reason(
    evidence: &EvidenceArtifact,
    corpus: &Corpus,
    decision_time: &str,
    valid_at: &str,
) -> Option<String> {
    if evidence.recorded_at.as_str() > decision_time {
        return Some("EVIDENCE_NOT_KNOWN_AT_DECISION_TIME".to_owned());
    }
    if let Some(reason) = revocation_reason(&evidence.evidence_id, corpus, decision_time, valid_at) {
        return Some(reason.to_owned());
    }
    if evidence.state != "VALID" {
        return Some(format!("EVIDENCE_STATE_{}", evidence.state));
    }
    None
}

fn valid_interval_contains(start: &str, end: &str, instant: &str) -> bool {
    start <= instant && instant < end
}

fn comparable_scope_reason(assertion: &Assertion, claim: &Claim) -> Option<(&'static str, String)> {
    if assertion.subject_id != claim.subject_id {
        return Some(("SUBJECT_MISMATCH", "assertion subject differs from the claim subject".to_owned()));
    }
    if assertion.configuration_digest != claim.configuration_digest {
        return Some(("CONFIGURATION_MISMATCH", "assertion configuration digest differs from the claim configuration".to_owned()));
    }
    if assertion.quantity != claim.quantity {
        return Some(("QUANTITY_MISMATCH", "assertion quantity differs from the claim quantity".to_owned()));
    }
    if assertion.unit != claim.unit {
        return Some(("UNIT_MISMATCH", "unit conversion is not declared; source units are not coerced".to_owned()));
    }
    if assertion.method_id != claim.method_id {
        return Some(("METHOD_MISMATCH", "measurement method differs from the claim method".to_owned()));
    }
    None
}

fn verified_independence(
    corpus: &Corpus,
    graph: &BTreeMap<String, BTreeSet<String>>,
    decision_time: &str,
    valid_at: &str,
) -> (
    Vec<String>,
    Vec<String>,
    Vec<IndependenceRejection>,
) {
    let evidence_by_id: BTreeMap<&str, &EvidenceArtifact> = corpus
        .evidence_artifacts
        .iter()
        .map(|item| (item.evidence_id.as_str(), item))
        .collect();
    let mut accepted_attestations = Vec::new();
    let mut independent_evidence = BTreeSet::new();
    let mut rejected = Vec::new();

    for attestation in &corpus.independence_attestations {
        let mut rejection = None;
        if attestation.status != "VERIFIED_INDEPENDENT" {
            rejection = Some("ATTESTATION_NOT_VERIFIED");
        } else if attestation.recorded_at.as_str() > decision_time {
            rejection = Some("ATTESTATION_NOT_KNOWN_AS_OF_DECISION_TIME");
        } else if attestation.claim_id != corpus.claim.claim_id
            || attestation.configuration_digest != corpus.claim.configuration_digest
            || attestation.quantity != corpus.claim.quantity
            || attestation.unit != corpus.claim.unit
            || attestation.method_id != corpus.claim.method_id
        {
            rejection = Some("ATTESTATION_SCOPE_MISMATCH");
        } else if !valid_interval_contains(
            &attestation.valid_from,
            &attestation.valid_to,
            valid_at,
        ) {
            rejection = Some("ATTESTATION_OUTSIDE_VALID_TIME");
        } else {
            for evidence_id in &attestation.evidence_ids {
                let evidence = evidence_by_id[evidence_id.as_str()];
                if let Some(reason) =
                    evidence_admissibility_reason(evidence, corpus, decision_time, valid_at)
                {
                    rejection = Some(match reason.as_str() {
                        "EVIDENCE_NOT_KNOWN_AT_DECISION_TIME" => "ATTESTED_EVIDENCE_NOT_KNOWN",
                        "REVOKED_EFFECTIVE" => "ATTESTED_EVIDENCE_REVOKED",
                        _ => "ATTESTED_EVIDENCE_NOT_ADMISSIBLE",
                    });
                    break;
                }
            }
        }
        if rejection.is_none() {
            'pairs: for (index, left) in attestation.evidence_ids.iter().enumerate() {
                for right in attestation.evidence_ids.iter().skip(index + 1) {
                    if dependency_path(graph, left, right) {
                        rejection = Some("DEPENDENCY_PATH_PREVENTS_INDEPENDENCE");
                        break 'pairs;
                    }
                }
            }
        }
        if let Some(reason) = rejection {
            rejected.push(IndependenceRejection {
                attestation_id: attestation.attestation_id.clone(),
                reason_code: reason.to_owned(),
            });
        } else {
            accepted_attestations.push(attestation.attestation_id.clone());
            independent_evidence.extend(attestation.evidence_ids.iter().cloned());
        }
    }
    accepted_attestations.sort();
    rejected.sort_by(|left, right| left.attestation_id.cmp(&right.attestation_id));
    (
        accepted_attestations,
        independent_evidence.into_iter().collect(),
        rejected,
    )
}

fn evaluate(
    corpus: &Corpus,
    decision_time: &str,
    valid_at: &str,
) -> Result<AdjudicationReport, String> {
    canonical_utc(decision_time, "decision_time")?;
    canonical_utc(valid_at, "valid_at")?;
    if decision_time < corpus.decision_time_default.as_str() && decision_time.is_empty() {
        return Err("decision_time is invalid".to_owned());
    }
    let evidence_by_id: BTreeMap<&str, &EvidenceArtifact> = corpus
        .evidence_artifacts
        .iter()
        .map(|item| (item.evidence_id.as_str(), item))
        .collect();
    let graph = dependency_graph(corpus);
    let mut admitted = Vec::new();
    let mut excluded = Vec::new();
    let mut incomparable = Vec::new();
    let mut comparable: Vec<&Assertion> = Vec::new();

    for assertion in corpus
        .assertions
        .iter()
        .filter(|item| item.claim_id == corpus.claim.claim_id)
    {
        if assertion.recorded_at.as_str() > decision_time {
            excluded.push(ExcludedAssertion {
                assertion_id: assertion.assertion_id.clone(),
                evidence_id: assertion.evidence_id.clone(),
                reason_code: "ASSERTION_NOT_KNOWN_AS_OF_DECISION_TIME".to_owned(),
            });
            continue;
        }
        let evidence = evidence_by_id[assertion.evidence_id.as_str()];
        if let Some(reason) = evidence_admissibility_reason(evidence, corpus, decision_time, valid_at) {
            excluded.push(ExcludedAssertion {
                assertion_id: assertion.assertion_id.clone(),
                evidence_id: assertion.evidence_id.clone(),
                reason_code: reason,
            });
            continue;
        }
        if !valid_interval_contains(&assertion.valid_from, &assertion.valid_to, valid_at) {
            incomparable.push(IncomparableFinding {
                assertion_id: assertion.assertion_id.clone(),
                evidence_id: assertion.evidence_id.clone(),
                reason_code: "OUTSIDE_VALID_TIME".to_owned(),
                detail: "assertion valid-time interval does not contain the requested valid_at instant".to_owned(),
            });
            continue;
        }
        if let Some((reason, detail)) = comparable_scope_reason(assertion, &corpus.claim) {
            incomparable.push(IncomparableFinding {
                assertion_id: assertion.assertion_id.clone(),
                evidence_id: assertion.evidence_id.clone(),
                reason_code: reason.to_owned(),
                detail,
            });
            continue;
        }
        admitted.push(assertion.assertion_id.clone());
        comparable.push(assertion);
    }

    admitted.sort();
    excluded.sort_by(|left, right| left.assertion_id.cmp(&right.assertion_id));
    incomparable.sort_by(|left, right| left.assertion_id.cmp(&right.assertion_id));

    let mut pairwise = Vec::new();
    let mut conflicted_assertions = BTreeSet::new();
    let mut conflicted_evidence = BTreeSet::new();
    for (index, left) in comparable.iter().enumerate() {
        for right in comparable.iter().skip(index + 1) {
            let difference = (left.value - right.value).abs();
            if difference > corpus.claim.absolute_tolerance {
                pairwise.push(ConflictPair {
                    left_assertion_id: left.assertion_id.clone(),
                    right_assertion_id: right.assertion_id.clone(),
                    left_value: left.value,
                    right_value: right.value,
                    absolute_difference: difference,
                    unit: corpus.claim.unit.clone(),
                });
                conflicted_assertions.insert(left.assertion_id.clone());
                conflicted_assertions.insert(right.assertion_id.clone());
                conflicted_evidence.insert(left.evidence_id.clone());
                conflicted_evidence.insert(right.evidence_id.clone());
            }
        }
    }
    pairwise.sort_by(|left, right| {
        (&left.left_assertion_id, &left.right_assertion_id)
            .cmp(&(&right.left_assertion_id, &right.right_assertion_id))
    });
    let conflict_sets = if pairwise.is_empty() {
        Vec::new()
    } else {
        vec![ConflictSet {
            conflict_id: format!("conflict:{}:{}", corpus.claim.claim_id, valid_at),
            claim_id: corpus.claim.claim_id.clone(),
            subject_id: corpus.claim.subject_id.clone(),
            configuration_digest: corpus.claim.configuration_digest.clone(),
            quantity: corpus.claim.quantity.clone(),
            unit: corpus.claim.unit.clone(),
            method_id: corpus.claim.method_id.clone(),
            valid_at: valid_at.to_owned(),
            assertion_ids: conflicted_assertions.into_iter().collect(),
            evidence_ids: conflicted_evidence.into_iter().collect(),
            reason_code: "VALUE_DISAGREEMENT",
            pairwise_disagreements: pairwise,
        }]
    };

    let (verified_attestations, verified_independent_evidence, rejected_attestations) =
        verified_independence(corpus, &graph, decision_time, valid_at);
    let status = if !conflict_sets.is_empty() {
        "UNRESOLVED_CONFLICT"
    } else if comparable.is_empty() {
        "ABSTAIN_NO_ADMISSIBLE_COMPARABLE_ASSERTIONS"
    } else if !incomparable.is_empty() {
        "ABSTAIN_INCOMPARABLE_EVIDENCE_PRESENT"
    } else {
        "NO_CONFLICT_DETECTED_NOT_QUALIFIED"
    };
    let resolution_action = if !conflict_sets.is_empty() {
        Some(ResolutionAction {
            action_type: "INDEPENDENT_REPEAT_MEASUREMENT",
            reason: "compatible same-scope observations disagree beyond the declared absolute tolerance",
            acceptance_criteria: vec![
                "same asset and exact configuration digest",
                "same quantity, unit and named measurement method",
                "valid-time overlap with the decision interval",
                "source and calibration independence explicitly evidenced",
                "original measurement and calibration artefacts retained",
                "no unresolved dependence path to the conflicting source set",
            ],
            expected_discriminatory_value: "a new independent measurement can distinguish whether the disagreement persists without averaging existing assertions",
        })
    } else {
        None
    };
    Ok(AdjudicationReport {
        schema_version: RULE_VERSION,
        rule_version: RULE_VERSION,
        claim_ceiling: CLAIM_CEILING,
        decision_time: decision_time.to_owned(),
        valid_at: valid_at.to_owned(),
        adjudication_status: status,
        admitted_assertion_ids: admitted,
        excluded_assertions: excluded,
        incomparable_assertions: incomparable,
        conflict_sets,
        verified_independence_attestation_ids: verified_attestations,
        verified_independent_evidence_ids: verified_independent_evidence,
        rejected_independence_attestations: rejected_attestations,
        dependence_edge_count: corpus.dependencies.len(),
        unknown_dependence_edge_count: corpus
            .dependencies
            .iter()
            .filter(|edge| edge.relation == "UNKNOWN_DEPENDENCE")
            .count(),
        resolution_action,
        numeric_aggregate_emitted: false,
        source_receipt: SourceReceipt {
            fixture_sha256: digest(CORPUS_BYTES),
            fixture_byte_length: CORPUS_BYTES.len(),
            evidence_class: "synthetic",
            claim_ceiling: CLAIM_CEILING,
        },
    })
}

fn default_report() -> Result<AdjudicationReport, String> {
    let corpus = parse_corpus()?;
    validate_timestamps(&corpus)?;
    validate_structure(&corpus)?;
    evaluate(
        &corpus,
        &corpus.decision_time_default,
        &corpus.valid_at_default,
    )
}

fn main() {
    match default_report() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("E7 report serialization")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        let value: Value = serde_json::from_slice(CORPUS_BYTES).expect("fixture JSON");
        serde_json::from_value(value).expect("fixture model")
    }

    fn default_eval(data: &Corpus) -> AdjudicationReport {
        evaluate(
            data,
            &data.decision_time_default,
            &data.valid_at_default,
        )
        .expect("E7 evaluation")
    }

    #[test]
    fn valid_corpus_returns_unresolved_conflict_and_never_averages() {
        let report = default_report().expect("valid E7 corpus");
        assert_eq!(report.adjudication_status, "UNRESOLVED_CONFLICT");
        assert_eq!(report.conflict_sets.len(), 1);
        assert!(!report.numeric_aggregate_emitted);
        assert!(report.resolution_action.is_some());
    }

    #[test]
    fn copied_vendor_and_shared_calibration_reports_do_not_count_as_independent() {
        let report = default_report().expect("report");
        assert_eq!(report.verified_independence_attestation_ids, vec!["ind-flow-001"]);
        assert_eq!(
            report.verified_independent_evidence_ids,
            vec!["ev-independent-e", "ev-sensor-a"]
        );
        assert!(report.verified_independent_evidence_ids.contains(&"ev-sensor-a".to_owned()));
        assert!(!report.verified_independent_evidence_ids.contains(&"ev-copy-a".to_owned()));
        assert!(!report.verified_independent_evidence_ids.contains(&"ev-vendor-b".to_owned()));
        assert!(!report.verified_independent_evidence_ids.contains(&"ev-calibration-c".to_owned()));
    }

    #[test]
    fn unknown_dependence_does_not_count_as_proven_independence() {
        let mut data = corpus();
        let graph = dependency_graph(&data);
        assert!(dependency_path(&graph, "ev-unknown-d", "ev-sensor-a"));
        let (_, independent, _) = verified_independence(
            &data,
            &graph,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        );
        assert!(!independent.contains(&"ev-unknown-d".to_owned()));
    }

    #[test]
    fn a_dependency_path_invalidates_an_independence_attestation() {
        let mut data = corpus();
        data.dependencies.push(Dependency {
            dependent_evidence_id: "ev-independent-e".to_owned(),
            upstream_evidence_id: "ev-sensor-a".to_owned(),
            relation: "UNKNOWN_DEPENDENCE".to_owned(),
        });
        let graph = dependency_graph(&data);
        let (accepted, independent, rejected) = verified_independence(
            &data,
            &graph,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        );
        assert!(accepted.is_empty());
        assert!(independent.is_empty());
        assert_eq!(
            rejected[0].reason_code,
            "DEPENDENCY_PATH_PREVENTS_INDEPENDENCE"
        );
    }

    #[test]
    fn independent_attestation_is_scope_and_time_bound() {
        let mut data = corpus();
        data.independence_attestations[0].configuration_digest = "sha256:config-B".to_owned();
        let (accepted, _, rejected) = verified_independence(
            &data,
            &dependency_graph(&data),
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        );
        assert!(accepted.is_empty());
        assert_eq!(rejected[0].reason_code, "ATTESTATION_SCOPE_MISMATCH");
    }

    #[test]
    fn configuration_mismatch_is_not_compared_as_the_same_claim() {
        let report = default_report().expect("report");
        let finding = report
            .incomparable_assertions
            .iter()
            .find(|item| item.assertion_id == "as-config-b")
            .expect("configuration finding");
        assert_eq!(finding.reason_code, "CONFIGURATION_MISMATCH");
        assert!(!report.conflict_sets[0].assertion_ids.contains(&"as-config-b".to_owned()));
    }

    #[test]
    fn unit_mismatch_is_not_silently_converted() {
        let report = default_report().expect("report");
        let finding = report
            .incomparable_assertions
            .iter()
            .find(|item| item.assertion_id == "as-unit-i")
            .expect("unit finding");
        assert_eq!(finding.reason_code, "UNIT_MISMATCH");
    }

    #[test]
    fn method_mismatch_is_not_silently_compared() {
        let report = default_report().expect("report");
        let finding = report
            .incomparable_assertions
            .iter()
            .find(|item| item.assertion_id == "as-method-j")
            .expect("method finding");
        assert_eq!(finding.reason_code, "METHOD_MISMATCH");
    }

    #[test]
    fn conflicting_same_scope_values_are_retained_as_pairs() {
        let report = default_report().expect("report");
        let conflict = &report.conflict_sets[0];
        assert!(conflict
            .pairwise_disagreements
            .iter()
            .any(|pair| pair.left_assertion_id == "as-sensor-a"
                && pair.right_assertion_id == "as-independent-e"));
        assert_eq!(conflict.reason_code, "VALUE_DISAGREEMENT");
    }

    #[test]
    fn no_conflict_resolution_by_majority_or_average_is_emitted() {
        let report = default_report().expect("report");
        assert!(!report.numeric_aggregate_emitted);
        assert!(report.resolution_action.is_some());
        assert_eq!(report.adjudication_status, "UNRESOLVED_CONFLICT");
    }

    #[test]
    fn late_arriving_assertion_is_absent_from_earlier_as_of_view() {
        let report = default_report().expect("report");
        assert!(!report.admitted_assertion_ids.contains(&"as-late-f".to_owned()));
        assert!(report.excluded_assertions.iter().any(|item| {
            item.assertion_id == "as-late-f"
                && item.reason_code == "ASSERTION_NOT_KNOWN_AS_OF_DECISION_TIME"
        }));
    }

    #[test]
    fn late_arriving_assertion_changes_current_view_but_not_historical_replay() {
        let data = corpus();
        let historical_a = evaluate(
            &data,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("historical A");
        let historical_b = evaluate(
            &data,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("historical replay");
        let current = evaluate(
            &data,
            "2026-11-16T17:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("current view");
        assert_eq!(historical_a, historical_b);
        assert!(!historical_a.admitted_assertion_ids.contains(&"as-late-f".to_owned()));
        assert!(current.admitted_assertion_ids.contains(&"as-late-f".to_owned()));
        assert_ne!(
            historical_a.conflict_sets[0].assertion_ids,
            current.conflict_sets[0].assertion_ids
        );
    }

    #[test]
    fn backdated_revocation_is_applied_only_after_it_is_recorded() {
        let data = corpus();
        let before = evaluate(
            &data,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("pre-revocation knowledge");
        let after = evaluate(
            &data,
            "2026-11-16T17:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("post-revocation knowledge");
        assert!(before.admitted_assertion_ids.contains(&"as-revocable-g".to_owned()));
        assert!(after.excluded_assertions.iter().any(|item| {
            item.assertion_id == "as-revocable-g" && item.reason_code == "REVOKED_EFFECTIVE"
        }));
    }

    #[test]
    fn unknown_evidence_state_is_not_admitted() {
        let report = default_report().expect("report");
        assert!(report.excluded_assertions.iter().any(|item| {
            item.assertion_id == "as-invalid-k" && item.reason_code == "EVIDENCE_STATE_UNKNOWN"
        }));
    }

    #[test]
    fn dependency_cycle_is_not_misread_as_independence() {
        let mut data = corpus();
        data.dependencies.push(Dependency {
            dependent_evidence_id: "ev-sensor-a".to_owned(),
            upstream_evidence_id: "ev-copy-a".to_owned(),
            relation: "SAME_UPSTREAM_OBSERVATION".to_owned(),
        });
        let graph = dependency_graph(&data);
        assert!(dependency_path(&graph, "ev-sensor-a", "ev-copy-a"));
        let (accepted, _, _) = verified_independence(
            &data,
            &graph,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        );
        assert_eq!(accepted, vec!["ind-flow-001"]);
    }

    #[test]
    fn revocation_recorded_after_decision_cutoff_does_not_rewrite_history() {
        let mut data = corpus();
        data.lifecycle_events[0].recorded_at = "2026-11-16T18:00:00Z".to_owned();
        let report = evaluate(
            &data,
            "2026-11-16T17:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("revocation not yet recorded");
        assert!(report.admitted_assertion_ids.contains(&"as-revocable-g".to_owned()));
    }

    #[test]
    fn noncanonical_timezone_timestamp_is_rejected() {
        let mut data = corpus();
        data.assertions[0].recorded_at = "2026-11-16T12:20:00+02:00".to_owned();
        assert!(validate_timestamps(&data).is_err());
    }

    #[test]
    fn duplicate_evidence_identity_is_rejected() {
        let mut data = corpus();
        data.evidence_artifacts.push(data.evidence_artifacts[0].clone());
        assert!(validate_structure(&data).is_err());
    }

    #[test]
    fn dependency_to_unknown_evidence_is_rejected() {
        let mut data = corpus();
        data.dependencies.push(Dependency {
            dependent_evidence_id: "ev-sensor-a".to_owned(),
            upstream_evidence_id: "ev-does-not-exist".to_owned(),
            relation: "UNKNOWN_DEPENDENCE".to_owned(),
        });
        assert!(validate_structure(&data).is_err());
    }

    #[test]
    fn dependency_edges_are_not_treated_as_directional_independence_evidence() {
        let mut data = corpus();
        let graph = dependency_graph(&data);
        assert!(dependency_path(&graph, "ev-copy-a", "ev-sensor-a"));
        assert!(dependency_path(&graph, "ev-sensor-a", "ev-copy-a"));
    }

    #[test]
    fn incompatible_scope_assertions_are_retained_as_findings() {
        let report = default_report().expect("report");
        let reasons = report
            .incomparable_assertions
            .iter()
            .map(|item| item.reason_code.as_str())
            .collect::<BTreeSet<_>>();
        assert!(reasons.contains("CONFIGURATION_MISMATCH"));
        assert!(reasons.contains("UNIT_MISMATCH"));
        assert!(reasons.contains("METHOD_MISMATCH"));
    }

    #[test]
    fn evidence_is_never_admitted_after_its_source_recording_time() {
        let mut data = corpus();
        data.evidence_artifacts
            .iter_mut()
            .find(|item| item.evidence_id == "ev-independent-e")
            .expect("independent evidence")
            .recorded_at = "2026-11-16T12:10:00Z".to_owned();
        let report = evaluate(
            &data,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("report");
        assert!(report.excluded_assertions.iter().any(|item| {
            item.assertion_id == "as-independent-e"
                && item.reason_code == "EVIDENCE_NOT_KNOWN_AT_DECISION_TIME"
        }));
    }

    #[test]
    fn attestation_recorded_after_cutoff_cannot_prove_independence() {
        let mut data = corpus();
        data.independence_attestations[0].recorded_at = "2026-11-16T12:30:00Z".to_owned();
        let (accepted, independent, rejected) = verified_independence(
            &data,
            &dependency_graph(&data),
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        );
        assert!(accepted.is_empty());
        assert!(independent.is_empty());
        assert_eq!(
            rejected[0].reason_code,
            "ATTESTATION_NOT_KNOWN_AS_OF_DECISION_TIME"
        );
    }

    #[test]
    fn no_admissible_assertions_returns_abstention_not_clearance() {
        let mut data = corpus();
        for item in &mut data.evidence_artifacts {
            item.state = "UNKNOWN".to_owned();
        }
        let report = evaluate(
            &data,
            "2026-11-16T17:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("report");
        assert_eq!(
            report.adjudication_status,
            "ABSTAIN_NO_ADMISSIBLE_COMPARABLE_ASSERTIONS"
        );
        assert!(report.conflict_sets.is_empty());
    }

    #[test]
    fn report_is_deterministic_for_identical_frozen_inputs() {
        let data = corpus();
        let first = evaluate(
            &data,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("first");
        let second = evaluate(
            &data,
            "2026-11-16T12:00:00Z",
            "2026-11-16T11:30:00Z",
        )
        .expect("second");
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
    }

    #[test]
    fn verified_independence_is_not_a_conflict_resolution() {
        let report = default_report().expect("report");
        assert_eq!(report.verified_independent_evidence_ids.len(), 2);
        assert_eq!(report.adjudication_status, "UNRESOLVED_CONFLICT");
        assert!(report.resolution_action.is_some());
    }
}
