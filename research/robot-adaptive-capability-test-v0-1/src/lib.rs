//! Deterministic adaptive item selection for one calibrated robot capability axis.
//!
//! Evaluator-side measurement utility only. It does not control a robot,
//! generate physical challenges, authenticate evidence, or qualify safety.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const MAX_ABS_LOGIT: f64 = 20.0;
const NEWTON_ITERATIONS: usize = 64;
const NEWTON_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StratumMinimum {
    pub stratum: String,
    pub minimum_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationProfile {
    pub axis_id: String,
    pub calibration_id: String,
    pub calibration_digest: String,
    pub prior_mean: f64,
    pub prior_std_dev: f64,
    pub target_posterior_sem: f64,
    pub max_adaptive_items: usize,
    pub minimum_per_stratum: Vec<StratumMinimum>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeItem {
    pub item_id: String,
    pub stratum: String,
    pub difficulty: f64,
    pub mandatory: bool,
    pub common_anchor: bool,
    pub safety_critical: bool,
    /// Whether this item is calibrated on the profile's one capability axis.
    /// Mandatory gate-only controls should set this to false.
    pub contributes_to_capability_estimate: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemBank {
    pub bank_id: String,
    pub bank_digest: String,
    pub axis_id: String,
    pub calibration_id: String,
    pub calibration_digest: String,
    pub items: Vec<ChallengeItem>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseOutcome {
    Success,
    Failure,
    /// An observation was attempted but did not support a binary judgment.
    Unknown,
    /// The independent evaluator detected an unsafe outcome.
    UnsafeViolation,
    /// Evidence was malformed, stale, or otherwise inadmissible.
    InvalidEvidence,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Response {
    pub sequence: u32,
    pub item_id: String,
    /// Supplied by the independent task evaluator, not the robot itself.
    pub outcome: ResponseOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionReason {
    SafetyCriticalControl,
    CommonAnchor,
    Mandatory,
    StratumMinimum,
    MaximumInformation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopReason {
    PrecisionReached,
    BudgetExhausted,
    BankExhausted,
    EvidenceRequired,
    SafetyGateFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedItem {
    pub item_id: String,
    pub reason: SelectionReason,
    pub expected_information: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StratumCoverage {
    pub stratum: String,
    pub administered: usize,
    pub required: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub selected: Option<SelectedItem>,
    pub stop_reason: Option<StopReason>,
    pub theta_map: f64,
    pub posterior_sem: f64,
    pub mandatory_total: usize,
    pub mandatory_administered: usize,
    pub adaptive_administered: usize,
    pub stratum_coverage: Vec<StratumCoverage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectionRecord {
    pub sequence: u32,
    pub item_id: String,
    pub reason: SelectionReason,
    pub theta_before: f64,
    pub posterior_sem_before: f64,
    pub expected_information: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunReceipt {
    pub axis_id: String,
    pub bank_id: String,
    pub bank_digest: String,
    pub calibration_id: String,
    pub calibration_digest: String,
    pub selection_trace: Vec<SelectionRecord>,
    /// Ordered evaluator outcomes, including Unknown and safety violations.
    pub responses: Vec<Response>,
    pub decision: Decision,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate(bank: &ItemBank, profile: &CalibrationProfile) -> Result<(), String> {
    if !nonempty(&bank.bank_id) || !nonempty(&profile.axis_id) || !nonempty(&profile.calibration_id)
    {
        return Err("bank, capability axis or calibration identity is missing".to_owned());
    }
    if !valid_digest(&bank.bank_digest)
        || !valid_digest(&bank.calibration_digest)
        || !valid_digest(&profile.calibration_digest)
    {
        return Err("bank/calibration digest must be canonical lowercase SHA-256 hex".to_owned());
    }
    if bank.axis_id != profile.axis_id {
        return Err("item bank and profile capability axes do not match".to_owned());
    }
    if bank.calibration_id != profile.calibration_id
        || bank.calibration_digest != profile.calibration_digest
    {
        return Err("item bank and profile calibration identities do not match".to_owned());
    }
    if !profile.prior_mean.is_finite() || profile.prior_mean.abs() > MAX_ABS_LOGIT {
        return Err("prior mean must be finite and within the supported logit range".to_owned());
    }
    if !profile.prior_std_dev.is_finite()
        || profile.prior_std_dev <= 0.0
        || profile.prior_std_dev > MAX_ABS_LOGIT
    {
        return Err("prior standard deviation must be finite and in (0, 20]".to_owned());
    }
    if !profile.target_posterior_sem.is_finite() || profile.target_posterior_sem <= 0.0 {
        return Err("target posterior standard error must be finite and positive".to_owned());
    }
    if bank.items.is_empty() {
        return Err("item bank must contain at least one challenge".to_owned());
    }

    let mut item_ids = BTreeSet::new();
    let mut counts_by_stratum = BTreeMap::<&str, usize>::new();
    for item in &bank.items {
        if !nonempty(&item.item_id) || !nonempty(&item.stratum) {
            return Err("challenge item identity and stratum must be nonempty".to_owned());
        }
        if !item_ids.insert(item.item_id.as_str()) {
            return Err(format!("duplicate challenge item ID: {}", item.item_id));
        }
        if !item.difficulty.is_finite() || item.difficulty.abs() > MAX_ABS_LOGIT {
            return Err(format!(
                "item {} has invalid calibrated difficulty",
                item.item_id
            ));
        }
        if item.common_anchor && !item.mandatory {
            return Err(format!("common anchor {} must be mandatory", item.item_id));
        }
        if item.common_anchor && !item.contributes_to_capability_estimate {
            return Err(format!(
                "common anchor {} must measure the declared capability axis",
                item.item_id
            ));
        }
        if item.safety_critical && !item.mandatory {
            return Err(format!(
                "safety-critical item {} must be mandatory",
                item.item_id
            ));
        }
        *counts_by_stratum.entry(item.stratum.as_str()).or_default() += 1;
    }

    let mut quota_strata = BTreeSet::new();
    for quota in &profile.minimum_per_stratum {
        if !nonempty(&quota.stratum) || quota.minimum_count == 0 {
            return Err(
                "stratum minimum requires a nonempty stratum and positive count".to_owned(),
            );
        }
        if !quota_strata.insert(quota.stratum.as_str()) {
            return Err(format!("duplicate minimum for stratum {}", quota.stratum));
        }
        let available = counts_by_stratum
            .get(quota.stratum.as_str())
            .copied()
            .unwrap_or(0);
        if available < quota.minimum_count {
            return Err(format!(
                "stratum {} requires {} items but bank contains {}",
                quota.stratum, quota.minimum_count, available
            ));
        }
    }
    Ok(())
}

fn sigmoid(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exponential = value.exp();
        exponential / (1.0 + exponential)
    }
}

fn item_information(theta: f64, difficulty: f64) -> f64 {
    let probability = sigmoid(theta - difficulty);
    probability * (1.0 - probability)
}

/// MAP estimate under a fixed Normal prior and a one-parameter logistic Rasch model.
/// Responses are sorted by item ID before summation for order-independent arithmetic.
fn posterior(
    bank: &ItemBank,
    profile: &CalibrationProfile,
    responses: &[Response],
) -> Result<(f64, f64), String> {
    let item_by_id: BTreeMap<&str, &ChallengeItem> = bank
        .items
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect();
    let mut ordered: Vec<&Response> = responses.iter().collect();
    ordered.sort_by(|left, right| left.item_id.cmp(&right.item_id));

    let prior_variance = profile.prior_std_dev * profile.prior_std_dev;
    let prior_precision = 1.0 / prior_variance;
    if !prior_precision.is_finite() || prior_precision <= 0.0 {
        return Err("prior precision is not numerically representable".to_owned());
    }

    let mut theta = profile.prior_mean;
    for _ in 0..NEWTON_ITERATIONS {
        let mut gradient = -(theta - profile.prior_mean) * prior_precision;
        let mut information = prior_precision;
        for response in &ordered {
            let item = item_by_id
                .get(response.item_id.as_str())
                .copied()
                .ok_or_else(|| format!("response references unknown item {}", response.item_id))?;
            if !item.contributes_to_capability_estimate {
                continue;
            }
            let success = match response.outcome {
                ResponseOutcome::Success => true,
                ResponseOutcome::Failure => false,
                ResponseOutcome::Unknown
                | ResponseOutcome::UnsafeViolation
                | ResponseOutcome::InvalidEvidence => continue,
            };
            let probability = sigmoid(theta - item.difficulty);
            gradient += if success {
                1.0 - probability
            } else {
                -probability
            };
            information += probability * (1.0 - probability);
        }
        let step = gradient / information;
        if !step.is_finite() {
            return Err("posterior Newton step is non-finite".to_owned());
        }
        let next = theta + step;
        if !next.is_finite() || next.abs() > MAX_ABS_LOGIT {
            return Err("posterior estimate left the supported logit range".to_owned());
        }
        theta = next;
        if step.abs() <= NEWTON_TOLERANCE {
            break;
        }
    }

    let mut information = prior_precision;
    for response in ordered {
        let item = item_by_id[response.item_id.as_str()];
        if !item.contributes_to_capability_estimate {
            continue;
        }
        if !matches!(
            response.outcome,
            ResponseOutcome::Success | ResponseOutcome::Failure
        ) {
            continue;
        }
        let probability = sigmoid(theta - item.difficulty);
        information += probability * (1.0 - probability);
    }
    let sem = 1.0 / information.sqrt();
    if !sem.is_finite() || sem <= 0.0 {
        return Err("posterior standard error is not numerically representable".to_owned());
    }
    Ok((theta, sem))
}

fn validate_responses(bank: &ItemBank, responses: &[Response]) -> Result<(), String> {
    if responses.len() > bank.items.len() {
        return Err("response count exceeds the item bank size".to_owned());
    }
    let item_ids: BTreeSet<&str> = bank
        .items
        .iter()
        .map(|item| item.item_id.as_str())
        .collect();
    let mut seen = BTreeSet::new();
    for (index, response) in responses.iter().enumerate() {
        if response.sequence as usize != index {
            return Err("response sequence numbers must be contiguous from zero".to_owned());
        }
        if !item_ids.contains(response.item_id.as_str()) {
            return Err(format!(
                "response references unknown item {}",
                response.item_id
            ));
        }
        if !seen.insert(response.item_id.as_str()) {
            return Err(format!("duplicate response for item {}", response.item_id));
        }
        if response.outcome == ResponseOutcome::InvalidEvidence {
            return Err(format!(
                "response for item {} contains inadmissible evidence",
                response.item_id
            ));
        }
    }
    Ok(())
}

fn response_counts(
    bank: &ItemBank,
    responses: &[Response],
) -> (BTreeSet<String>, BTreeMap<String, usize>) {
    let mut administered = BTreeSet::new();
    let mut strata = BTreeMap::<String, usize>::new();
    for response in responses {
        administered.insert(response.item_id.clone());
        if let Some(item) = bank
            .items
            .iter()
            .find(|item| item.item_id == response.item_id)
        {
            *strata.entry(item.stratum.clone()).or_default() += 1;
        }
    }
    (administered, strata)
}

fn coverage(
    profile: &CalibrationProfile,
    counts: &BTreeMap<String, usize>,
) -> Vec<StratumCoverage> {
    let mut quotas = profile.minimum_per_stratum.clone();
    quotas.sort_by(|left, right| left.stratum.cmp(&right.stratum));
    quotas
        .into_iter()
        .map(|quota| StratumCoverage {
            administered: counts.get(&quota.stratum).copied().unwrap_or(0),
            stratum: quota.stratum,
            required: quota.minimum_count,
        })
        .collect()
}

fn choose_most_informative(
    candidates: Vec<&ChallengeItem>,
    theta: f64,
    reason: SelectionReason,
) -> Option<SelectedItem> {
    let mut scored: Vec<(&ChallengeItem, f64)> = candidates
        .into_iter()
        .map(|item| {
            let information = if item.contributes_to_capability_estimate {
                item_information(theta, item.difficulty)
            } else {
                0.0
            };
            (item, information)
        })
        .collect();
    scored.sort_by(|(left_item, left_info), (right_item, right_info)| {
        right_info
            .total_cmp(left_info)
            .then_with(|| left_item.item_id.cmp(&right_item.item_id))
    });
    scored.first().map(|(item, information)| SelectedItem {
        item_id: item.item_id.clone(),
        reason,
        expected_information: *information,
    })
}

fn decide_validated(
    bank: &ItemBank,
    profile: &CalibrationProfile,
    responses: &[Response],
) -> Result<Decision, String> {
    let (theta, sem) = posterior(bank, profile, responses)?;
    let (administered, counts) = response_counts(bank, responses);
    let mandatory_total = bank.items.iter().filter(|item| item.mandatory).count();
    let mandatory_administered = bank
        .items
        .iter()
        .filter(|item| item.mandatory && administered.contains(&item.item_id))
        .count();
    let adaptive_administered = responses
        .iter()
        .filter(|response| {
            bank.items
                .iter()
                .any(|item| item.item_id == response.item_id && !item.mandatory)
        })
        .count();
    let stratum_coverage = coverage(profile, &counts);

    // Unknown observation evidence is not a binary failure. A safety-critical
    // failure or explicit unsafe outcome terminates the test before adaptation.
    for response in responses {
        let item = bank
            .items
            .iter()
            .find(|item| item.item_id == response.item_id)
            .ok_or_else(|| format!("response references unknown item {}", response.item_id))?;
        let terminal_reason = match response.outcome {
            ResponseOutcome::Unknown => Some(StopReason::EvidenceRequired),
            ResponseOutcome::UnsafeViolation => Some(StopReason::SafetyGateFailed),
            ResponseOutcome::Failure if item.safety_critical => {
                Some(StopReason::SafetyGateFailed)
            }
            ResponseOutcome::Success | ResponseOutcome::Failure => None,
            ResponseOutcome::InvalidEvidence => {
                return Err(format!(
                    "response for item {} contains inadmissible evidence",
                    response.item_id
                ));
            }
        };
        if let Some(stop_reason) = terminal_reason {
            return Ok(Decision {
                selected: None,
                stop_reason: Some(stop_reason),
                theta_map: theta,
                posterior_sem: sem,
                mandatory_total,
                mandatory_administered,
                adaptive_administered,
                stratum_coverage,
            });
        }
    }

    let mut mandatory_candidates: Vec<&ChallengeItem> = bank
        .items
        .iter()
        .filter(|item| item.mandatory && !administered.contains(&item.item_id))
        .collect();
    mandatory_candidates.sort_by(|left, right| {
        let priority = |item: &ChallengeItem| {
            if item.safety_critical {
                0_u8
            } else if item.common_anchor {
                1_u8
            } else {
                2_u8
            }
        };
        priority(left)
            .cmp(&priority(right))
            .then_with(|| left.item_id.cmp(&right.item_id))
    });
    if let Some(item) = mandatory_candidates.first() {
        let reason = if item.safety_critical {
            SelectionReason::SafetyCriticalControl
        } else if item.common_anchor {
            SelectionReason::CommonAnchor
        } else {
            SelectionReason::Mandatory
        };
        return Ok(Decision {
            selected: Some(SelectedItem {
                item_id: item.item_id.clone(),
                reason,
                expected_information: if item.contributes_to_capability_estimate {
                    item_information(theta, item.difficulty)
                } else {
                    0.0
                },
            }),
            stop_reason: None,
            theta_map: theta,
            posterior_sem: sem,
            mandatory_total,
            mandatory_administered,
            adaptive_administered,
            stratum_coverage,
        });
    }

    let mut quotas = profile.minimum_per_stratum.clone();
    quotas.sort_by(|left, right| left.stratum.cmp(&right.stratum));
    for quota in quotas {
        let count = counts.get(&quota.stratum).copied().unwrap_or(0);
        if count < quota.minimum_count {
            if adaptive_administered >= profile.max_adaptive_items {
                return Ok(Decision {
                    selected: None,
                    stop_reason: Some(StopReason::BudgetExhausted),
                    theta_map: theta,
                    posterior_sem: sem,
                    mandatory_total,
                    mandatory_administered,
                    adaptive_administered,
                    stratum_coverage,
                });
            }
            let candidates: Vec<&ChallengeItem> = bank
                .items
                .iter()
                .filter(|item| {
                    item.stratum == quota.stratum
                        && !item.mandatory
                        && !administered.contains(&item.item_id)
                })
                .collect();
            let selected =
                choose_most_informative(candidates, theta, SelectionReason::StratumMinimum);
            if selected.is_none() {
                return Ok(Decision {
                    selected: None,
                    stop_reason: Some(StopReason::BankExhausted),
                    theta_map: theta,
                    posterior_sem: sem,
                    mandatory_total,
                    mandatory_administered,
                    adaptive_administered,
                    stratum_coverage,
                });
            }
            return Ok(Decision {
                selected,
                stop_reason: None,
                theta_map: theta,
                posterior_sem: sem,
                mandatory_total,
                mandatory_administered,
                adaptive_administered,
                stratum_coverage,
            });
        }
    }

    if sem <= profile.target_posterior_sem {
        return Ok(Decision {
            selected: None,
            stop_reason: Some(StopReason::PrecisionReached),
            theta_map: theta,
            posterior_sem: sem,
            mandatory_total,
            mandatory_administered,
            adaptive_administered,
            stratum_coverage,
        });
    }

    if adaptive_administered >= profile.max_adaptive_items {
        return Ok(Decision {
            selected: None,
            stop_reason: Some(StopReason::BudgetExhausted),
            theta_map: theta,
            posterior_sem: sem,
            mandatory_total,
            mandatory_administered,
            adaptive_administered,
            stratum_coverage,
        });
    }

    let candidates: Vec<&ChallengeItem> = bank
        .items
        .iter()
        .filter(|item| {
            !item.mandatory
                && item.contributes_to_capability_estimate
                && !administered.contains(&item.item_id)
        })
        .collect();
    let selected = choose_most_informative(candidates, theta, SelectionReason::MaximumInformation);
    if selected.is_none() {
        return Ok(Decision {
            selected: None,
            stop_reason: Some(StopReason::BankExhausted),
            theta_map: theta,
            posterior_sem: sem,
            mandatory_total,
            mandatory_administered,
            adaptive_administered,
            stratum_coverage,
        });
    }
    Ok(Decision {
        selected,
        stop_reason: None,
        theta_map: theta,
        posterior_sem: sem,
        mandatory_total,
        mandatory_administered,
        adaptive_administered,
        stratum_coverage,
    })
}

/// Replay a proposed adaptive run and verify every item was the selector's
/// actual next choice given the preceding response history.
///
/// The returned decision is evaluator-side. Do not pass posterior state,
/// calibrated difficulty, expected information or future selection trace to
/// the policy being evaluated.
pub fn evaluate_run(
    bank: &ItemBank,
    profile: &CalibrationProfile,
    responses: &[Response],
) -> Result<RunReceipt, String> {
    validate(bank, profile)?;
    validate_responses(bank, responses)?;

    let mut accepted_responses = Vec::<Response>::new();
    let mut selection_trace = Vec::<SelectionRecord>::new();

    for response in responses {
        let decision = decide_validated(bank, profile, &accepted_responses)?;
        let selected = decision.selected.ok_or_else(|| {
            format!(
                "response {} was recorded after the selector had stopped ({:?})",
                response.item_id, decision.stop_reason
            )
        })?;
        if selected.item_id != response.item_id {
            return Err(format!(
                "response sequence diverges from deterministic selector: expected {}, got {}",
                selected.item_id, response.item_id
            ));
        }
        selection_trace.push(SelectionRecord {
            sequence: response.sequence,
            item_id: selected.item_id,
            reason: selected.reason,
            theta_before: decision.theta_map,
            posterior_sem_before: decision.posterior_sem,
            expected_information: selected.expected_information,
        });
        accepted_responses.push(response.clone());
    }

    let decision = decide_validated(bank, profile, &accepted_responses)?;
    Ok(RunReceipt {
        axis_id: bank.axis_id.clone(),
        bank_id: bank.bank_id.clone(),
        bank_digest: bank.bank_digest.clone(),
        calibration_id: bank.calibration_id.clone(),
        calibration_digest: bank.calibration_digest.clone(),
        selection_trace,
        responses: accepted_responses,
        decision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn item(
        id: &str,
        stratum: &str,
        difficulty: f64,
        mandatory: bool,
        common_anchor: bool,
        safety_critical: bool,
    ) -> ChallengeItem {
        ChallengeItem {
            item_id: id.to_owned(),
            stratum: stratum.to_owned(),
            difficulty,
            mandatory,
            common_anchor,
            safety_critical,
            contributes_to_capability_estimate: !safety_critical,
        }
    }

    fn profile() -> CalibrationProfile {
        CalibrationProfile {
            axis_id: "manipulation-assembly".to_owned(),
            calibration_id: "calibration-2026-11-v1".to_owned(),
            calibration_digest: DIGEST_A.to_owned(),
            prior_mean: 0.0,
            prior_std_dev: 1.0,
            target_posterior_sem: 0.05,
            max_adaptive_items: 8,
            minimum_per_stratum: vec![],
        }
    }

    fn bank() -> ItemBank {
        ItemBank {
            bank_id: "assembly-bank-v1".to_owned(),
            bank_digest: DIGEST_B.to_owned(),
            axis_id: "manipulation-assembly".to_owned(),
            calibration_id: "calibration-2026-11-v1".to_owned(),
            calibration_digest: DIGEST_A.to_owned(),
            items: vec![
                item("anchor-common", "nominal", 0.0, true, true, false),
                item("adaptive-easy", "nominal", -2.0, false, false, false),
                item("adaptive-hard", "nominal", 2.0, false, false, false),
                item(
                    "safety-negative",
                    "negative-control",
                    0.0,
                    true,
                    false,
                    true,
                ),
            ],
        }
    }

    fn response(sequence: u32, item_id: &str, success: bool) -> Response {
        Response {
            sequence,
            item_id: item_id.to_owned(),
            outcome: if success {
                ResponseOutcome::Success
            } else {
                ResponseOutcome::Failure
            },
        }
    }

    #[test]
    fn safety_control_precedes_common_anchor_and_adaptive_items() {
        let decision = evaluate_run(&bank(), &profile(), &[]).unwrap().decision;
        let selected = decision.selected.unwrap();
        assert_eq!(selected.item_id, "safety-negative");
        assert_eq!(selected.reason, SelectionReason::SafetyCriticalControl);
    }

    #[test]
    fn common_anchor_is_administered_before_adaptive_items() {
        let result = evaluate_run(
            &bank(),
            &profile(),
            &[response(0, "safety-negative", true)],
        )
        .unwrap();
        let selected = result.decision.selected.unwrap();
        assert_eq!(selected.item_id, "anchor-common");
        assert_eq!(selected.reason, SelectionReason::CommonAnchor);
    }

    #[test]
    fn high_capability_history_selects_harder_informative_item() {
        let result = evaluate_run(
            &bank(),
            &profile(),
            &[
                response(0, "safety-negative", true),
                response(1, "anchor-common", true),
            ],
        )
        .unwrap();
        assert_eq!(result.decision.selected.unwrap().item_id, "adaptive-hard");
    }

    #[test]
    fn low_capability_history_selects_easier_informative_item() {
        let result = evaluate_run(
            &bank(),
            &profile(),
            &[
                response(0, "safety-negative", true),
                response(1, "anchor-common", false),
            ],
        )
        .unwrap();
        assert_eq!(result.decision.selected.unwrap().item_id, "adaptive-easy");
    }

    #[test]
    fn equal_information_tie_uses_canonical_item_id() {
        let mut test_bank = bank();
        test_bank.items = vec![
            item("adaptive-z", "nominal", -1.0, false, false, false),
            item("adaptive-a", "nominal", 1.0, false, false, false),
        ];
        let result = evaluate_run(&test_bank, &profile(), &[]).unwrap();
        assert_eq!(result.decision.selected.unwrap().item_id, "adaptive-a");
    }

    #[test]
    fn minimum_stratum_coverage_is_met_before_precision_stopping() {
        let mut test_profile = profile();
        test_profile.target_posterior_sem = 10.0;
        test_profile.minimum_per_stratum = vec![StratumMinimum {
            stratum: "payload-shift".to_owned(),
            minimum_count: 1,
        }];
        let mut test_bank = bank();
        test_bank.items.push(item(
            "payload-item",
            "payload-shift",
            0.0,
            false,
            false,
            false,
        ));
        let result = evaluate_run(
            &test_bank,
            &test_profile,
            &[
                response(0, "safety-negative", true),
                response(1, "anchor-common", true),
            ],
        )
        .unwrap();
        let selected = result.decision.selected.unwrap();
        assert_eq!(selected.item_id, "payload-item");
        assert_eq!(selected.reason, SelectionReason::StratumMinimum);
    }

    #[test]
    fn stratum_minimum_cannot_exceed_adaptive_budget() {
        let mut test_profile = profile();
        test_profile.max_adaptive_items = 0;
        test_profile.target_posterior_sem = 10.0;
        test_profile.minimum_per_stratum = vec![StratumMinimum {
            stratum: "payload-shift".to_owned(),
            minimum_count: 1,
        }];
        let mut test_bank = bank();
        test_bank.items.push(item(
            "payload-item",
            "payload-shift",
            0.0,
            false,
            false,
            false,
        ));
        let result = evaluate_run(
            &test_bank,
            &test_profile,
            &[
                response(0, "safety-negative", true),
                response(1, "anchor-common", true),
            ],
        )
        .unwrap();
        assert_eq!(
            result.decision.stop_reason,
            Some(StopReason::BudgetExhausted)
        );
        assert_eq!(result.decision.adaptive_administered, 0);
        assert_eq!(result.decision.stratum_coverage[0].administered, 0);
    }

    #[test]
    fn precision_stop_occurs_only_after_mandatory_and_stratum_coverage() {
        let mut test_profile = profile();
        test_profile.target_posterior_sem = 10.0;
        let responses = vec![
            response(0, "safety-negative", true),
            response(1, "anchor-common", true),
        ];
        let result = evaluate_run(&bank(), &test_profile, &responses).unwrap();
        assert_eq!(
            result.decision.stop_reason,
            Some(StopReason::PrecisionReached)
        );
        assert!(result.decision.selected.is_none());
    }

    #[test]
    fn adaptive_budget_exhaustion_is_not_precision_success() {
        let mut test_profile = profile();
        test_profile.max_adaptive_items = 0;
        let responses = vec![
            response(0, "safety-negative", true),
            response(1, "anchor-common", true),
        ];
        let result = evaluate_run(&bank(), &test_profile, &responses).unwrap();
        assert_eq!(
            result.decision.stop_reason,
            Some(StopReason::BudgetExhausted)
        );
        assert!(result.decision.posterior_sem > test_profile.target_posterior_sem);
    }

    #[test]
    fn bank_exhaustion_is_explicit() {
        let mut test_profile = profile();
        test_profile.max_adaptive_items = 5;
        let mut test_bank = bank();
        test_bank.items.retain(|item| item.mandatory);
        let result = evaluate_run(
            &test_bank,
            &test_profile,
            &[
                response(0, "safety-negative", true),
                response(1, "anchor-common", true),
            ],
        )
        .unwrap();
        assert_eq!(result.decision.stop_reason, Some(StopReason::BankExhausted));
    }

    #[test]
    fn duplicate_item_id_is_invalid() {
        let mut test_bank = bank();
        test_bank
            .items
            .push(item("anchor-common", "other", 0.0, false, false, false));
        assert!(evaluate_run(&test_bank, &profile(), &[]).is_err());
    }

    #[test]
    fn duplicate_response_is_invalid() {
        let responses = vec![
            response(0, "safety-negative", true),
            response(1, "safety-negative", false),
        ];
        assert!(evaluate_run(&bank(), &profile(), &responses).is_err());
    }

    #[test]
    fn unknown_item_response_is_invalid() {
        assert!(evaluate_run(&bank(), &profile(), &[response(0, "unknown-item", true)]).is_err());
    }

    #[test]
    fn mismatched_calibration_digest_is_invalid() {
        let mut test_profile = profile();
        test_profile.calibration_digest = DIGEST_B.to_owned();
        assert!(evaluate_run(&bank(), &test_profile, &[]).is_err());
    }

    #[test]
    fn invalid_item_difficulty_is_rejected() {
        let mut test_bank = bank();
        test_bank.items[1].difficulty = f64::NAN;
        assert!(evaluate_run(&test_bank, &profile(), &[]).is_err());
    }

    #[test]
    fn common_anchor_and_safety_control_cannot_be_adaptive() {
        let mut test_bank = bank();
        test_bank.items[0].mandatory = false;
        assert!(evaluate_run(&test_bank, &profile(), &[]).is_err());
        test_bank = bank();
        test_bank.items[3].mandatory = false;
        assert!(evaluate_run(&test_bank, &profile(), &[]).is_err());
    }

    #[test]
    fn response_cannot_skip_the_deterministic_next_item() {
        assert!(evaluate_run(&bank(), &profile(), &[response(0, "anchor-common", true)]).is_err());
    }

    #[test]
    fn response_sequence_must_be_contiguous() {
        assert!(
            evaluate_run(&bank(), &profile(), &[response(1, "safety-negative", true)]).is_err()
        );
    }

    #[test]
    fn changing_history_changes_the_next_challenge() {
        let success = vec![
            response(0, "safety-negative", true),
            response(1, "anchor-common", true),
        ];
        let failure = vec![
            response(0, "safety-negative", true),
            response(1, "anchor-common", false),
        ];
        let success_next = evaluate_run(&bank(), &profile(), &success)
            .unwrap()
            .decision
            .selected
            .unwrap()
            .item_id;
        let failure_next = evaluate_run(&bank(), &profile(), &failure)
            .unwrap()
            .decision
            .selected
            .unwrap()
            .item_id;
        assert_ne!(success_next, failure_next);
    }

    #[test]
    fn posterior_estimate_is_independent_of_response_container_order() {
        let responses = vec![
            response(0, "safety-negative", true),
            response(1, "anchor-common", false),
        ];
        let mut reversed = responses.clone();
        reversed.reverse();
        let first = posterior(&bank(), &profile(), &responses).unwrap();
        let second = posterior(&bank(), &profile(), &reversed).unwrap();
        assert!((first.0 - second.0).abs() < 1e-14);
        assert!((first.1 - second.1).abs() < 1e-14);
    }

    #[test]
    fn ordered_selection_trace_is_replayable() {
        let responses = vec![
            response(0, "safety-negative", true),
            response(1, "anchor-common", true),
            response(2, "adaptive-hard", true),
        ];
        let receipt = evaluate_run(&bank(), &profile(), &responses).unwrap();
        assert_eq!(receipt.selection_trace.len(), responses.len());
        for (index, record) in receipt.selection_trace.iter().enumerate() {
            assert_eq!(record.sequence as usize, index);
            assert_eq!(record.item_id, responses[index].item_id);
        }
        assert_eq!(receipt.bank_digest, DIGEST_B);
        assert_eq!(receipt.calibration_digest, DIGEST_A);
    }

    #[test]
    fn distinct_capability_axes_must_not_be_pooled() {
        let mut test_profile = profile();
        test_profile.axis_id = "terrain-mobility".to_owned();
        assert!(evaluate_run(&bank(), &test_profile, &[]).is_err());
    }
}
