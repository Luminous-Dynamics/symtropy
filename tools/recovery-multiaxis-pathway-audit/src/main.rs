use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../docs/tech/recovery-multiaxis-pathway-corpus-v0.1.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: String,
    algorithm_id: String,
    status: String,
    decision_profile: DecisionProfile,
    family_set_registry: BTreeMap<String, FamilySetDefinition>,
    commitment_profiles: BTreeMap<String, CommitmentProfile>,
    interaction_rules: InteractionRules,
    claim_ceiling: ClaimCeiling,
    expected_case_count: usize,
    expected_counts: ExpectedCounts,
    cases: Vec<PathwayCase>,
    temporal_evidence_records: Vec<TemporalEvidenceRecord>,
    temporal_policy_decisions: Vec<TemporalPolicyDecision>,
    expected_temporal_counts: ExpectedTemporalCounts,
    criteria_sensitivity_control: CriteriaSensitivityControl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DecisionProfile {
    profile_id: String,
    allowed_taxonomy_version: String,
    allowed_criteria_regime: String,
    covered_failure_families: Vec<String>,
    required_evidence_tags: Vec<String>,
    max_evidence_age_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FamilySetDefinition {
    classification: String,
    families: Vec<String>,
    capacity_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CommitmentProfile {
    required_lead_days: u64,
    reversal_days: u64,
    lock_in_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InteractionRules {
    capacity_sensitive_pair: String,
    if_capacity_degraded_with_sensitive_pair: String,
    if_capacity_degraded_otherwise: String,
    if_sensitive_pair_without_degradation: String,
    if_evidence_conflicting: String,
    evidence_sufficient_only_when: String,
    criteria_regime_reversion_requires_new_profile_version: bool,
    required_action_lead_days_by_commitment: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ClaimCeiling {
    all_scenarios_synthetic: bool,
    real_world_measurements: bool,
    probability_weights_provided: bool,
    expected_actuation: bool,
    execution_authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExpectedCounts {
    scope: BTreeMap<String, usize>,
    evidence: BTreeMap<String, usize>,
    hard_gate: BTreeMap<String, usize>,
    timing: BTreeMap<String, usize>,
    disposition: BTreeMap<String, usize>,
    lock_in_warning: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PathwayCase {
    case_id: String,
    family_set_id: String,
    observed_failure_families: Vec<String>,
    family_classification: String,
    classification_review_status: String,
    taxonomy_version: String,
    criteria_regime: String,
    evidence_state: String,
    evidence_age_days: u64,
    observed_evidence_tags: Vec<String>,
    capacity_state: String,
    remaining_lead_days: u64,
    required_lead_days: u64,
    commitment_mode: String,
    reversal_days: u64,
    lock_in_warning: bool,
    expected_scope_status: String,
    expected_evidence_status: String,
    expected_hard_gate_status: String,
    expected_timing_status: String,
    expected_disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TemporalEvidenceRecord {
    evidence_id: String,
    observed_day: u64,
    available_day: u64,
    source_authenticity_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TemporalPolicyDecision {
    decision_id: String,
    policy_id: String,
    scenario_id: String,
    stage_id: String,
    decision_day: u64,
    visible_history_id: String,
    action_id: String,
    evidence_refs: Vec<String>,
    expected_actuation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExpectedTemporalCounts {
    evidence_record_count: usize,
    decision_count: usize,
    visible_history_group_count: usize,
    nonanticipativity_violation_count: usize,
    future_evidence_violation_count: usize,
    missing_evidence_reference_count: usize,
    actuation_authorized_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RankedStrategy {
    strategy_id: String,
    metric_values: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PriorityCriterion {
    metric: String,
    direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct AdmissibilityConstraint {
    metric: String,
    operator: String,
    threshold: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CriteriaRegimeControl {
    regime_id: String,
    profile_version: String,
    criteria_regime: String,
    adopted_by_current_profile: bool,
    priority_order: Vec<PriorityCriterion>,
    admissibility_constraints: Vec<AdmissibilityConstraint>,
    expected_ranked_strategy_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct CriteriaSensitivityControl {
    control_id: String,
    strategies: Vec<RankedStrategy>,
    regimes: Vec<CriteriaRegimeControl>,
    expected_top_strategy_reversal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CaseResult {
    case_id: String,
    scope_status: String,
    evidence_status: String,
    hard_gate_status: String,
    timing_status: String,
    disposition: String,
    lock_in_warning: bool,
    actuation_authorized: bool,
    expected_outcome_matches: bool,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    algorithm_id: String,
    canonical_corpus_sha256: String,
    source_commit: Option<String>,
    case_count: usize,
    temporal_evidence_record_count: usize,
    temporal_decision_count: usize,
    visible_history_group_count: usize,
    nonanticipativity_violation_count: usize,
    future_evidence_violation_count: usize,
    missing_evidence_reference_count: usize,
    temporal_actuation_authorized_count: usize,
    expected_temporal_counts_match: bool,
    criteria_regime_rankings: BTreeMap<String, Vec<String>>,
    criteria_regime_top_strategy_reversal: bool,
    criteria_sensitivity_control_matches: bool,
    scope_counts: BTreeMap<String, usize>,
    evidence_counts: BTreeMap<String, usize>,
    hard_gate_counts: BTreeMap<String, usize>,
    timing_counts: BTreeMap<String, usize>,
    disposition_counts: BTreeMap<String, usize>,
    lock_in_warning_counts: BTreeMap<String, usize>,
    expected_outcomes_match: bool,
    aggregate_counts_match: bool,
    all_synthetic: bool,
    all_actuation_disabled: bool,
    case_results: Vec<CaseResult>,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TemporalSummary {
    evidence_record_count: usize,
    decision_count: usize,
    visible_history_group_count: usize,
    nonanticipativity_violation_count: usize,
    future_evidence_violation_count: usize,
    missing_evidence_reference_count: usize,
    actuation_authorized_count: usize,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut result = values.to_vec();
    result.sort();
    result
}

fn inc(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn canonical_corpus_sha256(corpus: &Corpus) -> String {
    let mut canonical = corpus.clone();
    canonical.decision_profile.covered_failure_families.sort();
    canonical.decision_profile.required_evidence_tags.sort();

    for definition in canonical.family_set_registry.values_mut() {
        definition.families.sort();
    }
    canonical.cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    for case in &mut canonical.cases {
        case.observed_failure_families.sort();
        case.observed_evidence_tags.sort();
    }
    canonical
        .temporal_evidence_records
        .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    canonical
        .temporal_policy_decisions
        .sort_by(|a, b| a.decision_id.cmp(&b.decision_id));
    for decision in &mut canonical.temporal_policy_decisions {
        decision.evidence_refs.sort();
    }
    canonical
        .criteria_sensitivity_control
        .strategies
        .sort_by(|a, b| a.strategy_id.cmp(&b.strategy_id));
    canonical
        .criteria_sensitivity_control
        .regimes
        .sort_by(|a, b| a.regime_id.cmp(&b.regime_id));
    for regime in &mut canonical.criteria_sensitivity_control.regimes {
        regime.admissibility_constraints.sort_by(|a, b| {
            (&a.metric, &a.operator, a.threshold).cmp(&(&b.metric, &b.operator, b.threshold))
        });
    }
    serde_json::to_vec(&canonical)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_default()
}

fn validate_corpus(corpus: &Corpus, source_commit: Option<&str>, errors: &mut Vec<String>) {
    if corpus.schema_version != "recovery-multiaxis-pathway-corpus-v0.1" {
        errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
    }
    if corpus.algorithm_id != "multi-axis-recovery-pathway-audit-v0.1" {
        errors.push("ALGORITHM_ID_UNSUPPORTED".to_string());
    }
    if corpus.status != "synthetic-factorial-challenge-cases" {
        errors.push("CORPUS_STATUS_UNEXPECTED".to_string());
    }
    if !corpus.claim_ceiling.all_scenarios_synthetic {
        errors.push("SCENARIOS_MUST_BE_DECLARED_SYNTHETIC".to_string());
    }
    if corpus.claim_ceiling.real_world_measurements {
        errors.push("REAL_WORLD_MEASUREMENTS_MUST_NOT_BE_CLAIMED".to_string());
    }
    if corpus.claim_ceiling.probability_weights_provided {
        errors.push("PROBABILITY_WEIGHTS_MUST_REMAIN_UNDECLARED".to_string());
    }
    if corpus.claim_ceiling.expected_actuation {
        errors.push("ACTUATION_CEILING_MUST_BE_FALSE".to_string());
    }
    if corpus.claim_ceiling.execution_authority != "NO_ACTUATION_AUTHORIZED" {
        errors.push("EXECUTION_AUTHORITY_INVALID".to_string());
    }
    if let Some(sha) = source_commit {
        if !is_commit_sha(sha) {
            errors.push("SOURCE_COMMIT_INVALID".to_string());
        }
    }

    for value in [
        corpus.decision_profile.profile_id.as_str(),
        corpus.decision_profile.allowed_taxonomy_version.as_str(),
        corpus.decision_profile.allowed_criteria_regime.as_str(),
    ] {
        if !is_nonempty(value) {
            errors.push("DECISION_PROFILE_IDENTITY_EMPTY".to_string());
        }
    }
    if corpus.decision_profile.covered_failure_families.is_empty() {
        errors.push("PROFILE_FAMILY_REGISTRY_EMPTY".to_string());
    }
    if corpus.decision_profile.required_evidence_tags.is_empty() {
        errors.push("PROFILE_REQUIRED_EVIDENCE_EMPTY".to_string());
    }
    if corpus.family_set_registry.is_empty() {
        errors.push("FAMILY_SET_REGISTRY_EMPTY".to_string());
    }
    for (id, definition) in &corpus.family_set_registry {
        if !is_nonempty(id) || definition.families.is_empty() {
            errors.push(format!("FAMILY_SET_INVALID:{id}"));
        }
        if !matches!(
            definition.classification.as_str(),
            "KNOWN_IN_PROFILE" | "OUT_OF_PROFILE"
        ) {
            errors.push(format!("FAMILY_SET_CLASSIFICATION_INVALID:{id}"));
        }
        if definition
            .families
            .iter()
            .any(|family| !is_nonempty(family))
            || sorted(&definition.families).len() != definition.families.len()
        {
            errors.push(format!("FAMILY_SET_FAMILIES_INVALID:{id}"));
        }
    }

    for (id, profile) in &corpus.commitment_profiles {
        if !matches!(id.as_str(), "REVERSIBLE_STAGED" | "IRREVERSIBLE_COMMIT") {
            errors.push(format!("COMMITMENT_PROFILE_UNSUPPORTED:{id}"));
        }
        if id != "REVERSIBLE_STAGED" && id != "IRREVERSIBLE_COMMIT" {
            continue;
        }
        let expected_warning = id == "IRREVERSIBLE_COMMIT";
        if profile.lock_in_warning != expected_warning {
            errors.push(format!("COMMITMENT_LOCK_IN_POLICY_MISMATCH:{id}"));
        }
    }
    if corpus.commitment_profiles.len() != 2 {
        errors.push("COMMITMENT_PROFILE_COUNT_INVALID".to_string());
    }

    if corpus.interaction_rules.capacity_sensitive_pair != "known_capacity_sensitive_pair"
        || corpus
            .interaction_rules
            .if_capacity_degraded_with_sensitive_pair
            != "BLOCK"
        || corpus.interaction_rules.if_capacity_degraded_otherwise != "UNKNOWN"
        || corpus
            .interaction_rules
            .if_sensitive_pair_without_degradation
            != "UNKNOWN"
        || corpus.interaction_rules.if_evidence_conflicting != "UNKNOWN"
        || corpus.interaction_rules.evidence_sufficient_only_when != "FRESH"
        || !corpus
            .interaction_rules
            .criteria_regime_reversion_requires_new_profile_version
    {
        errors.push("INTERACTION_RULES_UNSUPPORTED".to_string());
    }
    for (mode, profile) in &corpus.commitment_profiles {
        if corpus
            .interaction_rules
            .required_action_lead_days_by_commitment
            .get(mode)
            != Some(&profile.required_lead_days)
        {
            errors.push(format!("COMMITMENT_LEAD_RULE_MISMATCH:{mode}"));
        }
    }

    if corpus.expected_case_count != corpus.cases.len() {
        errors.push("DECLARED_CASE_COUNT_MISMATCH".to_string());
    }
    if corpus.cases.is_empty() {
        errors.push("CASES_MISSING_OR_EMPTY".to_string());
    }
    if corpus.temporal_evidence_records.is_empty() {
        errors.push("TEMPORAL_EVIDENCE_RECORDS_EMPTY".to_string());
    }
    if corpus.temporal_policy_decisions.is_empty() {
        errors.push("TEMPORAL_POLICY_DECISIONS_EMPTY".to_string());
    }
    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !is_nonempty(&case.case_id) {
            errors.push("CASE_ID_EMPTY".to_string());
        } else if !case_ids.insert(&case.case_id) {
            errors.push(format!("DUPLICATE_CASE_ID:{}", case.case_id));
        }
        if !corpus.family_set_registry.contains_key(&case.family_set_id) {
            errors.push(format!("FAMILY_SET_UNKNOWN:{}", case.case_id));
        }
        if !corpus
            .commitment_profiles
            .contains_key(&case.commitment_mode)
        {
            errors.push(format!("COMMITMENT_MODE_UNKNOWN:{}", case.case_id));
        }
        if case.observed_failure_families.is_empty() {
            errors.push(format!("OBSERVED_FAMILIES_EMPTY:{}", case.case_id));
        }
        if case.observed_evidence_tags.is_empty() {
            errors.push(format!("OBSERVED_EVIDENCE_TAGS_EMPTY:{}", case.case_id));
        }
        if case.remaining_lead_days == 0 || case.required_lead_days == 0 {
            errors.push(format!("LEAD_TIME_MUST_BE_POSITIVE:{}", case.case_id));
        }
        if case.reversal_days == 0 {
            errors.push(format!("REVERSAL_TIME_MUST_BE_POSITIVE:{}", case.case_id));
        }
        if !matches!(
            case.family_classification.as_str(),
            "COMPLETE" | "INCOMPLETE"
        ) {
            errors.push(format!("FAMILY_CLASSIFICATION_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.classification_review_status.as_str(),
            "REVIEWED" | "UNREVIEWED" | "UNKNOWN"
        ) {
            errors.push(format!(
                "CLASSIFICATION_REVIEW_STATUS_INVALID:{}",
                case.case_id
            ));
        }
        if !matches!(
            case.evidence_state.as_str(),
            "FRESH" | "STALE" | "CONFLICTING"
        ) {
            errors.push(format!("EVIDENCE_STATE_INVALID:{}", case.case_id));
        }
        if !matches!(case.capacity_state.as_str(), "NORMAL" | "DEGRADED") {
            errors.push(format!("CAPACITY_STATE_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.expected_scope_status.as_str(),
            "IN_SCOPE" | "OUT_OF_SCOPE"
        ) {
            errors.push(format!("EXPECTED_SCOPE_STATUS_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.expected_evidence_status.as_str(),
            "SUFFICIENT" | "INSUFFICIENT"
        ) {
            errors.push(format!("EXPECTED_EVIDENCE_STATUS_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.expected_hard_gate_status.as_str(),
            "PASS" | "BLOCK" | "UNKNOWN"
        ) {
            errors.push(format!(
                "EXPECTED_HARD_GATE_STATUS_INVALID:{}",
                case.case_id
            ));
        }
        if !matches!(
            case.expected_timing_status.as_str(),
            "ADEQUATE" | "TOO_LATE"
        ) {
            errors.push(format!("EXPECTED_TIMING_STATUS_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.expected_disposition.as_str(),
            "REVIEW_ONLY_CANDIDATE"
                | "REASSESS_SCOPE"
                | "BLOCKED_HARD_GATE"
                | "HOLD_EVIDENCE"
                | "REPLAN_TOO_LATE"
        ) {
            errors.push(format!("EXPECTED_DISPOSITION_INVALID:{}", case.case_id));
        }
    }
}

fn evaluate_case(corpus: &Corpus, case: &PathwayCase) -> CaseResult {
    let family_definition = corpus.family_set_registry.get(&case.family_set_id);
    let commitment = corpus.commitment_profiles.get(&case.commitment_mode);

    let family_record_matches = family_definition
        .map(|definition| sorted(&definition.families) == sorted(&case.observed_failure_families))
        .unwrap_or(false);

    let all_families_covered = case.observed_failure_families.iter().all(|family| {
        corpus
            .decision_profile
            .covered_failure_families
            .contains(family)
    });

    let scope_ok = family_definition
        .map(|definition| definition.classification == "KNOWN_IN_PROFILE")
        .unwrap_or(false)
        && family_record_matches
        && all_families_covered
        && case.criteria_regime == corpus.decision_profile.allowed_criteria_regime
        && case.taxonomy_version == corpus.decision_profile.allowed_taxonomy_version
        && case.family_classification == "COMPLETE"
        && case.classification_review_status == "REVIEWED";

    let age_fresh = case.evidence_age_days <= corpus.decision_profile.max_evidence_age_days;
    let tags_complete = corpus
        .decision_profile
        .required_evidence_tags
        .iter()
        .all(|tag| case.observed_evidence_tags.contains(tag));
    let evidence_ok = case.evidence_state == "FRESH" && age_fresh && tags_complete;

    let sensitive_pair = case.family_set_id == corpus.interaction_rules.capacity_sensitive_pair;
    let hard_gate = if sensitive_pair && case.capacity_state == "DEGRADED" {
        "BLOCK"
    } else if case.capacity_state == "DEGRADED"
        || (family_definition
            .map(|definition| definition.capacity_sensitive)
            .unwrap_or(false))
        || case.evidence_state == "CONFLICTING"
    {
        "UNKNOWN"
    } else {
        "PASS"
    };

    let required_lead = commitment
        .map(|profile| profile.required_lead_days)
        .unwrap_or(u64::MAX);
    let timing_ok = case.remaining_lead_days >= required_lead;
    let lock_in_warning = commitment
        .map(|profile| profile.lock_in_warning)
        .unwrap_or(true);

    let scope_status = if scope_ok { "IN_SCOPE" } else { "OUT_OF_SCOPE" };
    let evidence_status = if evidence_ok {
        "SUFFICIENT"
    } else {
        "INSUFFICIENT"
    };
    let timing_status = if timing_ok { "ADEQUATE" } else { "TOO_LATE" };

    let disposition = if !scope_ok {
        "REASSESS_SCOPE"
    } else if hard_gate == "BLOCK" {
        "BLOCKED_HARD_GATE"
    } else if hard_gate == "UNKNOWN" || !evidence_ok {
        "HOLD_EVIDENCE"
    } else if !timing_ok {
        "REPLAN_TOO_LATE"
    } else {
        "REVIEW_ONLY_CANDIDATE"
    };

    let expected_outcome_matches = scope_status == case.expected_scope_status
        && evidence_status == case.expected_evidence_status
        && hard_gate == case.expected_hard_gate_status
        && timing_status == case.expected_timing_status
        && disposition == case.expected_disposition
        && case.required_lead_days == required_lead
        && case.lock_in_warning == lock_in_warning
        && commitment.map(|profile| profile.reversal_days) == Some(case.reversal_days);

    CaseResult {
        case_id: case.case_id.clone(),
        scope_status: scope_status.to_string(),
        evidence_status: evidence_status.to_string(),
        hard_gate_status: hard_gate.to_string(),
        timing_status: timing_status.to_string(),
        disposition: disposition.to_string(),
        lock_in_warning,
        actuation_authorized: false,
        expected_outcome_matches,
    }
}

fn evaluate_temporal(corpus: &Corpus, errors: &mut Vec<String>) -> TemporalSummary {
    let mut evidence_by_id: BTreeMap<String, &TemporalEvidenceRecord> = BTreeMap::new();
    for record in &corpus.temporal_evidence_records {
        if !is_nonempty(&record.evidence_id) {
            errors.push("TEMPORAL_EVIDENCE_ID_EMPTY".to_string());
        } else if evidence_by_id
            .insert(record.evidence_id.clone(), record)
            .is_some()
        {
            errors.push(format!(
                "TEMPORAL_EVIDENCE_ID_DUPLICATE:{}",
                record.evidence_id
            ));
        }
        if record.available_day < record.observed_day {
            errors.push(format!(
                "TEMPORAL_EVIDENCE_AVAILABLE_BEFORE_OBSERVED:{}",
                record.evidence_id
            ));
        }
        if record.source_authenticity_status != "UNVERIFIED_SYNTHETIC" {
            errors.push(format!(
                "TEMPORAL_EVIDENCE_AUTHENTICITY_CEILING_VIOLATION:{}",
                record.evidence_id
            ));
        }
    }

    let mut decision_ids = BTreeSet::new();
    let mut visible_history_actions: BTreeMap<(String, String, String), BTreeSet<String>> =
        BTreeMap::new();
    let mut future_violation_decisions = BTreeSet::new();
    let mut missing_reference_decisions = BTreeSet::new();
    let mut actuation_authorized_count = 0usize;

    for decision in &corpus.temporal_policy_decisions {
        if !is_nonempty(&decision.decision_id)
            || !is_nonempty(&decision.policy_id)
            || !is_nonempty(&decision.scenario_id)
            || !is_nonempty(&decision.stage_id)
            || !is_nonempty(&decision.visible_history_id)
            || !is_nonempty(&decision.action_id)
        {
            errors.push(format!(
                "TEMPORAL_DECISION_IDENTITY_EMPTY:{}",
                decision.decision_id
            ));
        }
        if !decision_ids.insert(&decision.decision_id) {
            errors.push(format!(
                "TEMPORAL_DECISION_ID_DUPLICATE:{}",
                decision.decision_id
            ));
        }
        if decision.evidence_refs.is_empty() {
            missing_reference_decisions.insert(decision.decision_id.clone());
            errors.push(format!(
                "TEMPORAL_EVIDENCE_REFS_EMPTY:{}",
                decision.decision_id
            ));
        }
        if decision.expected_actuation {
            actuation_authorized_count += 1;
            errors.push(format!(
                "TEMPORAL_ACTUATION_AUTHORIZED:{}",
                decision.decision_id
            ));
        }

        let key = (
            decision.policy_id.clone(),
            decision.stage_id.clone(),
            decision.visible_history_id.clone(),
        );
        visible_history_actions
            .entry(key)
            .or_default()
            .insert(decision.action_id.clone());

        for reference in &decision.evidence_refs {
            match evidence_by_id.get(reference) {
                None => {
                    missing_reference_decisions.insert(decision.decision_id.clone());
                    errors.push(format!(
                        "TEMPORAL_EVIDENCE_REFERENCE_MISSING:{}:{}",
                        decision.decision_id, reference
                    ));
                }
                Some(record)
                    if record.observed_day > decision.decision_day
                        || record.available_day > decision.decision_day =>
                {
                    future_violation_decisions.insert(decision.decision_id.clone());
                    errors.push(format!(
                        "FUTURE_INFORMATION_LEAKAGE:{}:{}",
                        decision.decision_id, reference
                    ));
                }
                Some(_) => {}
            }
        }
    }

    let mut nonanticipativity_violation_count = 0usize;
    for ((policy_id, stage_id, history_id), actions) in &visible_history_actions {
        if actions.len() > 1 {
            nonanticipativity_violation_count += 1;
            errors.push(format!(
                "NONANTICIPATIVITY_VIOLATION:{policy_id}:{stage_id}:{history_id}"
            ));
        }
    }

    TemporalSummary {
        evidence_record_count: corpus.temporal_evidence_records.len(),
        decision_count: corpus.temporal_policy_decisions.len(),
        visible_history_group_count: visible_history_actions.len(),
        nonanticipativity_violation_count,
        future_evidence_violation_count: future_violation_decisions.len(),
        missing_evidence_reference_count: missing_reference_decisions.len(),
        actuation_authorized_count,
    }
}

fn evaluate_criteria_sensitivity(
    corpus: &Corpus,
    errors: &mut Vec<String>,
) -> (BTreeMap<String, Vec<String>>, bool, bool) {
    let control = &corpus.criteria_sensitivity_control;
    let error_start = errors.len();

    if !is_nonempty(&control.control_id) {
        errors.push("CRITERIA_SENSITIVITY_CONTROL_ID_EMPTY".to_string());
    }
    if control.strategies.len() < 2 {
        errors.push("CRITERIA_SENSITIVITY_NEEDS_TWO_STRATEGIES".to_string());
    }
    if control.regimes.len() != 2 {
        errors.push("CRITERIA_SENSITIVITY_NEEDS_TWO_REGIMES".to_string());
    }

    let mut strategy_ids = BTreeSet::new();
    for strategy in &control.strategies {
        if !is_nonempty(&strategy.strategy_id) {
            errors.push("CRITERIA_STRATEGY_ID_EMPTY".to_string());
        }
        if !strategy_ids.insert(&strategy.strategy_id) {
            errors.push(format!(
                "CRITERIA_STRATEGY_ID_DUPLICATE:{}",
                strategy.strategy_id
            ));
        }
        if strategy.metric_values.is_empty() {
            errors.push(format!(
                "CRITERIA_STRATEGY_METRICS_EMPTY:{}",
                strategy.strategy_id
            ));
        }
    }

    let mut regime_ids = BTreeSet::new();
    let mut criteria_regime_ids = BTreeSet::new();
    let mut rankings = BTreeMap::new();

    for regime in &control.regimes {
        if !is_nonempty(&regime.regime_id) || !is_nonempty(&regime.criteria_regime) {
            errors.push("CRITERIA_REGIME_IDENTITY_EMPTY".to_string());
        }
        if !regime_ids.insert(&regime.regime_id) {
            errors.push(format!("CRITERIA_REGIME_ID_DUPLICATE:{}", regime.regime_id));
        }
        if !criteria_regime_ids.insert(&regime.criteria_regime) {
            errors.push(format!(
                "CRITERIA_REGIME_NAME_DUPLICATE:{}",
                regime.criteria_regime
            ));
        }

        let profile_binding_matches = regime.profile_version == corpus.decision_profile.profile_id
            && regime.criteria_regime == corpus.decision_profile.allowed_criteria_regime;
        if regime.adopted_by_current_profile && !profile_binding_matches {
            errors.push(format!(
                "CRITERIA_REGIME_CLAIMS_UNADOPTED_PROFILE:{}",
                regime.regime_id
            ));
        }
        if !regime.adopted_by_current_profile && profile_binding_matches {
            errors.push(format!(
                "CRITERIA_REGIME_CURRENT_PROFILE_MARKED_UNADOPTED:{}",
                regime.regime_id
            ));
        }

        if regime.priority_order.is_empty() {
            errors.push(format!(
                "CRITERIA_PRIORITY_ORDER_EMPTY:{}",
                regime.regime_id
            ));
        }
        if regime.admissibility_constraints.is_empty() {
            errors.push(format!(
                "CRITERIA_ADMISSIBILITY_CONSTRAINTS_EMPTY:{}",
                regime.regime_id
            ));
        }

        let mut priority_metrics = BTreeSet::new();
        for priority in &regime.priority_order {
            if !matches!(priority.direction.as_str(), "MAXIMIZE" | "MINIMIZE") {
                errors.push(format!(
                    "CRITERIA_PRIORITY_DIRECTION_INVALID:{}:{}",
                    regime.regime_id, priority.metric
                ));
            }
            if !priority_metrics.insert(&priority.metric) {
                errors.push(format!(
                    "CRITERIA_PRIORITY_METRIC_DUPLICATE:{}:{}",
                    regime.regime_id, priority.metric
                ));
            }
        }

        for constraint in &regime.admissibility_constraints {
            if !matches!(constraint.operator.as_str(), "LTE" | "GTE") {
                errors.push(format!(
                    "CRITERIA_CONSTRAINT_OPERATOR_INVALID:{}:{}",
                    regime.regime_id, constraint.metric
                ));
            }
        }

        for strategy in &control.strategies {
            for metric in regime
                .priority_order
                .iter()
                .map(|priority| priority.metric.as_str())
                .chain(
                    regime
                        .admissibility_constraints
                        .iter()
                        .map(|constraint| constraint.metric.as_str()),
                )
            {
                if !strategy.metric_values.contains_key(metric) {
                    errors.push(format!(
                        "CRITERIA_STRATEGY_METRIC_MISSING:{}:{}:{}",
                        regime.regime_id, strategy.strategy_id, metric
                    ));
                }
            }
        }

        let mut eligible: Vec<&RankedStrategy> = control
            .strategies
            .iter()
            .filter(|strategy| {
                regime.admissibility_constraints.iter().all(|constraint| {
                    strategy
                        .metric_values
                        .get(&constraint.metric)
                        .map(|value| match constraint.operator.as_str() {
                            "LTE" => *value <= constraint.threshold,
                            "GTE" => *value >= constraint.threshold,
                            _ => false,
                        })
                        .unwrap_or(false)
                })
            })
            .collect();

        eligible.sort_by(|left, right| {
            for priority in &regime.priority_order {
                let left_value = left
                    .metric_values
                    .get(&priority.metric)
                    .copied()
                    .unwrap_or(0);
                let right_value = right
                    .metric_values
                    .get(&priority.metric)
                    .copied()
                    .unwrap_or(0);
                let ordering = match priority.direction.as_str() {
                    "MAXIMIZE" => right_value.cmp(&left_value),
                    "MINIMIZE" => left_value.cmp(&right_value),
                    _ => std::cmp::Ordering::Equal,
                };
                if ordering != std::cmp::Ordering::Equal {
                    return ordering;
                }
            }
            left.strategy_id.cmp(&right.strategy_id)
        });

        let actual_ranked_ids: Vec<String> = eligible
            .iter()
            .map(|strategy| strategy.strategy_id.clone())
            .collect();
        if actual_ranked_ids != regime.expected_ranked_strategy_ids {
            errors.push(format!("CRITERIA_RANKING_MISMATCH:{}", regime.regime_id));
        }
        rankings.insert(regime.criteria_regime.clone(), actual_ranked_ids);
    }

    let first_ids: Vec<String> = control
        .regimes
        .first()
        .and_then(|regime| rankings.get(&regime.criteria_regime))
        .cloned()
        .unwrap_or_default();
    let second_ids: Vec<String> = control
        .regimes
        .get(1)
        .and_then(|regime| rankings.get(&regime.criteria_regime))
        .cloned()
        .unwrap_or_default();
    let top_strategy_reversal = first_ids.first().is_some()
        && second_ids.first().is_some()
        && first_ids.first() != second_ids.first();

    if control.expected_top_strategy_reversal != top_strategy_reversal {
        errors.push("CRITERIA_TOP_STRATEGY_REVERSAL_MISMATCH".to_string());
    }

    let matches = errors.len() == error_start
        && control.expected_top_strategy_reversal
        && top_strategy_reversal;
    (rankings, top_strategy_reversal, matches)
}

fn audit(corpus: &Corpus, source_commit: Option<String>, mut errors: Vec<String>) -> AuditReceipt {
    validate_corpus(corpus, source_commit.as_deref(), &mut errors);
    let (
        criteria_regime_rankings,
        criteria_regime_top_strategy_reversal,
        criteria_sensitivity_control_matches,
    ) = evaluate_criteria_sensitivity(corpus, &mut errors);
    if !criteria_sensitivity_control_matches {
        errors.push("CRITERIA_SENSITIVITY_CONTROL_MISMATCH".to_string());
    }
    let temporal = evaluate_temporal(corpus, &mut errors);
    let expected_temporal_counts_match = temporal.evidence_record_count
        == corpus.expected_temporal_counts.evidence_record_count
        && temporal.decision_count == corpus.expected_temporal_counts.decision_count
        && temporal.visible_history_group_count
            == corpus.expected_temporal_counts.visible_history_group_count
        && temporal.nonanticipativity_violation_count
            == corpus
                .expected_temporal_counts
                .nonanticipativity_violation_count
        && temporal.future_evidence_violation_count
            == corpus
                .expected_temporal_counts
                .future_evidence_violation_count
        && temporal.missing_evidence_reference_count
            == corpus
                .expected_temporal_counts
                .missing_evidence_reference_count
        && temporal.actuation_authorized_count
            == corpus.expected_temporal_counts.actuation_authorized_count;
    if !expected_temporal_counts_match {
        errors.push("TEMPORAL_EXPECTED_COUNTS_MISMATCH".to_string());
    }

    let mut case_results: Vec<CaseResult> = corpus
        .cases
        .iter()
        .map(|case| evaluate_case(corpus, case))
        .collect();
    case_results.sort_by(|a, b| a.case_id.cmp(&b.case_id));

    let mut scope_counts = BTreeMap::new();
    let mut evidence_counts = BTreeMap::new();
    let mut hard_gate_counts = BTreeMap::new();
    let mut timing_counts = BTreeMap::new();
    let mut disposition_counts = BTreeMap::new();
    let mut lock_in_warning_counts = BTreeMap::new();

    for result in &case_results {
        inc(&mut scope_counts, &result.scope_status);
        inc(&mut evidence_counts, &result.evidence_status);
        inc(&mut hard_gate_counts, &result.hard_gate_status);
        inc(&mut timing_counts, &result.timing_status);
        inc(&mut disposition_counts, &result.disposition);
        inc(
            &mut lock_in_warning_counts,
            if result.lock_in_warning {
                "true"
            } else {
                "false"
            },
        );
        if !result.expected_outcome_matches {
            errors.push(format!("EXPECTED_OUTCOME_MISMATCH:{}", result.case_id));
        }
        if result.actuation_authorized {
            errors.push(format!("ACTUATION_AUTHORIZED_OUTPUT:{}", result.case_id));
        }
    }

    let aggregate_counts_match = scope_counts == corpus.expected_counts.scope
        && evidence_counts == corpus.expected_counts.evidence
        && hard_gate_counts == corpus.expected_counts.hard_gate
        && timing_counts == corpus.expected_counts.timing
        && disposition_counts == corpus.expected_counts.disposition
        && lock_in_warning_counts == corpus.expected_counts.lock_in_warning;
    if !aggregate_counts_match {
        errors.push("AGGREGATE_COUNTS_MISMATCH".to_string());
    }

    let expected_outcomes_match = case_results
        .iter()
        .all(|result| result.expected_outcome_matches);
    let all_synthetic = corpus.claim_ceiling.all_scenarios_synthetic
        && !corpus.claim_ceiling.real_world_measurements
        && !corpus.claim_ceiling.probability_weights_provided
        && corpus
            .temporal_evidence_records
            .iter()
            .all(|record| record.source_authenticity_status == "UNVERIFIED_SYNTHETIC");
    let all_actuation_disabled = !corpus.claim_ceiling.expected_actuation
        && case_results
            .iter()
            .all(|result| !result.actuation_authorized)
        && temporal.actuation_authorized_count == 0;
    if !all_synthetic {
        errors.push("SYNTHETIC_CLAIM_CEILING_VIOLATED".to_string());
    }
    if !all_actuation_disabled {
        errors.push("ACTUATION_CEILING_VIOLATED".to_string());
    }
    errors.sort();
    errors.dedup();

    let passed = errors.is_empty()
        && expected_outcomes_match
        && aggregate_counts_match
        && all_synthetic
        && all_actuation_disabled
        && criteria_regime_top_strategy_reversal
        && criteria_sensitivity_control_matches
        && expected_temporal_counts_match
        && temporal.nonanticipativity_violation_count == 0
        && temporal.future_evidence_violation_count == 0
        && temporal.missing_evidence_reference_count == 0
        && temporal.actuation_authorized_count == 0
        && case_results.len() == corpus.expected_case_count;

    AuditReceipt {
        result: if passed { "pass" } else { "fail" }.to_string(),
        schema_version: corpus.schema_version.clone(),
        algorithm_id: corpus.algorithm_id.clone(),
        canonical_corpus_sha256: canonical_corpus_sha256(corpus),
        source_commit,
        case_count: case_results.len(),
        temporal_evidence_record_count: temporal.evidence_record_count,
        temporal_decision_count: temporal.decision_count,
        visible_history_group_count: temporal.visible_history_group_count,
        nonanticipativity_violation_count: temporal.nonanticipativity_violation_count,
        future_evidence_violation_count: temporal.future_evidence_violation_count,
        missing_evidence_reference_count: temporal.missing_evidence_reference_count,
        temporal_actuation_authorized_count: temporal.actuation_authorized_count,
        expected_temporal_counts_match,
        criteria_regime_rankings,
        criteria_regime_top_strategy_reversal,
        criteria_sensitivity_control_matches,
        scope_counts,
        evidence_counts,
        hard_gate_counts,
        timing_counts,
        disposition_counts,
        lock_in_warning_counts,
        expected_outcomes_match,
        aggregate_counts_match,
        all_synthetic,
        all_actuation_disabled,
        case_results,
        errors,
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut path: Option<String> = None;
    let mut source_commit: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--source-commit" {
            if source_commit.is_some() {
                return usage_error("--source-commit may only appear once");
            }
            match args.next() {
                Some(value) => source_commit = Some(value),
                None => return usage_error("--source-commit requires a 40-character SHA"),
            }
        } else if arg.starts_with("--") {
            return usage_error("unsupported command-line option");
        } else if path.is_none() {
            path = Some(arg);
        } else {
            return usage_error("provide at most one corpus path");
        }
    }

    let raw = match path {
        Some(value) => match fs::read(&value) {
            Ok(bytes) => bytes,
            Err(error) => return usage_error(&format!("could not read corpus: {error}")),
        },
        None => EMBEDDED_CORPUS.as_bytes().to_vec(),
    };
    let corpus: Corpus = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => return usage_error(&format!("CORPUS_JSON_INVALID: {error}")),
    };
    let receipt = audit(&corpus, source_commit, Vec::new());
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => println!("{output}"),
        Err(error) => return usage_error(&format!("RECEIPT_SERIALIZATION_FAILED: {error}")),
    }
    if receipt.result == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        serde_json::from_str(EMBEDDED_CORPUS).expect("embedded corpus should parse")
    }

    fn run(corpus: &Corpus) -> AuditReceipt {
        audit(corpus, Some("a".repeat(40)), Vec::new())
    }

    fn case_mut<'a>(corpus: &'a mut Corpus, case_id: &str) -> &'a mut PathwayCase {
        corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == case_id)
            .expect("matrix case should exist")
    }

    fn result_for<'a>(receipt: &'a AuditReceipt, case_id: &str) -> &'a CaseResult {
        receipt
            .case_results
            .iter()
            .find(|result| result.case_id == case_id)
            .expect("case result should exist")
    }

    #[test]
    fn frozen_matrix_and_interaction_controls_match_every_expected_outcome() {
        let receipt = run(&corpus());
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.case_count, 868);
        assert!(receipt.expected_outcomes_match);
        assert!(receipt.aggregate_counts_match);
        assert_eq!(receipt.scope_counts.get("IN_SCOPE"), Some(&112));
        assert_eq!(receipt.scope_counts.get("OUT_OF_SCOPE"), Some(&756));
        assert_eq!(receipt.evidence_counts.get("SUFFICIENT"), Some(&292));
        assert_eq!(receipt.evidence_counts.get("INSUFFICIENT"), Some(&576));
        assert_eq!(receipt.hard_gate_counts.get("PASS"), Some(&218));
        assert_eq!(receipt.hard_gate_counts.get("UNKNOWN"), Some(&542));
        assert_eq!(receipt.hard_gate_counts.get("BLOCK"), Some(&108));
        assert_eq!(receipt.timing_counts.get("ADEQUATE"), Some(&436));
        assert_eq!(receipt.timing_counts.get("TOO_LATE"), Some(&432));
        assert_eq!(
            receipt.disposition_counts.get("REVIEW_ONLY_CANDIDATE"),
            Some(&8)
        );
        assert_eq!(receipt.disposition_counts.get("REASSESS_SCOPE"), Some(&756));
        assert_eq!(
            receipt.disposition_counts.get("BLOCKED_HARD_GATE"),
            Some(&18)
        );
        assert_eq!(receipt.disposition_counts.get("HOLD_EVIDENCE"), Some(&80));
        assert_eq!(receipt.disposition_counts.get("REPLAN_TOO_LATE"), Some(&6));
        assert_eq!(receipt.lock_in_warning_counts.get("true"), Some(&432));
        assert_eq!(receipt.lock_in_warning_counts.get("false"), Some(&436));
        assert!(receipt.all_synthetic);
        assert!(receipt.all_actuation_disabled);
    }

    #[test]
    fn infeasible_high_reward_strategy_is_excluded_from_both_regime_rankings() {
        let receipt = run(&corpus());
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert!(receipt
            .criteria_regime_rankings
            .values()
            .all(|ranked| {
                !ranked.contains(&"constraint-violating-high-yield-shortcut".to_string())
            }));
    }

    #[test]
    fn weakening_a_hard_constraint_exposes_the_inadmissible_strategy_and_fails() {
        let mut value = corpus();
        for regime in &mut value.criteria_sensitivity_control.regimes {
            let constraint = regime
                .admissibility_constraints
                .iter_mut()
                .find(|constraint| constraint.metric == "habitat_disturbance")
                .expect("habitat constraint exists");
            constraint.threshold = 99;
            // Frozen expected ranking remains unchanged: newly eligible shortcut must be caught.
        }
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(!receipt.criteria_sensitivity_control_matches);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("CRITERIA_RANKING_MISMATCH")));
    }

    #[test]
    fn ranking_reverses_when_the_declared_criteria_priority_changes() {
        let receipt = run(&corpus());
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert!(receipt.criteria_sensitivity_control_matches);
        assert!(receipt.criteria_regime_top_strategy_reversal);
        assert_eq!(
            receipt
                .criteria_regime_rankings
                .get("protect-habitat-and-recover-material-v1")
                .unwrap(),
            &vec![
                "rapid_material_recovery".to_string(),
                "retain_inspect_replan".to_string()
            ]
        );
        assert_eq!(
            receipt
                .criteria_regime_rankings
                .get("biosecurity-priority-v2")
                .unwrap(),
            &vec![
                "retain_inspect_replan".to_string(),
                "rapid_material_recovery".to_string()
            ]
        );
        assert!(!corpus().criteria_sensitivity_control.regimes[1].adopted_by_current_profile);
    }

    #[test]
    fn incorrect_expected_ranking_fails_even_when_case_matrix_passes() {
        let mut value = corpus();
        value.criteria_sensitivity_control.regimes[0]
            .expected_ranked_strategy_ids
            .reverse();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(!receipt.criteria_sensitivity_control_matches);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("CRITERIA_RANKING_MISMATCH")));
    }

    #[test]
    fn proposed_new_regime_cannot_claim_current_profile_adoption() {
        let mut value = corpus();
        value.criteria_sensitivity_control.regimes[1].adopted_by_current_profile = true;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("CRITERIA_REGIME_CLAIMS_UNADOPTED_PROFILE")));
    }

    #[test]
    fn temporal_baseline_has_no_anticipativity_or_information_cutoff_violations() {
        let receipt = run(&corpus());
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.temporal_evidence_record_count, 3);
        assert_eq!(receipt.temporal_decision_count, 4);
        assert_eq!(receipt.visible_history_group_count, 3);
        assert_eq!(receipt.nonanticipativity_violation_count, 0);
        assert_eq!(receipt.future_evidence_violation_count, 0);
        assert_eq!(receipt.missing_evidence_reference_count, 0);
        assert_eq!(receipt.temporal_actuation_authorized_count, 0);
        assert!(receipt.expected_temporal_counts_match);
    }

    #[test]
    fn same_visible_history_cannot_choose_different_actions_across_futures() {
        let mut value = corpus();
        let decision = value
            .temporal_policy_decisions
            .iter_mut()
            .find(|decision| decision.decision_id == "shared-prefix-shift-stage-0")
            .expect("shared-prefix decision exists");
        decision.action_id = "COMMIT_IRREVERSIBLY".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.nonanticipativity_violation_count, 1);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("NONANTICIPATIVITY_VIOLATION")));
    }

    #[test]
    fn future_evidence_cannot_become_an_earlier_decision_input() {
        let mut value = corpus();
        let decision = value
            .temporal_policy_decisions
            .iter_mut()
            .find(|decision| decision.decision_id == "shared-prefix-baseline-stage-0")
            .expect("shared-prefix decision exists");
        decision
            .evidence_refs
            .push("synthetic://temporal-evidence/criteria-shift-signpost-day-5".to_string());
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.future_evidence_violation_count, 1);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("FUTURE_INFORMATION_LEAKAGE")));
    }

    #[test]
    fn missing_temporal_evidence_reference_fails_closed() {
        let mut value = corpus();
        let decision = value
            .temporal_policy_decisions
            .iter_mut()
            .find(|decision| decision.decision_id == "shared-prefix-baseline-stage-0")
            .expect("shared-prefix decision exists");
        decision
            .evidence_refs
            .push("synthetic://missing-record".to_string());
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.missing_evidence_reference_count, 1);
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("TEMPORAL_EVIDENCE_REFERENCE_MISSING")));
    }

    #[test]
    fn temporal_decision_cannot_authorize_actuation() {
        let mut value = corpus();
        value.temporal_policy_decisions[0].expected_actuation = true;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert_eq!(receipt.temporal_actuation_authorized_count, 1);
        assert!(!receipt.all_actuation_disabled);
    }

    #[test]
    fn duplicate_temporal_decision_ids_fail_closed() {
        let mut value = corpus();
        let duplicate = value.temporal_policy_decisions[0].clone();
        value.temporal_policy_decisions.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("TEMPORAL_DECISION_ID_DUPLICATE")));
    }

    #[test]
    fn novel_family_never_becomes_in_scope_with_fresh_evidence() {
        let receipt = run(&corpus());
        let case = result_for(
            &receipt,
            "mx-novel_mixed-baseline-complete-fresh-normal-5-reversible_staged",
        );
        assert_eq!(case.scope_status, "OUT_OF_SCOPE");
        assert_eq!(case.disposition, "REASSESS_SCOPE");
    }

    #[test]
    fn changed_and_reverted_criteria_regimes_stay_out_of_scope() {
        let receipt = run(&corpus());
        for id in [
            "mx-known_single-changed-complete-fresh-normal-5-reversible_staged",
            "mx-known_single-reinstated-complete-fresh-normal-5-reversible_staged",
        ] {
            let case = result_for(&receipt, id);
            assert_eq!(case.scope_status, "OUT_OF_SCOPE");
            assert_eq!(case.disposition, "REASSESS_SCOPE");
        }
    }

    #[test]
    fn incomplete_classification_forces_reassessment() {
        let receipt = run(&corpus());
        let case = result_for(
            &receipt,
            "mx-known_single-baseline-incomplete-fresh-normal-5-reversible_staged",
        );
        assert_eq!(case.scope_status, "OUT_OF_SCOPE");
        assert_eq!(case.disposition, "REASSESS_SCOPE");
    }

    #[test]
    fn capacity_sensitive_pair_plus_degraded_capacity_blocks() {
        let receipt = run(&corpus());
        let case = result_for(
            &receipt,
            "mx-known_capacity_sensitive_pair-baseline-complete-fresh-degraded-5-reversible_staged",
        );
        assert_eq!(case.hard_gate_status, "BLOCK");
        assert_eq!(case.disposition, "BLOCKED_HARD_GATE");
    }

    #[test]
    fn sensitive_pair_interaction_is_checked_against_both_single_hazard_controls() {
        let receipt = run(&corpus());
        let habitat_normal = result_for(
            &receipt,
            "mx-control-known_habitat_only-baseline-complete-fresh-normal-5-reversible_staged",
        );
        let route_normal = result_for(
            &receipt,
            "mx-control-known_route_only-baseline-complete-fresh-normal-5-reversible_staged",
        );
        let habitat_degraded = result_for(
            &receipt,
            "mx-control-known_habitat_only-baseline-complete-fresh-degraded-5-reversible_staged",
        );
        let route_degraded = result_for(
            &receipt,
            "mx-control-known_route_only-baseline-complete-fresh-degraded-5-reversible_staged",
        );
        let pair_degraded = result_for(
            &receipt,
            "mx-known_capacity_sensitive_pair-baseline-complete-fresh-degraded-5-reversible_staged",
        );

        assert_eq!(habitat_normal.hard_gate_status, "PASS");
        assert_eq!(route_normal.hard_gate_status, "PASS");
        assert_eq!(habitat_degraded.hard_gate_status, "UNKNOWN");
        assert_eq!(route_degraded.hard_gate_status, "UNKNOWN");
        assert_eq!(pair_degraded.hard_gate_status, "BLOCK");
        assert_eq!(pair_degraded.disposition, "BLOCKED_HARD_GATE");
    }

    #[test]
    fn sensitive_pair_without_degraded_capacity_is_unknown() {
        let receipt = run(&corpus());
        let case = result_for(
            &receipt,
            "mx-known_capacity_sensitive_pair-baseline-complete-fresh-normal-5-reversible_staged",
        );
        assert_eq!(case.hard_gate_status, "UNKNOWN");
        assert_eq!(case.disposition, "HOLD_EVIDENCE");
    }

    #[test]
    fn conflicting_evidence_never_passes_the_gate() {
        let receipt = run(&corpus());
        let case = result_for(
            &receipt,
            "mx-known_single-baseline-complete-conflicting-normal-5-reversible_staged",
        );
        assert_eq!(case.hard_gate_status, "UNKNOWN");
        assert_eq!(case.evidence_status, "INSUFFICIENT");
        assert_eq!(case.disposition, "HOLD_EVIDENCE");
    }

    #[test]
    fn stale_evidence_is_insufficient_even_when_age_is_still_inside_limit() {
        let mut value = corpus();
        let id = "mx-known_single-baseline-complete-stale-normal-5-reversible_staged";
        case_mut(&mut value, id).evidence_age_days = 1;
        let receipt = run(&value);
        let case = result_for(&receipt, id);
        assert_eq!(case.evidence_status, "INSUFFICIENT");
        assert_eq!(case.disposition, "HOLD_EVIDENCE");
    }

    #[test]
    fn three_day_window_allows_reversible_but_not_irreversible_commitment() {
        let receipt = run(&corpus());
        let reversible = result_for(
            &receipt,
            "mx-known_single-baseline-complete-fresh-normal-3-reversible_staged",
        );
        let irreversible = result_for(
            &receipt,
            "mx-known_single-baseline-complete-fresh-normal-3-irreversible_commit",
        );
        assert_eq!(reversible.timing_status, "ADEQUATE");
        assert_eq!(reversible.disposition, "REVIEW_ONLY_CANDIDATE");
        assert_eq!(irreversible.timing_status, "TOO_LATE");
        assert_eq!(irreversible.disposition, "REPLAN_TOO_LATE");
    }

    #[test]
    fn irreversibility_warning_is_kept_separate_from_the_disposition() {
        let receipt = run(&corpus());
        let reversible = result_for(
            &receipt,
            "mx-known_single-baseline-complete-fresh-normal-5-reversible_staged",
        );
        let irreversible = result_for(
            &receipt,
            "mx-known_single-baseline-complete-fresh-normal-5-irreversible_commit",
        );
        assert!(!reversible.lock_in_warning);
        assert!(irreversible.lock_in_warning);
        assert_eq!(irreversible.disposition, "REVIEW_ONLY_CANDIDATE");
    }

    #[test]
    fn malicious_candidate_label_does_not_override_unknown_gate() {
        let mut value = corpus();
        let id = "mx-known_single-baseline-complete-conflicting-normal-5-reversible_staged";
        case_mut(&mut value, id).expected_disposition = "REVIEW_ONLY_CANDIDATE".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error == &format!("EXPECTED_OUTCOME_MISMATCH:{id}")));
        assert!(!result_for(&receipt, id).expected_outcome_matches);
    }

    #[test]
    fn input_row_order_does_not_change_canonical_digest_or_receipt() {
        let original = corpus();
        let mut permuted = original.clone();
        permuted.cases.reverse();
        permuted.temporal_evidence_records.reverse();
        permuted.temporal_policy_decisions.reverse();
        for decision in &mut permuted.temporal_policy_decisions {
            decision.evidence_refs.reverse();
        }
        permuted.criteria_sensitivity_control.strategies.reverse();
        permuted.criteria_sensitivity_control.regimes.reverse();
        for regime in &mut permuted.criteria_sensitivity_control.regimes {
            regime.admissibility_constraints.reverse();
        }
        permuted.decision_profile.covered_failure_families.reverse();
        permuted.decision_profile.required_evidence_tags.reverse();
        for value in permuted.family_set_registry.values_mut() {
            value.families.reverse();
        }
        let first = run(&original);
        let second = run(&permuted);
        assert_eq!(
            first.canonical_corpus_sha256,
            second.canonical_corpus_sha256
        );
        assert_eq!(
            serde_json::to_vec(&first).expect("receipt serializes"),
            serde_json::to_vec(&second).expect("receipt serializes")
        );
    }

    #[test]
    fn duplicate_case_ids_fail_closed() {
        let mut value = corpus();
        let duplicate = value.cases[0].clone();
        value.cases.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("DUPLICATE_CASE_ID")));
    }

    #[test]
    fn expected_count_lie_fails_closed() {
        let mut value = corpus();
        value.expected_case_count += 1;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error == "DECLARED_CASE_COUNT_MISMATCH"));
    }

    #[test]
    fn unknown_family_set_fails_closed() {
        let mut value = corpus();
        let case_id = value.cases[0].case_id.clone();
        case_mut(&mut value, &case_id).family_set_id = "unregistered-family-set".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("FAMILY_SET_UNKNOWN")));
    }

    #[test]
    fn zero_lead_time_is_rejected() {
        let mut value = corpus();
        value.cases[0].remaining_lead_days = 0;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("LEAD_TIME_MUST_BE_POSITIVE")));
    }

    #[test]
    fn wrong_expected_gate_is_detected() {
        let mut value = corpus();
        let id =
            "mx-known_capacity_sensitive_pair-baseline-complete-fresh-degraded-5-reversible_staged";
        case_mut(&mut value, id).expected_hard_gate_status = "PASS".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("EXPECTED_OUTCOME_MISMATCH")));
    }

    #[test]
    fn unsupported_claim_ceiling_fails_closed() {
        let mut value = corpus();
        value.claim_ceiling.expected_actuation = true;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(!receipt.all_actuation_disabled);
    }

    #[test]
    fn source_commit_must_be_full_hex_sha() {
        let receipt = audit(&corpus(), Some("short".to_string()), Vec::new());
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error == "SOURCE_COMMIT_INVALID"));
    }

    #[test]
    fn independent_reference_formula_agrees_with_evaluator_for_all_cases() {
        let value = corpus();
        for case in &value.cases {
            let actual = evaluate_case(&value, case);
            let definition = value.family_set_registry.get(&case.family_set_id).unwrap();
            let commitment = value
                .commitment_profiles
                .get(&case.commitment_mode)
                .unwrap();

            let family_is_admissible = definition.classification == "KNOWN_IN_PROFILE"
                && sorted(&definition.families) == sorted(&case.observed_failure_families)
                && case.observed_failure_families.iter().all(|family| {
                    value
                        .decision_profile
                        .covered_failure_families
                        .contains(family)
                });
            let scope_should_pass = family_is_admissible
                && case.criteria_regime == value.decision_profile.allowed_criteria_regime
                && case.taxonomy_version == value.decision_profile.allowed_taxonomy_version
                && case.family_classification == "COMPLETE"
                && case.classification_review_status == "REVIEWED";
            let evidence_should_pass = case.evidence_state == "FRESH"
                && case.evidence_age_days <= value.decision_profile.max_evidence_age_days
                && value
                    .decision_profile
                    .required_evidence_tags
                    .iter()
                    .all(|tag| case.observed_evidence_tags.contains(tag));
            let gate_should_be =
                if definition.capacity_sensitive && case.capacity_state == "DEGRADED" {
                    "BLOCK"
                } else if case.capacity_state == "DEGRADED"
                    || definition.capacity_sensitive
                    || case.evidence_state == "CONFLICTING"
                {
                    "UNKNOWN"
                } else {
                    "PASS"
                };
            let timing_should_pass = case.remaining_lead_days >= commitment.required_lead_days;
            let disposition_should_be = if !scope_should_pass {
                "REASSESS_SCOPE"
            } else if gate_should_be == "BLOCK" {
                "BLOCKED_HARD_GATE"
            } else if gate_should_be == "UNKNOWN" || !evidence_should_pass {
                "HOLD_EVIDENCE"
            } else if !timing_should_pass {
                "REPLAN_TOO_LATE"
            } else {
                "REVIEW_ONLY_CANDIDATE"
            };

            assert_eq!(
                actual.scope_status,
                if scope_should_pass {
                    "IN_SCOPE"
                } else {
                    "OUT_OF_SCOPE"
                }
            );
            assert_eq!(
                actual.evidence_status,
                if evidence_should_pass {
                    "SUFFICIENT"
                } else {
                    "INSUFFICIENT"
                }
            );
            assert_eq!(actual.hard_gate_status, gate_should_be);
            assert_eq!(
                actual.timing_status,
                if timing_should_pass {
                    "ADEQUATE"
                } else {
                    "TOO_LATE"
                }
            );
            assert_eq!(actual.disposition, disposition_should_be);
        }
    }

    #[test]
    fn no_output_authorizes_an_operation() {
        let receipt = run(&corpus());
        assert!(receipt.all_actuation_disabled);
        assert!(receipt
            .case_results
            .iter()
            .all(|result| !result.actuation_authorized));
    }
}
