use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

pub const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub claim_ceiling: String,
    pub scope_id: String,
    pub likelihood_denominator: u64,
    pub prior_serviceable_weight: u64,
    pub prior_degraded_weight: u64,
    pub actions: Vec<LossAction>,
    pub inspections: Vec<Inspection>,
    pub expected_baseline_action: String,
    pub expected_baseline_loss_numerator: u64,
    pub expected_perfect_information_loss_numerator: u64,
    pub expected_max_gross_evpi_numerator: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LossAction {
    pub id: String,
    pub loss_if_serviceable: u64,
    pub loss_if_degraded: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Inspection {
    pub id: String,
    pub cost: u64,
    pub serviceable_alarm_weight: u64,
    pub degraded_alarm_weight: u64,
    pub expected: InspectionExpectation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InspectionExpectation {
    pub alarm_action: String,
    pub clear_action: String,
    pub decision_loss_numerator: u64,
    pub inspection_cost_numerator: u64,
    pub total_loss_numerator: u64,
    pub gross_evsi_numerator: i64,
    pub net_evsi_numerator: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeChoice {
    pub outcome: String,
    pub action_id: String,
    pub weighted_loss_numerator: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateResult {
    pub inspection_id: String,
    pub inspection_cost: u64,
    pub choices: Vec<OutcomeChoice>,
    pub decision_loss_numerator: u64,
    pub inspection_cost_numerator: u64,
    pub total_loss_numerator: u64,
    pub gross_evsi_numerator: i64,
    pub net_evsi_numerator: i64,
    pub oracle_total_loss_numerator: u64,
    pub oracle_policy_agreement: bool,
    pub fixture_expectations_match: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Report {
    pub tool_version: String,
    pub schema_version: String,
    pub source_revision: String,
    pub manifest_sha256: String,
    pub claim_ceiling: String,
    pub denominator: u64,
    pub baseline_action: String,
    pub baseline_loss_numerator: u64,
    pub perfect_information_loss_numerator: u64,
    pub maximum_gross_evpi_numerator: u64,
    pub candidates: Vec<CandidateResult>,
    pub overall_status: String,
}

pub fn evaluate(bytes: &[u8], source_revision: &str) -> Result<Report, String> {
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| format!("manifest rejected: {error}"))?;
    validate(&manifest)?;

    let actions: BTreeMap<&str, &LossAction> = manifest.actions.iter()
        .map(|action| (action.id.as_str(), action)).collect();
    let denominator = manifest.prior_serviceable_weight
        .checked_add(manifest.prior_degraded_weight)
        .and_then(|sum| sum.checked_mul(manifest.likelihood_denominator))
        .ok_or_else(|| "score denominator overflow".to_owned())?;

    let baseline = best_unobserved_action(&manifest)?;
    let baseline_loss_numerator = baseline.1;
    let perfect_information_loss_numerator = perfect_information_loss(&manifest)?;
    let maximum_gross_evpi_numerator = baseline_loss_numerator
        .checked_sub(perfect_information_loss_numerator)
        .ok_or_else(|| "perfect-information lower bound exceeds baseline".to_owned())?;

    if manifest.expected_baseline_action != baseline.0.id
        || manifest.expected_baseline_loss_numerator != baseline_loss_numerator
        || manifest.expected_perfect_information_loss_numerator != perfect_information_loss_numerator
        || manifest.expected_max_gross_evpi_numerator != maximum_gross_evpi_numerator
    {
        return Err("frozen baseline or perfect-information oracle disagrees with computed values".into());
    }

    let mut inspections: Vec<&Inspection> = manifest.inspections.iter().collect();
    inspections.sort_by(|a, b| a.id.cmp(&b.id));
    let mut candidates = vec![evaluate_no_inspection(
        &manifest, baseline.0, baseline_loss_numerator, denominator,
    )?];
    for inspection in inspections {
        candidates.push(evaluate_inspection(
            &manifest, inspection, &actions, baseline_loss_numerator, denominator,
        )?);
    }
    candidates.sort_by(|a, b| a.inspection_id.cmp(&b.inspection_id));
    let all_pass = candidates.iter().all(|candidate| {
        candidate.oracle_policy_agreement
            && candidate.fixture_expectations_match
            && candidate.status == "PASS"
    });
    Ok(Report {
        tool_version: TOOL_VERSION.to_owned(),
        schema_version: manifest.schema_version,
        source_revision: source_revision.to_owned(),
        manifest_sha256: sha256_hex(bytes),
        claim_ceiling: manifest.claim_ceiling,
        denominator,
        baseline_action: baseline.0.id.clone(),
        baseline_loss_numerator,
        perfect_information_loss_numerator,
        maximum_gross_evpi_numerator,
        candidates,
        overall_status: if all_pass { "PASS_SYNTHETIC_CONTRACT" } else { "FAIL" }.into(),
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn validate(manifest: &Manifest) -> Result<(), String> {
    if manifest.schema_version != "0.1" || manifest.claim_ceiling != "synthetic_only" {
        return Err("schema_version must be 0.1 and claim_ceiling must be synthetic_only".into());
    }
    if manifest.scope_id.trim().is_empty()
        || manifest.likelihood_denominator == 0
        || manifest.likelihood_denominator > 100_000
        || manifest.prior_serviceable_weight == 0
        || manifest.prior_degraded_weight == 0
        || manifest.prior_serviceable_weight > 100_000
        || manifest.prior_degraded_weight > 100_000
    {
        return Err("scope, denominator, and prior weights must be explicitly bounded and positive".into());
    }
    if manifest.actions.is_empty() || manifest.inspections.is_empty() {
        return Err("at least one action and one inspection profile are required".into());
    }
    let mut action_ids = BTreeSet::new();
    for action in &manifest.actions {
        if action.id.trim().is_empty()
            || !action_ids.insert(action.id.as_str())
            || action.loss_if_serviceable > 100_000
            || action.loss_if_degraded > 100_000
        {
            return Err(format!("invalid action identity or bounded loss: {}", action.id));
        }
    }
    let action_ids: BTreeSet<&str> = manifest.actions.iter().map(|action| action.id.as_str()).collect();
    let mut inspection_ids = BTreeSet::new();
    for inspection in &manifest.inspections {
        if inspection.id.trim().is_empty()
            || !inspection_ids.insert(inspection.id.as_str())
            || inspection.cost > 100_000
            || inspection.serviceable_alarm_weight > manifest.likelihood_denominator
            || inspection.degraded_alarm_weight > manifest.likelihood_denominator
            || !action_ids.contains(inspection.expected.alarm_action.as_str())
            || !action_ids.contains(inspection.expected.clear_action.as_str())
        {
            return Err(format!("invalid or unbounded inspection profile: {}", inspection.id));
        }
    }
    Ok(())
}

fn weighted_loss(action: &LossAction, serviceable_weight: u64, degraded_weight: u64) -> Result<u64, String> {
    serviceable_weight.checked_mul(action.loss_if_serviceable)
        .and_then(|left| degraded_weight.checked_mul(action.loss_if_degraded).and_then(|right| left.checked_add(right)))
        .ok_or_else(|| "weighted decision loss overflow".to_owned())
}

fn best_unobserved_action(manifest: &Manifest) -> Result<(&LossAction, u64), String> {
    let mut scored = manifest.actions.iter()
        .map(|action| {
            weighted_loss(action, manifest.prior_serviceable_weight, manifest.prior_degraded_weight)
                .and_then(|loss| loss.checked_mul(manifest.likelihood_denominator)
                    .map(|scaled| (action, scaled))
                    .ok_or_else(|| "baseline loss scaling overflow".to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    scored.sort_by(|a, b| (a.1, a.0.id.as_str()).cmp(&(b.1, b.0.id.as_str())));
    scored.into_iter().next().ok_or_else(|| "no eligible actions".into())
}

fn perfect_information_loss(manifest: &Manifest) -> Result<u64, String> {
    let serviceable_best = manifest.actions.iter().map(|action| action.loss_if_serviceable).min()
        .ok_or_else(|| "no action available for serviceable state".to_owned())?;
    let degraded_best = manifest.actions.iter().map(|action| action.loss_if_degraded).min()
        .ok_or_else(|| "no action available for degraded state".to_owned())?;
    manifest.prior_serviceable_weight.checked_mul(serviceable_best)
        .and_then(|left| manifest.prior_degraded_weight.checked_mul(degraded_best).and_then(|right| left.checked_add(right)))
        .and_then(|sum| sum.checked_mul(manifest.likelihood_denominator))
        .ok_or_else(|| "perfect-information loss overflow".to_owned())
}

fn weighted_outcome_loss(
    action: &LossAction,
    manifest: &Manifest,
    serviceable_likelihood: u64,
    degraded_likelihood: u64,
) -> Result<u64, String> {
    let serviceable_weight = manifest.prior_serviceable_weight.checked_mul(serviceable_likelihood)
        .ok_or_else(|| "serviceable joint weight overflow".to_owned())?;
    let degraded_weight = manifest.prior_degraded_weight.checked_mul(degraded_likelihood)
        .ok_or_else(|| "degraded joint weight overflow".to_owned())?;
    weighted_loss(action, serviceable_weight, degraded_weight)
}

fn best_action_for_outcome<'a>(
    manifest: &'a Manifest,
    serviceable_likelihood: u64,
    degraded_likelihood: u64,
) -> Result<(&'a LossAction, u64), String> {
    let mut values = manifest.actions.iter()
        .map(|action| weighted_outcome_loss(action, manifest, serviceable_likelihood, degraded_likelihood)
            .map(|loss| (action, loss)))
        .collect::<Result<Vec<_>, _>>()?;
    values.sort_by(|a, b| (a.1, a.0.id.as_str()).cmp(&(b.1, b.0.id.as_str())));
    values.into_iter().next().ok_or_else(|| "no action for observation outcome".into())
}

fn evaluate_no_inspection(
    manifest: &Manifest,
    action: &LossAction,
    baseline_loss: u64,
    denominator: u64,
) -> Result<CandidateResult, String> {
    let scaled_loss = weighted_loss(action, manifest.prior_serviceable_weight, manifest.prior_degraded_weight)?
        .checked_mul(manifest.likelihood_denominator)
        .ok_or_else(|| "no-inspection loss scaling overflow".to_owned())?;
    let oracle = exhaustive_policy(manifest, None, denominator)?;
    let oracle_match = oracle.0.len() == 1
        && oracle.0[0].action_id == action.id
        && oracle.1 == scaled_loss;
    Ok(CandidateResult {
        inspection_id: "no-inspection".into(),
        inspection_cost: 0,
        choices: vec![OutcomeChoice {
            outcome: "no_inspection".into(), action_id: action.id.clone(), weighted_loss_numerator: scaled_loss,
        }],
        decision_loss_numerator: scaled_loss,
        inspection_cost_numerator: 0,
        total_loss_numerator: scaled_loss,
        gross_evsi_numerator: 0,
        net_evsi_numerator: 0,
        oracle_total_loss_numerator: oracle.1,
        oracle_policy_agreement: oracle_match,
        fixture_expectations_match: scaled_loss == baseline_loss,
        status: if oracle_match && scaled_loss == baseline_loss { "PASS" } else { "FAIL" }.into(),
    })
}

fn evaluate_inspection(
    manifest: &Manifest,
    inspection: &Inspection,
    actions: &BTreeMap<&str, &LossAction>,
    baseline_loss: u64,
    denominator: u64,
) -> Result<CandidateResult, String> {
    let clear_serviceable = manifest.likelihood_denominator - inspection.serviceable_alarm_weight;
    let clear_degraded = manifest.likelihood_denominator - inspection.degraded_alarm_weight;
    let (alarm_action, alarm_loss) = best_action_for_outcome(
        manifest, inspection.serviceable_alarm_weight, inspection.degraded_alarm_weight,
    )?;
    let (clear_action, clear_loss) = best_action_for_outcome(
        manifest, clear_serviceable, clear_degraded,
    )?;
    let decision_loss = alarm_loss.checked_add(clear_loss)
        .ok_or_else(|| "decision loss numerator overflow".to_owned())?;
    let inspection_cost = inspection.cost.checked_mul(denominator)
        .ok_or_else(|| "inspection cost numerator overflow".to_owned())?;
    let total_loss = decision_loss.checked_add(inspection_cost)
        .ok_or_else(|| "total loss numerator overflow".to_owned())?;
    let gross_evsi = i64::try_from(baseline_loss)
        .and_then(|baseline| i64::try_from(decision_loss).map(|decision| baseline - decision))
        .map_err(|_| "gross EVSI exceeds signed reporting range".to_owned())?;
    let net_evsi = i64::try_from(baseline_loss)
        .and_then(|baseline| i64::try_from(total_loss).map(|total| baseline - total))
        .map_err(|_| "net EVSI exceeds signed reporting range".to_owned())?;
    let choices = vec![
        OutcomeChoice { outcome: "alarm".into(), action_id: alarm_action.id.clone(), weighted_loss_numerator: alarm_loss },
        OutcomeChoice { outcome: "clear".into(), action_id: clear_action.id.clone(), weighted_loss_numerator: clear_loss },
    ];
    let oracle = exhaustive_policy(manifest, Some(inspection), denominator)?;
    let oracle_match = oracle.1 == total_loss
        && oracle.0.len() == 2
        && oracle.0[0].action_id == alarm_action.id
        && oracle.0[1].action_id == clear_action.id;
    let expected = &inspection.expected;
    let fixture_match = expected.alarm_action == alarm_action.id
        && expected.clear_action == clear_action.id
        && expected.decision_loss_numerator == decision_loss
        && expected.inspection_cost_numerator == inspection_cost
        && expected.total_loss_numerator == total_loss
        && expected.gross_evsi_numerator == gross_evsi
        && expected.net_evsi_numerator == net_evsi;
    Ok(CandidateResult {
        inspection_id: inspection.id.clone(),
        inspection_cost: inspection.cost,
        choices,
        decision_loss_numerator: decision_loss,
        inspection_cost_numerator: inspection_cost,
        total_loss_numerator: total_loss,
        gross_evsi_numerator: gross_evsi,
        net_evsi_numerator: net_evsi,
        oracle_total_loss_numerator: oracle.1,
        oracle_policy_agreement: oracle_match,
        fixture_expectations_match: fixture_match,
        status: if oracle_match && fixture_match && total_loss >= perfect_information_loss(manifest)? { "PASS" } else { "FAIL" }.into(),
    })
}

fn exhaustive_policy(
    manifest: &Manifest,
    inspection: Option<&Inspection>,
    denominator: u64,
) -> Result<(Vec<OutcomeChoice>, u64), String> {
    let mut candidates = Vec::new();
    if let Some(test) = inspection {
        let h_alarm = test.serviceable_alarm_weight;
        let d_alarm = test.degraded_alarm_weight;
        let h_clear = manifest.likelihood_denominator - h_alarm;
        let d_clear = manifest.likelihood_denominator - d_alarm;
        let cost = test.cost.checked_mul(denominator).ok_or_else(|| "oracle inspection cost overflow".to_owned())?;
        for alarm in &manifest.actions {
            let alarm_loss = weighted_outcome_loss(alarm, manifest, h_alarm, d_alarm)?;
            for clear in &manifest.actions {
                let clear_loss = weighted_outcome_loss(clear, manifest, h_clear, d_clear)?;
                let decision_loss = alarm_loss.checked_add(clear_loss).ok_or_else(|| "oracle loss overflow".to_owned())?;
                let total = decision_loss.checked_add(cost).ok_or_else(|| "oracle total overflow".to_owned())?;
                candidates.push((total, alarm.id.as_str(), clear.id.as_str(), alarm_loss, clear_loss));
            }
        }
    } else {
        for action in &manifest.actions {
            let loss = weighted_loss(action, manifest.prior_serviceable_weight, manifest.prior_degraded_weight)?
                .checked_mul(manifest.likelihood_denominator)
                .ok_or_else(|| "oracle baseline scaling overflow".to_owned())?;
            candidates.push((loss, action.id.as_str(), "", loss, 0));
        }
    }
    candidates.sort_by(|a, b| (a.0, a.1, a.2).cmp(&(b.0, b.1, b.2)));
    let best = candidates.into_iter().next().ok_or_else(|| "empty exhaustive policy set".to_owned())?;
    let choices = if inspection.is_some() {
        vec![
            OutcomeChoice { outcome: "alarm".into(), action_id: best.1.to_owned(), weighted_loss_numerator: best.3 },
            OutcomeChoice { outcome: "clear".into(), action_id: best.2.to_owned(), weighted_loss_numerator: best.4 },
        ]
    } else {
        vec![OutcomeChoice { outcome: "no_inspection".into(), action_id: best.1.to_owned(), weighted_loss_numerator: best.3 }]
    };
    Ok((choices, best.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../fixtures/inspection-value-v0-1.json");
    fn manifest() -> Manifest { serde_json::from_str(FIXTURE).expect("fixture parses") }
    fn report() -> Report { evaluate(FIXTURE.as_bytes(), "test-revision").expect("fixture evaluates") }
    fn candidate<'a>(report: &'a Report, id: &str) -> &'a CandidateResult {
        report.candidates.iter().find(|item| item.inspection_id == id).unwrap()
    }

    #[test]
    fn frozen_scores_match_exhaustive_policy_enumeration() {
        let report = report();
        assert_eq!(report.overall_status, "PASS_SYNTHETIC_CONTRACT");
        assert_eq!(report.baseline_action, "repair");
        for candidate in &report.candidates {
            assert!(candidate.oracle_policy_agreement, "{}", candidate.inspection_id);
            assert!(candidate.fixture_expectations_match, "{}", candidate.inspection_id);
            assert_eq!(candidate.status, "PASS", "{}", candidate.inspection_id);
            assert_eq!(candidate.total_loss_numerator, candidate.oracle_total_loss_numerator);
        }
    }

    #[test]
    fn low_cost_test_can_outperform_a_more_accurate_but_expensive_test() {
        let report = report();
        let low_cost = candidate(&report, "low-cost-visual-check");
        let high_accuracy = candidate(&report, "high-accuracy-inspection");
        assert!(low_cost.net_evsi_numerator > high_accuracy.net_evsi_numerator);
        assert_eq!(low_cost.net_evsi_numerator, 53_000);
        assert_eq!(high_accuracy.net_evsi_numerator, 30_350);
    }

    #[test]
    fn uninformative_inspection_can_have_negative_net_value() {
        let report = report();
        let result = candidate(&report, "uninformative-check");
        assert_eq!(result.gross_evsi_numerator, 0);
        assert_eq!(result.net_evsi_numerator, -5_000);
        assert!(result.total_loss_numerator > report.baseline_loss_numerator);
    }

    #[test]
    fn perfect_information_is_a_lower_bound_on_decision_loss() {
        let report = report();
        assert_eq!(report.perfect_information_loss_numerator, 175_000);
        for candidate in &report.candidates {
            assert!(candidate.decision_loss_numerator >= report.perfect_information_loss_numerator);
        }
        assert_eq!(report.maximum_gross_evpi_numerator, 140_000);
        for test in report.candidates.iter().filter(|item| item.inspection_id != "no-inspection") {
            assert!(test.gross_evsi_numerator as u64 <= report.maximum_gross_evpi_numerator);
        }
    }

    #[test]
    fn no_inspection_selects_repair_from_the_declared_prior() {
        let report = report();
        let baseline = candidate(&report, "no-inspection");
        assert_eq!(baseline.choices[0].action_id, "repair");
        assert_eq!(baseline.total_loss_numerator, 315_000);
        assert_eq!(baseline.net_evsi_numerator, 0);
    }

    #[test]
    fn changing_likelihoods_changes_the_frozen_expected_result() {
        let mut manifest = manifest();
        manifest.inspections.iter_mut().find(|test| test.id == "low-cost-visual-check").unwrap()
            .degraded_alarm_weight = 40;
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let result = evaluate(&bytes, "test-revision").unwrap();
        assert_eq!(candidate(&result, "low-cost-visual-check").fixture_expectations_match, false);
        assert_eq!(candidate(&result, "low-cost-visual-check").oracle_policy_agreement, true);
        assert_eq!(result.overall_status, "FAIL");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let bad = FIXTURE.replace(
            r#""claim_ceiling": "synthetic_only""#,
            r#""claim_ceiling": "synthetic_only", "unreviewed_prior": 99"#,
        );
        assert!(evaluate(bad.as_bytes(), "test").is_err());
    }

    #[test]
    fn probabilities_are_bounded_by_the_declared_denominator() {
        let mut manifest = manifest();
        manifest.inspections[0].serviceable_alarm_weight = manifest.likelihood_denominator + 1;
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert!(evaluate(&bytes, "test").is_err());
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}
