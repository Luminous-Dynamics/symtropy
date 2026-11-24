//! Incumbent policy adequacy adapter over an already-qualified policy frontier.
//!
//! This module consumes finite policy metrics from the feasibility-discovery owner.
//! It does not enumerate policy trees, infer observation kernels, estimate probabilities,
//! select a candidate policy, execute queries, or grant operational authority.
//!
//! Terminal loss is compared in its native rational unit. Query, exposure, latency,
//! energy and transition costs remain a separate resource vector. No arbitrary scalar
//! exchange rate is introduced between incomparable units.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const CLAIM_CEILING: &str = "synthetic-observable-policy-maintenance-algebra-only";
const SOURCE_RECEIPT_SCHEMA: &str = "luminous.orbital-feasibility-discovery.receipt.v0.1";
const SOURCE_CLAIM_CEILING: &str = "finite-passive-query-policy-tree-algebra-only";
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_RATIONAL_COMPONENT: u64 = 1_000_000_000_000;
const MAX_SCENARIOS: usize = 16_384;
const MAX_POLICIES: usize = 4_096;
const NO_QUERY_OUTCOME: &str = "__NO_QUERY__";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rational {
    pub numerator: u64,
    pub denominator: u64,
}

impl Rational {
    fn valid(&self) -> bool {
        self.denominator > 0
            && self.numerator <= MAX_RATIONAL_COMPONENT
            && self.denominator <= MAX_RATIONAL_COMPONENT
    }

    fn compare_exact(&self, other: &Self) -> Ordering {
        (u128::from(self.numerator) * u128::from(other.denominator))
            .cmp(&(u128::from(other.numerator) * u128::from(self.denominator)))
    }

