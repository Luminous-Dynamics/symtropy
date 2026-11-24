//! Probability-qualified finite-distribution CVaR of policy regret.
//!
//! This is a separate research track. It never supplies probabilities to the
//! distribution-free incumbent adapter. Missing or unqualified probability
//! evidence abstains from this track; it does not change the worst-case result.
//!
//! The evaluator computes CVaR with the exact Rockafellar-Uryasev variational
//! formula over finite support. Rational operations are bounded and fail closed
//! on overflow. A receipt's digest is a structural pin, not issuer authentication.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-cvar-regret.corpus.v0.1";
const MODEL_SCHEMA: &str = "luminous.orbital-probability-model.receipt.v0.1";
const CLAIM_CEILING: &str = "probability-conditioned-synthetic-tail-risk-algebra-only";
const MODEL_CLAIM_CEILING: &str = "explicit-joint-probability-model-receipt-only";
const MAX_COMPONENT: u64 = 1_000_000_000_000;
const MAX_CASES: usize = 128;
const MAX_SCENARIOS: usize = 16_384;
const MAX_POLICIES: usize = 4_096;
const MAX_ALPHAS: usize = 16;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rational {
    pub numerator: u64,
    pub denominator: u64,
}

impl Rational {
    pub fn new(numerator: u64, denominator: u64) -> Result<Self, String> {
        if denominator == 0 || numerator > MAX_COMPONENT || denominator > MAX_COMPONENT {
            return Err("rational component is zero-denominator or exceeds exact bound".to_owned());
        }
        let divisor = gcd_u128(u128::from(numerator), u128::from(denominator));
        let numerator = u128::from(numerator) / divisor;
        let denominator = u128::from(denominator) / divisor;
        if numerator > u128::from(MAX_COMPONENT) || denominator > u128::from(MAX_COMPONENT) {
            return Err("reduced rational exceeds exact bound".to_owned());
        }
        Ok(Self {
            numerator: numerator as u64,
            denominator: denominator as u64,
        })
    }

    pub fn zero() -> Self {
        Self { numerator: 0, denominator: 1 }
    }

    pub fn one() -> Self {
        Self { numerator: 1, denominator: 1 }
    }

    pub fn valid(&self) -> bool {
        self.denominator > 0
            && self.numerator <= MAX_COMPONENT
            && self.denominator <= MAX_COMPONENT
            && gcd_u128(u128::from(self.numerator), u128::from(self.denominator)) == 1
    }

    pub fn compare_exact(&self, other: &Self) -> Ordering {
        (u128::from(self.numerator) * u128::from(other.denominator))
            .cmp(&(u128::from(other.numerator) * u128::from(self.denominator)))
    }

    fn from_wide(numerator: u128, denominator: u128) -> Result<Self, String> {
        if denominator == 0 {
            return Err("rational division by zero".to_owned());
        }
        let divisor = gcd_u128(numerator, denominator);
        let numerator = numerator / divisor;
        let denominator = denominator / divisor;
        if numerator > u128::from(MAX_COMPONENT) || denominator > u128::from(MAX_COMPONENT) {
            return Err("exact rational operation exceeds configured bound".to_owned());
        }
        Ok(Self {
            numerator: numerator as u64,
            denominator: denominator as u64,
        })
    }

