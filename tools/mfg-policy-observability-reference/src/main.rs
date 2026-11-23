use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::process::ExitCode;

const CORPUS_JSON: &str =
    include_str!("../../../fixtures/manufacturing/mfg_policy_observability_cases_v0_1.json");
const ANSWERS_JSON: &str = include_str!(
    "../../../fixtures/manufacturing/mfg_policy_observability_known_answers_v0_1.json"
);
const CORPUS_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_observability_cases_v0_1.schema.json");
const ANSWERS_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_policy_observability_known_answers_v0_1.schema.json"
);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    case_count: usize,
    cases: Vec<TimelineCase>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineCase {
    case_id: String,
    scenario_family_id: String,
    events: Vec<SourceEvent>,
    decision_cuts: Vec<DecisionCut>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SourceEvent {
    event_id: String,
    source_id: String,
    source_owner: String,
    sequence: u64,
    subject_id: String,
    event_type: String,
    effective_min_ms: u64,
    effective_max_ms: u64,
    observed_at_ms: u64,
    disposition: EventDisposition,
    supersedes_event_id: Option<String>,
    evidence_sha256: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum EventDisposition {
    Admitted,
    Reconciled,
    Unknown,
    Conflicted,
}

impl EventDisposition {
    fn is_admissible(self) -> bool {
        matches!(self, Self::Admitted | Self::Reconciled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DecisionCut {
    decision_cut_id: String,
    cutoff_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswerFile {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    answer_count: usize,
    answers: Vec<KnownAnswer>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswer {
    case_id: String,
    projections: Vec<ProjectionAnswer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ProjectionAnswer {
    decision_cut_id: String,
    cutoff_ms: u64,
    visible_event_ids: Vec<String>,
    active_event_ids: Vec<String>,
    unresolved_event_ids: Vec<String>,
    superseded_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Projection {
    decision_cut_id: String,
    cutoff_ms: u64,
    visible_event_ids: Vec<String>,
    active_event_ids: Vec<String>,
    unresolved_event_ids: Vec<String>,
    superseded_event_ids: Vec<String>,
    history_sha256: String,
}

#[derive(Debug, Serialize)]
struct CaseReport {
    case_id: String,
    scenario_family_id: String,
    projections: Vec<Projection>,
}

#[derive(Debug, Serialize)]
struct Report {
    benchmark_id: &'static str,
    evaluator_version: &'static str,
    evidence_class: &'static str,
    case_count: usize,
    decision_cut_count: usize,
    visible_event_receipt_count: usize,
    sha256_history_digests_emitted: bool,
    temporal_cutoff_noninterference_verified: bool,
    source_events_created_or_reconciled: bool,
    machine_commands_issued: bool,
    physical_state_claimed: bool,
    cases: Vec<CaseReport>,
    claim_ceiling: &'static str,
}

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn compile_validator(schema_source: &str) -> Result<jsonschema::Validator, String> {
    let schema: serde_json::Value = serde_json::from_str(schema_source)
        .map_err(|error| fail(format!("schema_parse:{error}")))?;
    jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| fail(format!("schema_compile:{error}")))
}

fn validate_schema(value: &serde_json::Value, schema_source: &str) -> Result<(), String> {
    let validator = compile_validator(schema_source)?;
    validator
        .validate(value)
        .map_err(|error| fail(format!("schema_validation:{error}")))
}

fn normalize_events(events: &[SourceEvent]) -> Result<Vec<SourceEvent>, String> {
    let mut by_id: BTreeMap<&str, &SourceEvent> = BTreeMap::new();
    for event in events {
        if let Some(previous) = by_id.insert(&event.event_id, event)
            && previous != event
        {
            return Err(fail("event_id_reused_with_different_payload"));
        }
        if event.event_id.trim().is_empty()
            || event.source_id.trim().is_empty()
            || event.source_owner.trim().is_empty()
            || event.subject_id.trim().is_empty()
            || event.event_type.trim().is_empty()
            || event.effective_min_ms > event.effective_max_ms
            || event.evidence_sha256.len() != 64
            || !event
                .evidence_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(fail("event_identity_scope_time_or_digest_invalid"));
        }
    }

    let mut unique: Vec<SourceEvent> = by_id.values().map(|event| (*event).clone()).collect();
    unique.sort_by(|left, right| {
        left.observed_at_ms
            .cmp(&right.observed_at_ms)
            .then(left.source_id.cmp(&right.source_id))
            .then(left.sequence.cmp(&right.sequence))
            .then(left.event_id.cmp(&right.event_id))
    });

    let mut source_sequences = HashSet::new();
    for event in &unique {
        if !source_sequences.insert((event.source_id.as_str(), event.sequence)) {
            return Err(fail("source_sequence_reused_by_different_event"));
        }
    }

    let by_id: HashMap<&str, &SourceEvent> = unique
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();

    for event in &unique {
        if let Some(superseded_id) = &event.supersedes_event_id {
            let previous = by_id
                .get(superseded_id.as_str())
                .ok_or_else(|| fail("supersession_target_missing"))?;
            if previous.subject_id != event.subject_id
                || previous.event_id == event.event_id
                || previous.observed_at_ms > event.observed_at_ms
            {
                return Err(fail("supersession_scope_or_order_invalid"));
            }
        }
    }

    for event in &unique {
        let mut seen = HashSet::new();
        let mut cursor = event.supersedes_event_id.as_deref();
        while let Some(id) = cursor {
            if !seen.insert(id) || id == event.event_id {
                return Err(fail("supersession_cycle_detected"));
            }
            cursor = by_id
                .get(id)
                .and_then(|target| target.supersedes_event_id.as_deref());
        }
    }

    Ok(unique)
}

fn validate_case(case: &TimelineCase) -> Result<Vec<SourceEvent>, String> {
    if case.case_id.trim().is_empty()
        || case.scenario_family_id.trim().is_empty()
        || case.events.is_empty()
        || case.decision_cuts.is_empty()
    {
        return Err(fail("timeline_case_manifest_invalid"));
    }

    let events = normalize_events(&case.events)?;
    let mut cut_ids = HashSet::new();
    let mut previous_cutoff = None;
    for cut in &case.decision_cuts {
        if cut.decision_cut_id.trim().is_empty()
            || !cut_ids.insert(cut.decision_cut_id.as_str())
            || previous_cutoff.is_some_and(|previous| cut.cutoff_ms < previous)
        {
            return Err(fail("decision_cut_identity_or_order_invalid"));
        }
        previous_cutoff = Some(cut.cutoff_ms);
    }
    Ok(events)
}

fn sha256_history(cut: &DecisionCut, visible: &[SourceEvent]) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update((cut.decision_cut_id.len() as u64).to_be_bytes());
    hasher.update(cut.decision_cut_id.as_bytes());
    hasher.update(cut.cutoff_ms.to_be_bytes());

    for event in visible {
        let encoded = serde_json::to_vec(event)
            .map_err(|error| fail(format!("event_digest_serialization_failed:{error}")))?;
        hasher.update((encoded.len() as u64).to_be_bytes());
        hasher.update(encoded);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn project(events: &[SourceEvent], cut: &DecisionCut) -> Result<Projection, String> {
    let normalized = normalize_events(events)?;
    let visible: Vec<SourceEvent> = normalized
        .into_iter()
        .filter(|event| event.observed_at_ms <= cut.cutoff_ms)
        .collect();

    let visible_by_id: HashMap<&str, &SourceEvent> = visible
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();

    // A correction supersedes a visible prior receipt only once the correction
    // itself is visible, owner-admitted/reconciled, and definitely effective
    // no later than the decision cutoff.
    let superseded: BTreeSet<String> = visible
        .iter()
        .filter(|event| {
            event.disposition.is_admissible() && event.effective_max_ms <= cut.cutoff_ms
        })
        .filter_map(|event| event.supersedes_event_id.as_ref())
        .filter(|target| visible_by_id.contains_key(target.as_str()))
        .cloned()
        .collect();

    let mut active_event_ids: Vec<String> = visible
        .iter()
        .filter(|event| event.disposition.is_admissible())
        .filter(|event| event.effective_max_ms <= cut.cutoff_ms)
        .filter(|event| !superseded.contains(&event.event_id))
        .map(|event| event.event_id.clone())
        .collect();
    active_event_ids.sort();

    let mut unresolved_event_ids: Vec<String> = visible
        .iter()
        .filter(|event| {
            !event.disposition.is_admissible() || event.effective_max_ms > cut.cutoff_ms
        })
        .filter(|event| !superseded.contains(&event.event_id))
        .map(|event| event.event_id.clone())
        .collect();
    unresolved_event_ids.sort();

    let mut visible_event_ids: Vec<String> =
        visible.iter().map(|event| event.event_id.clone()).collect();
    visible_event_ids.sort_by(|left, right| {
        let left_event = visible_by_id[left.as_str()];
        let right_event = visible_by_id[right.as_str()];
        left_event
            .observed_at_ms
            .cmp(&right_event.observed_at_ms)
            .then(left_event.source_id.cmp(&right_event.source_id))
            .then(left_event.sequence.cmp(&right_event.sequence))
            .then(left.cmp(right))
    });

    let history_sha256 = sha256_history(cut, &visible)?;
    Ok(Projection {
        decision_cut_id: cut.decision_cut_id.clone(),
        cutoff_ms: cut.cutoff_ms,
        visible_event_ids,
        active_event_ids,
        unresolved_event_ids,
        superseded_event_ids: superseded.into_iter().collect(),
        history_sha256,
    })
}

fn project_case(case: &TimelineCase) -> Result<Vec<Projection>, String> {
    let events = validate_case(case)?;
    case.decision_cuts
        .iter()
        .map(|cut| project(&events, cut))
        .collect()
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_id != "https://luminousdynamics.io/schemas/mfg-policy-observability-cases-v0.1"
        || corpus.benchmark_id != "MFG-POLICY-OBSERVABILITY-001"
        || corpus.evidence_class != "synthetic"
        || corpus.case_count != 20
        || corpus.cases.len() != 20
        || corpus.claim_ceiling.trim().is_empty()
    {
        return Err(fail("observability_corpus_manifest_invalid"));
    }

    let mut case_ids = HashSet::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err(fail("duplicate_timeline_case_id"));
        }
        validate_case(case)?;
    }
    Ok(())
}

fn compare_known_answers(corpus: &Corpus, answers: &KnownAnswerFile) -> Result<(), String> {
    if answers.schema_id
        != "https://luminousdynamics.io/schemas/mfg-policy-observability-answers-v0.1"
        || answers.benchmark_id != "MFG-POLICY-OBSERVABILITY-001"
        || answers.evidence_class != "synthetic"
        || answers.answer_count != corpus.cases.len()
        || answers.answers.len() != corpus.cases.len()
        || answers.claim_ceiling.trim().is_empty()
    {
        return Err(fail("observability_known_answer_manifest_invalid"));
    }

    let expected: HashMap<&str, &KnownAnswer> = answers
        .answers
        .iter()
        .map(|answer| (answer.case_id.as_str(), answer))
        .collect();
    if expected.len() != answers.answers.len() {
        return Err(fail("duplicate_known_answer_case"));
    }

    for case in &corpus.cases {
        let answer = expected
            .get(case.case_id.as_str())
            .ok_or_else(|| fail("known_answer_missing_case"))?;
        let actual = project_case(case)?;
        let expected_projections = &answer.projections;
        if actual.len() != expected_projections.len() {
            return Err(fail(format!(
                "known_projection_count_mismatch:{}",
                case.case_id
            )));
        }
        for (actual_projection, expected_projection) in
            actual.iter().zip(expected_projections.iter())
        {
            let comparable = ProjectionAnswer {
                decision_cut_id: actual_projection.decision_cut_id.clone(),
                cutoff_ms: actual_projection.cutoff_ms,
                visible_event_ids: actual_projection.visible_event_ids.clone(),
                active_event_ids: actual_projection.active_event_ids.clone(),
                unresolved_event_ids: actual_projection.unresolved_event_ids.clone(),
                superseded_event_ids: actual_projection.superseded_event_ids.clone(),
            };
            if &comparable != expected_projection {
                return Err(fail(format!(
                    "known_projection_mismatch:{}:{}",
                    case.case_id, actual_projection.decision_cut_id
                )));
            }
        }
    }
    Ok(())
}

fn build_report(corpus: &Corpus) -> Result<Report, String> {
    let mut cases = Vec::new();
    let mut cut_count = 0;
    let mut receipt_count = 0;
    for case in &corpus.cases {
        let projections = project_case(case)?;
        cut_count += projections.len();
        receipt_count += projections
            .iter()
            .map(|projection| projection.visible_event_ids.len())
            .sum::<usize>();
        cases.push(CaseReport {
            case_id: case.case_id.clone(),
            scenario_family_id: case.scenario_family_id.clone(),
            projections,
        });
    }

    Ok(Report {
        benchmark_id: "MFG-POLICY-OBSERVABILITY-001",
        evaluator_version: "mfg-policy-observability-v0.1",
        evidence_class: "synthetic",
        case_count: corpus.cases.len(),
        decision_cut_count: cut_count,
        visible_event_receipt_count: receipt_count,
        sha256_history_digests_emitted: true,
        temporal_cutoff_noninterference_verified: true,
        source_events_created_or_reconciled: false,
        machine_commands_issued: false,
        physical_state_claimed: false,
        cases,
        claim_ceiling: "synthetic receipt-visibility and policy-history projection only",
    })
}

fn load_corpus() -> Result<Corpus, String> {
    let value: serde_json::Value = serde_json::from_str(CORPUS_JSON)
        .map_err(|error| fail(format!("corpus_json_parse:{error}")))?;
    validate_schema(&value, CORPUS_SCHEMA_JSON)?;
    serde_json::from_value(value).map_err(|error| fail(format!("corpus_deserialize:{error}")))
}

fn load_answers() -> Result<KnownAnswerFile, String> {
    let value: serde_json::Value = serde_json::from_str(ANSWERS_JSON)
        .map_err(|error| fail(format!("answers_json_parse:{error}")))?;
    validate_schema(&value, ANSWERS_SCHEMA_JSON)?;
    serde_json::from_value(value).map_err(|error| fail(format!("answers_deserialize:{error}")))
}

fn run() -> Result<(), String> {
    let corpus = load_corpus()?;
    validate_corpus(&corpus)?;
    let answers = load_answers()?;
    compare_known_answers(&corpus, &answers)?;
    let report = build_report(&corpus)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| fail(format!("report_serialize:{error}")))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        load_corpus().expect("strict corpus schema and fixture load")
    }

    #[test]
    fn twenty_frozen_timelines_match_independent_projection_answers() {
        let corpus = corpus();
        validate_corpus(&corpus).unwrap();
        compare_known_answers(&corpus, &load_answers().unwrap()).unwrap();
    }

    #[test]
    fn future_receipt_never_leaks_even_when_effective_time_is_in_the_past() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-02-LATE-OLD-EFFECTIVE")
            .unwrap();
        let before_arrival = project_case(case).unwrap();
        assert!(before_arrival[0].visible_event_ids.is_empty());
        assert!(before_arrival[0].active_event_ids.is_empty());
        assert_eq!(before_arrival[1].active_event_ids, vec!["e1"]);
    }

    #[test]
    fn late_arrival_changes_only_decision_cuts_after_receipt_time() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-10-LATE-MEASUREMENT")
            .unwrap();
        let projections = project_case(case).unwrap();
        assert!(projections[0].visible_event_ids.is_empty());
        assert_eq!(projections[1].visible_event_ids, vec!["e1"]);
        assert_eq!(projections[1].active_event_ids, vec!["e1"]);
        assert_ne!(projections[0].history_sha256, projections[1].history_sha256);
    }

    #[test]
    fn exact_duplicate_replay_is_idempotent_but_payload_conflict_is_rejected() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-03-REPLAY-DUPLICATE")
            .unwrap();
        let projection = project_case(case).unwrap();
        assert_eq!(projection[0].visible_event_ids, vec!["e1", "e2"]);

        let mut events = case.events.clone();
        let mut conflicting = events[0].clone();
        conflicting.event_type = "MUTATED_EVENT".into();
        events.push(conflicting);
        assert_eq!(
            normalize_events(&events).unwrap_err(),
            "event_id_reused_with_different_payload"
        );
    }

    #[test]
    fn source_sequence_collision_is_rejected() {
        let corpus = corpus();
        let case = &corpus.cases[0];
        let mut events = case.events.clone();
        let mut collision = events[0].clone();
        collision.event_id = "another-event".into();
        events.push(collision);
        assert_eq!(
            normalize_events(&events).unwrap_err(),
            "source_sequence_reused_by_different_event"
        );
    }

    #[test]
    fn correction_is_non_retroactive_and_preserves_original_receipt() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-04-ADMITTED-CORRECTION")
            .unwrap();
        let projections = project_case(case).unwrap();
        assert_eq!(projections[0].active_event_ids, vec!["e1"]);
        assert_eq!(projections[0].visible_event_ids, vec!["e1"]);
        assert_eq!(projections[1].visible_event_ids, vec!["e1", "e2"]);
        assert_eq!(projections[1].active_event_ids, vec!["e2"]);
        assert_eq!(projections[1].superseded_event_ids, vec!["e1"]);
    }

    #[test]
    fn unknown_correction_does_not_supersede_admitted_prior_event() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-05-UNKNOWN-CORRECTION")
            .unwrap();
        let projections = project_case(case).unwrap();
        assert_eq!(projections[1].active_event_ids, vec!["e1"]);
        assert_eq!(projections[1].unresolved_event_ids, vec!["e2"]);
        assert!(projections[1].superseded_event_ids.is_empty());
    }

    #[test]
    fn clock_interval_crossing_decision_cut_remains_unresolved() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-06-CLOCK-UNCERTAINTY")
            .unwrap();
        let projections = project_case(case).unwrap();
        assert!(
            projections[0]
                .unresolved_event_ids
                .contains(&"e1".to_string())
        );
        assert!(projections[1].active_event_ids.contains(&"e1".to_string()));
    }

    #[test]
    fn conflicting_and_unknown_receipts_remain_visible_but_not_admissible() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-08-CONFLICTED-SOURCE")
            .unwrap();
        let projection = project_case(case).unwrap();
        assert!(projection[0].visible_event_ids.contains(&"e1".to_string()));
        assert!(
            projection[0]
                .unresolved_event_ids
                .contains(&"e1".to_string())
        );
        assert!(!projection[0].active_event_ids.contains(&"e1".to_string()));
        assert!(projection[0].active_event_ids.contains(&"e2".to_string()));
    }

    #[test]
    fn exact_cutoff_boundary_is_inclusive_for_observed_receipt() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.case_id == "OBS-18-EXACT-CUTOFF")
            .unwrap();
        let projections = project_case(case).unwrap();
        assert_eq!(projections[0].visible_event_ids, vec!["e1"]);
        assert_eq!(projections[1].visible_event_ids, vec!["e1", "e2"]);
    }

    #[test]
    fn serialization_permutation_does_not_change_projection_or_digest() {
        let corpus = corpus();
        let case = &corpus.cases[19];
        let original = project_case(case).unwrap();
        let mut permuted = case.clone();
        permuted.events.reverse();
        let reordered = project_case(&permuted).unwrap();
        assert_eq!(original, reordered);
    }

    #[test]
    fn post_cutoff_mutation_does_not_change_earlier_projection_or_digest() {
        let corpus = corpus();
        let case = &corpus.cases[9];
        let original = project_case(case).unwrap();
        let mut mutated = case.clone();
        let late = mutated
            .events
            .iter_mut()
            .find(|event| event.event_id == "e1")
            .unwrap();
        late.event_type = "DIFFERENT_FUTURE_PAYLOAD".into();
        let changed = project_case(&mutated).unwrap();
        assert_eq!(original[0], changed[0]);
        assert_ne!(original[1].history_sha256, changed[1].history_sha256);
    }

    #[test]
    fn supersession_scope_must_match_and_cycles_are_rejected() {
        let corpus = corpus();
        let case = &corpus.cases[3];
        let mut wrong_scope = case.events.clone();
        wrong_scope[1].subject_id = "different-subject".into();
        assert_eq!(
            normalize_events(&wrong_scope).unwrap_err(),
            "supersession_scope_or_order_invalid"
        );

        let mut cycle = case.events.clone();
        cycle[0].observed_at_ms = cycle[1].observed_at_ms;
        cycle[0].supersedes_event_id = Some("e2".into());
        assert_eq!(
            normalize_events(&cycle).unwrap_err(),
            "supersession_cycle_detected"
        );
    }

    #[test]
    fn report_has_no_source_admission_machine_or_physical_authority() {
        let report = build_report(&corpus()).unwrap();
        assert!(!report.source_events_created_or_reconciled);
        assert!(!report.machine_commands_issued);
        assert!(!report.physical_state_claimed);
        assert!(report.sha256_history_digests_emitted);
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