    fn equal_exact(&self, other: &Self) -> bool {
        self.compare_exact(other) == Ordering::Equal
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActionCost {
    pub reference_trials: u64,
    pub exposure_micro_units: u64,
    pub elapsed_millis: u64,
}

impl ActionCost {
    fn within(&self, budget: &Self) -> bool {
        self.reference_trials <= budget.reference_trials
            && self.exposure_micro_units <= budget.exposure_micro_units
            && self.elapsed_millis <= budget.elapsed_millis
    }

    fn valid(&self) -> bool {
        self.reference_trials <= MAX_SAFE_INTEGER
            && self.exposure_micro_units <= MAX_SAFE_INTEGER
            && self.elapsed_millis <= MAX_SAFE_INTEGER
    }

    fn checked_add(&self, other: &Self) -> Option<Self> {
        let result = Self {
            reference_trials: self.reference_trials.checked_add(other.reference_trials)?,
            exposure_micro_units: self
                .exposure_micro_units
                .checked_add(other.exposure_micro_units)?,
            elapsed_millis: self.elapsed_millis.checked_add(other.elapsed_millis)?,
        };
        result.valid().then_some(result)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceReceipt {
    pub receipt_schema: String,
    pub claim_ceiling: String,
    pub source_commit: String,
    pub source_case_id: String,
    pub source_scope_id: String,
    pub source_scope_sha256: String,
    pub source_decision_cutoff_id: String,
    pub source_world_set_receipt_id: String,
    pub source_joint_loss_receipt_id: String,
    pub source_joint_scenario_manifest_sha256: String,
    pub policy_payload_sha256: String,
    pub receipt_id: String,
    pub receipt_sha256: String,
    pub state: String,
    pub result: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JointScenarioIdentity {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyScenario {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub observed_outcome_id: String,
    pub selected_action_id: String,
    pub loss: Rational,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicySnapshot {
    pub policy_id: String,
    pub query_id: Option<String>,
    /// Keys are public observation outcomes, or __NO_QUERY__ for a static policy.
    pub branch_actions: BTreeMap<String, String>,
    pub declared_worst_case_loss: Rational,
    pub query_cost: ActionCost,
    pub branch_action_cost_upper_bound: ActionCost,
    /// Must equal query cost plus the maximum branch execution cost in each native unit.
    pub total_cost_upper_bound: ActionCost,
    pub per_scenario: Vec<PolicyScenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub source_policy_case_id: String,
    pub decision_scope_id: String,
    pub scope_sha256: String,
    pub decision_cutoff_id: String,
    pub world_set_receipt_id: String,
    pub joint_loss_receipt_id: String,
    pub joint_scenario_manifest_sha256: String,
    pub source_receipt: SourceReceipt,
    pub world_set_state: String,
    pub loss_model_state: String,
    pub model_family_state: String,
    pub domain_closure_state: String,
    pub dependency_graph_state: String,
    pub evidence_state: String,
    pub incumbent_feasibility_state: String,
    pub loss_unit: String,
    pub incumbent_policy_id: String,
    pub materiality_margin: Rational,
    /// Policy transition cost remains a separate vector; it is not added to loss.
    pub policy_switch_cost: ActionCost,
    pub resource_budget: ActionCost,
    pub expected_joint_scenarios: Vec<JointScenarioIdentity>,
    pub expected_scenario_ids_sha256: String,
    pub expected_joint_scenarios_sha256: String,
    pub policy_pareto_frontier_ids: Vec<String>,
    pub policies: Vec<PolicySnapshot>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WideRational {
    /// Decimal strings preserve exact values beyond JSON's safe-integer range.
    pub numerator: String,
    pub denominator: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SignedDifference {
    pub direction: String,
    pub magnitude: WideRational,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub trigger_reason: String,
    pub loss_unit: String,
    pub materiality_margin: Rational,
    pub incumbent_worst_case_loss: Option<Rational>,
    pub best_eligible_reference_worst_case_loss: Option<Rational>,
    pub worst_case_improvement: Option<SignedDifference>,
    pub eligible_reference_policy_count: usize,
    pub resource_blocked_policy_count: usize,
    pub authority_granted: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub case_count: usize,
    pub retain_incumbent_policy_cases: usize,
    pub policy_review_warranted_cases: usize,
    pub abstain_cases: usize,
    pub invalid_input_cases: usize,
    pub resource_blocked_cases: usize,
    pub expected_result_mismatches: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub authority_granted: bool,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("cannot encode canonical scenario IDs: {error}"))?;
    use sha2::{Digest, Sha256};
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn digest_scenarios(scenarios: &[JointScenarioIdentity]) -> Result<String, String> {
    let mut sorted = scenarios.to_vec();
    sorted.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("cannot encode canonical joint scenarios: {error}"))?;
    use sha2::{Digest, Sha256};
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn policy_payload_digest(
    frontier_ids: &[String],
    policies: &[PolicySnapshot],
) -> Result<String, String> {
    #[derive(Serialize)]
    struct Payload {
        policy_pareto_frontier_ids: Vec<String>,
        policies: Vec<PolicySnapshot>,
    }
    let mut frontier = frontier_ids.to_vec();
    frontier.sort();
    let mut sorted_policies = policies.to_vec();
    for policy in &mut sorted_policies {
        policy
            .per_scenario
            .sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    }
    sorted_policies.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));
    let bytes = serde_json::to_vec(&Payload {
        policy_pareto_frontier_ids: frontier,
        policies: sorted_policies,
    })
    .map_err(|error| format!("cannot encode canonical policy frontier: {error}"))?;
    use sha2::{Digest, Sha256};
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn invalid(case: &Case, reason: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        status: "INVALID_INPUT".to_owned(),
        trigger_reason: reason.to_owned(),
        loss_unit: case.loss_unit.clone(),
        materiality_margin: case.materiality_margin.clone(),
        incumbent_worst_case_loss: None,
        best_eligible_reference_worst_case_loss: None,
        worst_case_improvement: None,
        eligible_reference_policy_count: 0,
        resource_blocked_policy_count: 0,
        authority_granted: false,
    }
}

fn abstain(case: &Case, reason: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        status: "ABSTAIN".to_owned(),
        trigger_reason: reason.to_owned(),
        loss_unit: case.loss_unit.clone(),
        materiality_margin: case.materiality_margin.clone(),
        incumbent_worst_case_loss: None,
        best_eligible_reference_worst_case_loss: None,
        worst_case_improvement: None,
        eligible_reference_policy_count: 0,
        resource_blocked_policy_count: 0,
        authority_granted: false,
    }
}

fn validate_source(case: &Case) -> Result<(), String> {
    let source = &case.source_receipt;
    if source.receipt_schema != SOURCE_RECEIPT_SCHEMA
        || source.claim_ceiling != SOURCE_CLAIM_CEILING
        || !valid_commit(&source.source_commit)
        || !valid_id(&source.source_case_id)
        || source.source_case_id != case.source_policy_case_id
        || !valid_id(&source.source_scope_id)
        || source.source_scope_id != case.decision_scope_id
        || !valid_sha256(&source.source_scope_sha256)
        || source.source_scope_sha256 != case.scope_sha256
        || !valid_id(&source.source_decision_cutoff_id)
        || source.source_decision_cutoff_id != case.decision_cutoff_id
        || !valid_id(&source.source_world_set_receipt_id)
        || source.source_world_set_receipt_id != case.world_set_receipt_id
        || !valid_id(&source.source_joint_loss_receipt_id)
        || source.source_joint_loss_receipt_id != case.joint_loss_receipt_id
        || !valid_sha256(&source.source_joint_scenario_manifest_sha256)
        || source.source_joint_scenario_manifest_sha256 != case.joint_scenario_manifest_sha256
        || !valid_sha256(&source.policy_payload_sha256)
        || !valid_id(&source.receipt_id)
        || !valid_sha256(&source.receipt_sha256)
        || source.result != "pass"
    {
        return Err("upstream feasibility-discovery receipt pins are malformed".to_owned());
    }
    if source.state != "CURRENT"
        && !matches!(
            source.state.as_str(),
            "STALE" | "REVOKED" | "CONTRADICTED" | "INDETERMINATE"
        )
    {
        return Err("upstream source receipt state is malformed".to_owned());
    }
    Ok(())
}

fn validate_case_identity(case: &Case) -> Result<(), String> {
    if !valid_id(&case.case_id)
        || !valid_id(&case.decision_scope_id)
        || !valid_sha256(&case.scope_sha256)
        || !valid_id(&case.decision_cutoff_id)
        || !valid_id(&case.world_set_receipt_id)
        || !valid_id(&case.joint_loss_receipt_id)
        || !valid_sha256(&case.joint_scenario_manifest_sha256)
        || !valid_id(&case.loss_unit)
        || !valid_id(&case.incumbent_policy_id)
        || !case.materiality_margin.valid()
        || !case.policy_switch_cost.valid()
        || !case.resource_budget.valid()
    {
        return Err("case identity, scope, loss unit or cost bounds are malformed".to_owned());
    }
    if !matches!(
        case.world_set_state.as_str(),
        "CURRENT" | "STALE" | "UNAVAILABLE"
    ) || !matches!(
        case.loss_model_state.as_str(),
        "CURRENT" | "STALE" | "INDETERMINATE" | "INADEQUATE" | "OUT_OF_PROFILE"
    ) || !matches!(
        case.model_family_state.as_str(),
        "ADEQUATE" | "INADEQUATE" | "INDETERMINATE"
    ) || !matches!(
        case.domain_closure_state.as_str(),
        "CLOSED_WITHIN_DECLARED_SCOPE" | "OPEN" | "STALE" | "INADEQUATE"
    ) || !matches!(
        case.dependency_graph_state.as_str(),
        "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR" | "INADEQUATE" | "STALE" | "INDETERMINATE"
    ) || !matches!(
        case.evidence_state.as_str(),
        "CURRENT" | "STALE" | "CONTRADICTED" | "INDETERMINATE"
    ) || !matches!(
        case.incumbent_feasibility_state.as_str(),
        "FEASIBLE" | "INFEASIBLE" | "INDETERMINATE"
    ) {
        return Err("case gate state is malformed".to_owned());
    }
    Ok(())
}

fn add_world_cost(left: &ActionCost, right: &ActionCost) -> Option<ActionCost> {
    left.checked_add(right)
}

fn signed_difference(incumbent: &Rational, alternative: &Rational) -> SignedDifference {
    match incumbent.compare_exact(alternative) {
        Ordering::Greater => {
            let left = u128::from(incumbent.numerator) * u128::from(alternative.denominator);
            let right = u128::from(alternative.numerator) * u128::from(incumbent.denominator);
            let numerator = left - right;
            let denominator =
                u128::from(incumbent.denominator) * u128::from(alternative.denominator);
            let gcd = gcd_u128(numerator, denominator);
            SignedDifference {
                direction: "positive_improvement".to_owned(),
                magnitude: WideRational {
                    numerator: (numerator / gcd).to_string(),
                    denominator: (denominator / gcd).to_string(),
                },
            }
        }
        Ordering::Less => {
            let left = u128::from(alternative.numerator) * u128::from(incumbent.denominator);
            let right = u128::from(incumbent.numerator) * u128::from(alternative.denominator);
            let numerator = left - right;
            let denominator =
                u128::from(incumbent.denominator) * u128::from(alternative.denominator);
            let gcd = gcd_u128(numerator, denominator);
            SignedDifference {
                direction: "negative_improvement".to_owned(),
                magnitude: WideRational {
                    numerator: (numerator / gcd).to_string(),
                    denominator: (denominator / gcd).to_string(),
                },
            }
        }
        Ordering::Equal => SignedDifference {
            direction: "no_change".to_owned(),
            magnitude: WideRational {
                numerator: "0".to_owned(),
                denominator: "1".to_owned(),
            },
        },
    }
}

fn improvement_exceeds_margin(improvement: &SignedDifference, margin: &Rational) -> bool {
    if improvement.direction != "positive_improvement" {
        return false;
    }
    let numerator = improvement.magnitude.numerator.parse::<u128>();
    let denominator = improvement.magnitude.denominator.parse::<u128>();
    let (Ok(numerator), Ok(denominator)) = (numerator, denominator) else {
        return false;
    };
    numerator * u128::from(margin.denominator) > u128::from(margin.numerator) * denominator
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

fn validate_policy(
    policy: &PolicySnapshot,
    expected_scenarios: &BTreeMap<&str, (&str, &str)>,
) -> Result<Rational, String> {
    if !valid_id(&policy.policy_id)
        || policy.query_id.as_deref().is_some_and(|id| !valid_id(id))
        || !policy.declared_worst_case_loss.valid()
        || !policy.query_cost.valid()
        || !policy.branch_action_cost_upper_bound.valid()
        || !policy.total_cost_upper_bound.valid()
        || policy.branch_actions.is_empty()
        || policy.per_scenario.is_empty()
    {
        return Err(format!(
            "policy {} identity, cost or bounds are malformed",
            policy.policy_id
        ));
    }

    let recomputed_total = policy
        .query_cost
        .checked_add(&policy.branch_action_cost_upper_bound)
        .ok_or_else(|| format!("policy {} cost addition overflows", policy.policy_id))?;
    if recomputed_total != policy.total_cost_upper_bound {
        return Err(format!(
            "policy {} total cost does not equal query plus branch cost",
            policy.policy_id
        ));
    }

    for (outcome, action) in &policy.branch_actions {
        if !valid_id(outcome) || !valid_id(action) {
            return Err(format!(
                "policy {} contains malformed branch identity",
                policy.policy_id
            ));
        }
    }
    if policy.query_id.is_none()
        && (policy.branch_actions.len() != 1
            || !policy.branch_actions.contains_key(NO_QUERY_OUTCOME))
    {
        return Err(format!(
            "static incumbent policy {} must use the no-query branch",
            policy.policy_id
        ));
    }
    if policy.query_id.is_some() && policy.branch_actions.contains_key(NO_QUERY_OUTCOME) {
        return Err(format!(
            "query policy {} cannot use the no-query sentinel",
            policy.policy_id
        ));
    }

    let mut seen_scenarios = BTreeSet::new();
    let mut observed_outcomes = BTreeSet::new();
    let mut world_outcomes = BTreeMap::<&str, &str>::new();
    let mut worst_loss: Option<Rational> = None;

    for row in &policy.per_scenario {
        if !valid_id(&row.scenario_id)
            || !valid_id(&row.world_id)
            || !valid_id(&row.loss_model_id)
            || !valid_id(&row.observed_outcome_id)
            || !valid_id(&row.selected_action_id)
            || !row.loss.valid()
            || !seen_scenarios.insert(row.scenario_id.as_str())
        {
            return Err(format!(
                "policy {} has malformed or duplicate scenario rows",
                policy.policy_id
            ));
        }
        let Some((expected_world, expected_model)) =
            expected_scenarios.get(row.scenario_id.as_str())
        else {
            return Err(format!(
                "policy {} contains an unplanned joint scenario",
                policy.policy_id
            ));
        };
        if *expected_world != row.world_id || *expected_model != row.loss_model_id {
            return Err(format!(
                "policy {} scenario identity differs from the source joint inventory",
                policy.policy_id
            ));
        }
        if policy.branch_actions.get(&row.observed_outcome_id) != Some(&row.selected_action_id) {
            return Err(format!(
                "policy {} selected action does not match its observable branch",
                policy.policy_id
            ));
        }
        if let Some(previous) = world_outcomes.insert(&row.world_id, &row.observed_outcome_id)
            && previous != row.observed_outcome_id
        {
            return Err(format!(
                "policy {} branches on hidden loss-model identity within one physical world",
                policy.policy_id
            ));
        }
        observed_outcomes.insert(row.observed_outcome_id.as_str());
        if worst_loss
            .as_ref()
            .is_none_or(|best| row.loss.compare_exact(best) == Ordering::Greater)
        {
            worst_loss = Some(row.loss.clone());
        }
    }

    let expected_ids = expected_scenarios.keys().copied().collect::<BTreeSet<_>>();
    if seen_scenarios != expected_ids {
        return Err(format!(
            "policy {} does not cover the exact joint scenario inventory",
            policy.policy_id
        ));
    }
    let branch_ids = policy
        .branch_actions
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let hidden_identity_ids = expected_scenarios
        .iter()
        .flat_map(|(scenario_id, (world_id, loss_model_id))| {
            [*scenario_id, *world_id, *loss_model_id]
        })
        .collect::<BTreeSet<_>>();
    if observed_outcomes
        .iter()
        .any(|outcome| hidden_identity_ids.contains(outcome))
    {
        return Err(format!(
            "policy {} uses a hidden scenario/world/loss-model identity as an observable branch",
            policy.policy_id
        ));
    }
    if branch_ids != observed_outcomes {
        return Err(format!(
            "policy {} has missing or extra observable branches",
            policy.policy_id
        ));
    }

    let worst_loss = worst_loss.expect("nonempty policy scenarios were validated");
    if !worst_loss.equal_exact(&policy.declared_worst_case_loss) {
        return Err(format!(
            "policy {} declared worst-case loss disagrees with scenario rows",
            policy.policy_id
        ));
    }
    Ok(worst_loss)
}

fn evaluate_case(case: &Case) -> CaseResult {
    if let Err(reason) = validate_case_identity(case) {
        return invalid(case, &reason);
    }
    if let Err(reason) = validate_source(case) {
        return invalid(case, &reason);
    }
    if case.source_receipt.state != "CURRENT" {
        return abstain(case, "SOURCE_RECEIPT_NOT_CURRENT");
    }
    if case.world_set_state != "CURRENT" {
        return abstain(case, "WORLD_SET_NOT_CURRENT");
    }
    if case.loss_model_state != "CURRENT" {
        return abstain(case, "LOSS_MODEL_SET_NOT_CURRENT");
    }
    if case.model_family_state != "ADEQUATE" {
        return abstain(case, "MODEL_FAMILY_NOT_ADEQUATE");
    }
    if case.domain_closure_state != "CLOSED_WITHIN_DECLARED_SCOPE" {
        return abstain(case, "ACTION_DOMAIN_NOT_CLOSED");
    }
    if case.dependency_graph_state != "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR" {
        return abstain(case, "DEPENDENCY_GRAPH_NOT_ADEQUATE");
    }
    if case.evidence_state != "CURRENT" {
        return abstain(case, "EVIDENCE_NOT_CURRENT");
    }
    if case.expected_joint_scenarios.is_empty() {
        return abstain(case, "NO_SURVIVING_JOINT_SCENARIOS");
    }
    if case.expected_joint_scenarios.len() > MAX_SCENARIOS
        || case.policies.is_empty()
        || case.policies.len() > MAX_POLICIES
    {
        return invalid(
            case,
            "scenario or policy count exceeds the declared finite bound",
        );
    }
    let scenario_digest = match digest_scenarios(&case.expected_joint_scenarios) {
        Ok(digest) => digest,
        Err(_) => return invalid(case, "joint scenario inventory cannot be encoded"),
    };
    if scenario_digest != case.expected_joint_scenarios_sha256 {
        return invalid(case, "joint scenario identity mapping digest mismatch");
    }
    let scenario_ids = case
        .expected_joint_scenarios
        .iter()
        .map(|item| item.scenario_id.clone())
        .collect::<Vec<_>>();
    let scenario_ids_digest = match digest_ids(&scenario_ids) {
        Ok(digest) => digest,
        Err(_) => return invalid(case, "joint scenario ID inventory cannot be encoded"),
    };
    if scenario_ids_digest != case.expected_scenario_ids_sha256 {
        return invalid(case, "joint scenario ID inventory digest mismatch");
    }
    let payload_digest =
        match policy_payload_digest(&case.policy_pareto_frontier_ids, &case.policies) {
            Ok(digest) => digest,
            Err(_) => return invalid(case, "source policy payload cannot be encoded"),
        };
    if payload_digest != case.source_receipt.policy_payload_sha256 {
        return invalid(
            case,
            "policy frontier payload digest differs from source receipt pin",
        );
    }

    let mut expected_scenarios = BTreeMap::new();
    for scenario in &case.expected_joint_scenarios {
        if !valid_id(&scenario.scenario_id)
            || !valid_id(&scenario.world_id)
            || !valid_id(&scenario.loss_model_id)
            || expected_scenarios
                .insert(
                    scenario.scenario_id.as_str(),
                    (scenario.world_id.as_str(), scenario.loss_model_id.as_str()),
                )
                .is_some()
        {
            return invalid(
                case,
                "planned joint scenario identities are malformed or duplicated",
            );
        }
    }

    let mut frontier_ids = BTreeSet::new();
    for policy_id in &case.policy_pareto_frontier_ids {
        if !valid_id(policy_id) || !frontier_ids.insert(policy_id.as_str()) {
            return invalid(case, "policy frontier contains malformed or duplicate IDs");
        }
    }

    let mut policy_metrics = BTreeMap::new();
    for policy in &case.policies {
        let loss = match validate_policy(policy, &expected_scenarios) {
            Ok(loss) => loss,
            Err(reason) => return invalid(case, &reason),
        };
        if policy_metrics
            .insert(policy.policy_id.as_str(), (policy, loss))
            .is_some()
        {
            return invalid(case, "policy IDs are duplicated");
        }
    }
    if frontier_ids
        .iter()
        .any(|policy_id| !policy_metrics.contains_key(policy_id))
    {
        return invalid(case, "Pareto frontier references an unknown policy");
    }

    if case.incumbent_feasibility_state == "INDETERMINATE" {
        return abstain(case, "INCUMBENT_FEASIBILITY_INDETERMINATE");
    }
    if case.incumbent_feasibility_state == "INFEASIBLE" {
        return CaseResult {
            case_id: case.case_id.clone(),
            status: "POLICY_REVIEW_WARRANTED".to_owned(),
            trigger_reason: "INCUMBENT_POLICY_INFEASIBLE".to_owned(),
            loss_unit: case.loss_unit.clone(),
            materiality_margin: case.materiality_margin.clone(),
            incumbent_worst_case_loss: None,
            best_eligible_reference_worst_case_loss: None,
            worst_case_improvement: None,
            eligible_reference_policy_count: 0,
            resource_blocked_policy_count: 0,
            authority_granted: false,
        };
    }

    let Some((_, incumbent_loss)) = policy_metrics.get(case.incumbent_policy_id.as_str()) else {
        return abstain(case, "INCUMBENT_POLICY_NOT_PRESENT_IN_SOURCE_FRONTIER");
    };
    let incumbent = policy_metrics[case.incumbent_policy_id.as_str()].0;
    if !incumbent
        .total_cost_upper_bound
        .within(&case.resource_budget)
    {
        return CaseResult {
            case_id: case.case_id.clone(),
            status: "POLICY_REVIEW_WARRANTED".to_owned(),
            trigger_reason: "INCUMBENT_POLICY_EXCEEDS_RESOURCE_ENVELOPE".to_owned(),
            loss_unit: case.loss_unit.clone(),
            materiality_margin: case.materiality_margin.clone(),
            incumbent_worst_case_loss: Some(incumbent_loss.clone()),
            best_eligible_reference_worst_case_loss: None,
            worst_case_improvement: None,
            eligible_reference_policy_count: 0,
            resource_blocked_policy_count: 0,
            authority_granted: false,
        };
    }

    let mut best_reference: Option<Rational> = None;
    let mut eligible_count = 0usize;
    let mut resource_blocked_count = 0usize;
    for policy_id in &frontier_ids {
        if *policy_id == case.incumbent_policy_id {
            continue;
        }
        let (policy, loss) = &policy_metrics[*policy_id];
        let Some(total_with_switch) =
            add_world_cost(&policy.total_cost_upper_bound, &case.policy_switch_cost)
        else {
            return invalid(case, "candidate policy plus transition cost overflows");
        };
        if !total_with_switch.within(&case.resource_budget) {
            resource_blocked_count += 1;
            continue;
        }
        eligible_count += 1;
        if best_reference
            .as_ref()
            .is_none_or(|best| loss.compare_exact(best) == Ordering::Less)
        {
            best_reference = Some(loss.clone());
        }
    }

    let Some(reference_loss) = best_reference else {
        return CaseResult {
            case_id: case.case_id.clone(),
            status: "RETAIN_INCUMBENT_POLICY".to_owned(),
            trigger_reason: if resource_blocked_count > 0 {
                "ALTERNATIVES_EXCEED_RESOURCE_ENVELOPE".to_owned()
            } else {
                "NO_ADMISSIBLE_ALTERNATIVE_POLICY".to_owned()
            },
            loss_unit: case.loss_unit.clone(),
            materiality_margin: case.materiality_margin.clone(),
            incumbent_worst_case_loss: Some(incumbent_loss.clone()),
            best_eligible_reference_worst_case_loss: None,
            worst_case_improvement: None,
            eligible_reference_policy_count: 0,
            resource_blocked_policy_count: resource_blocked_count,
            authority_granted: false,
        };
    };

    let improvement = signed_difference(incumbent_loss, &reference_loss);
    let material = improvement_exceeds_margin(&improvement, &case.materiality_margin);
    CaseResult {
        case_id: case.case_id.clone(),
        status: if material {
            "POLICY_REVIEW_WARRANTED"
        } else {
            "RETAIN_INCUMBENT_POLICY"
        }
        .to_owned(),
        trigger_reason: if material {
            "OBSERVABLE_POLICY_IMPROVEMENT_EXCEEDS_MATERIALITY_MARGIN"
        } else {
            "WITHIN_POLICY_RETENTION_ENVELOPE"
        }
        .to_owned(),
        loss_unit: case.loss_unit.clone(),
        materiality_margin: case.materiality_margin.clone(),
        incumbent_worst_case_loss: Some(incumbent_loss.clone()),
        best_eligible_reference_worst_case_loss: Some(reference_loss),
        worst_case_improvement: Some(improvement),
        eligible_reference_policy_count: eligible_count,
        resource_blocked_policy_count: resource_blocked_count,
        authority_granted: false,
    }
}

pub fn evaluate(corpus: &Corpus) -> Result<Evaluation, String> {
    if corpus.schema != "luminous.orbital-incumbent-policy-adequacy.corpus.v0.1"
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 128
    {
        return Err("unsupported incumbent-policy-adequacy corpus or claim ceiling".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("policy adequacy case IDs must be unique".to_owned());
        }
        cases.push(evaluate_case(case));
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let count = |status: &str| cases.iter().filter(|case| case.status == status).count();
    let summary = Summary {
        case_count: cases.len(),
        retain_incumbent_policy_cases: count("RETAIN_INCUMBENT_POLICY"),
        policy_review_warranted_cases: count("POLICY_REVIEW_WARRANTED"),
        abstain_cases: count("ABSTAIN"),
        invalid_input_cases: count("INVALID_INPUT"),
        resource_blocked_cases: cases
            .iter()
            .filter(|case| case.resource_blocked_policy_count > 0)
            .count(),
        expected_result_mismatches: 0,
        authority_grants: cases.iter().filter(|case| case.authority_granted).count(),
    };
    let pass = summary.authority_grants == 0;
    Ok(Evaluation {
        result: if pass { "pass" } else { "fail" }.to_owned(),
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cost(trials: u64, exposure: u64, millis: u64) -> ActionCost {
        ActionCost {
            reference_trials: trials,
            exposure_micro_units: exposure,
            elapsed_millis: millis,
        }
    }

    fn row(
        scenario: &str,
        world: &str,
        model: &str,
        outcome: &str,
        action: &str,
        loss: u64,
    ) -> PolicyScenario {
        PolicyScenario {
            scenario_id: scenario.to_owned(),
            world_id: world.to_owned(),
            loss_model_id: model.to_owned(),
            observed_outcome_id: outcome.to_owned(),
            selected_action_id: action.to_owned(),
            loss: Rational {
                numerator: loss,
                denominator: 1,
            },
        }
    }

    fn refresh_source_digests(case: &mut Case) {
        let ids = case
            .expected_joint_scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.clone())
            .collect::<Vec<_>>();
        case.expected_scenario_ids_sha256 = digest_ids(&ids).expect("scenario ID digest");
        case.expected_joint_scenarios_sha256 =
            digest_scenarios(&case.expected_joint_scenarios).expect("joint scenario digest");
        case.source_receipt.policy_payload_sha256 =
            policy_payload_digest(&case.policy_pareto_frontier_ids, &case.policies)
                .expect("policy payload digest");
    }

    fn fixture_case(candidate_loss: u64, margin: u64, budget_trials: u64) -> Case {
        let scenarios = vec![
            JointScenarioIdentity {
                scenario_id: "s-A".to_owned(),
                world_id: "w-A".to_owned(),
                loss_model_id: "loss-A".to_owned(),
            },
            JointScenarioIdentity {
                scenario_id: "s-B".to_owned(),
                world_id: "w-B".to_owned(),
                loss_model_id: "loss-B".to_owned(),
            },
        ];
        let scenario_ids = scenarios
            .iter()
            .map(|item| item.scenario_id.clone())
            .collect::<Vec<_>>();
        let scenario_digest = digest_ids(&scenario_ids).expect("scenario digest");
        let incumbent = PolicySnapshot {
            policy_id: "incumbent-static".to_owned(),
            query_id: None,
            branch_actions: BTreeMap::from([(NO_QUERY_OUTCOME.to_owned(), "safe".to_owned())]),
            declared_worst_case_loss: Rational {
                numerator: 10,
                denominator: 1,
            },
            query_cost: cost(0, 0, 0),
            branch_action_cost_upper_bound: cost(0, 0, 0),
            total_cost_upper_bound: cost(0, 0, 0),
            per_scenario: vec![
                row("s-A", "w-A", "loss-A", NO_QUERY_OUTCOME, "safe", 10),
                row("s-B", "w-B", "loss-B", NO_QUERY_OUTCOME, "safe", 10),
            ],
        };
        let candidate = PolicySnapshot {
            policy_id: "candidate-observable".to_owned(),
            query_id: Some("safe-measurement".to_owned()),
            branch_actions: BTreeMap::from([
                ("north".to_owned(), "north-action".to_owned()),
                ("south".to_owned(), "south-action".to_owned()),
            ]),
            declared_worst_case_loss: Rational {
                numerator: candidate_loss,
                denominator: 1,
            },
            query_cost: cost(1, 5, 10),
            branch_action_cost_upper_bound: cost(1, 0, 5),
            total_cost_upper_bound: cost(2, 5, 15),
            per_scenario: vec![
                row(
                    "s-A",
                    "w-A",
                    "loss-A",
                    "north",
                    "north-action",
                    candidate_loss,
                ),
                row(
                    "s-B",
                    "w-B",
                    "loss-B",
                    "south",
                    "south-action",
                    candidate_loss,
                ),
            ],
        };
        let mut case = Case {
            case_id: "policy-case".to_owned(),
            source_policy_case_id: "discovery-case-1".to_owned(),
            decision_scope_id: "scope-1".to_owned(),
            scope_sha256: "c".repeat(64),
            decision_cutoff_id: "cutoff-106".to_owned(),
            world_set_receipt_id: "world-receipt-1".to_owned(),
            joint_loss_receipt_id: "loss-receipt-1".to_owned(),
            joint_scenario_manifest_sha256: "d".repeat(64),
            source_receipt: SourceReceipt {
                receipt_schema: SOURCE_RECEIPT_SCHEMA.to_owned(),
                claim_ceiling: SOURCE_CLAIM_CEILING.to_owned(),
                source_commit: "a".repeat(40),
                source_case_id: "discovery-case-1".to_owned(),
                source_scope_id: "scope-1".to_owned(),
                source_scope_sha256: "c".repeat(64),
                source_decision_cutoff_id: "cutoff-106".to_owned(),
                source_world_set_receipt_id: "world-receipt-1".to_owned(),
                source_joint_loss_receipt_id: "loss-receipt-1".to_owned(),
                source_joint_scenario_manifest_sha256: "d".repeat(64),
                policy_payload_sha256: "0".repeat(64),
                receipt_id: "discovery-receipt-1".to_owned(),
                receipt_sha256: "b".repeat(64),
                state: "CURRENT".to_owned(),
                result: "pass".to_owned(),
            },
            world_set_state: "CURRENT".to_owned(),
            loss_model_state: "CURRENT".to_owned(),
            model_family_state: "ADEQUATE".to_owned(),
            domain_closure_state: "CLOSED_WITHIN_DECLARED_SCOPE".to_owned(),
            dependency_graph_state: "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR".to_owned(),
            evidence_state: "CURRENT".to_owned(),
            incumbent_feasibility_state: "FEASIBLE".to_owned(),
            loss_unit: "loss_micro_units".to_owned(),
            incumbent_policy_id: "incumbent-static".to_owned(),
            materiality_margin: Rational {
                numerator: margin,
                denominator: 1,
            },
            policy_switch_cost: cost(1, 2, 20),
            resource_budget: cost(budget_trials, 10, 1_000),
            expected_joint_scenarios: scenarios,
            expected_scenario_ids_sha256: scenario_digest,
            expected_joint_scenarios_sha256: "0".repeat(64),
            policy_pareto_frontier_ids: vec!["candidate-observable".to_owned()],
            policies: vec![incumbent, candidate],
        };
        case.expected_joint_scenarios_sha256 =
            digest_scenarios(&case.expected_joint_scenarios).expect("joint scenario digest");
        case.source_receipt.policy_payload_sha256 =
            policy_payload_digest(&case.policy_pareto_frontier_ids, &case.policies)
                .expect("policy frontier digest");
        case
    }

    #[test]
    fn observable_policy_can_justify_review_without_selecting_or_authorizing_it() {
        let result = evaluate_case(&fixture_case(1, 2, 3));
        assert_eq!(result.status, "POLICY_REVIEW_WARRANTED");
        assert_eq!(
            result.trigger_reason,
            "OBSERVABLE_POLICY_IMPROVEMENT_EXCEEDS_MATERIALITY_MARGIN"
        );
        let improvement = result.worst_case_improvement.expect("improvement");
        assert_eq!(improvement.direction, "positive_improvement");
        assert_eq!(
            improvement.magnitude,
            WideRational {
                numerator: "9".to_owned(),
                denominator: "1".to_owned()
            }
        );
        assert_eq!(result.eligible_reference_policy_count, 1);
        assert!(!result.authority_granted);
    }

    #[test]
    fn exact_materiality_equality_retains_the_incumbent_policy() {
        let result = evaluate_case(&fixture_case(8, 2, 3));
        assert_eq!(result.status, "RETAIN_INCUMBENT_POLICY");
        assert_eq!(result.trigger_reason, "WITHIN_POLICY_RETENTION_ENVELOPE");
        assert_eq!(
            result.worst_case_improvement.unwrap().magnitude,
            WideRational {
                numerator: "2".to_owned(),
                denominator: "1".to_owned()
            }
        );
    }

    #[test]
    fn a_lower_loss_policy_outside_resource_budget_is_not_admissible() {
        let result = evaluate_case(&fixture_case(1, 2, 2));
        assert_eq!(result.status, "RETAIN_INCUMBENT_POLICY");
        assert_eq!(
            result.trigger_reason,
            "ALTERNATIVES_EXCEED_RESOURCE_ENVELOPE"
        );
        assert_eq!(result.resource_blocked_policy_count, 1);
        assert_eq!(result.eligible_reference_policy_count, 0);
    }

    #[test]
    fn multiple_loss_models_for_one_physical_world_must_share_the_observable_branch() {
        let mut case = fixture_case(1, 2, 3);
        case.expected_joint_scenarios.insert(
            1,
            JointScenarioIdentity {
                scenario_id: "s-A-model-2".to_owned(),
                world_id: "w-A".to_owned(),
                loss_model_id: "loss-A2".to_owned(),
            },
        );
        case.expected_scenario_ids_sha256 = digest_ids(
            &case
                .expected_joint_scenarios
                .iter()
                .map(|item| item.scenario_id.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        case.policies[0].per_scenario.insert(
            1,
            row(
                "s-A-model-2",
                "w-A",
                "loss-A2",
                NO_QUERY_OUTCOME,
                "safe",
                10,
            ),
        );
        case.policies[0].declared_worst_case_loss = Rational {
            numerator: 10,
            denominator: 1,
        };
        case.policies[1].per_scenario.insert(
            1,
            row("s-A-model-2", "w-A", "loss-A2", "south", "south-action", 1),
        );
        refresh_source_digests(&mut case);
        let result = evaluate_case(&case);
        assert_eq!(result.status, "INVALID_INPUT");
        assert!(
            result
                .trigger_reason
                .contains("branches on hidden loss-model identity")
        );
    }

    #[test]
    fn hidden_world_id_cannot_be_used_as_a_policy_branch_key() {
        let mut case = fixture_case(1, 2, 3);
        case.policies[1].branch_actions = BTreeMap::from([
            ("w-A".to_owned(), "north-action".to_owned()),
            ("w-B".to_owned(), "south-action".to_owned()),
        ]);
        let result = evaluate_case(&case);
        assert_eq!(result.status, "INVALID_INPUT");
        assert!(
            result
                .trigger_reason
                .contains("selected action does not match its observable branch")
        );
    }

    #[test]
    fn hidden_identity_cannot_be_smuggled_in_as_an_observation_label() {
        let mut case = fixture_case(1, 2, 3);
        case.policies[1].branch_actions = BTreeMap::from([
            ("w-A".to_owned(), "north-action".to_owned()),
            ("w-B".to_owned(), "south-action".to_owned()),
        ]);
        case.policies[1].per_scenario[0].observed_outcome_id = "w-A".to_owned();
        case.policies[1].per_scenario[1].observed_outcome_id = "w-B".to_owned();
        refresh_source_digests(&mut case);
        let result = evaluate_case(&case);
        assert_eq!(result.status, "INVALID_INPUT");
        assert!(
            result
                .trigger_reason
                .contains("uses a hidden scenario/world/loss-model identity")
        );
    }

    #[test]
    fn incomplete_observation_branch_or_scenario_coverage_fails_closed() {
        let mut missing_branch = fixture_case(1, 2, 3);
        missing_branch.policies[1].branch_actions.remove("south");
        let result = evaluate_case(&missing_branch);
        assert_eq!(result.status, "INVALID_INPUT");
        assert!(
            result
                .trigger_reason
                .contains("selected action does not match its observable branch")
        );

        let mut missing_scenario = fixture_case(1, 2, 3);
        missing_scenario.policies[1].per_scenario.pop();
        let result = evaluate_case(&missing_scenario);
        assert_eq!(result.status, "INVALID_INPUT");
        assert!(
            result
                .trigger_reason
                .contains("does not cover the exact joint scenario inventory")
        );
    }

    #[test]
    fn source_receipt_scope_and_cutoff_must_match_the_policy_case() {
        let mut wrong_scope = fixture_case(1, 2, 3);
        wrong_scope.source_receipt.source_scope_sha256 = "e".repeat(64);
        assert_eq!(
            evaluate_case(&wrong_scope).trigger_reason,
            "upstream feasibility-discovery receipt pins are malformed"
        );

        let mut wrong_cutoff = fixture_case(1, 2, 3);
        wrong_cutoff.source_receipt.source_decision_cutoff_id = "cutoff-future".to_owned();
        assert_eq!(
            evaluate_case(&wrong_cutoff).trigger_reason,
            "upstream feasibility-discovery receipt pins are malformed"
        );
    }

    #[test]
    fn altered_joint_world_or_loss_model_mapping_fails_its_full_record_digest() {
        let mut case = fixture_case(1, 2, 3);
        case.expected_joint_scenarios[0].loss_model_id = "substituted-loss-model".to_owned();
        let result = evaluate_case(&case);
        assert_eq!(result.status, "INVALID_INPUT");
        assert_eq!(
            result.trigger_reason,
            "joint scenario identity mapping digest mismatch"
        );
    }

    #[test]
    fn altered_policy_frontier_metrics_fail_the_source_payload_pin() {
        let mut case = fixture_case(1, 2, 3);
        case.policies[1].per_scenario[0].loss = Rational {
            numerator: 0,
            denominator: 1,
        };
        let result = evaluate_case(&case);
        assert_eq!(result.status, "INVALID_INPUT");
        assert_eq!(
            result.trigger_reason,
            "policy frontier payload digest differs from source receipt pin"
        );
    }

    #[test]
    fn candidate_transition_cost_is_applied_in_each_native_resource_dimension() {
        let mut case = fixture_case(1, 2, 3);
        case.policy_switch_cost.exposure_micro_units = 6;
        case.resource_budget.exposure_micro_units = 10;
        let result = evaluate_case(&case);
        assert_eq!(result.status, "RETAIN_INCUMBENT_POLICY");
        assert_eq!(result.resource_blocked_policy_count, 1);
        assert_eq!(result.trigger_reason, "ALTERNATIVES_EXCEED_RESOURCE_ENVELOPE");
    }

    #[test]
    fn stale_source_abstains_and_infeasible_incumbent_triggers_review() {
        let mut stale = fixture_case(1, 2, 3);
        stale.source_receipt.state = "STALE".to_owned();
        assert_eq!(
            evaluate_case(&stale).trigger_reason,
            "SOURCE_RECEIPT_NOT_CURRENT"
        );

        let mut infeasible = fixture_case(1, 2, 3);
        infeasible.incumbent_feasibility_state = "INFEASIBLE".to_owned();
        let result = evaluate_case(&infeasible);
        assert_eq!(result.status, "POLICY_REVIEW_WARRANTED");
        assert_eq!(result.trigger_reason, "INCUMBENT_POLICY_INFEASIBLE");
    }

    #[test]
    fn multiple_loss_models_same_world_with_same_observation_are_valid() {
        let mut case = fixture_case(1, 2, 3);
        case.expected_joint_scenarios.insert(
            1,
            JointScenarioIdentity {
                scenario_id: "s-A-model-2".to_owned(),
                world_id: "w-A".to_owned(),
                loss_model_id: "loss-A2".to_owned(),
            },
        );
        case.expected_scenario_ids_sha256 = digest_ids(
            &case
                .expected_joint_scenarios
                .iter()
                .map(|item| item.scenario_id.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        case.policies[0].per_scenario.insert(
            1,
            row(
                "s-A-model-2",
                "w-A",
                "loss-A2",
                NO_QUERY_OUTCOME,
                "safe",
                10,
            ),
        );
        case.policies[1].per_scenario.insert(
            1,
            row("s-A-model-2", "w-A", "loss-A2", "north", "north-action", 1),
        );
        refresh_source_digests(&mut case);
        assert_eq!(evaluate_case(&case).status, "POLICY_REVIEW_WARRANTED");
    }
}
