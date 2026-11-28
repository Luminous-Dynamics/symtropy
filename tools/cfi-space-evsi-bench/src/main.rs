#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::{env, fs, path::Path, process};

const PROBABILITY_DENOMINATOR: u128 = 10_000;
const JOINT_DENOMINATOR: u128 = PROBABILITY_DENOMINATOR * PROBABILITY_DENOMINATOR;
const MAX_HYPOTHESES: usize = 64;
const MAX_DECISIONS: usize = 32;
const MAX_PROPOSALS: usize = 128;
const MAX_OUTCOMES: usize = 64;
const MAX_LOSS_UNITS: u32 = 1_000_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    schema_version: u32,
    scenario_id: String,
    mission: String,
    probability_denominator: u32,
    hypotheses: Vec<Hypothesis>,
    prior_probability_bps: Vec<u32>,
    decisions: Vec<Decision>,
    decision_loss_units: Vec<Vec<u32>>,
    proposals: Vec<ObservationProposal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hypothesis {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decision {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationProposal {
    id: String,
    cost_units: u32,
    estimated_latency_us: u64,
    max_latency_us: u64,
    capability_available: bool,
    preconditions_met: bool,
    dependencies_known: bool,
    advisory_only: bool,
    actuation_authority: bool,
    outcomes: Vec<ObservationOutcome>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationOutcome {
    id: String,
    likelihood_bps: Vec<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ProposalDisposition {
    AdmissiblePositiveEvsi,
    AdmissibleNonPositiveEvsi,
    Rejected,
}

#[derive(Debug, Serialize)]
struct ProposalEvaluation {
    proposal_id: String,
    disposition: ProposalDisposition,
    rejection_reason: Option<String>,
    prior_best_decision_id: Option<String>,
    gross_evsi_numerator: Option<u128>,
    cost_numerator: Option<u128>,
    net_evsi_numerator: Option<i128>,
    denominator: u128,
    entropy_gain_millibits: Option<i64>,
}

#[derive(Debug, Serialize)]
struct EvaluationReport {
    schema_version: u32,
    scenario_id: String,
    mission: String,
    prior_best_decision_id: String,
    prior_expected_loss_numerator: u128,
    prior_loss_denominator: u128,
    evsi_recommendation_id: Option<String>,
    information_gain_recommendation_id: Option<String>,
    evaluations: Vec<ProposalEvaluation>,
    observer_only: bool,
    hardware_action_issued: bool,
    actuation_authority: bool,
}

fn require_nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn validate_scenario(scenario: &Scenario) -> Result<(), String> {
    if scenario.schema_version != 1 {
        return Err(format!(
            "unsupported EVSI scenario schema_version {}",
            scenario.schema_version
        ));
    }
    require_nonempty(&scenario.scenario_id, "scenario_id")?;
    require_nonempty(&scenario.mission, "mission")?;
    if scenario.probability_denominator != PROBABILITY_DENOMINATOR as u32 {
        return Err(format!(
            "probability_denominator must be {PROBABILITY_DENOMINATOR}"
        ));
    }
    if scenario.hypotheses.len() < 2 || scenario.hypotheses.len() > MAX_HYPOTHESES {
        return Err(format!(
            "hypothesis count must be in 2..={MAX_HYPOTHESES}"
        ));
    }
    if scenario.decisions.len() < 2 || scenario.decisions.len() > MAX_DECISIONS {
        return Err(format!("decision count must be in 2..={MAX_DECISIONS}"));
    }
    if scenario.proposals.is_empty() || scenario.proposals.len() > MAX_PROPOSALS {
        return Err(format!("proposal count must be in 1..={MAX_PROPOSALS}"));
    }
    if scenario.prior_probability_bps.len() != scenario.hypotheses.len() {
        return Err("prior probability vector length must match hypothesis count".to_string());
    }
    let prior_sum: u128 = scenario
        .prior_probability_bps
        .iter()
        .map(|value| u128::from(*value))
        .sum();
    if prior_sum != PROBABILITY_DENOMINATOR {
        return Err(format!(
            "prior probabilities must sum to {PROBABILITY_DENOMINATOR}, got {prior_sum}"
        ));
    }

    let mut hypothesis_ids = BTreeSet::new();
    for hypothesis in &scenario.hypotheses {
        require_nonempty(&hypothesis.id, "hypothesis.id")?;
        if !hypothesis_ids.insert(&hypothesis.id) {
            return Err(format!("duplicate hypothesis id {}", hypothesis.id));
        }
    }

    let mut decision_ids = BTreeSet::new();
    for decision in &scenario.decisions {
        require_nonempty(&decision.id, "decision.id")?;
        if !decision_ids.insert(&decision.id) {
            return Err(format!("duplicate decision id {}", decision.id));
        }
    }

    if scenario.decision_loss_units.len() != scenario.hypotheses.len() {
        return Err("decision loss matrix row count must match hypothesis count".to_string());
    }
    for (row_index, row) in scenario.decision_loss_units.iter().enumerate() {
        if row.len() != scenario.decisions.len() {
            return Err(format!(
                "decision loss row {row_index} must contain {} decisions",
                scenario.decisions.len()
            ));
        }
        if row.iter().any(|loss| *loss > MAX_LOSS_UNITS) {
            return Err(format!(
                "decision loss row {row_index} exceeds {MAX_LOSS_UNITS}"
            ));
        }
    }

    let mut proposal_ids = BTreeSet::new();
    for proposal in &scenario.proposals {
        require_nonempty(&proposal.id, "proposal.id")?;
        if !proposal_ids.insert(&proposal.id) {
            return Err(format!("duplicate proposal id {}", proposal.id));
        }
        if proposal.cost_units > MAX_LOSS_UNITS {
            return Err(format!(
                "proposal {} cost exceeds {MAX_LOSS_UNITS}",
                proposal.id
            ));
        }
        if proposal.max_latency_us == 0 || proposal.estimated_latency_us == 0 {
            return Err(format!(
                "proposal {} latency bounds must be non-zero",
                proposal.id
            ));
        }
        if proposal.outcomes.is_empty() || proposal.outcomes.len() > MAX_OUTCOMES {
            return Err(format!(
                "proposal {} outcome count must be in 1..={MAX_OUTCOMES}",
                proposal.id
            ));
        }
        let mut outcome_ids = BTreeSet::new();
        let mut likelihood_sums = vec![0_u128; scenario.hypotheses.len()];
        for outcome in &proposal.outcomes {
            require_nonempty(&outcome.id, "outcome.id")?;
            if !outcome_ids.insert(&outcome.id) {
                return Err(format!(
                    "proposal {} contains duplicate outcome id {}",
                    proposal.id, outcome.id
                ));
            }
            if outcome.likelihood_bps.len() != scenario.hypotheses.len() {
                return Err(format!(
                    "proposal {} outcome {} likelihood vector length mismatch",
                    proposal.id, outcome.id
                ));
            }
            for (index, likelihood) in outcome.likelihood_bps.iter().enumerate() {
                if *likelihood > PROBABILITY_DENOMINATOR as u32 {
                    return Err(format!(
                        "proposal {} outcome {} likelihood exceeds denominator",
                        proposal.id, outcome.id
                    ));
                }
                likelihood_sums[index] += u128::from(*likelihood);
            }
        }
        for (index, sum) in likelihood_sums.iter().enumerate() {
            if *sum != PROBABILITY_DENOMINATOR {
                return Err(format!(
                    "proposal {} likelihoods for hypothesis {} must sum to {PROBABILITY_DENOMINATOR}, got {sum}",
                    proposal.id, scenario.hypotheses[index].id
                ));
            }
        }
    }
    Ok(())
}

fn best_decision_numerator(scenario: &Scenario, hypothesis_weights: &[u128]) -> (usize, u128) {
    let mut best_index = 0;
    let mut best_numerator = u128::MAX;
    for decision_index in 0..scenario.decisions.len() {
        let numerator = hypothesis_weights
            .iter()
            .zip(&scenario.decision_loss_units)
            .map(|(weight, row)| weight * u128::from(row[decision_index]))
            .sum::<u128>();
        if numerator < best_numerator {
            best_index = decision_index;
            best_numerator = numerator;
        }
    }
    (best_index, best_numerator)
}

fn binary_entropy_bits(probabilities: &[f64]) -> f64 {
    probabilities
        .iter()
        .filter(|probability| **probability > 0.0)
        .map(|probability| -probability * probability.log2())
        .sum()
}

/// Shannon information gain is a baseline diagnostic only. EVSI selection uses
/// exact integer decision-loss arithmetic and never this floating-point value.
fn entropy_gain_millibits(scenario: &Scenario, proposal: &ObservationProposal) -> i64 {
    let priors: Vec<f64> = scenario
        .prior_probability_bps
        .iter()
        .map(|value| f64::from(*value) / PROBABILITY_DENOMINATOR as f64)
        .collect();
    let prior_entropy = binary_entropy_bits(&priors);
    let mut conditional_entropy = 0.0;
    for outcome in &proposal.outcomes {
        let joint_weights: Vec<u128> = scenario
            .prior_probability_bps
            .iter()
            .zip(&outcome.likelihood_bps)
            .map(|(prior, likelihood)| u128::from(*prior) * u128::from(*likelihood))
            .collect();
        let outcome_weight: u128 = joint_weights.iter().sum();
        if outcome_weight == 0 {
            continue;
        }
        let outcome_probability = outcome_weight as f64 / JOINT_DENOMINATOR as f64;
        let posterior: Vec<f64> = joint_weights
            .iter()
            .map(|weight| *weight as f64 / outcome_weight as f64)
            .collect();
        conditional_entropy += outcome_probability * binary_entropy_bits(&posterior);
    }
    ((prior_entropy - conditional_entropy).max(0.0) * 1_000.0).round() as i64
}

fn reject_reason(proposal: &ObservationProposal) -> Option<String> {
    if !proposal.advisory_only {
        return Some("proposal must be advisory-only".to_string());
    }
    if proposal.actuation_authority {
        return Some("proposal must not carry actuation authority".to_string());
    }
    if !proposal.capability_available {
        return Some("required sensing capability is unavailable".to_string());
    }
    if !proposal.preconditions_met {
        return Some("proposal preconditions are not satisfied".to_string());
    }
    if !proposal.dependencies_known {
        return Some("proposal observation-model dependencies are unknown".to_string());
    }
    if proposal.estimated_latency_us > proposal.max_latency_us {
        return Some("proposal exceeds its declared latency budget".to_string());
    }
    None
}

fn evaluate_proposal(
    scenario: &Scenario,
    proposal: &ObservationProposal,
    prior_best_decision_index: usize,
    prior_best_numerator: u128,
) -> ProposalEvaluation {
    if let Some(reason) = reject_reason(proposal) {
        return ProposalEvaluation {
            proposal_id: proposal.id.clone(),
            disposition: ProposalDisposition::Rejected,
            rejection_reason: Some(reason),
            prior_best_decision_id: Some(scenario.decisions[prior_best_decision_index].id.clone()),
            gross_evsi_numerator: None,
            cost_numerator: None,
            net_evsi_numerator: None,
            denominator: JOINT_DENOMINATOR,
            entropy_gain_millibits: None,
        };
    }

    let mut expected_posterior_loss_numerator = 0_u128;
    for outcome in &proposal.outcomes {
        let joint_weights: Vec<u128> = scenario
            .prior_probability_bps
            .iter()
            .zip(&outcome.likelihood_bps)
            .map(|(prior, likelihood)| u128::from(*prior) * u128::from(*likelihood))
            .collect();
        let outcome_weight: u128 = joint_weights.iter().sum();
        if outcome_weight == 0 {
            continue;
        }
        let (_, best_outcome_loss_numerator) = best_decision_numerator(scenario, &joint_weights);
        expected_posterior_loss_numerator += best_outcome_loss_numerator;
    }

    let prior_scaled = prior_best_numerator * PROBABILITY_DENOMINATOR;
    let gross_evsi_numerator = prior_scaled.saturating_sub(expected_posterior_loss_numerator);
    let cost_numerator = u128::from(proposal.cost_units) * JOINT_DENOMINATOR;
    let net_evsi_numerator = i128::try_from(gross_evsi_numerator).unwrap_or(i128::MAX)
        - i128::try_from(cost_numerator).unwrap_or(i128::MAX);
    let disposition = if net_evsi_numerator > 0 {
        ProposalDisposition::AdmissiblePositiveEvsi
    } else {
        ProposalDisposition::AdmissibleNonPositiveEvsi
    };

    ProposalEvaluation {
        proposal_id: proposal.id.clone(),
        disposition,
        rejection_reason: None,
        prior_best_decision_id: Some(scenario.decisions[prior_best_decision_index].id.clone()),
        gross_evsi_numerator: Some(gross_evsi_numerator),
        cost_numerator: Some(cost_numerator),
        net_evsi_numerator: Some(net_evsi_numerator),
        denominator: JOINT_DENOMINATOR,
        entropy_gain_millibits: Some(entropy_gain_millibits(scenario, proposal)),
    }
}

fn evaluate_scenario(scenario: &Scenario) -> Result<EvaluationReport, String> {
    validate_scenario(scenario)?;
    let priors: Vec<u128> = scenario
        .prior_probability_bps
        .iter()
        .map(|value| u128::from(*value))
        .collect();
    let (prior_best_decision_index, prior_best_numerator) =
        best_decision_numerator(scenario, &priors);

    let evaluations: Vec<ProposalEvaluation> = scenario
        .proposals
        .iter()
        .map(|proposal| {
            evaluate_proposal(
                scenario,
                proposal,
                prior_best_decision_index,
                prior_best_numerator,
            )
        })
        .collect();

    let evsi_recommendation_id = evaluations
        .iter()
        .filter(|evaluation| evaluation.disposition == ProposalDisposition::AdmissiblePositiveEvsi)
        .max_by_key(|evaluation| evaluation.net_evsi_numerator.unwrap_or(i128::MIN))
        .map(|evaluation| evaluation.proposal_id.clone());

    let information_gain_recommendation_id = evaluations
        .iter()
        .filter(|evaluation| evaluation.disposition != ProposalDisposition::Rejected)
        .max_by_key(|evaluation| evaluation.entropy_gain_millibits.unwrap_or(i64::MIN))
        .map(|evaluation| evaluation.proposal_id.clone());

    Ok(EvaluationReport {
        schema_version: 1,
        scenario_id: scenario.scenario_id.clone(),
        mission: scenario.mission.clone(),
        prior_best_decision_id: scenario.decisions[prior_best_decision_index].id.clone(),
        prior_expected_loss_numerator: prior_best_numerator,
        prior_loss_denominator: PROBABILITY_DENOMINATOR,
        evsi_recommendation_id,
        information_gain_recommendation_id,
        evaluations,
        observer_only: true,
        hardware_action_issued: false,
        actuation_authority: false,
    })
}

fn read_scenario(path: &Path) -> Result<Scenario, String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot stat scenario {}: {error}", path.display()))?;
    if metadata.len() > 4 * 1024 * 1024 {
        return Err("EVSI scenario exceeds 4 MiB".to_string());
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read scenario {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid EVSI scenario JSON: {error}"))
}

fn run() -> Result<(), String> {
    let mut scenario_path = None;
    let mut expected_evsi = None;
    let mut expected_information_gain = None;
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            println!(
                "cfi-space-evsi-bench --scenario FILE [--expect-evsi ID] [--expect-information-gain ID]"
            );
            return Ok(());
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for argument {flag}"))?;
        match flag.as_str() {
            "--scenario" => scenario_path = Some(value),
            "--expect-evsi" => expected_evsi = Some(value),
            "--expect-information-gain" => expected_information_gain = Some(value),
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    let scenario_path = scenario_path.ok_or_else(|| "missing --scenario".to_string())?;
    let scenario = read_scenario(Path::new(&scenario_path))?;
    let report = evaluate_scenario(&scenario)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize EVSI report: {error}"))?
    );
    if let Some(expected) = expected_evsi {
        if report.evsi_recommendation_id.as_deref() != Some(expected.as_str()) {
            return Err(format!(
                "EVSI recommendation mismatch: expected {expected}, got {:?}",
                report.evsi_recommendation_id
            ));
        }
    }
    if let Some(expected) = expected_information_gain {
        if report.information_gain_recommendation_id.as_deref() != Some(expected.as_str()) {
            return Err(format!(
                "information-gain recommendation mismatch: expected {expected}, got {:?}",
                report.information_gain_recommendation_id
            ));
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("cfi-space-evsi-bench: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIO: &str = include_str!("fixtures/synthetic-evsi-scenario-v0.1.json");

    fn scenario() -> Scenario {
        serde_json::from_str(SCENARIO).unwrap()
    }

    #[test]
    fn evsi_prefers_decision_value_over_higher_entropy_gain() {
        let report = evaluate_scenario(&scenario()).unwrap();
        assert_eq!(report.prior_best_decision_id, "reobserve");
        assert_eq!(
            report.information_gain_recommendation_id.as_deref(),
            Some("illumination-survey")
        );
        assert_eq!(
            report.evsi_recommendation_id.as_deref(),
            Some("independent-reference-check")
        );
        let illumination = report
            .evaluations
            .iter()
            .find(|evaluation| evaluation.proposal_id == "illumination-survey")
            .unwrap();
        let independent = report
            .evaluations
            .iter()
            .find(|evaluation| evaluation.proposal_id == "independent-reference-check")
            .unwrap();
        assert!(illumination.entropy_gain_millibits > independent.entropy_gain_millibits);
        assert_eq!(illumination.gross_evsi_numerator, Some(0));
        assert!(illumination.net_evsi_numerator.unwrap() < 0);
        assert_eq!(independent.net_evsi_numerator, Some(4_130_000_000));
    }

    #[test]
    fn unavailable_or_unauthorized_proposals_are_rejected() {
        let mut scenario = scenario();
        scenario.proposals[2].capability_available = false;
        let report = evaluate_scenario(&scenario).unwrap();
        let rejected = report
            .evaluations
            .iter()
            .find(|evaluation| evaluation.proposal_id == "unavailable-lidar-revisit")
            .unwrap();
        assert_eq!(rejected.disposition, ProposalDisposition::Rejected);
        assert!(report.evsi_recommendation_id.is_some());

        scenario.proposals[2].capability_available = true;
        scenario.proposals[2].actuation_authority = true;
        let report = evaluate_scenario(&scenario).unwrap();
        let rejected = report
            .evaluations
            .iter()
            .find(|evaluation| evaluation.proposal_id == "unavailable-lidar-revisit")
            .unwrap();
        assert_eq!(rejected.disposition, ProposalDisposition::Rejected);
        assert!(
            rejected
                .rejection_reason
                .as_deref()
                .unwrap()
                .contains("actuation")
        );
    }

    #[test]
    fn unknown_dependencies_and_latency_overrun_are_rejected() {
        let mut scenario = scenario();
        scenario.proposals[1].dependencies_known = false;
        let report = evaluate_scenario(&scenario).unwrap();
        let rejected = report
            .evaluations
            .iter()
            .find(|evaluation| evaluation.proposal_id == "independent-reference-check")
            .unwrap();
        assert_eq!(rejected.disposition, ProposalDisposition::Rejected);
        assert!(
            rejected
                .rejection_reason
                .as_deref()
                .unwrap()
                .contains("dependencies")
        );

        let mut scenario = scenario();
        scenario.proposals[1].estimated_latency_us = 20_000;
        let report = evaluate_scenario(&scenario).unwrap();
        let rejected = report
            .evaluations
            .iter()
            .find(|evaluation| evaluation.proposal_id == "independent-reference-check")
            .unwrap();
        assert_eq!(rejected.disposition, ProposalDisposition::Rejected);
    }

    #[test]
    fn no_action_is_returned_when_all_net_evsi_values_are_non_positive() {
        let mut scenario = scenario();
        scenario.proposals[0].cost_units = 100;
        scenario.proposals[1].cost_units = 100;
        let report = evaluate_scenario(&scenario).unwrap();
        assert_eq!(report.evsi_recommendation_id, None);
    }

    #[test]
    fn incomplete_likelihood_columns_are_rejected() {
        let mut scenario = scenario();
        scenario.proposals[0].outcomes[0].likelihood_bps[2] = 4_999;
        assert!(evaluate_scenario(&scenario).is_err());
    }

    #[test]
    fn missing_advisory_only_flag_or_preconditions_are_rejected() {
        let mut scenario = scenario();
        scenario.proposals[1].advisory_only = false;
        let report = evaluate_scenario(&scenario).unwrap();
        assert_eq!(
            report.evaluations[1].disposition,
            ProposalDisposition::Rejected
        );

        let mut scenario = scenario();
        scenario.proposals[1].preconditions_met = false;
        let report = evaluate_scenario(&scenario).unwrap();
        assert_eq!(
            report.evaluations[1].disposition,
            ProposalDisposition::Rejected
        );
    }
}
