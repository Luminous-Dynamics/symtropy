#![forbid(unsafe_code)]

//! Tiny exact non-anticipative resource pre-positioning policy oracle.
//!
//! v0.1 deliberately models one homogeneous resource, two service sites, one
//! pre-observation commitment stage, and one post-observation delivery stage.
//! It distinguishes scenario-specific full recourse from a shared
//! here-and-now pre-positioning decision. It is a finite reference model, not
//! a general network-flow, stochastic-programming or dispatch engine.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const NONANTICIPATIVE_POLICY_SCHEMA: &str = "safrb-nonanticipative-preposition-policy-v0.1";
pub const POLICY_TREE_SCHEMA: &str = "safrb-two-stage-resource-tree-v0.1";
pub const FULL_RECOURSE_ASSUMPTION: &str = "scenario_observed_before_zero_time_repositioning";
pub const POLICY_CLAIM_CEILING: &str = "Exact finite two-site, one-resource, two-stage non-anticipative pre-positioning result for the supplied synthetic scenario tree only. Full recourse is an information-relaxation bound that assumes scenario observation before repositioning and zero travel time, relocation cost and movement constraints; it is not a physical dispatch-feasibility claim. No scenario-set completeness, evidence authority, probabilities, physical asset qualification, dispatch authorization or actual beneficiary-service claim.";