    fn checked_add(&self, other: &Self) -> Result<Self, String> {
        let left = u128::from(self.numerator)
            .checked_mul(u128::from(other.denominator))
            .ok_or_else(|| "rational addition overflow".to_owned())?;
        let right = u128::from(other.numerator)
            .checked_mul(u128::from(self.denominator))
            .ok_or_else(|| "rational addition overflow".to_owned())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition overflow".to_owned())?;
        let denominator = u128::from(self.denominator)
            .checked_mul(u128::from(other.denominator))
            .ok_or_else(|| "rational addition overflow".to_owned())?;
        Self::from_wide(numerator, denominator)
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, String> {
        if self.compare_exact(other) == Ordering::Less {
            return Err("negative rational result is outside nonnegative regret domain".to_owned());
        }
        let left = u128::from(self.numerator) * u128::from(other.denominator);
        let right = u128::from(other.numerator) * u128::from(self.denominator);
        let denominator = u128::from(self.denominator) * u128::from(other.denominator);
        Self::from_wide(left - right, denominator)
    }

    fn checked_mul(&self, other: &Self) -> Result<Self, String> {
        let numerator = u128::from(self.numerator)
            .checked_mul(u128::from(other.numerator))
            .ok_or_else(|| "rational multiplication overflow".to_owned())?;
        let denominator = u128::from(self.denominator)
            .checked_mul(u128::from(other.denominator))
            .ok_or_else(|| "rational multiplication overflow".to_owned())?;
        Self::from_wide(numerator, denominator)
    }

    fn checked_div(&self, other: &Self) -> Result<Self, String> {
        if other.numerator == 0 {
            return Err("rational division by zero".to_owned());
        }
        let numerator = u128::from(self.numerator)
            .checked_mul(u128::from(other.denominator))
            .ok_or_else(|| "rational division overflow".to_owned())?;
        let denominator = u128::from(self.denominator)
            .checked_mul(u128::from(other.numerator))
            .ok_or_else(|| "rational division overflow".to_owned())?;
        Self::from_wide(numerator, denominator)
    }
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii() && value.bytes().all(|b| b.is_ascii_graphic() || b == b' ')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JointScenario {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProbabilityWeight {
    pub scenario_id: String,
    pub weight: Rational,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProbabilityAssumptions {
    pub sampling_design: String,
    pub dependence_model: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProbabilityReceipt {
    pub receipt_schema: String,
    pub claim_ceiling: String,
    pub source_commit: String,
    pub probability_model_id: String,
    pub receipt_id: String,
    pub receipt_sha256: String,
    pub source_case_id: String,
    pub scope_id: String,
    pub scope_sha256: String,
    pub decision_cutoff_id: String,
    pub world_set_receipt_id: String,
    pub joint_loss_receipt_id: String,
    pub joint_scenario_manifest_sha256: String,
    pub expected_scenario_ids_sha256: String,
    pub calibration_receipt_id: String,
    pub calibration_receipt_sha256: String,
    pub validation_split_id: String,
    pub validation_split_sha256: String,
    pub probability_payload_sha256: String,
    pub state: String,
    pub calibration_state: String,
    pub coverage_state: String,
    pub model_family_state: String,
    pub assumptions: ProbabilityAssumptions,
    pub result: String,
    pub weights: Vec<ProbabilityWeight>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyRegretRow {
    pub scenario_id: String,
    pub wait_and_see_regret: Rational,
    pub outcome_class_regret: Rational,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyRegret {
    pub policy_id: String,
    pub per_scenario: Vec<PolicyRegretRow>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub scope_id: String,
    pub scope_sha256: String,
    pub decision_cutoff_id: String,
    pub world_set_receipt_id: String,
    pub joint_loss_receipt_id: String,
    pub joint_scenario_manifest_sha256: String,
    pub expected_scenario_ids: Vec<String>,
    pub expected_scenario_ids_sha256: String,
    pub joint_scenarios: Vec<JointScenario>,
    pub probability_model: Option<ProbabilityReceipt>,
    pub policies: Vec<PolicyRegret>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub alphas: Vec<Rational>,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CvarPoint {
    pub alpha: Rational,
    pub var: Rational,
    pub cvar: Rational,
    pub tail_mass: Rational,
    pub var_atom_fraction: Rational,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RegretRisk {
    pub expected_regret: Rational,
    pub worst_case_regret: Rational,
    pub cvar: Vec<CvarPoint>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PolicyRisk {
    pub policy_id: String,
    pub wait_and_see_regret: RegretRisk,
    pub outcome_class_regret: RegretRisk,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub reason: String,
    pub probability_model_id: Option<String>,
    pub scenario_count: usize,
    pub policies: Vec<PolicyRisk>,
    pub authority_granted: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Summary {
    pub case_count: usize,
    pub pass_cases: usize,
    pub abstain_cases: usize,
    pub invalid_input_cases: usize,
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

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("cannot encode scenario IDs: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn digest_joint_scenarios(scenarios: &[JointScenario]) -> Result<String, String> {
    let mut sorted = scenarios.to_vec();
    sorted.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("cannot encode joint scenario manifest: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn digest_weights(weights: &[ProbabilityWeight]) -> Result<String, String> {
    let mut sorted = weights.to_vec();
    sorted.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("cannot encode probability weights: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn invalid(case: &Case, reason: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        status: "INVALID_INPUT".to_owned(),
        reason: reason.to_owned(),
        probability_model_id: None,
        scenario_count: 0,
        policies: Vec::new(),
        authority_granted: false,
    }
}

fn abstain(case: &Case, reason: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        status: "ABSTAIN".to_owned(),
        reason: reason.to_owned(),
        probability_model_id: None,
        scenario_count: 0,
        policies: Vec::new(),
        authority_granted: false,
    }
}

fn validate_receipt(
    case: &Case,
    receipt: &ProbabilityReceipt,
) -> Result<BTreeMap<String, Rational>, String> {
    if receipt.receipt_schema != MODEL_SCHEMA
        || receipt.claim_ceiling != MODEL_CLAIM_CEILING
        || !valid_commit(&receipt.source_commit)
        || !valid_id(&receipt.probability_model_id)
        || !valid_id(&receipt.receipt_id)
        || !valid_sha256(&receipt.receipt_sha256)
        || receipt.source_case_id != case.case_id
        || receipt.scope_id != case.scope_id
        || receipt.scope_sha256 != case.scope_sha256
        || receipt.decision_cutoff_id != case.decision_cutoff_id
        || receipt.world_set_receipt_id != case.world_set_receipt_id
        || receipt.joint_loss_receipt_id != case.joint_loss_receipt_id
        || receipt.joint_scenario_manifest_sha256 != case.joint_scenario_manifest_sha256
        || receipt.expected_scenario_ids_sha256 != case.expected_scenario_ids_sha256
        || !valid_id(&receipt.calibration_receipt_id)
        || !valid_sha256(&receipt.calibration_receipt_sha256)
        || !valid_id(&receipt.validation_split_id)
        || !valid_sha256(&receipt.validation_split_sha256)
        || !valid_sha256(&receipt.probability_payload_sha256)
        || receipt.result != "pass"
    {
        return Err("probability model receipt pins or provenance are malformed".to_owned());
    }
    if receipt.state != "CURRENT" {
        return Err("ABSTAIN:PROBABILITY_MODEL_NOT_CURRENT".to_owned());
    }
    if receipt.calibration_state != "QUALIFIED" {
        return Err("ABSTAIN:PROBABILITY_CALIBRATION_NOT_QUALIFIED".to_owned());
    }
    if receipt.coverage_state != "ADEQUATE" {
        return Err("ABSTAIN:PROBABILITY_SUPPORT_COVERAGE_NOT_ADEQUATE".to_owned());
    }
    if receipt.model_family_state != "ADEQUATE" {
        return Err("ABSTAIN:PROBABILITY_MODEL_FAMILY_NOT_ADEQUATE".to_owned());
    }
    if receipt.assumptions.sampling_design != "EXPLICIT_JOINT_DISTRIBUTION"
        || receipt.assumptions.dependence_model != "JOINT_SCENARIO_WEIGHTS"
    {
        return Err(
            "probability model assumptions do not declare an explicit joint distribution".to_owned(),
        );
    }
    let expected_ids = case.expected_scenario_ids.iter().cloned().collect::<BTreeSet<_>>();
    let scenario_ids_digest = digest_ids(&case.expected_scenario_ids)?;
    if case.expected_scenario_ids.is_empty()
        || expected_ids.len() != case.expected_scenario_ids.len()
        || case.expected_scenario_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || scenario_ids_digest != case.expected_scenario_ids_sha256
    {
        return Err(
            "expected joint scenario ID inventory is malformed or digest-inconsistent".to_owned(),
        );
    }
    if case.joint_scenarios.len() != expected_ids.len()
        || case
            .joint_scenarios
            .iter()
            .any(|row| !valid_id(&row.scenario_id)
                || !valid_id(&row.world_id)
                || !valid_id(&row.loss_model_id))
        || case.joint_scenarios.iter().map(|row| row.scenario_id.as_str()).collect::<BTreeSet<_>>()
            != expected_ids.iter().map(String::as_str).collect::<BTreeSet<_>>()
        || digest_joint_scenarios(&case.joint_scenarios)?
            != case.joint_scenario_manifest_sha256
    {
        return Err(
            "joint scenario identity records or manifest digest are inconsistent".to_owned(),
        );
    }
    if receipt.weights.len() != expected_ids.len() {
        return Err("probability weights do not cover the exact joint scenario inventory".to_owned());
    }
    let mut weights = BTreeMap::new();
    let mut total = Rational::zero();
    for row in &receipt.weights {
        if !expected_ids.contains(&row.scenario_id)
            || !valid_id(&row.scenario_id)
            || !row.weight.valid()
            || weights.insert(row.scenario_id.clone(), row.weight.clone()).is_some()
        {
            return Err("probability weights contain malformed, duplicate or unknown scenario IDs".to_owned());
        }
        total = total.checked_add(&row.weight)?;
    }
    if weights.len() != expected_ids.len() || total != Rational::one() {
        return Err("probability weights do not sum exactly to one".to_owned());
    }
    if digest_weights(&receipt.weights)? != receipt.probability_payload_sha256 {
        return Err("probability weight payload digest mismatch".to_owned());
    }
    Ok(weights)
}

fn weighted_expected(values: &[(Rational, Rational)]) -> Result<Rational, String> {
    let mut total = Rational::zero();
    for (value, weight) in values {
        total = total.checked_add(&weight.checked_mul(value)?)?;
    }
    Ok(total)
}

fn cvar_upper_tail(values: &[(Rational, Rational)], alpha: &Rational) -> Result<CvarPoint, String> {
    if !alpha.valid() || alpha.numerator == 0 || alpha.compare_exact(&Rational::one()) != Ordering::Less {
        return Err("CVaR alpha must be strictly between zero and one".to_owned());
    }
    let total_weight = values.iter().try_fold(Rational::zero(), |sum, (_, weight)| {
        sum.checked_add(weight)
    })?;
    if values.is_empty() || total_weight != Rational::one() {
        return Err("CVaR values must have normalized probability weights".to_owned());
    }
    let mut candidates = values.iter().map(|(value, _)| value.clone()).collect::<Vec<_>>();
    candidates.sort_by(Rational::compare_exact);
    candidates.dedup_by(|left, right| left.compare_exact(right) == Ordering::Equal);

    let one_minus_alpha = Rational::one().checked_sub(alpha)?;
    let mut best: Option<Rational> = None;
    for eta in &candidates {
        let mut excess = Rational::zero();
        for (loss, weight) in values {
            if loss.compare_exact(eta) == Ordering::Greater {
                excess = excess.checked_add(&weight.checked_mul(&loss.checked_sub(eta)?)?)?;
            }
        }
        let objective = eta.checked_add(&excess.checked_div(&one_minus_alpha)?)?;
        if best.as_ref().is_none_or(|current| objective.compare_exact(current) == Ordering::Less) {
            best = Some(objective);
        }
    }
    let cvar = best.expect("validated nonempty support has a candidate eta");

    let mut cumulative = Rational::zero();
    let mut var: Option<Rational> = None;
    for value in &candidates {
        let mass = values
            .iter()
            .filter(|(loss, _)| loss.compare_exact(value) == Ordering::Equal)
            .try_fold(Rational::zero(), |sum, (_, weight)| sum.checked_add(weight))?;
        cumulative = cumulative.checked_add(&mass)?;
        if cumulative.compare_exact(alpha) != Ordering::Less {
            var = Some(value.clone());
            break;
        }
    }
    let var = var.expect("normalized weights produce an alpha quantile");
    let mut mass_above = Rational::zero();
    let mut mass_at = Rational::zero();
    for (loss, weight) in values {
        match loss.compare_exact(&var) {
            Ordering::Greater => mass_above = mass_above.checked_add(weight)?,
            Ordering::Equal => mass_at = mass_at.checked_add(weight)?,
            Ordering::Less => {}
        }
    }
    let tail_mass = one_minus_alpha;
    let required_at_var = if tail_mass.compare_exact(&mass_above) == Ordering::Greater {
        tail_mass.checked_sub(&mass_above)?
    } else {
        Rational::zero()
    };
    let var_atom_fraction = if mass_at.numerator == 0 {
        Rational::zero()
    } else {
        required_at_var.checked_div(&mass_at)?
    };
    if var_atom_fraction.compare_exact(&Rational::one()) == Ordering::Greater {
        return Err("CVaR tail atom accounting exceeds unit mass".to_owned());
    }
    Ok(CvarPoint {
        alpha: alpha.clone(),
        var,
        cvar,
        tail_mass,
        var_atom_fraction,
    })
}

fn evaluate_metric(
    rows: &[PolicyRegretRow],
    weights: &BTreeMap<String, Rational>,
    metric: impl Fn(&PolicyRegretRow) -> &Rational,
    alphas: &[Rational],
) -> Result<RegretRisk, String> {
    let mut values = Vec::with_capacity(rows.len());
    for row in rows {
        let value = metric(row);
        if !value.valid() {
            return Err("policy contains malformed regret value".to_owned());
        }
        let weight = weights
            .get(&row.scenario_id)
            .ok_or_else(|| "policy regret row has no probability weight".to_owned())?;
        values.push((value.clone(), weight.clone()));
    }
    let expected_regret = weighted_expected(&values)?;
    let worst_case_regret = values
        .iter()
        .map(|(value, _)| value)
        .max_by(|left, right| left.compare_exact(right))
        .expect("policy row set was validated")
        .clone();
    let cvar = alphas
        .iter()
        .map(|alpha| cvar_upper_tail(&values, alpha))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RegretRisk {
        expected_regret,
        worst_case_regret,
        cvar,
    })
}

fn evaluate_case(case: &Case, alphas: &[Rational]) -> CaseResult {
    if !valid_id(&case.case_id)
        || !valid_id(&case.scope_id)
        || !valid_sha256(&case.scope_sha256)
        || !valid_id(&case.decision_cutoff_id)
        || !valid_id(&case.world_set_receipt_id)
        || !valid_id(&case.joint_loss_receipt_id)
        || !valid_sha256(&case.joint_scenario_manifest_sha256)
    {
        return invalid(case, "case identity and source scope are malformed");
    }
    let Some(receipt) = &case.probability_model else {
        return abstain(case, "PROBABILITY_MODEL_RECEIPT_ABSENT");
    };
    let weights = match validate_receipt(case, receipt) {
        Ok(weights) => weights,
        Err(reason) if reason.starts_with("ABSTAIN:") => {
            return abstain(case, reason.trim_start_matches("ABSTAIN:"));
        }
        Err(reason) => return invalid(case, &reason),
    };
    if case.policies.is_empty() || case.policies.len() > MAX_POLICIES {
        return invalid(case, "policy set is empty or exceeds finite bound");
    }
    let expected_ids = weights.keys().cloned().collect::<BTreeSet<_>>();
    let mut policy_ids = BTreeSet::new();
    let mut policies = Vec::with_capacity(case.policies.len());
    for policy in &case.policies {
        if !valid_id(&policy.policy_id) || !policy_ids.insert(policy.policy_id.as_str()) {
            return invalid(case, "policy IDs are malformed or duplicated");
        }
        if policy.per_scenario.len() != expected_ids.len() {
            return invalid(case, "policy does not cover exact joint scenario support");
        }
        let row_ids = policy
            .per_scenario
            .iter()
            .map(|row| row.scenario_id.as_str())
            .collect::<BTreeSet<_>>();
        if row_ids.len() != policy.per_scenario.len()
            || row_ids != expected_ids.iter().map(String::as_str).collect::<BTreeSet<_>>()
        {
            return invalid(
                case,
                "policy regret rows have missing, duplicate or unknown scenario IDs",
            );
        }
        let wait = match evaluate_metric(
            &policy.per_scenario,
            &weights,
            |row| &row.wait_and_see_regret,
            alphas,
        ) {
            Ok(value) => value,
            Err(reason) => return invalid(case, &reason),
        };
        let outcome = match evaluate_metric(
            &policy.per_scenario,
            &weights,
            |row| &row.outcome_class_regret,
            alphas,
        ) {
            Ok(value) => value,
            Err(reason) => return invalid(case, &reason),
        };
        policies.push(PolicyRisk {
            policy_id: policy.policy_id.clone(),
            wait_and_see_regret: wait,
            outcome_class_regret: outcome,
        });
    }
    policies.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));
    CaseResult {
        case_id: case.case_id.clone(),
        status: "PASS".to_owned(),
        reason: "PROBABILITY_MODEL_CONDITIONED_METRICS_COMPUTED".to_owned(),
        probability_model_id: Some(receipt.probability_model_id.clone()),
        scenario_count: weights.len(),
        policies,
        authority_granted: false,
    }
}

pub fn evaluate(corpus: &Corpus) -> Result<Evaluation, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
        || corpus.alphas.is_empty()
        || corpus.alphas.len() > MAX_ALPHAS
    {
        return Err("unsupported CVaR corpus or claim ceiling".to_owned());
    }
    let mut alpha_set = BTreeSet::new();
    for alpha in &corpus.alphas {
        if !alpha.valid()
            || alpha.numerator == 0
            || alpha.compare_exact(&Rational::one()) != Ordering::Less
            || !alpha_set.insert((alpha.numerator, alpha.denominator))
        {
            return Err(
                "CVaR alphas must be unique exact rationals between zero and one".to_owned(),
            );
        }
    }
    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("CVaR case IDs must be unique".to_owned());
        }
        results.push(evaluate_case(case, &corpus.alphas));
    }
    results.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    let count = |status: &str| results.iter().filter(|item| item.status == status).count();
    let invalid_input_cases = count("INVALID_INPUT");
    let summary = Summary {
        case_count: results.len(),
        pass_cases: count("PASS"),
        abstain_cases: count("ABSTAIN"),
        invalid_input_cases,
        authority_grants: results.iter().filter(|item| item.authority_granted).count(),
    };
    Ok(Evaluation {
        result: if invalid_input_cases == 0 && summary.authority_grants == 0 {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases: results,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(numerator: u64, denominator: u64) -> Rational {
        Rational::new(numerator, denominator).expect("valid rational")
    }

    #[test]
    fn tail_smaller_than_one_atom_uses_fractional_var_mass() {
        let values = vec![(r(0, 1), r(9, 10)), (r(10, 1), r(1, 10))];
        let result = cvar_upper_tail(&values, &r(19, 20)).expect("exact CVaR");
        assert_eq!(result.var, r(10, 1));
        assert_eq!(result.cvar, r(10, 1));
        assert_eq!(result.tail_mass, r(1, 20));
        assert_eq!(result.var_atom_fraction, r(1, 2));
    }

    #[test]
    fn partial_var_atom_uses_exact_weighted_boundary() {
        let values = vec![(r(0, 1), r(7, 10)), (r(3, 1), r(1, 5)), (r(8, 1), r(1, 10))];
        let result = cvar_upper_tail(&values, &r(17, 20)).expect("exact CVaR");
        assert_eq!(result.var, r(3, 1));
        assert_eq!(result.cvar, r(19, 3));
        assert_eq!(result.var_atom_fraction, r(1, 4));
    }

    #[test]
    fn zero_mass_high_loss_does_not_enter_probability_tail() {
        let values = vec![(r(2, 1), r(1, 1)), (r(1000, 1), r(0, 1))];
        let result = cvar_upper_tail(&values, &r(9, 10)).expect("zero-mass support");
        assert_eq!(result.var, r(2, 1));
        assert_eq!(result.cvar, r(2, 1));
    }

    fn fixture_corpus() -> Corpus {
        serde_json::from_str(include_str!("../fixtures/v0.1/cvar-regret-cases.json"))
            .expect("CVaR fixture corpus must parse")
    }

    #[test]
    fn calibrated_fixture_matches_exact_known_answers_and_abstains_without_evidence() {
        let corpus = fixture_corpus();
        let result = evaluate(&corpus).expect("CVaR fixture evaluates");
        assert_eq!(result.result, "pass");
        assert_eq!(result.summary.case_count, 4);
        assert_eq!(result.summary.pass_cases, 1);
        assert_eq!(result.summary.abstain_cases, 3);
        assert_eq!(result.summary.invalid_input_cases, 0);
        assert_eq!(result.summary.authority_grants, 0);

        let qualified = result
            .cases
            .iter()
            .find(|case| case.case_id == "cvar-case-1")
            .expect("qualified case exists");
        assert_eq!(qualified.status, "PASS");
        let incumbent = qualified
            .policies
            .iter()
            .find(|policy| policy.policy_id == "incumbent-static")
            .expect("incumbent policy exists");
        assert_eq!(incumbent.wait_and_see_regret.expected_regret, r(16, 25));
        assert_eq!(incumbent.wait_and_see_regret.worst_case_regret, r(10, 1));
        assert_eq!(
            incumbent
                .wait_and_see_regret
                .cvar
                .iter()
                .map(|point| point.cvar.clone())
                .collect::<Vec<_>>(),
            vec![r(22, 5), r(29, 5), r(10, 1)]
        );

        let candidate = qualified
            .policies
            .iter()
            .find(|policy| policy.policy_id == "candidate-observable")
            .expect("candidate policy exists");
        assert_eq!(candidate.wait_and_see_regret.expected_regret, r(19, 50));
        assert_eq!(
            candidate
                .wait_and_see_regret
                .cvar
                .iter()
                .map(|point| point.cvar.clone())
                .collect::<Vec<_>>(),
            vec![r(14, 5), r(18, 5), r(6, 1)]
        );

        let absent = result
            .cases
            .iter()
            .find(|case| case.case_id == "cvar-case-no-probability")
            .expect("absence case exists");
        assert_eq!(absent.status, "ABSTAIN");
        assert_eq!(absent.reason, "PROBABILITY_MODEL_RECEIPT_ABSENT");
        assert!(!result.authority_granted);
    }

    #[test]
    fn non_normalized_probability_receipt_fails_closed() {
        let mut corpus = fixture_corpus();
        corpus.cases[0].probability_model.as_mut().unwrap().weights[0].weight = r(1, 2);
        let result = evaluate(&corpus).expect("invalid case produces a report");
        assert_eq!(result.result, "fail");
        assert_eq!(result.summary.invalid_input_cases, 1);
        assert_eq!(
            result.cases[0].reason,
            "probability weights do not sum exactly to one"
        );
        assert_eq!(result.summary.authority_grants, 0);
    }

    #[test]
    fn probability_model_support_must_match_full_joint_inventory() {
        let mut corpus = fixture_corpus();
        corpus.cases[0]
            .probability_model
            .as_mut()
            .unwrap()
            .weights
            .pop();
        let result = evaluate(&corpus).expect("invalid support produces a report");
        assert_eq!(result.cases[0].status, "INVALID_INPUT");
        assert_eq!(
            result.cases[0].reason,
            "probability weights do not cover the exact joint scenario inventory"
        );
    }

    #[test]
    fn stale_and_unqualified_probability_models_abstain() {
        let corpus = fixture_corpus();
        let result = evaluate(&corpus).expect("unqualified cases produce an abstention report");
        let stale = result
            .cases
            .iter()
            .find(|case| case.case_id == "cvar-case-stale-model")
            .unwrap();
        let unqualified = result
            .cases
            .iter()
            .find(|case| case.case_id == "cvar-case-unqualified-calibration")
            .unwrap();
        assert_eq!(stale.status, "ABSTAIN");
        assert_eq!(stale.reason, "PROBABILITY_MODEL_NOT_CURRENT");
        assert_eq!(unqualified.status, "ABSTAIN");
        assert_eq!(unqualified.reason, "PROBABILITY_CALIBRATION_NOT_QUALIFIED");
    }

    #[test]
    fn weights_must_normalize_exactly() {
        let values = vec![(r(1, 1), r(1, 2)), (r(2, 1), r(1, 4))];
        assert!(cvar_upper_tail(&values, &r(9, 10)).is_err());
    }
}
