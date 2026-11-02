use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const EXPECTED_SCENARIOS: usize = 11;
type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EventStatus {
    Qualified,
    Rejected,
    Missing,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Visibility {
    PolicyVisible,
    EvaluatorOnly,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Action {
    Hold,
    ChangePlan,
    BlendLots,
    ScheduleExtraction,
    RequestMeasurement,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    Pass,
    Reject,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    expected_disposition: Disposition,
    expected_reason_codes: Vec<String>,
    #[serde(default)]
    counterfactual: Option<Counterfactual>,
    events: Vec<ObservationEvent>,
    decisions: Vec<PolicyDecision>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Counterfactual {
    group_id: String,
    fork_hour: u32,
    branch_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ObservationEvent {
    event_id: String,
    sample_lot_id: String,
    observed_at_hour: u32,
    delivered_at_hour: u32,
    status: EventStatus,
    visibility: Visibility,
    source_record_id: Option<String>,
    qc_pass: bool,
    result_code: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PolicyDecision {
    decision_id: String,
    at_hour: u32,
    action: Action,
    used_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioReceipt {
    scenario_id: String,
    expected_disposition: Disposition,
    actual_disposition: Disposition,
    expected_reason_codes: Vec<String>,
    actual_reason_codes: Vec<String>,
    expected_control_matched: bool,
    observation_count: usize,
    decision_count: usize,
    counterfactual_group_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    result: String,
    suite_id: String,
    suite_version: String,
    validator_version: String,
    source_commit: String,
    corpus_sha256: String,
    scenario_count: usize,
    expected_controls_matched: usize,
    causal_violation_count: usize,
    counterfactual_group_count: usize,
    counterfactual_groups_rejected: usize,
    scenarios: Vec<ScenarioReceipt>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EventSignature {
    event_id: String,
    sample_lot_id: String,
    observed_at_hour: u32,
    delivered_at_hour: u32,
    status: EventStatus,
    visibility: Visibility,
    source_record_id: Option<String>,
    qc_pass: bool,
    result_code: String,
}

impl From<&ObservationEvent> for EventSignature {
    fn from(event: &ObservationEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            sample_lot_id: event.sample_lot_id.clone(),
            observed_at_hour: event.observed_at_hour,
            delivered_at_hour: event.delivered_at_hour,
            status: event.status,
            visibility: event.visibility,
            source_record_id: event.source_record_id.clone(),
            qc_pass: event.qc_pass,
            result_code: event.result_code.clone(),
        }
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value == value.to_ascii_lowercase()
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_corpus(corpus: &Corpus) -> Check<()> {
    if corpus.suite_id != "MINE-ADAPTIVE-CAUSALITY-001"
        || corpus.suite_version != VERSION
        || corpus.evidence_class != "authored_synthetic_trace_contract"
        || !nonempty(&corpus.claim_ceiling)
        || corpus.scenarios.len() != EXPECTED_SCENARIOS
    {
        return Err(
            "invalid suite identity, evidence class, claim ceiling, or frozen scenario count"
                .to_owned(),
        );
    }
    let mut ids = BTreeSet::new();
    for scenario in &corpus.scenarios {
        if !nonempty(&scenario.scenario_id) || !ids.insert(scenario.scenario_id.clone()) {
            return Err("scenario IDs must be non-empty and unique".to_owned());
        }
        let mut expected_codes = BTreeSet::new();
        for code in &scenario.expected_reason_codes {
            if !nonempty(code) || !expected_codes.insert(code.clone()) {
                return Err(format!(
                    "{}: expected reason codes must be non-empty and unique",
                    scenario.scenario_id
                ));
            }
        }
        if (scenario.expected_disposition == Disposition::Pass)
            != scenario.expected_reason_codes.is_empty()
        {
            return Err(format!(
                "{}: disposition and expected reason codes disagree",
                scenario.scenario_id
            ));
        }
        if scenario.decisions.is_empty() {
            return Err(format!(
                "{}: at least one policy decision is required",
                scenario.scenario_id
            ));
        }
        if let Some(pair) = &scenario.counterfactual {
            if !nonempty(&pair.group_id) || !nonempty(&pair.branch_id) {
                return Err(format!(
                    "{}: counterfactual group and branch IDs must be non-empty",
                    scenario.scenario_id
                ));
            }
        }
    }
    Ok(())
}

fn audit_scenario(scenario: &Scenario) -> BTreeSet<String> {
    let mut reasons = BTreeSet::new();
    let mut events_by_id: BTreeMap<String, &ObservationEvent> = BTreeMap::new();

    for event in &scenario.events {
        if !nonempty(&event.event_id) || events_by_id.contains_key(&event.event_id) {
            reasons.insert("duplicate_or_empty_event_id".to_owned());
        } else {
            events_by_id.insert(event.event_id.clone(), event);
        }
        if !nonempty(&event.sample_lot_id) {
            reasons.insert("empty_sample_lot_id".to_owned());
        }
        if !nonempty(&event.result_code) {
            reasons.insert("empty_result_code".to_owned());
        }
        if event.delivered_at_hour < event.observed_at_hour {
            reasons.insert("delivery_before_observation".to_owned());
        }
        if event
            .source_record_id
            .as_ref()
            .is_some_and(|source| !nonempty(source))
        {
            reasons.insert("empty_source_record_id".to_owned());
        }
    }

    let mut decision_ids = BTreeSet::new();
    let mut previous_hour = None;
    for decision in &scenario.decisions {
        if !nonempty(&decision.decision_id) || !decision_ids.insert(decision.decision_id.clone()) {
            reasons.insert("duplicate_or_empty_decision_id".to_owned());
        }
        if previous_hour.is_some_and(|previous| decision.at_hour < previous) {
            reasons.insert("decisions_out_of_order".to_owned());
        }
        previous_hour = Some(decision.at_hour);

        let mut used_ids = BTreeSet::new();
        for event_id in &decision.used_event_ids {
            if !used_ids.insert(event_id) {
                reasons.insert("duplicate_event_reference".to_owned());
                continue;
            }
            let Some(event) = events_by_id.get(event_id) else {
                reasons.insert("unknown_event_used".to_owned());
                continue;
            };
            if event.visibility == Visibility::EvaluatorOnly {
                reasons.insert("evaluator_only_event_used".to_owned());
            }
            if event.delivered_at_hour > decision.at_hour {
                reasons.insert("event_used_before_delivery".to_owned());
            }
            if event.status != EventStatus::Qualified {
                reasons.insert("event_not_qualified".to_owned());
            }
            if event
                .source_record_id
                .as_ref()
                .is_none_or(|source| !nonempty(source))
            {
                reasons.insert("event_source_missing".to_owned());
            }
            if !event.qc_pass {
                reasons.insert("event_qc_failed".to_owned());
            }
        }
    }
    reasons
}

fn pre_fork_decisions(scenario: &Scenario, fork_hour: u32) -> Vec<(u32, Action, Vec<String>)> {
    scenario
        .decisions
        .iter()
        .filter(|decision| decision.at_hour < fork_hour)
        .map(|decision| {
            (
                decision.at_hour,
                decision.action,
                decision.used_event_ids.clone(),
            )
        })
        .collect()
}

fn event_history_before_fork(
    scenario: &Scenario,
    fork_hour: u32,
) -> BTreeMap<String, EventSignature> {
    scenario
        .events
        .iter()
        .filter(|event| event.delivered_at_hour < fork_hour)
        .map(|event| (event.event_id.clone(), EventSignature::from(event)))
        .collect()
}

fn post_fork_events(scenario: &Scenario, fork_hour: u32) -> BTreeMap<String, EventSignature> {
    scenario
        .events
        .iter()
        .filter(|event| event.delivered_at_hour >= fork_hour)
        .map(|event| (event.event_id.clone(), EventSignature::from(event)))
        .collect()
}

fn audit_counterfactual_groups(
    scenarios: &[Scenario],
    reasons_by_scenario: &mut [BTreeSet<String>],
) -> (usize, usize) {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        if let Some(pair) = &scenario.counterfactual {
            groups.entry(pair.group_id.clone()).or_default().push(index);
        }
    }

    let mut rejected_groups = 0_usize;
    for indexes in groups.values() {
        let mut group_reasons = BTreeSet::new();
        if indexes.len() != 2 {
            group_reasons.insert("counterfactual_pair_cardinality_mismatch".to_owned());
        } else {
            let left_index = indexes[0];
            let right_index = indexes[1];
            let left = &scenarios[left_index];
            let right = &scenarios[right_index];
            let left_pair = left
                .counterfactual
                .as_ref()
                .expect("group member has metadata");
            let right_pair = right
                .counterfactual
                .as_ref()
                .expect("group member has metadata");

            if left_pair.fork_hour != right_pair.fork_hour {
                group_reasons.insert("counterfactual_fork_hour_mismatch".to_owned());
            }
            if left_pair.branch_id == right_pair.branch_id {
                group_reasons.insert("counterfactual_branch_id_duplicate".to_owned());
            }
            if left_pair.fork_hour == right_pair.fork_hour {
                let fork_hour = left_pair.fork_hour;
                if pre_fork_decisions(left, fork_hour) != pre_fork_decisions(right, fork_hour) {
                    group_reasons.insert("counterfactual_pre_fork_decision_mismatch".to_owned());
                }
                if event_history_before_fork(left, fork_hour)
                    != event_history_before_fork(right, fork_hour)
                {
                    group_reasons.insert("counterfactual_shared_history_mismatch".to_owned());
                }
                if post_fork_events(left, fork_hour) == post_fork_events(right, fork_hour) {
                    group_reasons.insert("counterfactual_no_post_fork_divergence".to_owned());
                }
            }
            if group_reasons.is_empty() {
                continue;
            }
            reasons_by_scenario[left_index].extend(group_reasons.iter().cloned());
            reasons_by_scenario[right_index].extend(group_reasons.iter().cloned());
            rejected_groups += 1;
            continue;
        }

        for index in indexes {
            reasons_by_scenario[*index].extend(group_reasons.iter().cloned());
        }
        rejected_groups += 1;
    }
    (groups.len(), rejected_groups)
}

fn source_commit_from_args(args: &[String]) -> Check<String> {
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: mine-adaptive-causality-bench <corpus.json> --source-commit <40-hex-sha>"
                .to_owned(),
        );
    }
    let source_commit = args[3].to_ascii_lowercase();
    if !is_lower_hex(&source_commit, 40) {
        return Err("source commit must be a 40-character hexadecimal SHA".to_owned());
    }
    Ok(source_commit)
}

fn run() -> Check<ExitCode> {
    let args: Vec<String> = env::args().collect();
    let source_commit = source_commit_from_args(&args)?;
    let corpus_path = &args[1];
    let bytes = fs::read(corpus_path)
        .map_err(|error| format!("cannot read corpus {corpus_path}: {error}"))?;
    let corpus: Corpus =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid corpus JSON: {error}"))?;
    validate_corpus(&corpus)?;

    let mut reasons: Vec<BTreeSet<String>> = corpus.scenarios.iter().map(audit_scenario).collect();
    let (counterfactual_group_count, counterfactual_groups_rejected) =
        audit_counterfactual_groups(&corpus.scenarios, &mut reasons);

    let scenarios: Vec<ScenarioReceipt> = corpus
        .scenarios
        .iter()
        .zip(reasons.iter())
        .map(|(scenario, actual_reasons)| {
            let actual_reason_codes: Vec<String> = actual_reasons.iter().cloned().collect();
            let actual_disposition = if actual_reason_codes.is_empty() {
                Disposition::Pass
            } else {
                Disposition::Reject
            };
            ScenarioReceipt {
                scenario_id: scenario.scenario_id.clone(),
                expected_disposition: scenario.expected_disposition,
                actual_disposition,
                expected_reason_codes: scenario.expected_reason_codes.clone(),
                expected_control_matched: actual_disposition == scenario.expected_disposition
                    && actual_reason_codes == scenario.expected_reason_codes,
                actual_reason_codes,
                observation_count: scenario.events.len(),
                decision_count: scenario.decisions.len(),
                counterfactual_group_id: scenario
                    .counterfactual
                    .as_ref()
                    .map(|pair| pair.group_id.clone()),
            }
        })
        .collect();

    let expected_controls_matched = scenarios
        .iter()
        .filter(|scenario| scenario.expected_control_matched)
        .count();
    let causal_reason_names = [
        "event_used_before_delivery",
        "evaluator_only_event_used",
        "counterfactual_pre_fork_decision_mismatch",
        "counterfactual_shared_history_mismatch",
    ];
    let causal_violation_count = scenarios
        .iter()
        .flat_map(|scenario| scenario.actual_reason_codes.iter())
        .filter(|reason| causal_reason_names.contains(&reason.as_str()))
        .count();

    let all_controls_matched =
        expected_controls_matched == corpus.scenarios.len() && counterfactual_groups_rejected == 1;

    let receipt = Receipt {
        result: if all_controls_matched {
            "validated"
        } else {
            "control_mismatch"
        }
        .to_owned(),
        suite_id: corpus.suite_id,
        suite_version: corpus.suite_version,
        validator_version: VERSION.to_owned(),
        source_commit,
        corpus_sha256: sha256(&bytes),
        scenario_count: scenarios.len(),
        expected_controls_matched,
        causal_violation_count,
        counterfactual_group_count,
        counterfactual_groups_rejected,
        scenarios,
        claim_ceiling: corpus.claim_ceiling,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize receipt: {error}"))?
    );
    Ok(if all_controls_matched {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        serde_json::from_str(include_str!(
            "../../../docs/tech/mine-adaptive-causality-corpus-v0.1.json"
        ))
        .expect("frozen causal trace corpus must deserialize")
    }

    fn audited_corpus() -> (Corpus, Vec<BTreeSet<String>>) {
        let corpus = corpus();
        validate_corpus(&corpus).expect("corpus metadata should validate");
        let mut reasons: Vec<BTreeSet<String>> =
            corpus.scenarios.iter().map(audit_scenario).collect();
        audit_counterfactual_groups(&corpus.scenarios, &mut reasons);
        (corpus, reasons)
    }

    #[test]
    fn frozen_corpus_matches_all_eleven_expected_dispositions() {
        let (corpus, reasons) = audited_corpus();
        assert_eq!(EXPECTED_SCENARIOS, corpus.scenarios.len());
        for (scenario, reason_codes) in corpus.scenarios.iter().zip(reasons.iter()) {
            let actual_disposition = if reason_codes.is_empty() {
                Disposition::Pass
            } else {
                Disposition::Reject
            };
            assert_eq!(
                scenario.expected_disposition, actual_disposition,
                "{}",
                scenario.scenario_id
            );
            assert_eq!(
                scenario.expected_reason_codes,
                reason_codes.iter().cloned().collect::<Vec<_>>(),
                "{}",
                scenario.scenario_id
            );
        }
    }

    #[test]
    fn collection_time_does_not_authorize_pre_delivery_use() {
        let corpus = corpus();
        let scenario = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "COLLECTION_TIME_IS_NOT_DELIVERY_TIME")
            .expect("late result scenario exists");
        let reasons = audit_scenario(scenario);
        assert!(reasons.contains("event_used_before_delivery"));
    }

    #[test]
    fn evaluator_only_truth_cannot_enter_a_policy_decision() {
        let corpus = corpus();
        let scenario = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "EVALUATOR_ONLY_TRUTH_CANNOT_ENTER_POLICY")
            .expect("evaluator-only scenario exists");
        assert!(audit_scenario(scenario).contains("evaluator_only_event_used"));
    }

    #[test]
    fn qc_failure_and_missing_source_are_separate_reasons() {
        let corpus = corpus();
        let qc = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "QC_FAILURE_BLOCKS_DECISION")
            .expect("QC scenario exists");
        let qc_reasons = audit_scenario(qc);
        assert!(qc_reasons.contains("event_not_qualified"));
        assert!(qc_reasons.contains("event_qc_failed"));

        let missing = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "MISSING_SOURCE_RECORD_BLOCKS_DECISION")
            .expect("source scenario exists");
        assert!(audit_scenario(missing).contains("event_source_missing"));
    }

    #[test]
    fn a_valid_future_fork_allows_different_outcomes_but_not_early_policy_changes() {
        let corpus = corpus();
        let pair: Vec<Scenario> = corpus
            .scenarios
            .into_iter()
            .filter(|scenario| {
                scenario
                    .counterfactual
                    .as_ref()
                    .is_some_and(|pair| pair.group_id == "VALID_FUTURE_FORK")
            })
            .collect();
        assert_eq!(2, pair.len());
        assert_eq!(
            pre_fork_decisions(&pair[0], 2),
            pre_fork_decisions(&pair[1], 2)
        );
        assert_ne!(post_fork_events(&pair[0], 2), post_fork_events(&pair[1], 2));
    }

    #[test]
    fn a_policy_change_before_the_future_observation_is_rejected() {
        let corpus = corpus();
        let mut reasons: Vec<BTreeSet<String>> =
            corpus.scenarios.iter().map(audit_scenario).collect();
        audit_counterfactual_groups(&corpus.scenarios, &mut reasons);
        for (scenario, actual) in corpus.scenarios.iter().zip(reasons.iter()) {
            if scenario
                .scenario_id
                .starts_with("COUNTERFACTUAL_EARLY_CHANGE_")
            {
                assert!(actual.contains("counterfactual_pre_fork_decision_mismatch"));
            }
        }
    }

    #[test]
    fn decisions_must_be_replayed_in_time_order() {
        let mut scenario = corpus()
            .scenarios
            .into_iter()
            .find(|scenario| scenario.scenario_id == "QUALIFIED_RESULT_DELIVERED_BEFORE_DECISION")
            .expect("valid scenario exists");
        scenario.decisions.push(PolicyDecision {
            decision_id: "D1".to_owned(),
            at_hour: 2,
            action: Action::Hold,
            used_event_ids: Vec::new(),
        });
        scenario.decisions.swap(0, 1);
        assert!(audit_scenario(&scenario).contains("decisions_out_of_order"));
    }

    #[test]
    fn unknown_trace_fields_fail_closed() {
        let raw = include_str!("../../../docs/tech/mine-adaptive-causality-corpus-v0.1.json")
            .replacen(
                "\"suite_id\":",
                "\"future_policy_authority\":true,\"suite_id\":",
                1,
            );
        assert!(serde_json::from_str::<Corpus>(&raw).is_err());
    }
}
