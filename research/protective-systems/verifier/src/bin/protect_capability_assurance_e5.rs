//! E5 continuous capability assurance and change-triggered requalification.
//!
//! The gate computes which synthetic capability claims are no longer
//! qualified for the current configuration, then selects a minimum-cost
//! admissible requalification plan subject to methods and independence floors.
//! It never executes tests, changes assets, promotes a qualification, or
//! authorizes procurement/physical action.

use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process;

const SCENARIO: &[u8] = include_bytes!(
    "../../../fixtures/protect-capability-assurance-e5-v0.1/scenario.json"
);
const SCHEMA: &str = include_str!(
    "../../../schemas/protect-capability-assurance-e5-v0.1.schema.json"
);
const CLAIM_CEILING: &str = "CAPABILITY_ASSURANCE_AND_REQUALIFICATION_PLAN_ONLY";
const MAX_ACTIONS: usize = 20;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Scenario {
    schema_version: String,
    evidence_class: String,
    claim_ceiling: String,
    as_of: String,
    assets: Vec<Asset>,
    claims: Vec<Claim>,
    evidence: Vec<Evidence>,
    changes: Vec<ChangeEvent>,
    actions: Vec<RequalificationAction>,
    oracle_expected: OracleExpected,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Asset {
    id: String,
    revision: String,
    configuration_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Claim {
    id: String,
    previous_status: String,
    depends_on_assets: Vec<String>,
    depends_on_claims: Vec<String>,
    evidence_ids: Vec<String>,
    required_methods: Vec<String>,
    minimum_independence: String,
    mandatory_safety_floor: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Evidence {
    id: String,
    status: String,
    asset_id: Option<String>,
    configuration_digest: Option<String>,
    valid_from: String,
    valid_until: String,
    valid_until_exclusive: bool,
    source_class: String,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ChangeEvent {
    id: String,
    event_type: String,
    occurred_at: String,
    asset_id: Option<String>,
    evidence_id: Option<String>,
    from_digest: Option<String>,
    to_digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct RequalificationAction {
    id: String,
    covers_claim_ids: Vec<String>,
    methods: Vec<String>,
    independence: String,
    cost_cents: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct OracleExpected {
    affected_claim_ids: Vec<String>,
    qualified_claim_ids: Vec<String>,
    evidence_states: BTreeMap<String, String>,
    selected_action_ids: Vec<String>,
    selected_cost_cents: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct EvidenceResult {
    evidence_id: String,
    effective_state: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ClaimResult {
    claim_id: String,
    previous_status: String,
    current_status: String,
    affected: bool,
    reasons: Vec<String>,
    mandatory_safety_floor: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RequalificationPlan {
    feasible: bool,
    selected_action_ids: Vec<String>,
    total_cost_cents: Option<u64>,
    uncovered_claim_ids: Vec<String>,
    plan_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct GateReport {
    schema_version: String,
    claim_ceiling: String,
    disposition: String,
    as_of: String,
    scenario_sha256: String,
    asset_count: usize,
    claim_count: usize,
    evidence_count: usize,
    change_count: usize,
    action_catalog_count: usize,
    affected_claim_ids: Vec<String>,
    qualified_claim_ids: Vec<String>,
    evidence_results: Vec<EvidenceResult>,
    claim_results: Vec<ClaimResult>,
    requalification_plan: RequalificationPlan,
    executed_actions: bool,
    qualification_promoted: bool,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_scenario(bytes: &[u8]) -> Result<Scenario, String> {
    let schema: Value =
        serde_json::from_str(SCHEMA).map_err(|error| format!("E5 schema JSON invalid: {error}"))?;
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("E5 scenario JSON invalid: {error}"))?;
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("E5 Draft 2020-12 schema invalid: {error}"))?;
    validator
        .validate(&value)
        .map_err(|error| format!("E5 scenario schema validation failed: {error}"))?;
    serde_json::from_value(value).map_err(|error| format!("E5 typed scenario invalid: {error}"))
}

fn independence_rank(value: &str) -> Result<u8, String> {
    match value {
        "SELF_REPORTED" => Ok(0),
        "WITNESSED" => Ok(1),
        "INDEPENDENT" => Ok(2),
        other => Err(format!("unsupported independence level: {other}")),
    }
}

fn unique_map<'a, T, F>(
    items: &'a [T],
    identify: F,
    label: &str,
) -> Result<BTreeMap<&'a str, &'a T>, String>
where
    F: Fn(&'a T) -> &'a str,
{
    let mut map = BTreeMap::new();
    for item in items {
        let id = identify(item);
        if id.trim().is_empty() {
            return Err(format!("{label} contains an empty ID"));
        }
        if map.insert(id, item).is_some() {
            return Err(format!("{label} contains duplicate ID: {id}"));
        }
    }
    Ok(map)
}

fn validate_graph(scenario: &Scenario) -> Result<(), String> {
    if scenario.schema_version != "protect-capability-assurance-e5/0.1"
        || scenario.evidence_class != "synthetic"
        || scenario.claim_ceiling != CLAIM_CEILING
    {
        return Err("E5 scenario must remain synthetic and within the declared claim ceiling".to_owned());
    }
    if scenario.actions.len() > MAX_ACTIONS {
        return Err(format!(
            "E5 exact planner supports at most {MAX_ACTIONS} actions"
        ));
    }
    let assets = unique_map(&scenario.assets, |item| &item.id, "assets")?;
    let claims = unique_map(&scenario.claims, |item| &item.id, "claims")?;
    let evidence = unique_map(&scenario.evidence, |item| &item.id, "evidence")?;
    let changes = unique_map(&scenario.changes, |item| &item.id, "changes")?;
    let actions = unique_map(&scenario.actions, |item| &item.id, "actions")?;

    for asset in scenario.assets.iter() {
        if asset.configuration_digest.len() != 64
            || !asset.configuration_digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(format!("asset {} has invalid configuration digest", asset.id));
        }
    }

    for claim in scenario.claims.iter() {
        if claim.previous_status != "QUALIFIED" || !claim.mandatory_safety_floor {
            return Err(format!(
                "claim {} must declare previous QUALIFIED state and a mandatory safety floor",
                claim.id
            ));
        }
        if claim.required_methods.is_empty() || claim.evidence_ids.is_empty() {
            return Err(format!(
                "claim {} requires at least one method and evidence reference",
                claim.id
            ));
        }
        independence_rank(&claim.minimum_independence)?;
        for asset_id in &claim.depends_on_assets {
            if !assets.contains_key(asset_id.as_str()) {
                return Err(format!("claim {} references unknown asset {asset_id}", claim.id));
            }
        }
        for dependency in &claim.depends_on_claims {
            if !claims.contains_key(dependency.as_str()) {
                return Err(format!(
                    "claim {} references unknown claim dependency {dependency}",
                    claim.id
                ));
            }
            if dependency == &claim.id {
                return Err(format!("claim {} depends on itself", claim.id));
            }
        }
        for evidence_id in &claim.evidence_ids {
            if !evidence.contains_key(evidence_id.as_str()) {
                return Err(format!("claim {} references unknown evidence {evidence_id}", claim.id));
            }
        }
    }

    for item in scenario.evidence.iter() {
        if item.valid_until_exclusive != true || item.valid_from > item.valid_until {
            return Err(format!("evidence {} has invalid validity interval", item.id));
        }
        if let Some(asset_id) = &item.asset_id {
            if !assets.contains_key(asset_id.as_str()) {
                return Err(format!("evidence {} references unknown asset {asset_id}", item.id));
            }
            if item.configuration_digest.is_none() {
                return Err(format!(
                    "asset-scoped evidence {} must declare a configuration digest",
                    item.id
                ));
            }
        } else if item.configuration_digest.is_some() {
            return Err(format!(
                "unscoped evidence {} cannot declare an asset configuration digest",
                item.id
            ));
        }
    }

    let mut changed_asset_ids = BTreeSet::new();
    let mut validity_event_ids = BTreeSet::new();
    for change in scenario.changes.iter() {
        if change.occurred_at.len() < 10 || &change.occurred_at[..10] > scenario.as_of.as_str() {
            return Err(format!("change {} occurs after scenario as_of date", change.id));
        }
        match change.event_type.as_str() {
            "ASSET_CONFIGURATION_CHANGED" => {
                let asset_id = change
                    .asset_id
                    .as_deref()
                    .ok_or_else(|| format!("change {} requires asset_id", change.id))?;
                let asset = assets
                    .get(asset_id)
                    .ok_or_else(|| format!("change {} references unknown asset", change.id))?;
                let from_digest = change
                    .from_digest
                    .as_deref()
                    .ok_or_else(|| format!("change {} requires from_digest", change.id))?;
                let to_digest = change
                    .to_digest
                    .as_deref()
                    .ok_or_else(|| format!("change {} requires to_digest", change.id))?;
                if from_digest == to_digest || asset.configuration_digest != to_digest {
                    return Err(format!(
                        "change {} digest transition does not match current asset configuration",
                        change.id
                    ));
                }
                if change.evidence_id.is_some() {
                    return Err(format!("asset change {} must not specify evidence_id", change.id));
                }
                changed_asset_ids.insert(asset_id.to_owned());
            }
            "EVIDENCE_VALIDITY_BOUNDARY_REACHED" => {
                let evidence_id = change
                    .evidence_id
                    .as_deref()
                    .ok_or_else(|| format!("change {} requires evidence_id", change.id))?;
                let item = evidence
                    .get(evidence_id)
                    .ok_or_else(|| format!("change {} references unknown evidence", change.id))?;
                if item.valid_until > scenario.as_of {
                    return Err(format!(
                        "change {} claims an evidence expiry before its validity boundary",
                        change.id
                    ));
                }
                if change.asset_id.is_some()
                    || change.from_digest.is_some()
                    || change.to_digest.is_some()
                {
                    return Err(format!("evidence expiry change {} has incompatible fields", change.id));
                }
                validity_event_ids.insert(evidence_id.to_owned());
            }
            other => return Err(format!("unsupported E5 change event type: {other}")),
        }
    }

    for action in scenario.actions.iter() {
        independence_rank(&action.independence)?;
        for claim_id in &action.covers_claim_ids {
            if !claims.contains_key(claim_id.as_str()) {
                return Err(format!(
                    "action {} references unknown claim {claim_id}",
                    action.id
                ));
            }
        }
    }

    // Reject cycles in the claim dependency graph before impact propagation.
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    fn visit(
        claim_id: &str,
        claims: &BTreeMap<&str, &Claim>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        if visited.contains(claim_id) {
            return Ok(());
        }
        if !visiting.insert(claim_id.to_owned()) {
            return Err(format!("claim dependency cycle detected at {claim_id}"));
        }
        let claim = claims
            .get(claim_id)
            .ok_or_else(|| format!("claim dependency node missing: {claim_id}"))?;
        for dependency in &claim.depends_on_claims {
            visit(dependency, claims, visiting, visited)?;
        }
        visiting.remove(claim_id);
        visited.insert(claim_id.to_owned());
        Ok(())
    }
    for claim_id in claims.keys() {
        visit(claim_id, &claims, &mut visiting, &mut visited)?;
    }

    let _ = (changes, actions, validity_event_ids);
    Ok(())
}

fn effective_evidence_state(
    item: &Evidence,
    assets: &BTreeMap<&str, &Asset>,
    as_of: &str,
) -> (String, String) {
    if item.status != "VALID" {
        return (
            item.status.clone(),
            format!("source evidence status is {}", item.status),
        );
    }
    if let Some(asset_id) = &item.asset_id {
        if let Some(asset) = assets.get(asset_id.as_str()) {
            if item.configuration_digest.as_deref()
                != Some(asset.configuration_digest.as_str())
            {
                return (
                    "SUPERSEDED".to_owned(),
                    "evidence configuration digest does not match the current asset".to_owned(),
                );
            }
        } else {
            return (
                "UNKNOWN".to_owned(),
                "asset scope cannot be resolved".to_owned(),
            );
        }
    }
    if item.valid_from.as_str() > as_of {
        return (
            "NOT_YET_VALID".to_owned(),
            "evidence validity interval has not started".to_owned(),
        );
    }
    if item.valid_until.as_str() <= as_of {
        return (
            "EXPIRED".to_owned(),
            "exclusive valid_until boundary has been reached".to_owned(),
        );
    }
    ("VALID".to_owned(), "evidence is in scope and within its validity interval".to_owned())
}

fn action_covers_claim(action: &RequalificationAction, claim: &Claim) -> bool {
    if !action.covers_claim_ids.contains(&claim.id) {
        return false;
    }
    let action_methods: BTreeSet<&str> = action.methods.iter().map(String::as_str).collect();
    if !claim
        .required_methods
        .iter()
        .all(|method| action_methods.contains(method.as_str()))
    {
        return false;
    }
    matches!(
        (independence_rank(&action.independence), independence_rank(&claim.minimum_independence)),
        (Ok(action_level), Ok(required_level)) if action_level >= required_level
    )
}

fn exact_minimum_plan(
    affected_claims: &[&Claim],
    actions: &[RequalificationAction],
) -> Result<RequalificationPlan, String> {
    if affected_claims.is_empty() {
        return Ok(RequalificationPlan {
            feasible: true,
            selected_action_ids: Vec::new(),
            total_cost_cents: Some(0),
            uncovered_claim_ids: Vec::new(),
            plan_status: "NO_REQUALIFICATION_REQUIRED".to_owned(),
        });
    }
    if actions.len() > MAX_ACTIONS {
        return Err(format!("E5 exact planner is bounded to {MAX_ACTIONS} candidate actions"));
    }
    let subset_count = 1u64 << actions.len();
    let mut best: Option<(u64, usize, Vec<String>)> = None;
    for mask in 1..subset_count {
        let mut selected = Vec::new();
        let mut total_cost = 0u64;
        for (index, action) in actions.iter().enumerate() {
            if (mask & (1u64 << index)) != 0 {
                total_cost = total_cost
                    .checked_add(action.cost_cents)
                    .ok_or_else(|| "requalification cost sum overflow".to_owned())?;
                selected.push(action);
            }
        }
        let all_covered = affected_claims.iter().all(|claim| {
            selected
                .iter()
                .any(|action| action_covers_claim(action, claim))
        });
        if !all_covered {
            continue;
        }
        let mut ids: Vec<String> = selected.iter().map(|item| item.id.clone()).collect();
        ids.sort();
        let candidate = (total_cost, ids.len(), ids);
        let replace = match &best {
            None => true,
            Some(previous) => candidate < *previous,
        };
        if replace {
            best = Some(candidate);
        }
    }

    if let Some((cost, _count, selected_action_ids)) = best {
        return Ok(RequalificationPlan {
            feasible: true,
            selected_action_ids,
            total_cost_cents: Some(cost),
            uncovered_claim_ids: Vec::new(),
            plan_status: "MINIMUM_COST_ADMISSIBLE_PLAN_AVAILABLE_NOT_EXECUTED".to_owned(),
        });
    }

    let uncovered_claim_ids = affected_claims
        .iter()
        .filter(|claim| {
            !actions
                .iter()
                .any(|action| action_covers_claim(action, claim))
        })
        .map(|claim| claim.id.clone())
        .collect();
    Ok(RequalificationPlan {
        feasible: false,
        selected_action_ids: Vec::new(),
        total_cost_cents: None,
        uncovered_claim_ids,
        plan_status: "ABSTAIN_REQUALIFICATION_PLAN_INFEASIBLE".to_owned(),
    })
}

fn evaluate(scenario: &Scenario, scenario_bytes: &[u8]) -> Result<GateReport, String> {
    validate_graph(scenario)?;
    let assets = unique_map(&scenario.assets, |item| &item.id, "assets")?;
    let claims = unique_map(&scenario.claims, |item| &item.id, "claims")?;
    let evidence = unique_map(&scenario.evidence, |item| &item.id, "evidence")?;

    let evidence_results: Vec<EvidenceResult> = scenario
        .evidence
        .iter()
        .map(|item| {
            let (effective_state, reason) =
                effective_evidence_state(item, &assets, &scenario.as_of);
            EvidenceResult {
                evidence_id: item.id.clone(),
                effective_state,
                reason,
            }
        })
        .collect();
    let state_by_evidence: BTreeMap<&str, &str> = evidence_results
        .iter()
        .map(|item| (item.evidence_id.as_str(), item.effective_state.as_str()))
        .collect();

    let mut changed_asset_ids = BTreeSet::new();
    for change in &scenario.changes {
        if change.event_type == "ASSET_CONFIGURATION_CHANGED" {
            if let Some(asset_id) = &change.asset_id {
                changed_asset_ids.insert(asset_id.clone());
            }
        }
    }

    let mut affected = BTreeSet::new();
    let mut reasons_by_claim: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for claim in &scenario.claims {
        for asset_id in &claim.depends_on_assets {
            if changed_asset_ids.contains(asset_id) {
                affected.insert(claim.id.clone());
                reasons_by_claim
                    .entry(claim.id.clone())
                    .or_default()
                    .insert(format!("dependency asset {asset_id} changed"));
            }
        }
        for evidence_id in &claim.evidence_ids {
            let state = state_by_evidence
                .get(evidence_id.as_str())
                .copied()
                .unwrap_or("UNKNOWN");
            if state != "VALID" {
                affected.insert(claim.id.clone());
                reasons_by_claim
                    .entry(claim.id.clone())
                    .or_default()
                    .insert(format!("supporting evidence {evidence_id} is {state}"));
            }
        }
    }

    // Propagate invalidation only along explicit claim dependency edges.
    loop {
        let before = affected.len();
        for claim in &scenario.claims {
            for dependency in &claim.depends_on_claims {
                if affected.contains(dependency) {
                    affected.insert(claim.id.clone());
                    reasons_by_claim
                        .entry(claim.id.clone())
                        .or_default()
                        .insert(format!("depends on affected claim {dependency}"));
                }
            }
        }
        if affected.len() == before {
            break;
        }
    }

    let mut claim_results = Vec::new();
    let mut qualified_claim_ids = Vec::new();
    for claim in &scenario.claims {
        let is_affected = affected.contains(&claim.id);
        let mut reasons: Vec<String> = reasons_by_claim
            .get(&claim.id)
            .map(|items| items.iter().cloned().collect())
            .unwrap_or_default();
        if !is_affected {
            for evidence_id in &claim.evidence_ids {
                if state_by_evidence.get(evidence_id.as_str()).copied() != Some("VALID") {
                    reasons.push(format!("supporting evidence {evidence_id} is not valid"));
                }
            }
            for dependency in &claim.depends_on_claims {
                if affected.contains(dependency) {
                    reasons.push(format!("dependency claim {dependency} requires requalification"));
                }
            }
        }
        let current_status = if is_affected {
            "REQUALIFICATION_REQUIRED"
        } else if reasons.is_empty() {
            "QUALIFIED"
        } else {
            "NOT_QUALIFIED"
        };
        if current_status == "QUALIFIED" {
            qualified_claim_ids.push(claim.id.clone());
        }
        claim_results.push(ClaimResult {
            claim_id: claim.id.clone(),
            previous_status: claim.previous_status.clone(),
            current_status: current_status.to_owned(),
            affected: is_affected,
            reasons,
            mandatory_safety_floor: claim.mandatory_safety_floor,
        });
    }
    qualified_claim_ids.sort();
    let mut affected_claim_ids: Vec<String> = affected.into_iter().collect();
    affected_claim_ids.sort();
    let affected_claim_refs: Vec<&Claim> = affected_claim_ids
        .iter()
        .map(|id| {
            claims
                .get(id.as_str())
                .copied()
                .ok_or_else(|| format!("affected claim disappeared from graph: {id}"))
        })
        .collect::<Result<_, _>>()?;
    let plan = exact_minimum_plan(&affected_claim_refs, &scenario.actions)?;

    Ok(GateReport {
        schema_version: "protect-capability-assurance-e5/0.1",
        claim_ceiling: CLAIM_CEILING.to_owned(),
        disposition: if plan.feasible {
            "ABSTAIN_REQUALIFICATION_NOT_EXECUTED".to_owned()
        } else {
            "ABSTAIN_REQUALIFICATION_PLAN_INFEASIBLE".to_owned()
        },
        as_of: scenario.as_of.clone(),
        scenario_sha256: hash(scenario_bytes),
        asset_count: scenario.assets.len(),
        claim_count: scenario.claims.len(),
        evidence_count: scenario.evidence.len(),
        change_count: scenario.changes.len(),
        action_catalog_count: scenario.actions.len(),
        affected_claim_ids,
        qualified_claim_ids,
        evidence_results,
        claim_results,
        requalification_plan: plan,
        executed_actions: false,
        qualification_promoted: false,
    })
}

fn run() -> Result<GateReport, String> {
    let scenario = parse_scenario(SCENARIO)?;
    let report = evaluate(&scenario, SCENARIO)?;
    let expected = &scenario.oracle_expected;
    if report.affected_claim_ids != expected.affected_claim_ids
        || report.qualified_claim_ids != expected.qualified_claim_ids
        || report.requalification_plan.selected_action_ids != expected.selected_action_ids
        || report.requalification_plan.total_cost_cents != Some(expected.selected_cost_cents)
    {
        return Err("E5 evaluated impact/plan differs from the frozen oracle expectations".to_owned());
    }
    for (evidence_id, expected_state) in &expected.evidence_states {
        let actual = report
            .evidence_results
            .iter()
            .find(|item| &item.evidence_id == evidence_id)
            .ok_or_else(|| format!("E5 oracle evidence is missing: {evidence_id}"))?;
        if &actual.effective_state != expected_state {
            return Err(format!(
                "E5 evidence state mismatch for {evidence_id}: expected {expected_state}, got {}",
                actual.effective_state
            ));
        }
    }
    Ok(report)
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("E5 report serialization")
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

    fn scenario() -> Scenario {
        parse_scenario(SCENARIO).expect("frozen E5 schema and scenario")
    }

    fn report(scenario: &Scenario) -> GateReport {
        let bytes = serde_json::to_vec(scenario).expect("scenario serialization");
        evaluate(scenario, &bytes).expect("E5 evaluation")
    }

    #[test]
    fn frozen_scenario_matches_expected_impact_and_minimum_plan() {
        let result = run().expect("valid E5 gate");
        assert_eq!(
            result.affected_claim_ids,
            vec!["claim:air", "claim:power", "claim:shelter"]
        );
        assert_eq!(result.qualified_claim_ids, vec!["claim:comms"]);
        assert_eq!(
            result.requalification_plan.selected_action_ids,
            vec!["action:integrated-refuge-requalification"]
        );
        assert_eq!(result.requalification_plan.total_cost_cents, Some(280000));
        assert_eq!(result.disposition, "ABSTAIN_REQUALIFICATION_NOT_EXECUTED");
        assert!(!result.executed_actions);
        assert!(!result.qualification_promoted);
    }

    #[test]
    fn configuration_mismatch_supersedes_old_evidence() {
        let value = scenario();
        let result = report(&value);
        assert_eq!(
            result
                .evidence_results
                .iter()
                .find(|item| item.evidence_id == "evidence:ups-load-rev-a")
                .expect("UPS evidence")
                .effective_state,
            "SUPERSEDED"
        );
    }

    #[test]
    fn exclusive_valid_until_boundary_expires_on_the_boundary_date() {
        let value = scenario();
        let result = report(&value);
        assert_eq!(
            result
                .evidence_results
                .iter()
                .find(|item| item.evidence_id == "evidence:filter-flow")
                .expect("filter evidence")
                .effective_state,
            "EXPIRED"
        );
    }

    #[test]
    fn unrelated_communications_claim_remains_qualified() {
        let value = scenario();
        let result = report(&value);
        let comms = result
            .claim_results
            .iter()
            .find(|item| item.claim_id == "claim:comms")
            .expect("comms claim");
        assert_eq!(comms.current_status, "QUALIFIED");
        assert!(!comms.affected);
    }

    #[test]
    fn invalidation_propagates_through_explicit_shelter_dependencies() {
        let value = scenario();
        let result = report(&value);
        let shelter = result
            .claim_results
            .iter()
            .find(|item| item.claim_id == "claim:shelter")
            .expect("shelter claim");
        assert!(shelter.affected);
        assert!(shelter.reasons.iter().any(|reason| reason.contains("claim:air")));
        assert!(shelter.reasons.iter().any(|reason| reason.contains("claim:power")));
    }

    #[test]
    fn no_dependency_proximity_inference_invalidates_communications() {
        let value = scenario();
        let result = report(&value);
        assert!(!result.affected_claim_ids.contains(&"claim:comms".to_owned()));
    }

    #[test]
    fn integrated_admissible_action_beats_three_separate_requalification_actions() {
        let value = scenario();
        let result = report(&value);
        let separate_cost = 120000 + 80000 + 150000;
        assert!(result.requalification_plan.total_cost_cents.unwrap() < separate_cost);
        assert_eq!(result.requalification_plan.total_cost_cents, Some(280000));
    }

    #[test]
    fn self_reported_action_cannot_satisfy_independent_evidence_floor() {
        let mut value = scenario();
        value.actions = vec![value
            .actions
            .iter()
            .find(|item| item.id == "action:self-certified-suite")
            .expect("self-certified candidate")
            .clone()];
        let result = report(&value);
        assert!(!result.requalification_plan.feasible);
        assert_eq!(result.requalification_plan.total_cost_cents, None);
    }

    #[test]
    fn visual_only_action_cannot_satisfy_power_air_and_integrated_methods() {
        let mut value = scenario();
        value.actions = vec![value
            .actions
            .iter()
            .find(|item| item.id == "action:visual-walkthrough")
            .expect("visual candidate")
            .clone()];
        let result = report(&value);
        assert!(!result.requalification_plan.feasible);
    }

    #[test]
    fn unknown_evidence_state_never_qualifies_a_claim() {
        let mut value = scenario();
        let evidence = value
            .evidence
            .iter_mut()
            .find(|item| item.id == "evidence:radio-failover")
            .expect("radio evidence");
        evidence.status = "UNKNOWN".to_owned();
        let result = report(&value);
        assert!(!result.qualified_claim_ids.contains(&"claim:comms".to_owned()));
        assert!(result.affected_claim_ids.contains(&"claim:comms".to_owned()));
    }

    #[test]
    fn future_evidence_is_not_yet_valid_and_cannot_qualify() {
        let mut value = scenario();
        let evidence = value
            .evidence
            .iter_mut()
            .find(|item| item.id == "evidence:radio-failover")
            .expect("radio evidence");
        evidence.valid_from = "2026-11-17".to_owned();
        let result = report(&value);
        assert_eq!(
            result
                .evidence_results
                .iter()
                .find(|item| item.evidence_id == "evidence:radio-failover")
                .expect("radio evidence result")
                .effective_state,
            "NOT_YET_VALID"
        );
        assert!(!result.qualified_claim_ids.contains(&"claim:comms".to_owned()));
    }

    #[test]
    fn changed_asset_with_wrong_target_digest_is_rejected() {
        let mut value = scenario();
        value.changes[0].to_digest =
            Some("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_owned());
        assert!(validate_graph(&value).expect_err("invalid change digest").contains("does not match"));
    }

    #[test]
    fn claim_dependency_cycles_are_rejected() {
        let mut value = scenario();
        value
            .claims
            .iter_mut()
            .find(|item| item.id == "claim:power")
            .expect("power claim")
            .depends_on_claims
            .push("claim:shelter".to_owned());
        assert!(validate_graph(&value).expect_err("dependency cycle").contains("cycle"));
    }

    #[test]
    fn dangling_claim_evidence_reference_is_rejected() {
        let mut value = scenario();
        value
            .claims
            .iter_mut()
            .find(|item| item.id == "claim:power")
            .expect("power claim")
            .evidence_ids[0] = "evidence:missing".to_owned();
        assert!(validate_graph(&value).expect_err("missing evidence").contains("unknown evidence"));
    }

    #[test]
    fn duplicate_asset_ids_are_rejected() {
        let mut value = scenario();
        value.assets.push(value.assets[0].clone());
        assert!(validate_graph(&value).expect_err("duplicate asset").contains("duplicate ID"));
    }

    #[test]
    fn safety_floor_is_mandatory_and_cannot_be_disabled() {
        let mut value = scenario();
        value.claims[0].mandatory_safety_floor = false;
        assert!(validate_graph(&value).expect_err("safety floor removed").contains("mandatory safety floor"));
    }

    #[test]
    fn deterministic_replay_returns_identical_report() {
        let value = scenario();
        assert_eq!(report(&value), report(&value));
    }

    #[test]
    fn change_event_after_as_of_is_rejected() {
        let mut value = scenario();
        value.changes[0].occurred_at = "2026-11-17T00:00:00Z".to_owned();
        assert!(validate_graph(&value).expect_err("future change").contains("after scenario as_of"));
    }

    #[test]
    fn unknown_change_event_type_fails_closed() {
        let mut value = scenario();
        value.changes[0].event_type = "UNKNOWN_CHANGE".to_owned();
        assert!(validate_graph(&value).expect_err("unknown change type").contains("unsupported E5 change"));
    }

    #[test]
    fn every_selected_action_respects_each_affected_claim_method_and_independence_floor() {
        let value = scenario();
        let result = report(&value);
        let selected = &result.requalification_plan.selected_action_ids;
        for claim_id in &result.affected_claim_ids {
            let claim = value.claims.iter().find(|item| &item.id == claim_id).expect("claim");
            assert!(selected.iter().any(|action_id| {
                let action = value.actions.iter().find(|item| &item.id == action_id).expect("action");
                action_covers_claim(action, claim)
            }));
        }
    }
}