const MAX_SCENARIOS: usize = 256;
const MAX_RESOURCE_UNITS: u64 = 512;
const ROOT_STAGE: u64 = 0;
const DELIVERY_STAGE: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredSite {
    SiteA,
    SiteB,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDecisionNode {
    pub decision_node_id: String,
    pub scenario_id: String,
    pub stage: u64,
    pub information_set_id: String,
    /// Hash of only the observations and public state available at this cut.
    /// Hidden scenario identifiers must never be included before revelation.
    pub observed_history_sha256: String,
    pub required_site: Option<RequiredSite>,
    pub minimum_service_quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyScenarioPath {
    pub scenario_id: String,
    /// Exactly [pre-observation decision, post-observation delivery decision].
    pub decision_node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonanticipativePolicyTree {
    pub schema_version: String,
    pub tree_id: String,
    pub resource_ref: String,
    pub resource_unit_ref: String,
    pub site_a_ref: String,
    pub site_b_ref: String,
    pub available_resource_units: u64,
    pub nodes: Vec<PolicyDecisionNode>,
    pub paths: Vec<PolicyScenarioPath>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyAction {
    pub decision_node_id: String,
    /// Only stage-zero actions may pre-position resources.
    pub preposition_site_a: u64,
    pub preposition_site_b: u64,
    /// Only stage-one actions may deliver resources after the observation.
    pub deliver_site_a: u64,
    pub deliver_site_b: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonanticipativePolicyPlan {
    pub actions: Vec<PolicyAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NonanticipativePolicyDisposition {
    HereAndNowFeasible,
    FullRecourseOnly,
    NotFeasibleWithinSuppliedTree,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonanticipativePolicyReport {
    pub schema_version: String,
    pub tree_id: String,
    pub tree_sha256: String,
    pub disposition: NonanticipativePolicyDisposition,
    pub here_and_now_feasible: bool,
    pub full_recourse_feasible: bool,
    /// Explicit idealized recourse assumption; see FULL_RECOURSE_ASSUMPTION.
    pub full_recourse_assumption: String,
    /// Minimum resource count needed by one allocation shared across all leaves.
    pub minimum_common_preposition_units: u64,
    /// Minimum resource count needed when each scenario may choose after revelation.
    pub minimum_full_recourse_units: u64,
    /// Resource units avoided by scenario-specific recourse in this model.
    /// This is a physical-unit difference, not a monetary value of information.
    pub resource_units_saved_by_recourse: u64,
    pub preposition_allocation_space_size: u64,
    pub policy_sha256: Option<String>,
    pub plan: Option<NonanticipativePolicyPlan>,
    pub claim_ceiling: String,
}

fn require_nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn push_length_prefixed(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    let len = u64::try_from(value.len())
        .map_err(|_| "policy digest field exceeds u64 length".to_string())?;
    bytes.extend_from_slice(&len.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn canonical_tree_bytes(tree: &NonanticipativePolicyTree) -> Result<Vec<u8>, String> {
    let mut canonical = tree.clone();
    canonical
        .nodes
        .sort_by(|left, right| left.decision_node_id.cmp(&right.decision_node_id));
    canonical
        .paths
        .sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("could not encode canonical policy tree: {error}"))
}

fn tree_digest(tree: &NonanticipativePolicyTree) -> Result<String, String> {
    let mut bytes = b"LUMINOUS-SAFRB-NONANTICIPATIVE-TREE-V1\0".to_vec();
    let canonical = canonical_tree_bytes(tree)?;
    push_length_prefixed(&mut bytes, &canonical)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn canonical_plan_bytes(plan: &NonanticipativePolicyPlan) -> Result<Vec<u8>, String> {
    let mut canonical = plan.clone();
    canonical
        .actions
        .sort_by(|left, right| left.decision_node_id.cmp(&right.decision_node_id));
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("could not encode canonical policy plan: {error}"))
}

fn plan_digest(plan: &NonanticipativePolicyPlan) -> Result<String, String> {
    let mut bytes = b"LUMINOUS-SAFRB-NONANTICIPATIVE-PLAN-V1\0".to_vec();
    let canonical = canonical_plan_bytes(plan)?;
    push_length_prefixed(&mut bytes, &canonical)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn allocation_space_size(total: u64) -> Result<u64, String> {
    let mut left = total
        .checked_add(1)
        .ok_or_else(|| "pre-positioning allocation count overflowed u64".to_string())?;
    let mut right = total
        .checked_add(2)
        .ok_or_else(|| "pre-positioning allocation count overflowed u64".to_string())?;
    if left % 2 == 0 {
        left /= 2;
    } else {
        right /= 2;
    }
    left.checked_mul(right)
        .ok_or_else(|| "pre-positioning allocation count overflowed u64".to_string())
}

fn validate_tree(
    tree: &NonanticipativePolicyTree,
) -> Result<
    (
        BTreeMap<String, PolicyDecisionNode>,
        BTreeMap<String, (PolicyDecisionNode, PolicyDecisionNode)>,
    ),
    String,
> {
    require_nonempty(&tree.tree_id, "tree_id")?;
    require_nonempty(&tree.resource_ref, "resource_ref")?;
    require_nonempty(&tree.resource_unit_ref, "resource_unit_ref")?;
    require_nonempty(&tree.site_a_ref, "site_a_ref")?;
    require_nonempty(&tree.site_b_ref, "site_b_ref")?;
    if tree.schema_version != POLICY_TREE_SCHEMA {
        return Err("unsupported non-anticipative policy tree schema".to_string());
    }
    if tree.site_a_ref == tree.site_b_ref {
        return Err("policy service sites must have distinct canonical identities".to_string());
    }
    if tree.available_resource_units > MAX_RESOURCE_UNITS {
        return Err(format!(
            "resource units exceed exact oracle limit {MAX_RESOURCE_UNITS}"
        ));
    }
    if tree.paths.is_empty() || tree.paths.len() > MAX_SCENARIOS {
        return Err(format!("scenario count must be within 1..={MAX_SCENARIOS}"));
    }
    if tree.nodes.len() != tree.paths.len().saturating_mul(2) {
        return Err("v0.1 tree must have exactly two decision nodes per scenario".to_string());
    }

    let mut nodes = BTreeMap::new();
    let mut information_by_id = BTreeMap::<String, (u64, String)>::new();
    let mut information_by_observation = BTreeMap::<(u64, String), String>::new();

    for node in &tree.nodes {
        require_nonempty(&node.decision_node_id, "decision_node_id")?;
        require_nonempty(&node.scenario_id, "scenario_id")?;
        require_nonempty(&node.information_set_id, "information_set_id")?;
        if !is_sha256(&node.observed_history_sha256) {
            return Err(format!(
                "node {} has invalid observable-history digest",
                node.decision_node_id
            ));
        }
        if nodes
            .insert(node.decision_node_id.clone(), node.clone())
            .is_some()
        {
            return Err(format!(
                "duplicate policy decision node {}",
                node.decision_node_id
            ));
        }

        let observation_key = (node.stage, node.observed_history_sha256.clone());
        if let Some(existing) = information_by_id.get(&node.information_set_id) {
            if existing != &(node.stage, node.observed_history_sha256.clone()) {
                return Err(format!(
                    "information set {} combines different observable histories",
                    node.information_set_id
                ));
            }
        } else {
            information_by_id.insert(
                node.information_set_id.clone(),
                (node.stage, node.observed_history_sha256.clone()),
            );
        }
        if let Some(existing) = information_by_observation.get(&observation_key) {
            if existing != &node.information_set_id {
                return Err(
                    "identical observable histories were split across information sets".to_string(),
                );
            }
        } else {
            information_by_observation.insert(observation_key, node.information_set_id.clone());
        }

        match node.stage {
            ROOT_STAGE => {
                if node.required_site.is_some() || node.minimum_service_quantity != 0 {
                    return Err(format!(
                        "pre-observation node {} must not reveal service demand",
                        node.decision_node_id
                    ));
                }
            }
            DELIVERY_STAGE => {
                if node.required_site.is_none() || node.minimum_service_quantity == 0 {
                    return Err(format!(
                        "post-observation node {} must declare a positive service requirement",
                        node.decision_node_id
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "stage {} is unsupported by the two-stage v0.1 policy model",
                    node.stage
                ));
            }
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut referenced_nodes = BTreeSet::new();
    let mut paths = BTreeMap::new();
    let mut root_information_set: Option<(String, String)> = None;
    for path in &tree.paths {
        require_nonempty(&path.scenario_id, "scenario_id")?;
        if !scenario_ids.insert(path.scenario_id.as_str()) {
            return Err(format!("duplicate scenario path {}", path.scenario_id));
        }
        if path.decision_node_ids.len() != 2 {
            return Err(format!(
                "scenario {} must contain one root and one delivery decision",
                path.scenario_id
            ));
        }
        let root = nodes
            .get(&path.decision_node_ids[0])
            .ok_or_else(|| format!("scenario {} references unknown root node", path.scenario_id))?;
        let leaf = nodes.get(&path.decision_node_ids[1]).ok_or_else(|| {
            format!(
                "scenario {} references unknown delivery node",
                path.scenario_id
            )
        })?;
        if root.scenario_id != path.scenario_id
            || leaf.scenario_id != path.scenario_id
            || root.stage != ROOT_STAGE
            || leaf.stage != DELIVERY_STAGE
        {
            return Err(format!(
                "scenario {} has inconsistent node identity or stage order",
                path.scenario_id
            ));
        }
        if !referenced_nodes.insert(root.decision_node_id.as_str())
            || !referenced_nodes.insert(leaf.decision_node_id.as_str())
        {
            return Err("a decision node is reused by multiple scenario paths".to_string());
        }
        let root_info = (
            root.information_set_id.clone(),
            root.observed_history_sha256.clone(),
        );
        if let Some(expected) = &root_information_set {
            if expected != &root_info {
                return Err(
                    "all root decision copies must share one pre-observation information set"
                        .to_string(),
                );
            }
        } else {
            root_information_set = Some(root_info);
        }
        paths.insert(path.scenario_id.clone(), (root.clone(), leaf.clone()));
    }
    if referenced_nodes.len() != tree.nodes.len() {
        return Err("policy tree contains unreferenced decision nodes".to_string());
    }

    Ok((nodes, paths))
}

fn minimum_resource_requirements(
    paths: &BTreeMap<String, (PolicyDecisionNode, PolicyDecisionNode)>,
) -> Result<(u64, u64), String> {
    let mut maximum_a = 0_u64;
    let mut maximum_b = 0_u64;
    let mut maximum_leaf = 0_u64;
    for (_, leaf) in paths.values() {
        maximum_leaf = maximum_leaf.max(leaf.minimum_service_quantity);
        match leaf.required_site {
            Some(RequiredSite::SiteA) => {
                maximum_a = maximum_a.max(leaf.minimum_service_quantity);
            }
            Some(RequiredSite::SiteB) => {
                maximum_b = maximum_b.max(leaf.minimum_service_quantity);
            }
            None => return Err("delivery node omitted required site".to_string()),
        }
    }
    let common = maximum_a
        .checked_add(maximum_b)
        .ok_or_else(|| "common pre-positioning requirement overflowed u64".to_string())?;
    Ok((common, maximum_leaf))
}

fn build_plan(
    paths: &BTreeMap<String, (PolicyDecisionNode, PolicyDecisionNode)>,
    reserve_a: u64,
    reserve_b: u64,
) -> Result<NonanticipativePolicyPlan, String> {
    let mut actions = Vec::with_capacity(paths.len().saturating_mul(2));
    for (root, leaf) in paths.values() {
        actions.push(PolicyAction {
            decision_node_id: root.decision_node_id.clone(),
            preposition_site_a: reserve_a,
            preposition_site_b: reserve_b,
            deliver_site_a: 0,
            deliver_site_b: 0,
        });
        let (deliver_a, deliver_b) = match leaf.required_site {
            Some(RequiredSite::SiteA) => (leaf.minimum_service_quantity, 0),
            Some(RequiredSite::SiteB) => (0, leaf.minimum_service_quantity),
            None => return Err("delivery node omitted required site".to_string()),
        };
        actions.push(PolicyAction {
            decision_node_id: leaf.decision_node_id.clone(),
            preposition_site_a: 0,
            preposition_site_b: 0,
            deliver_site_a: deliver_a,
            deliver_site_b: deliver_b,
        });
    }
    actions.sort_by(|left, right| left.decision_node_id.cmp(&right.decision_node_id));
    Ok(NonanticipativePolicyPlan { actions })
}

fn validate_policy_plan(
    tree: &NonanticipativePolicyTree,
    nodes: &BTreeMap<String, PolicyDecisionNode>,
    paths: &BTreeMap<String, (PolicyDecisionNode, PolicyDecisionNode)>,
    plan: &NonanticipativePolicyPlan,
) -> Result<(), String> {
    if plan.actions.len() != nodes.len() {
        return Err("policy action count does not cover every decision node".to_string());
    }
    let mut actions = BTreeMap::new();
    let mut information_actions = BTreeMap::<String, PolicyAction>::new();
    for action in &plan.actions {
        let node = nodes.get(&action.decision_node_id).ok_or_else(|| {
            format!(
                "policy references unknown decision node {}",
                action.decision_node_id
            )
        })?;
        if actions
            .insert(action.decision_node_id.as_str(), action)
            .is_some()
        {
            return Err(format!(
                "duplicate action for decision node {}",
                action.decision_node_id
            ));
        }
        if let Some(existing) = information_actions.get(&node.information_set_id) {
            let same_decision = existing.preposition_site_a == action.preposition_site_a
                && existing.preposition_site_b == action.preposition_site_b
                && existing.deliver_site_a == action.deliver_site_a
                && existing.deliver_site_b == action.deliver_site_b;
            if !same_decision {
                return Err(format!(
                    "non-anticipativity violation in information set {}",
                    node.information_set_id
                ));
            }
        } else {
            information_actions.insert(node.information_set_id.clone(), action.clone());
        }
    }

    for (root, leaf) in paths.values() {
        let root_action = actions
            .get(root.decision_node_id.as_str())
            .ok_or_else(|| "policy omitted a root action".to_string())?;
        let leaf_action = actions
            .get(leaf.decision_node_id.as_str())
            .ok_or_else(|| "policy omitted a delivery action".to_string())?;

        if root_action.deliver_site_a != 0 || root_action.deliver_site_b != 0 {
            return Err("pre-observation root action cannot deliver service".to_string());
        }
        let root_total = root_action
            .preposition_site_a
            .checked_add(root_action.preposition_site_b)
            .ok_or_else(|| "pre-positioning resource sum overflowed u64".to_string())?;
        if root_total > tree.available_resource_units {
            return Err("pre-positioning action exceeds available resource units".to_string());
        }

        if leaf_action.preposition_site_a != 0 || leaf_action.preposition_site_b != 0 {
            return Err(
                "post-observation recourse cannot change the committed pre-position".to_string(),
            );
        }
        if leaf_action.deliver_site_a > root_action.preposition_site_a
            || leaf_action.deliver_site_b > root_action.preposition_site_b
        {
            return Err("delivery exceeds pre-positioned resource at a service site".to_string());
        }
        let delivered = match leaf.required_site {
            Some(RequiredSite::SiteA) => leaf_action.deliver_site_a,
            Some(RequiredSite::SiteB) => leaf_action.deliver_site_b,
            None => return Err("delivery node omitted required site".to_string()),
        };
        if delivered < leaf.minimum_service_quantity {
            return Err(format!(
                "scenario {} fails its service floor",
                leaf.scenario_id
            ));
        }
    }
    Ok(())
}

/// Enumerate all integer pre-positioning allocations for this tiny two-site
/// model. Scenario-specific delivery is chosen only after the observable
/// branch is known. The root allocation is common to every scenario.
pub fn solve_nonanticipative_policy(
    tree: &NonanticipativePolicyTree,
) -> Result<NonanticipativePolicyReport, String> {
    let (nodes, paths) = validate_tree(tree)?;
    let digest = tree_digest(tree)?;
    let space_size = allocation_space_size(tree.available_resource_units)?;
    let (minimum_common, minimum_full_recourse) = minimum_resource_requirements(&paths)?;
    let full_recourse = minimum_full_recourse <= tree.available_resource_units;

    let mut plan = None;
    'search: for reserve_a in 0..=tree.available_resource_units {
        for reserve_b in 0..=tree.available_resource_units - reserve_a {
            let satisfies_every_leaf = paths.values().all(|(_, leaf)| match leaf.required_site {
                Some(RequiredSite::SiteA) => reserve_a >= leaf.minimum_service_quantity,
                Some(RequiredSite::SiteB) => reserve_b >= leaf.minimum_service_quantity,
                None => false,
            });
            if satisfies_every_leaf {
                plan = Some(build_plan(&paths, reserve_a, reserve_b)?);
                break 'search;
            }
        }
    }

    let here_and_now = plan.is_some();
    let disposition = if here_and_now {
        NonanticipativePolicyDisposition::HereAndNowFeasible
    } else if full_recourse {
        NonanticipativePolicyDisposition::FullRecourseOnly
    } else {
        NonanticipativePolicyDisposition::NotFeasibleWithinSuppliedTree
    };
    let policy_sha256 = plan.as_ref().map(plan_digest).transpose()?;
    let resource_units_saved_by_recourse = minimum_common
        .checked_sub(minimum_full_recourse)
        .ok_or_else(|| "full-recourse requirement exceeded common commitment bound".to_string())?;
    let report = NonanticipativePolicyReport {
        schema_version: NONANTICIPATIVE_POLICY_SCHEMA.to_string(),
        tree_id: tree.tree_id.clone(),
        tree_sha256: digest,
        disposition,
        here_and_now_feasible: here_and_now,
        full_recourse_feasible: full_recourse,
        full_recourse_assumption: FULL_RECOURSE_ASSUMPTION.to_string(),
        minimum_common_preposition_units: minimum_common,
        minimum_full_recourse_units: minimum_full_recourse,
        resource_units_saved_by_recourse,
        preposition_allocation_space_size: space_size,
        policy_sha256,
        plan,
        claim_ceiling: POLICY_CLAIM_CEILING.to_string(),
    };
    verify_nonanticipative_policy_report(tree, &report)?;
    Ok(report)
}

type ReconstructedPolicyTree = (
    BTreeMap<String, PolicyDecisionNode>,
    BTreeMap<String, (PolicyDecisionNode, PolicyDecisionNode)>,
);

fn verifier_reconstruct_tree(
    tree: &NonanticipativePolicyTree,
) -> Result<ReconstructedPolicyTree, String> {
    require_nonempty(&tree.tree_id, "tree_id")?;
    require_nonempty(&tree.resource_ref, "resource_ref")?;
    require_nonempty(&tree.resource_unit_ref, "resource_unit_ref")?;
    require_nonempty(&tree.site_a_ref, "site_a_ref")?;
    require_nonempty(&tree.site_b_ref, "site_b_ref")?;
    if tree.schema_version != POLICY_TREE_SCHEMA {
        return Err("verifier: unsupported policy-tree schema".to_string());
    }
    if tree.site_a_ref == tree.site_b_ref
        || tree.available_resource_units > MAX_RESOURCE_UNITS
        || tree.paths.is_empty()
        || tree.paths.len() > MAX_SCENARIOS
        || tree.nodes.len() != tree.paths.len().saturating_mul(2)
    {
        return Err("verifier: policy-tree shape or resource bounds are invalid".to_string());
    }

    let mut node_index = BTreeMap::new();
    let mut by_information = BTreeMap::<String, (u64, String)>::new();
    let mut by_observation = BTreeMap::<(u64, String), String>::new();
    for node in &tree.nodes {
        require_nonempty(&node.decision_node_id, "decision_node_id")?;
        require_nonempty(&node.scenario_id, "scenario_id")?;
        require_nonempty(&node.information_set_id, "information_set_id")?;
        if !is_sha256(&node.observed_history_sha256) {
            return Err(format!(
                "verifier: invalid observation-history digest at {}",
                node.decision_node_id
            ));
        }
        if node_index
            .insert(node.decision_node_id.clone(), node.clone())
            .is_some()
        {
            return Err("verifier: duplicate decision-node identity".to_string());
        }
        match (
            node.stage,
            node.required_site,
            node.minimum_service_quantity,
        ) {
            (ROOT_STAGE, None, 0) => {}
            (DELIVERY_STAGE, Some(_), quantity) if quantity > 0 => {}
            _ => return Err("verifier: invalid stage-specific decision-node fields".to_string()),
        }

        let history = (node.stage, node.observed_history_sha256.clone());
        if let Some(bound_history) = by_information.get(&node.information_set_id) {
            if bound_history != &history {
                return Err("verifier: information set merges distinct histories".to_string());
            }
        } else {
            by_information.insert(node.information_set_id.clone(), history.clone());
        }
        if let Some(bound_information) = by_observation.get(&history) {
            if bound_information != &node.information_set_id {
                return Err(
                    "verifier: one observable history has multiple information sets".to_string(),
                );
            }
        } else {
            by_observation.insert(history, node.information_set_id.clone());
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut used_node_ids = BTreeSet::new();
    let mut path_index = BTreeMap::new();
    let mut root_information: Option<(String, String)> = None;
    for path in &tree.paths {
        require_nonempty(&path.scenario_id, "scenario_id")?;
        if !scenario_ids.insert(path.scenario_id.as_str()) || path.decision_node_ids.len() != 2 {
            return Err("verifier: duplicate scenario or unsupported path length".to_string());
        }
        let root = node_index
            .get(&path.decision_node_ids[0])
            .ok_or_else(|| "verifier: path references missing root node".to_string())?;
        let leaf = node_index
            .get(&path.decision_node_ids[1])
            .ok_or_else(|| "verifier: path references missing delivery node".to_string())?;
        if root.scenario_id != path.scenario_id
            || leaf.scenario_id != path.scenario_id
            || root.stage != ROOT_STAGE
            || leaf.stage != DELIVERY_STAGE
        {
            return Err("verifier: path scenario identity or decision order mismatch".to_string());
        }
        if !used_node_ids.insert(root.decision_node_id.as_str())
            || !used_node_ids.insert(leaf.decision_node_id.as_str())
        {
            return Err("verifier: decision node appears on more than one path".to_string());
        }
        let root_information_pair = (
            root.information_set_id.clone(),
            root.observed_history_sha256.clone(),
        );
        match &root_information {
            Some(existing) if existing != &root_information_pair => {
                return Err(
                    "verifier: pre-event decisions do not share one information set".to_string(),
                );
            }
            None => root_information = Some(root_information_pair),
            _ => {}
        }
        path_index.insert(path.scenario_id.clone(), (root.clone(), leaf.clone()));
    }
    if used_node_ids.len() != node_index.len() {
        return Err("verifier: unreferenced decision nodes present".to_string());
    }
    Ok((node_index, path_index))
}

fn verifier_resource_thresholds(
    paths: &BTreeMap<String, (PolicyDecisionNode, PolicyDecisionNode)>,
) -> Result<(u64, u64), String> {
    let mut site_a_requirement = 0_u64;
    let mut site_b_requirement = 0_u64;
    let mut largest_leaf_requirement = 0_u64;
    for pair in paths.values() {
        let leaf = &pair.1;
        largest_leaf_requirement =
            std::cmp::max(largest_leaf_requirement, leaf.minimum_service_quantity);
        if leaf.required_site == Some(RequiredSite::SiteA) {
            site_a_requirement = std::cmp::max(site_a_requirement, leaf.minimum_service_quantity);
        } else if leaf.required_site == Some(RequiredSite::SiteB) {
            site_b_requirement = std::cmp::max(site_b_requirement, leaf.minimum_service_quantity);
        } else {
            return Err("verifier: delivery node has no site".to_string());
        }
    }
    let common_requirement = site_a_requirement
        .checked_add(site_b_requirement)
        .ok_or_else(|| "verifier: common resource requirement overflowed".to_string())?;
    Ok((common_requirement, largest_leaf_requirement))
}

fn verifier_count_preposition_allocations(total: u64) -> Result<u64, String> {
    let mut count = 0_u64;
    for at_a in 0..=total {
        let remaining = total
            .checked_sub(at_a)
            .ok_or_else(|| "verifier: resource split subtraction underflowed".to_string())?;
        for _at_b in 0..=remaining {
            count = count
                .checked_add(1)
                .ok_or_else(|| "verifier: allocation count overflowed".to_string())?;
        }
    }
    Ok(count)
}

/// Independently verify the finite policy result. The here-and-now feasibility
/// check uses the analytical maximum required at each service site; it does
/// not reuse the producer's integer allocation enumeration.
pub fn verify_nonanticipative_policy_report(
    tree: &NonanticipativePolicyTree,
    report: &NonanticipativePolicyReport,
) -> Result<(), String> {
    // Deliberately rebuild node and information-set maps independently of
    // the producer's validate_tree path.
    let (nodes, paths) = verifier_reconstruct_tree(tree)?;
    if report.schema_version != NONANTICIPATIVE_POLICY_SCHEMA {
        return Err("unsupported non-anticipative policy report schema".to_string());
    }
    if report.claim_ceiling != POLICY_CLAIM_CEILING {
        return Err("non-anticipative policy claim ceiling mismatch".to_string());
    }
    if report.full_recourse_assumption != FULL_RECOURSE_ASSUMPTION {
        return Err("non-anticipative policy full-recourse assumption mismatch".to_string());
    }
    if report.tree_id != tree.tree_id || report.tree_sha256 != tree_digest(tree)? {
        return Err("non-anticipative policy tree identity/digest mismatch".to_string());
    }
    let expected_space = verifier_count_preposition_allocations(tree.available_resource_units)?;
    let (expected_common_units, expected_full_recourse_units) =
        verifier_resource_thresholds(&paths)?;
    let expected_full_recourse = expected_full_recourse_units <= tree.available_resource_units;
    let expected_here_and_now = expected_common_units <= tree.available_resource_units;
    if report.preposition_allocation_space_size != expected_space
        || report.minimum_common_preposition_units != expected_common_units
        || report.minimum_full_recourse_units != expected_full_recourse_units
        || report.resource_units_saved_by_recourse
            != expected_common_units
                .checked_sub(expected_full_recourse_units)
                .ok_or_else(|| {
                    "verifier full-recourse requirement exceeds common commitment bound".to_string()
                })?
        || report.full_recourse_feasible != expected_full_recourse
        || report.here_and_now_feasible != expected_here_and_now
    {
        return Err("non-anticipative policy feasibility summary mismatch".to_string());
    }

    let expected_disposition = if expected_here_and_now {
        NonanticipativePolicyDisposition::HereAndNowFeasible
    } else if expected_full_recourse {
        NonanticipativePolicyDisposition::FullRecourseOnly
    } else {
        NonanticipativePolicyDisposition::NotFeasibleWithinSuppliedTree
    };
    if report.disposition != expected_disposition {
        return Err("non-anticipative policy disposition mismatch".to_string());
    }

    match (&report.plan, &report.policy_sha256) {
        (Some(plan), Some(digest)) => {
            if !expected_here_and_now || *digest != plan_digest(plan)? {
                return Err("policy plan digest or feasibility claim mismatch".to_string());
            }
            validate_policy_plan(tree, &nodes, &paths, plan)?;
        }
        (None, None) => {
            if expected_here_and_now {
                return Err(
                    "feasible here-and-now result omitted its constructive policy".to_string(),
                );
            }
        }
        _ => return Err("policy plan and digest must be present or absent together".to_string()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(value: u64) -> String {
        format!("sha256:{value:064x}")
    }

    fn tree(resource_units: u64, sites: &[(RequiredSite, u64)]) -> NonanticipativePolicyTree {
        let mut nodes = Vec::new();
        let mut paths = Vec::new();
        for (index, (site, quantity)) in sites.iter().enumerate() {
            let scenario_id = format!("scenario-{index}");
            let root_id = format!("root-{index}");
            let leaf_id = format!("leaf-{index}");
            nodes.push(PolicyDecisionNode {
                decision_node_id: root_id.clone(),
                scenario_id: scenario_id.clone(),
                stage: ROOT_STAGE,
                information_set_id: "before-event".to_string(),
                observed_history_sha256: hash(0),
                required_site: None,
                minimum_service_quantity: 0,
            });
            nodes.push(PolicyDecisionNode {
                decision_node_id: leaf_id.clone(),
                scenario_id: scenario_id.clone(),
                stage: DELIVERY_STAGE,
                information_set_id: format!("after-event-{index}"),
                observed_history_sha256: hash(index as u64 + 1),
                required_site: Some(*site),
                minimum_service_quantity: *quantity,
            });
            paths.push(PolicyScenarioPath {
                scenario_id,
                decision_node_ids: vec![root_id, leaf_id],
            });
        }
        NonanticipativePolicyTree {
            schema_version: POLICY_TREE_SCHEMA.to_string(),
            tree_id: "synthetic-preposition-tree".to_string(),
            resource_ref: "synthetic:mobile-resource".to_string(),
            resource_unit_ref: "unit".to_string(),
            site_a_ref: "synthetic:site-a".to_string(),
            site_b_ref: "synthetic:site-b".to_string(),
            available_resource_units: resource_units,
            nodes,
            paths,
        }
    }

    #[test]
    fn full_recourse_can_be_feasible_when_no_common_prepositioning_policy_exists() {
        let model = tree(1, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let report = solve_nonanticipative_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            NonanticipativePolicyDisposition::FullRecourseOnly
        );
        assert!(report.full_recourse_feasible);
        assert!(!report.here_and_now_feasible);
        assert_eq!(report.minimum_common_preposition_units, 2);
        assert_eq!(report.minimum_full_recourse_units, 1);
        assert_eq!(report.resource_units_saved_by_recourse, 1);
        assert_eq!(report.full_recourse_assumption, FULL_RECOURSE_ASSUMPTION);
        assert!(report.plan.is_none());
        verify_nonanticipative_policy_report(&model, &report).unwrap();
    }

    #[test]
    fn common_prepositioning_policy_exists_when_resource_covers_both_sites() {
        let model = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let report = solve_nonanticipative_policy(&model).unwrap();
        assert_eq!(
            report.disposition,
            NonanticipativePolicyDisposition::HereAndNowFeasible
        );
        assert!(report.full_recourse_feasible);
        assert!(report.here_and_now_feasible);
        assert_eq!(report.minimum_common_preposition_units, 2);
        assert_eq!(report.minimum_full_recourse_units, 1);
        let plan = report.plan.as_ref().unwrap();
        verify_nonanticipative_policy_report(&model, &report).unwrap();
        assert!(
            plan.actions
                .iter()
                .filter(|a| a.preposition_site_a == 1)
                .count()
                > 0
        );
    }

    #[test]
    fn verifier_rejects_clairvoyant_root_actions_in_one_information_set() {
        let model = tree(1, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let roots = model
            .nodes
            .iter()
            .filter(|node| node.stage == ROOT_STAGE)
            .collect::<Vec<_>>();
        let leaves = model
            .nodes
            .iter()
            .filter(|node| node.stage == DELIVERY_STAGE)
            .collect::<Vec<_>>();
        let plan = NonanticipativePolicyPlan {
            actions: vec![
                PolicyAction {
                    decision_node_id: roots[0].decision_node_id.clone(),
                    preposition_site_a: 1,
                    preposition_site_b: 0,
                    deliver_site_a: 0,
                    deliver_site_b: 0,
                },
                PolicyAction {
                    decision_node_id: roots[1].decision_node_id.clone(),
                    preposition_site_a: 0,
                    preposition_site_b: 1,
                    deliver_site_a: 0,
                    deliver_site_b: 0,
                },
                PolicyAction {
                    decision_node_id: leaves[0].decision_node_id.clone(),
                    preposition_site_a: 0,
                    preposition_site_b: 0,
                    deliver_site_a: 1,
                    deliver_site_b: 0,
                },
                PolicyAction {
                    decision_node_id: leaves[1].decision_node_id.clone(),
                    preposition_site_a: 0,
                    preposition_site_b: 0,
                    deliver_site_a: 0,
                    deliver_site_b: 1,
                },
            ],
        };
        let nodes = model
            .nodes
            .iter()
            .map(|n| (n.decision_node_id.clone(), n.clone()))
            .collect();
        let paths = model
            .paths
            .iter()
            .map(|path| {
                let root = nodes.get(&path.decision_node_ids[0]).unwrap().clone();
                let leaf = nodes.get(&path.decision_node_ids[1]).unwrap().clone();
                (path.scenario_id.clone(), (root, leaf))
            })
            .collect();
        assert!(
            validate_policy_plan(&model, &nodes, &paths, &plan)
                .unwrap_err()
                .contains("non-anticipativity violation")
        );
    }

    #[test]
    fn report_verifier_rejects_clairvoyant_plan_even_with_recomputed_plan_digest() {
        let model = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let mut report = solve_nonanticipative_policy(&model).unwrap();
        let plan = report.plan.as_mut().unwrap();
        let roots = model
            .nodes
            .iter()
            .filter(|node| node.stage == ROOT_STAGE)
            .collect::<Vec<_>>();
        let first = plan
            .actions
            .iter_mut()
            .find(|action| action.decision_node_id == roots[0].decision_node_id)
            .unwrap();
        first.preposition_site_a = 2;
        first.preposition_site_b = 0;
        let second = plan
            .actions
            .iter_mut()
            .find(|action| action.decision_node_id == roots[1].decision_node_id)
            .unwrap();
        second.preposition_site_a = 0;
        second.preposition_site_b = 2;
        report.policy_sha256 = Some(plan_digest(plan).unwrap());

        assert!(
            verify_nonanticipative_policy_report(&model, &report)
                .unwrap_err()
                .contains("non-anticipativity violation")
        );
    }

    #[test]
    fn verifier_rejects_tampered_full_recourse_semantics() {
        let model = tree(1, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let mut report = solve_nonanticipative_policy(&model).unwrap();
        report.full_recourse_assumption = "scenario_aware_but_travel_constrained".to_string();
        assert!(
            verify_nonanticipative_policy_report(&model, &report)
                .unwrap_err()
                .contains("full-recourse assumption mismatch")
        );
    }

    #[test]
    fn equal_observed_histories_must_not_be_split_into_different_information_sets() {
        let mut model = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        model.nodes[1].observed_history_sha256 = hash(42);
        model.nodes[1].information_set_id = "shared-observed-history".to_string();
        model.nodes[3].observed_history_sha256 = hash(42);
        model.nodes[3].information_set_id = "different-info-set".to_string();
        assert!(
            validate_tree(&model)
                .unwrap_err()
                .contains("identical observable histories were split")
        );
    }

    #[test]
    fn one_information_set_must_not_claim_different_observation_history() {
        let mut model = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        model.nodes[3].information_set_id = model.nodes[1].information_set_id.clone();
        assert!(
            validate_tree(&model)
                .unwrap_err()
                .contains("combines different observable histories")
        );
    }

    #[test]
    fn hidden_scenario_label_changes_receipt_but_not_root_policy_action() {
        let original = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let original_report = solve_nonanticipative_policy(&original).unwrap();

        let mut renamed = original.clone();
        for (index, path) in renamed.paths.iter_mut().enumerate() {
            let replacement = format!("opaque-world-{index}");
            let old_id = path.scenario_id.clone();
            path.scenario_id = replacement.clone();
            for node in renamed
                .nodes
                .iter_mut()
                .filter(|node| node.scenario_id == old_id)
            {
                node.scenario_id = replacement.clone();
            }
        }
        let renamed_report = solve_nonanticipative_policy(&renamed).unwrap();
        assert_ne!(original_report.tree_sha256, renamed_report.tree_sha256);

        let root_actions = |report: &NonanticipativePolicyReport,
                            model: &NonanticipativePolicyTree| {
            let root_ids = model
                .nodes
                .iter()
                .filter(|node| node.stage == ROOT_STAGE)
                .map(|node| node.decision_node_id.as_str())
                .collect::<BTreeSet<_>>();
            let mut actions = report
                .plan
                .as_ref()
                .unwrap()
                .actions
                .iter()
                .filter(|action| root_ids.contains(action.decision_node_id.as_str()))
                .map(|action| (action.preposition_site_a, action.preposition_site_b))
                .collect::<Vec<_>>();
            actions.sort_unstable();
            actions
        };
        assert_eq!(
            root_actions(&original_report, &original),
            root_actions(&renamed_report, &renamed)
        );
    }

    #[test]
    fn scenario_and_node_array_order_do_not_change_tree_digest_or_report() {
        let mut model = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let report = solve_nonanticipative_policy(&model).unwrap();
        let digest = tree_digest(&model).unwrap();
        model.nodes.reverse();
        model.paths.reverse();
        let reordered = solve_nonanticipative_policy(&model).unwrap();
        assert_eq!(digest, tree_digest(&model).unwrap());
        assert_eq!(report, reordered);
    }

    #[test]
    fn profile_digest_mutation_invalidates_a_policy_report() {
        let model = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let mut report = solve_nonanticipative_policy(&model).unwrap();
        let mut changed = model.clone();
        let leaf_id = changed.paths[0].decision_node_ids[1].clone();
        let leaf = changed
            .nodes
            .iter_mut()
            .find(|node| node.decision_node_id == leaf_id)
            .unwrap();
        leaf.minimum_service_quantity += 1;
        assert!(
            verify_nonanticipative_policy_report(&changed, &report)
                .unwrap_err()
                .contains("tree identity/digest mismatch")
        );
        report.tree_sha256 = tree_digest(&changed).unwrap();
        assert!(
            verify_nonanticipative_policy_report(&changed, &report)
                .unwrap_err()
                .contains("policy feasibility summary mismatch")
        );
    }

    #[test]
    fn published_nonanticipative_policy_schemas_are_valid_json_documents() {
        let tree_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/nonanticipative-policy-tree-v0.1.schema.json"
        ))
        .unwrap();
        let report_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/nonanticipative-policy-report-v0.1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            tree_schema["properties"]["schema_version"]["const"],
            POLICY_TREE_SCHEMA
        );
        assert_eq!(
            report_schema["properties"]["schema_version"]["const"],
            NONANTICIPATIVE_POLICY_SCHEMA
        );
        assert_eq!(
            report_schema["properties"]["disposition"]["enum"][1],
            "full_recourse_only"
        );
        assert_eq!(tree_schema["additionalProperties"], false);
        assert_eq!(report_schema["additionalProperties"], false);
        assert_eq!(
            report_schema["properties"]["minimum_common_preposition_units"]["type"],
            "integer"
        );
        assert_eq!(
            report_schema["properties"]["minimum_full_recourse_units"]["type"],
            "integer"
        );
        assert_eq!(
            report_schema["properties"]["resource_units_saved_by_recourse"]["type"],
            "integer"
        );
        assert_eq!(
            report_schema["properties"]["full_recourse_assumption"]["const"],
            FULL_RECOURSE_ASSUMPTION
        );
    }

    #[test]
    fn strict_json_round_trip_and_unknown_fields_are_rejected() {
        let model = tree(2, &[(RequiredSite::SiteA, 1), (RequiredSite::SiteB, 1)]);
        let bytes = serde_json::to_vec(&model).unwrap();
        let decoded: NonanticipativePolicyTree = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, model);

        let mut unknown = serde_json::to_value(&model).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<NonanticipativePolicyTree>(unknown).is_err());

        let report = solve_nonanticipative_policy(&model).unwrap();
        let report_bytes = serde_json::to_vec(&report).unwrap();
        let decoded_report: NonanticipativePolicyReport =
            serde_json::from_slice(&report_bytes).unwrap();
        assert_eq!(decoded_report, report);
        verify_nonanticipative_policy_report(&decoded, &decoded_report).unwrap();
    }

    #[test]
    fn seeded_two_stage_trees_match_independent_closed_form_oracle() {
        let mut rng = 0x05AF_B202_6D15_CA11_u64;
        for seed in 0..250_u64 {
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let count = 2 + ((rng >> 32) % 3) as usize;
            let mut needs = Vec::with_capacity(count);
            let mut maximum_a = 0_u64;
            let mut maximum_b = 0_u64;
            let mut max_full_recourse_need = 0_u64;
            for _ in 0..count {
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let site = if (rng >> 32) % 2 == 0 {
                    RequiredSite::SiteA
                } else {
                    RequiredSite::SiteB
                };
                rng = rng
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let quantity = 1 + ((rng >> 32) % 3);
                match site {
                    RequiredSite::SiteA => maximum_a = maximum_a.max(quantity),
                    RequiredSite::SiteB => maximum_b = maximum_b.max(quantity),
                }
                max_full_recourse_need = max_full_recourse_need.max(quantity);
                needs.push((site, quantity));
            }
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let resources = (rng >> 32) % 7;
            let model = tree(resources, &needs);

            let expected_here_and_now = maximum_a + maximum_b <= resources;
            let expected_full_recourse = max_full_recourse_need <= resources;
            let expected = if expected_here_and_now {
                NonanticipativePolicyDisposition::HereAndNowFeasible
            } else if expected_full_recourse {
                NonanticipativePolicyDisposition::FullRecourseOnly
            } else {
                NonanticipativePolicyDisposition::NotFeasibleWithinSuppliedTree
            };

            let report = solve_nonanticipative_policy(&model).unwrap();
            assert_eq!(
                report.disposition, expected,
                "seed {seed}: non-anticipative policy oracle mismatch"
            );
            assert_eq!(report.here_and_now_feasible, expected_here_and_now);
            assert_eq!(report.full_recourse_feasible, expected_full_recourse);
            assert_eq!(
                report.minimum_common_preposition_units,
                maximum_a + maximum_b
            );
            assert_eq!(report.minimum_full_recourse_units, max_full_recourse_need);
            assert_eq!(
                report.resource_units_saved_by_recourse,
                maximum_a + maximum_b - max_full_recourse_need
            );
            verify_nonanticipative_policy_report(&model, &report).unwrap();
        }
    }
}
