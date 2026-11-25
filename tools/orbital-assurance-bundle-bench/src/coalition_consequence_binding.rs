//! Source-bound finite coalition consequence evaluation.
//!
//! Each coalition has a directly supplied loss in every admitted joint world/model
//! scenario. This module never derives portfolio loss by adding single-action losses.
//! It composes the incumbent bridge (#7161) and independent model grammar (#7220),
//! then adds canonical coalition and post-action risk-snapshot binding.

use crate::{incumbent_adequacy, loss_model_universe_adequacy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA: &str = "luminous.orbital-coalition-consequence-binding.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-finite-joint-scenario-coalition-binding-only";
const RISK_RECEIPT_SCHEMA: &str = "luminous.orbital-post-action-risk-snapshot.receipt.v0.1";
const MAX_ACTIONS: usize = 16;
const MAX_SCENARIOS: usize = 16_384;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CandidateAction {
    pub action_id: String,
    pub resource_cost: u64,
    pub actionable: bool,
    pub coordination_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RiskSnapshot {
    pub snapshot_id: String,
    pub receipt_schema: String,
    pub source_commit: String,
    pub source_receipt_id: String,
    pub source_receipt_sha256: String,
    pub action_set_sha256: String,
    pub catalog_revision_sha256: String,
    pub event_definition_sha256: String,
    pub event_inventory_sha256: String,
    pub physical_world_inventory_sha256: String,
    pub full_catalog_state: String,
    pub screening_state: String,
    pub threshold_numerator: u64,
    pub threshold_denominator: u64,
    pub snapshot_sha256: String,
}

#[derive(Serialize)]
struct RiskSnapshotDigest<'a> {
    snapshot_id: &'a str,
    receipt_schema: &'a str,
    source_commit: &'a str,
    source_receipt_id: &'a str,
    source_receipt_sha256: &'a str,
    action_set_sha256: &'a str,
    catalog_revision_sha256: &'a str,
    event_definition_sha256: &'a str,
    event_inventory_sha256: &'a str,
    physical_world_inventory_sha256: &'a str,
    full_catalog_state: &'a str,
    screening_state: &'a str,
    threshold_numerator: u64,
    threshold_denominator: u64,
}

impl RiskSnapshot {
    /// Digest the complete snapshot payload, excluding only the digest field itself.
    pub fn digest(&self) -> Result<String, String> {
        let payload = RiskSnapshotDigest {
            snapshot_id: &self.snapshot_id,
            receipt_schema: &self.receipt_schema,
            source_commit: &self.source_commit,
            source_receipt_id: &self.source_receipt_id,
            source_receipt_sha256: &self.source_receipt_sha256,
            action_set_sha256: &self.action_set_sha256,
            catalog_revision_sha256: &self.catalog_revision_sha256,
            event_definition_sha256: &self.event_definition_sha256,
            event_inventory_sha256: &self.event_inventory_sha256,
            physical_world_inventory_sha256: &self.physical_world_inventory_sha256,
            full_catalog_state: &self.full_catalog_state,
            screening_state: &self.screening_state,
            threshold_numerator: self.threshold_numerator,
            threshold_denominator: self.threshold_denominator,
        };
        canonical_digest(&payload)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CoalitionOutcome {
    pub portfolio_id: String,
    pub action_ids: Vec<String>,
    pub action_set_sha256: String,
    pub coordination_receipt_sha256: Option<String>,
    pub risk_snapshot: RiskSnapshot,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioProbability {
    pub scenario_id: String,
    pub weight: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NominalDistribution {
    pub receipt_schema: String,
    pub source_commit: String,
    pub source_receipt_id: String,
    pub source_receipt_sha256: String,
    pub scenario_set_sha256: String,
    /// Common integer denominator for every scenario probability.
    pub denominator: u64,
    pub probabilities: Vec<ScenarioProbability>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExpectedPortfolioMetric {
    pub portfolio_id: String,
    pub lower_bound_avoided_ticks: i64,
    pub worst_absolute_loss_ticks: i64,
    pub worst_scenario_regret_ticks: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub coalition_count: usize,
    pub scenario_count: usize,
    pub below_threshold_portfolio_count: usize,
    pub bridge_disposition: String,
    pub bridge_trigger_reason: String,
    pub bridge_worst_case_improvement: i64,
    pub portfolios: Vec<ExpectedPortfolioMetric>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub scope_sha256: String,
    pub loss_unit: String,
    pub resource_capacity: u64,
    pub candidate_actions: Vec<CandidateAction>,
    pub coalitions: Vec<CoalitionOutcome>,
    pub incumbent_bridge_case: incumbent_adequacy::Case,
    pub loss_model_universe_case: loss_model_universe_adequacy::Case,
    /// Exact revision of the independent grammar implementation being consumed.
    /// This is a structural source pin, not issuer authentication.
    pub loss_model_universe_checker_commit: String,
    pub nominal_distribution: Option<NominalDistribution>,
    pub expected: Expected,
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
pub struct ExactRational {
    pub numerator: String,
    pub denominator: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PortfolioMetric {
    pub portfolio_id: String,
    pub action_ids: Vec<String>,
    pub risk_screening_state: String,
/// True only when the supplied post-action snapshot states BELOW_THRESHOLD.
/// This field does not independently validate the snapshot issuer or Pc calculation.
    pub risk_snapshot_below_threshold: bool,
    /// min_s (L(no_action,s) - L(portfolio,s)); signed and never clamped.
    pub lower_bound_avoided_ticks: i64,
    pub worst_absolute_loss_ticks: i64,
    /// max_s (L(portfolio,s) - min_below_threshold_q L(q,s)); hindsight diagnostic only.
    pub worst_scenario_regret_ticks: Option<i64>,
    pub nominal_mean_loss: Option<ExactRational>,
    pub nominal_cvar95_loss: Option<ExactRational>,
    /// None until a separately verified #7381 Wasserstein-DRO receipt is consumed.
    pub wasserstein_dro_cvar95_loss: Option<ExactRational>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: String,
    pub reason: String,
    pub scenario_count: usize,
    pub coalition_count: usize,
    pub below_threshold_portfolio_count: usize,
    pub model_universe_disposition: String,
    pub model_universe_case_sha256: String,
    pub model_universe_checker_commit: String,
    pub world_set_receipt_sha256: String,
    pub source_joint_loss_receipt_sha256: String,
    pub projected_scenarios_sha256: String,
    pub scenario_inventory_sha256: String,
    pub catalog_revision_sha256: Option<String>,
    pub event_definition_sha256: Option<String>,
    pub bridge_disposition: Option<String>,
    pub bridge_trigger_reason: Option<String>,
    pub bridge_worst_case_improvement: Option<i64>,
    pub portfolios: Vec<PortfolioMetric>,
    pub nominal_distribution_consumed: bool,
    pub distributionally_robust_cvar95_status: String,
    pub authority_granted: bool,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub qualified_cases: usize,
    pub risk_abstain_cases: usize,
    pub invalid_cases: usize,
    pub coalition_rows: usize,
    pub scenario_rows: usize,
    pub expected_result_mismatches: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub authority_granted: bool,
}

#[derive(Serialize)]
struct PortfolioIdPayload<'a> {
    action_ids: &'a [String],
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn canonical_digest<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("canonical JSON encoding failed: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    canonical_digest(&sorted)
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.bytes().all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

fn valid_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_sha(value: &str) -> bool {
    valid_lower_hex(value, 64)
}

fn valid_commit(value: &str) -> bool {
    valid_lower_hex(value, 40)
}

fn portfolio_identity(action_ids: &[String]) -> Result<(String, String), String> {
    let mut sorted = action_ids.to_vec();
    sorted.sort();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("coalition action IDs contain duplicates".to_owned());
    }
    let payload = PortfolioIdPayload { action_ids: &sorted };
    let digest = canonical_digest(&payload)?;
    Ok((format!("portfolio-{digest}"), digest))
}

fn enumerate_portfolios(
    candidates: &[CandidateAction],
    resource_capacity: u64,
) -> Result<BTreeMap<String, Vec<String>>, String> {
    if candidates.is_empty() || candidates.len() > MAX_ACTIONS {
        return Err("candidate action count is outside the exact enumeration bound".to_owned());
    }
    let mut sorted = candidates.to_vec();
    sorted.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    if sorted.iter().any(|action| !valid_lower_hex(&action.action_id, 64)) {
        return Err("candidate action identity must be lowercase SHA-256 hex".to_owned());
    }
    if sorted.windows(2).any(|pair| pair[0].action_id == pair[1].action_id) {
        return Err("candidate action identity is duplicated".to_owned());
    }

    let subset_count = 1_usize
        .checked_shl(sorted.len() as u32)
        .ok_or_else(|| "coalition subset count overflowed".to_owned())?;
    let mut portfolios = BTreeMap::new();
    for mask in 0..subset_count {
        let mut action_ids = Vec::new();
        let mut resource_cost = 0_u64;
        let mut selectable = true;
        for (index, action) in sorted.iter().enumerate() {
            if mask & (1_usize << index) == 0 {
                continue;
            }
            if !action.actionable {
                selectable = false;
                break;
            }
            resource_cost = resource_cost
                .checked_add(action.resource_cost)
                .ok_or_else(|| "portfolio resource cost overflowed".to_owned())?;
            action_ids.push(action.action_id.clone());
        }
        if !selectable || resource_cost > resource_capacity {
            continue;
        }
        action_ids.sort();
        let (portfolio_id, _) = portfolio_identity(&action_ids)?;
        if portfolios.insert(portfolio_id, action_ids).is_some() {
            return Err("canonical portfolio identity collision".to_owned());
        }
    }
    Ok(portfolios)
}

fn exact_rational(numerator: u128, denominator: u128) -> Result<ExactRational, String> {
    if denominator == 0 {
        return Err("exact rational denominator is zero".to_owned());
    }
    let mut a = numerator;
    let mut b = denominator;
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    let divisor = a.max(1);
    Ok(ExactRational {
        numerator: (numerator / divisor).to_string(),
        denominator: (denominator / divisor).to_string(),
    })
}

fn scenario_loss_map(
    bridge: &incumbent_adequacy::Case,
) -> Result<BTreeMap<String, BTreeMap<String, i64>>, String> {
    let mut scenarios = BTreeMap::new();
    for world in &bridge.worlds {
        let mut losses = BTreeMap::new();
        for row in &world.losses {
            if losses.insert(row.action_id.clone(), row.loss).is_some() {
                return Err(format!(
                    "scenario {} repeats portfolio loss {}",
                    world.scenario_id, row.action_id
                ));
            }
        }
        if scenarios.insert(world.scenario_id.clone(), losses).is_some() {
            return Err("joint scenario identity is duplicated".to_owned());
        }
    }
    if scenarios.is_empty() || scenarios.len() > MAX_SCENARIOS {
        return Err("joint scenario count is empty or exceeds the adapter bound".to_owned());
    }
    Ok(scenarios)
}

fn validate_model_universe(case: &Case) -> Result<(String, String, String), String> {
    let model_case = &case.loss_model_universe_case;
    if model_case.scope_sha256 != case.scope_sha256
        || case.incumbent_bridge_case.scope_sha256 != case.scope_sha256
    {
        return Err("coalition, incumbent and model-universe scopes differ".to_owned());
    }
    if !valid_commit(&case.loss_model_universe_checker_commit) {
        return Err("independent model-universe checker commit is malformed".to_owned());
    }
    let evaluation = loss_model_universe_adequacy::evaluate_single_case(model_case);
    if evaluation.disposition != "LOSS_MODEL_UNIVERSE_CLOSED_WITHIN_DECLARED_GRAMMAR"
        || !evaluation.expected_result_matched
    {
        return Err(format!(
            "independent loss-model universe did not close: {}",
            evaluation.disposition
        ));
    }

    let bridge = &case.incumbent_bridge_case;
    let bridge_worlds = bridge
        .expected_world_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let model_worlds = model_case
        .worlds
        .iter()
        .map(|world| world.world_id.clone())
        .collect::<BTreeSet<_>>();
    if bridge_worlds != model_worlds || model_worlds.len() != model_case.worlds.len() {
        return Err(
            "independent model grammar and joint-loss bridge world inventories differ".to_owned(),
        );
    }
    for world in &model_case.worlds {
        if world.scope_sha256 != case.scope_sha256 {
            return Err("model-universe world scope does not match coalition scope".to_owned());
        }
    }

    let mut expected_pairs = BTreeSet::new();
    for pair in &model_case.joint_pairs {
        if !expected_pairs.insert((
            pair.scenario_id.clone(),
            pair.world_id.clone(),
            pair.model_id.clone(),
        )) {
            return Err("model-universe joint scenario tuple is duplicated".to_owned());
        }
    }
    let mut bridge_pairs = BTreeSet::new();
    for scenario in &bridge.worlds {
        if !bridge_pairs.insert((
            scenario.scenario_id.clone(),
            scenario.world_id.clone(),
            scenario.loss_model_id.clone(),
        )) {
            return Err("joint-loss bridge scenario tuple is duplicated".to_owned());
        }
    }
    if bridge_pairs != expected_pairs {
        return Err(
            "joint-loss bridge scenario tuples differ from independently checked model grammar"
                .to_owned(),
        );
    }
    let mut canonical_model_case = model_case.clone();
    for world in &mut canonical_model_case.worlds {
        world.profile_tags.sort();
    }
    canonical_model_case.worlds.sort_by(|left, right| left.world_id.cmp(&right.world_id));
    canonical_model_case.producer_models.sort_by(|left, right| left.model_id.cmp(&right.model_id));
    canonical_model_case.declared_model_ids.sort();
    canonical_model_case.applicability.sort_by(|left, right| {
        (&left.world_id, &left.model_id).cmp(&(&right.world_id, &right.model_id))
    });
    canonical_model_case.joint_pairs.sort_by(|left, right| {
        (&left.scenario_id, &left.world_id, &left.model_id)
            .cmp(&(&right.scenario_id, &right.world_id, &right.model_id))
    });
    canonical_model_case.expected.missing_model_ids.sort();
    canonical_model_case.expected.missing_pair_ids.sort();
    canonical_model_case.expected.unexpected_model_ids.sort();
    canonical_model_case.expected.unexpected_pair_ids.sort();
    let model_digest = canonical_digest(&canonical_model_case)?;
    let world_digest = digest_ids(&bridge.expected_world_ids)?;
    Ok((
        evaluation.disposition,
        model_digest,
        world_digest,
    ))
}

fn validate_coalitions<'a>(
    case: &'a Case,
    expected: &BTreeMap<String, Vec<String>>,
    world_digest: &str,
) -> Result<(BTreeMap<String, &'a CoalitionOutcome>, String, String), String> {
    if case.coalitions.len() != expected.len() {
        return Err(
            "coalition outcome count differs from selectable subset count".to_owned(),
        );
    }
    let mut records = BTreeMap::new();
    let mut catalog_revision: Option<String> = None;
    let mut event_definition: Option<String> = None;
    let mut threshold: Option<(u64, u64)> = None;
    let mut snapshot_ids = BTreeSet::new();
    let mut source_receipts = BTreeSet::new();
    for row in &case.coalitions {
        let mut action_ids = row.action_ids.clone();
        action_ids.sort();
        if action_ids != row.action_ids {
            return Err("coalition action IDs are not in canonical sorted order".to_owned());
        }
        let (expected_portfolio_id, expected_digest) = portfolio_identity(&row.action_ids)?;
        if row.portfolio_id != expected_portfolio_id
            || row.action_set_sha256 != expected_digest
            || !expected.contains_key(&row.portfolio_id)
            || expected.get(&row.portfolio_id) != Some(&row.action_ids)
        {
            return Err(
                "coalition identity or digest differs from canonical subset"
                    .to_owned(),
            );
        }
        if records.insert(row.portfolio_id.clone(), row).is_some() {
            return Err("duplicate coalition outcome row".to_owned());
        }

        let risk = &row.risk_snapshot;
        if risk.receipt_schema != RISK_RECEIPT_SCHEMA
            || !valid_id(&risk.snapshot_id)
            || !valid_commit(&risk.source_commit)
            || !valid_id(&risk.source_receipt_id)
            || !valid_sha(&risk.source_receipt_sha256)
            || !valid_sha(&risk.action_set_sha256)
            || !valid_sha(&risk.catalog_revision_sha256)
            || !valid_sha(&risk.event_definition_sha256)
            || !valid_sha(&risk.event_inventory_sha256)
            || !valid_sha(&risk.physical_world_inventory_sha256)
            || !valid_sha(&risk.snapshot_sha256)
            || risk.full_catalog_state != "COMPLETE"
            || risk.threshold_denominator == 0
            || risk.threshold_numerator == 0
            || risk.threshold_numerator > risk.threshold_denominator
            || risk.action_set_sha256 != expected_digest
            || risk.physical_world_inventory_sha256 != world_digest
        {
            return Err(
                "post-action risk snapshot has malformed pins or mismatched action/world identity"
                    .to_owned(),
            );
        }
        if risk.digest()? != risk.snapshot_sha256 {
            return Err("post-action risk snapshot digest mismatch".to_owned());
        }
        if !snapshot_ids.insert(risk.snapshot_id.as_str())
            || !source_receipts.insert(risk.source_receipt_id.as_str())
        {
            return Err("risk snapshot or source receipt identity is aliased".to_owned());
        }
        match risk.screening_state.as_str() {
            "BELOW_THRESHOLD" | "THRESHOLD_EXCEEDED" => {}
            "AMBIGUOUS" | "INCOMPLETE" | "UNRESOLVED" => {
                return Err(format!(
                    "risk evidence requires global abstention: {}",
                    risk.screening_state
                ));
            }
            _ => return Err("unknown risk screening state".to_owned()),
        }
        if catalog_revision
            .as_ref()
            .is_some_and(|value| value != &risk.catalog_revision_sha256)
            || event_definition
                .as_ref()
                .is_some_and(|value| value != &risk.event_definition_sha256)
            || threshold.is_some_and(|value| {
                value != (risk.threshold_numerator, risk.threshold_denominator)
            })
        {
            return Err(
                "post-action snapshots disagree on catalog, event definition or risk threshold"
                    .to_owned(),
            );
        }
        catalog_revision.get_or_insert_with(|| risk.catalog_revision_sha256.clone());
        event_definition.get_or_insert_with(|| risk.event_definition_sha256.clone());
        threshold.get_or_insert((risk.threshold_numerator, risk.threshold_denominator));

        let requires_coordination = row.action_ids.iter().any(|action_id| {
            case.candidate_actions
                .iter()
                .any(|candidate| {
                    &candidate.action_id == action_id && candidate.coordination_required
                })
        });
        match (&row.coordination_receipt_sha256, requires_coordination) {
            (Some(digest), _) if !valid_sha(digest) => {
                return Err("coalition coordination receipt digest is malformed".to_owned());
            }
            (None, true) => return Err("coalition omits required coordination receipt".to_owned()),
            _ => {}
        }
    }
    if records.len() != expected.len() || expected.keys().any(|id| !records.contains_key(id)) {
        return Err("coalition catalogue omits an expected subset".to_owned());
    }
    Ok((
        records,
        catalog_revision.expect("nonempty subset enumeration includes no-action"),
        event_definition.expect("nonempty subset enumeration includes no-action"),
    ))
}

fn nominal_metrics(
    distribution: Option<&NominalDistribution>,
    scenario_ids: &[String],
    losses: &BTreeMap<String, i64>,
) -> Result<(Option<ExactRational>, Option<ExactRational>), String> {
    let Some(distribution) = distribution else {
        return Ok((None, None));
    };
    if distribution.receipt_schema != "luminous.orbital-nominal-scenario-distribution.receipt.v0.1"
        || !valid_commit(&distribution.source_commit)
        || !valid_id(&distribution.source_receipt_id)
        || !valid_sha(&distribution.source_receipt_sha256)
        || distribution.denominator == 0
        || distribution.denominator > MAX_SAFE_INTEGER
        || distribution.denominator % 20 != 0
        || distribution.scenario_set_sha256 != digest_ids(scenario_ids)?
    {
        return Err(
            "nominal probability receipt pins or exact CVaR denominator are invalid".to_owned(),
        );
    }
    let mut weights = BTreeMap::new();
    let mut sum = 0_u64;
    for probability in &distribution.probabilities {
        if !valid_id(&probability.scenario_id)
            || probability.weight == 0
            || probability.weight > MAX_SAFE_INTEGER
            || weights.insert(probability.scenario_id.clone(), probability.weight).is_some()
        {
            return Err("nominal probability row is malformed or duplicated".to_owned());
        }
        sum = sum
            .checked_add(probability.weight)
            .ok_or_else(|| "nominal probability mass overflowed".to_owned())?;
    }
    let expected = scenario_ids.iter().cloned().collect::<BTreeSet<_>>();
    if weights.keys().cloned().collect::<BTreeSet<_>>() != expected
        || weights.len() != scenario_ids.len()
        || sum != distribution.denominator
    {
        return Err("nominal probabilities do not cover the exact joint scenario set".to_owned());
    }

    let mut mean_numerator = 0_u128;
    let mut ordered = Vec::with_capacity(scenario_ids.len());
    for scenario_id in scenario_ids {
        let weight = u128::from(weights[scenario_id]);
        let loss = u128::try_from(losses[scenario_id])
            .map_err(|_| "negative loss cannot enter nominal consequence metrics".to_owned())?;
        mean_numerator = mean_numerator
            .checked_add(
                weight
                    .checked_mul(loss)
                    .ok_or_else(|| "nominal mean product overflowed".to_owned())?,
            )
            .ok_or_else(|| "nominal mean accumulation overflowed".to_owned())?;
        ordered.push((loss, weight));
    }
    let mean = exact_rational(mean_numerator, u128::from(distribution.denominator))?;
    ordered.sort_by(|left, right| right.0.cmp(&left.0));
    let tail_mass = u128::from(distribution.denominator / 20);
    let mut remaining = tail_mass;
    let mut tail_numerator = 0_u128;
    for (loss, weight) in ordered {
        if remaining == 0 {
            break;
        }
        let take = weight.min(remaining);
        tail_numerator = tail_numerator
            .checked_add(
                take.checked_mul(loss)
                    .ok_or_else(|| "CVaR product overflowed".to_owned())?,
            )
            .ok_or_else(|| "CVaR accumulation overflowed".to_owned())?;
        remaining -= take;
    }
    if remaining != 0 {
        return Err("nominal probability mass cannot fill the exact 5% CVaR tail".to_owned());
    }
    let cvar = exact_rational(tail_numerator, tail_mass)?;
    Ok((Some(mean), Some(cvar)))
}

fn compute_portfolio_metrics(
    case: &Case,
    scenario_losses: &BTreeMap<String, BTreeMap<String, i64>>,
    records: &BTreeMap<String, &CoalitionOutcome>,
) -> Result<Vec<PortfolioMetric>, String> {
    let baseline_id = records
        .iter()
        .find_map(|(id, row)| row.action_ids.is_empty().then_some(id.clone()))
        .ok_or_else(|| "no-action portfolio control is absent".to_owned())?;
    if case.incumbent_bridge_case.incumbent_action_id != baseline_id {
        return Err(
            "incumbent bridge does not identify the canonical no-action portfolio".to_owned(),
        );
    }
    let below_threshold_ids = records
        .iter()
        .filter(|(_, row)| row.risk_snapshot.screening_state == "BELOW_THRESHOLD")
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if below_threshold_ids.is_empty() {
        return Err("no portfolio has a declared below-threshold screening state".to_owned());
    }

    let scenario_ids = scenario_losses.keys().cloned().collect::<Vec<_>>();
    let mut metrics = Vec::with_capacity(records.len());
    for (portfolio_id, row) in records {
        let mut lower_bound = i64::MAX;
        let mut worst_loss = i64::MIN;
        for losses in scenario_losses.values() {
            let baseline = *losses
                .get(&baseline_id)
                .ok_or_else(|| "no-action portfolio loss is missing in a joint scenario".to_owned())?;
            let portfolio = *losses
                .get(portfolio_id)
                .ok_or_else(|| {
                    format!("portfolio {portfolio_id} loss is missing in a joint scenario")
                })?;
            lower_bound = lower_bound.min(
                baseline
                    .checked_sub(portfolio)
                    .ok_or_else(|| "signed avoided-consequence subtraction overflowed".to_owned())?,
            );
            worst_loss = worst_loss.max(portfolio);
        }

        let is_below_threshold = row.risk_snapshot.screening_state == "BELOW_THRESHOLD";
        let regret = if is_below_threshold {
            let mut worst_regret = 0_i64;
            for losses in scenario_losses.values() {
                let portfolio_loss = *losses
                    .get(portfolio_id)
                    .ok_or_else(|| "below-threshold portfolio loss is missing".to_owned())?;
                let best_below_threshold_loss = below_threshold_ids
                    .iter()
                    .map(|below_threshold_id| losses.get(below_threshold_id).copied())
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        "below-threshold portfolio comparison loss is missing".to_owned()
                    })?
                    .into_iter()
                    .min()
                    .ok_or_else(|| "below-threshold portfolio set is empty".to_owned())?;
                let regret_here = portfolio_loss
                    .checked_sub(best_below_threshold_loss)
                    .ok_or_else(|| "scenario regret subtraction overflowed".to_owned())?;
                worst_regret = worst_regret.max(regret_here);
            }
            Some(worst_regret)
        } else {
            None
        };
        let portfolio_losses = scenario_ids
            .iter()
            .map(|scenario_id| {
                scenario_losses[scenario_id]
                    .get(portfolio_id)
                    .copied()
                    .map(|loss| (scenario_id.clone(), loss))
                    .ok_or_else(|| "portfolio scenario loss is missing".to_owned())
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let (nominal_mean_loss, nominal_cvar95_loss) = nominal_metrics(
            case.nominal_distribution.as_ref(),
            &scenario_ids,
            &portfolio_losses,
        )?;
        metrics.push(PortfolioMetric {
            portfolio_id: portfolio_id.clone(),
            action_ids: row.action_ids.clone(),
            risk_screening_state: row.risk_snapshot.screening_state.clone(),
            risk_snapshot_below_threshold: is_below_threshold,
            lower_bound_avoided_ticks: lower_bound,
            worst_absolute_loss_ticks: worst_loss,
            worst_scenario_regret_ticks: regret,
            nominal_mean_loss,
            nominal_cvar95_loss,
            wasserstein_dro_cvar95_loss: None,
        });
    }
    metrics.sort_by(|left, right| left.portfolio_id.cmp(&right.portfolio_id));
    Ok(metrics)
}

fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    if !valid_id(&case.case_id)
        || !valid_sha(&case.scope_sha256)
        || !valid_id(&case.loss_unit)
        || case.incumbent_bridge_case.loss_unit != case.loss_unit
        || case.incumbent_bridge_case.scope_sha256 != case.scope_sha256
        || !valid_commit(&case.loss_model_universe_checker_commit)
    {
        return Err("case identity, scope, unit or checker source pin is invalid".to_owned());
    }
    let enumerated = enumerate_portfolios(&case.candidate_actions, case.resource_capacity)?;
    let (model_disposition, model_case_digest, world_digest) = validate_model_universe(case)?;
    let (records, catalog_revision, event_definition) =
        validate_coalitions(case, &enumerated, &world_digest)?;

    let scenario_losses = scenario_loss_map(&case.incumbent_bridge_case)?;
    let mut scenario_ids = scenario_losses.keys().cloned().collect::<Vec<_>>();
    scenario_ids.sort();
    let bridge_scenario_ids = case
        .incumbent_bridge_case
        .worlds
        .iter()
        .map(|world| world.scenario_id.clone())
        .collect::<BTreeSet<_>>();
    let model_scenario_ids = case
        .loss_model_universe_case
        .joint_pairs
        .iter()
        .map(|pair| pair.scenario_id.clone())
        .collect::<BTreeSet<_>>();
    if bridge_scenario_ids != model_scenario_ids
        || bridge_scenario_ids.len() != scenario_ids.len()
        || scenario_ids.len() > MAX_SCENARIOS
    {
        return Err(
            "scenario inventory differs across coalition, joint-loss and independent model evidence"
                .to_owned(),
        );
    }
    let action_ids = enumerated.keys().cloned().collect::<BTreeSet<_>>();
    let bridge_action_ids = case
        .incumbent_bridge_case
        .action_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if action_ids != bridge_action_ids
        || action_ids.len() != case.incumbent_bridge_case.action_ids.len()
    {
        return Err(
            "incumbent bridge action universe differs from canonical coalition universe".to_owned(),
        );
    }
    for (scenario_id, losses) in &scenario_losses {
        if losses.keys().cloned().collect::<BTreeSet<_>>() != action_ids {
            return Err(format!(
                "scenario {scenario_id} does not carry exactly one direct loss for every coalition"
            ));
        }
    }

    let bridge_evaluation =
        incumbent_adequacy::evaluate_single_case(&case.incumbent_bridge_case);
    if !bridge_evaluation.expected_result_matched
        || bridge_evaluation.authority_granted
        || bridge_evaluation
            .world_assessments
            .iter()
            .any(|assessment| assessment.evidence_receipt_id.is_empty())
    {
        return Err(
            "source-pinned incumbent bridge failed expected-result or authority invariants"
                .to_owned(),
        );
    }

    let metrics = compute_portfolio_metrics(case, &scenario_losses, &records)?;
    let below_threshold_portfolio_count = metrics
        .iter()
        .filter(|metric| metric.risk_snapshot_below_threshold)
        .count();
    let expected_metrics = case
        .expected
        .portfolios
        .iter()
        .map(|metric| (metric.portfolio_id.as_str(), metric))
        .collect::<BTreeMap<_, _>>();
    let actual_metrics_match = metrics.len() == expected_metrics.len()
        && metrics.iter().all(|metric| {
            expected_metrics.get(metric.portfolio_id.as_str()).is_some_and(|expected| {
                metric.lower_bound_avoided_ticks == expected.lower_bound_avoided_ticks
                    && metric.worst_absolute_loss_ticks == expected.worst_absolute_loss_ticks
                    && metric.worst_scenario_regret_ticks == expected.worst_scenario_regret_ticks
            })
        });
    let scenario_digest = digest_ids(&scenario_ids)?;
    let expected_result_matched = case.expected.coalition_count == metrics.len()
        && case.expected.scenario_count == scenario_ids.len()
        && case.expected.below_threshold_portfolio_count == below_threshold_portfolio_count
        && case.expected.bridge_disposition == bridge_evaluation.disposition
        && case.expected.bridge_trigger_reason == bridge_evaluation.trigger_reason
        && Some(case.expected.bridge_worst_case_improvement)
            == bridge_evaluation.worst_case_improvement
        && actual_metrics_match;

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        disposition: if expected_result_matched {
            "QUALIFIED_SYNTHETIC_SCENARIO_BINDING".to_owned()
        } else {
            "FIXTURE_EXPECTATION_MISMATCH".to_owned()
        },
        reason: if expected_result_matched {
            "all finite source-bound coalition and model-universe checks matched".to_owned()
        } else {
            "one or more frozen expected values differ from the independent calculation".to_owned()
        },
        scenario_count: scenario_ids.len(),
        coalition_count: metrics.len(),
        below_threshold_portfolio_count,
        model_universe_disposition: model_disposition,
        model_universe_case_sha256: model_case_digest,
        model_universe_checker_commit: case.loss_model_universe_checker_commit.clone(),
        world_set_receipt_sha256: case.incumbent_bridge_case.world_set_receipt_sha256.clone(),
        source_joint_loss_receipt_sha256: case
            .incumbent_bridge_case
            .loss_model_set
            .source_receipt_sha256
            .clone(),
        projected_scenarios_sha256: case
            .incumbent_bridge_case
            .loss_model_set
            .projected_scenarios_sha256
            .clone(),
        scenario_inventory_sha256: scenario_digest,
        catalog_revision_sha256: Some(catalog_revision),
        event_definition_sha256: Some(event_definition),
        bridge_disposition: Some(bridge_evaluation.disposition),
        bridge_trigger_reason: Some(bridge_evaluation.trigger_reason),
        bridge_worst_case_improvement: bridge_evaluation.worst_case_improvement,
        portfolios: metrics,
        nominal_distribution_consumed: case.nominal_distribution.is_some(),
        distributionally_robust_cvar95_status: "NOT_CONSUMED_NO_VERIFIED_WASSERSTEIN_DRO_RECEIPT"
            .to_owned(),
        authority_granted: false,
        expected_result_matched,
    })
}

pub fn evaluate(corpus: &Corpus) -> Result<Evaluation, String> {
    if corpus.schema != SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 128
    {
        return Err("unsupported coalition-consequence corpus or claim ceiling".to_owned());
    }
    let mut seen = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !seen.insert(case.case_id.as_str()) {
            return Err("coalition-consequence case IDs are duplicated".to_owned());
        }
        cases.push(evaluate_case(case)?);
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    let summary = Summary {
        case_count: cases.len(),
        qualified_cases: cases
            .iter()
            .filter(|case| case.disposition == "QUALIFIED_SYNTHETIC_SCENARIO_BINDING")
            .count(),
        risk_abstain_cases: cases
            .iter()
            .filter(|case| case.disposition == "ABSTAIN_RISK_EVIDENCE")
            .count(),
        invalid_cases: cases.iter().filter(|case| case.disposition == "INVALID_INPUT").count(),
        coalition_rows: cases.iter().map(|case| case.coalition_count).sum(),
        scenario_rows: cases.iter().map(|case| case.scenario_count).sum(),
        expected_result_mismatches: cases
            .iter()
            .filter(|case| !case.expected_result_matched)
            .count(),
        authority_grants: cases.iter().filter(|case| case.authority_granted).count(),
    };
    let pass = summary.qualified_cases == summary.case_count
        && summary.expected_result_mismatches == 0
        && summary.authority_grants == 0;
    Ok(Evaluation {
        result: if pass { "pass" } else { "fail" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Corpus {
        serde_json::from_str(include_str!(
            "../fixtures/v0.1/coalition-consequence-cases.json"
        ))
        .expect("coalition consequence corpus parses")
    }

    fn result() -> CaseResult {
        let evaluation = evaluate(&fixture()).expect("finite coalition corpus evaluates");
        assert_eq!(evaluation.result, "pass");
        assert_eq!(evaluation.summary.expected_result_mismatches, 0);
        assert_eq!(evaluation.summary.authority_grants, 0);
        evaluation.cases.into_iter().next().expect("one case")
    }

    fn metric<'a>(result: &'a CaseResult, action_set: &[String]) -> &'a PortfolioMetric {
        let (id, _) = portfolio_identity(action_set).expect("portfolio identity");
        result.portfolios.iter().find(|row| row.portfolio_id == id).expect("portfolio metric")
    }

    #[test]
    fn joint_scenario_signed_deltas_preserve_adverse_interactions() {
        let result = result();
        let case = fixture();
        let action_a = case
            .candidate_actions
            .iter()
            .find(|a| {
                a.action_id
                    == "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            })
            .unwrap();
        let action_b = case
            .candidate_actions
            .iter()
            .find(|a| {
                a.action_id
                    == "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            })
            .unwrap();
        assert_eq!(metric(&result, &[]).lower_bound_avoided_ticks, 0);
        assert_eq!(metric(&result, &[action_a.action_id.clone()]).lower_bound_avoided_ticks, 5);
        assert_eq!(metric(&result, &[action_b.action_id.clone()]).lower_bound_avoided_ticks, -3);
        assert_eq!(
            metric(
                &result,
                &[action_a.action_id.clone(), action_b.action_id.clone()]
            )
            .lower_bound_avoided_ticks,
            -10
        );
        assert_eq!(
            metric(
                &result,
                &[action_a.action_id.clone(), action_b.action_id.clone()]
            )
            .worst_absolute_loss_ticks,
            118
        );
    }

    #[test]
    fn exact_common_action_bridge_and_separate_hindsight_regret_are_reported() {
        let result = result();
        assert_eq!(result.bridge_disposition.as_deref(), Some("REOPTIMIZATION_WARRANTED"));
        assert_eq!(result.bridge_worst_case_improvement, Some(5));
        let case = fixture();
        let a = case.candidate_actions[0].action_id.clone();
        let b = case.candidate_actions[1].action_id.clone();
        assert_eq!(metric(&result, &[a]).worst_scenario_regret_ticks, Some(5));
        assert_eq!(metric(&result, &[b]).worst_scenario_regret_ticks, Some(10));
    }

    #[test]
    fn digest_alias_and_action_set_mutations_are_rejected() {
        let mut corpus = fixture();
        corpus.cases[0].coalitions[1].action_set_sha256 = "0".repeat(64);
        assert!(evaluate(&corpus).is_err());
        let mut corpus = fixture();
        corpus.cases[0].coalitions[1].portfolio_id = "portfolio-".to_owned() + &"0".repeat(64);
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn missing_coalition_or_joint_scenario_is_rejected() {
        let mut corpus = fixture();
        corpus.cases[0].coalitions.pop();
        assert!(evaluate(&corpus).is_err());
        let mut corpus = fixture();
        corpus.cases[0].incumbent_bridge_case.loss_model_set.expected_scenario_ids.pop();
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn independent_model_grammar_omission_is_rejected() {
        let mut corpus = fixture();
        corpus.cases[0].loss_model_universe_case.declared_model_ids.pop();
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn risk_snapshot_must_bind_exact_portfolio_and_consistent_catalog() {
        let mut corpus = fixture();
        corpus.cases[0].coalitions[1].risk_snapshot.action_set_sha256 = "0".repeat(64);
        assert!(evaluate(&corpus).is_err());
        let mut corpus = fixture();
        corpus.cases[0].coalitions[1].risk_snapshot.catalog_revision_sha256 = "f".repeat(64);
        corpus.cases[0].coalitions[1].risk_snapshot.snapshot_sha256 =
            corpus.cases[0].coalitions[1].risk_snapshot.digest().unwrap();
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn no_nominal_or_wasserstein_claim_is_invented() {
        let result = result();
        assert!(result.portfolios.iter().all(|metric| metric.nominal_mean_loss.is_none()));
        assert!(result.portfolios.iter().all(|metric| metric.nominal_cvar95_loss.is_none()));
        assert!(result
            .portfolios
            .iter()
            .all(|metric| metric.wasserstein_dro_cvar95_loss.is_none()));
        assert_eq!(
            result.distributionally_robust_cvar95_status,
            "NOT_CONSUMED_NO_VERIFIED_WASSERSTEIN_DRO_RECEIPT"
        );
    }

    #[test]
    fn permutation_of_coalitions_is_invariant() {
        let mut corpus = fixture();
        let expected = evaluate(&corpus).expect("baseline evaluation");
        corpus.cases[0].coalitions.reverse();
        corpus.cases[0].incumbent_bridge_case.worlds.reverse();
        for world in &mut corpus.cases[0].incumbent_bridge_case.worlds {
            world.feasible_action_ids.reverse();
            world.losses.reverse();
        }
        corpus.cases[0].loss_model_universe_case.joint_pairs.reverse();
        corpus.cases[0].loss_model_universe_case.worlds.reverse();
        let actual = evaluate(&corpus).expect("permuted evaluation");
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }

    #[test]
    fn authority_is_never_granted() {
        let result = result();
        assert!(!result.authority_granted);
    }
}
