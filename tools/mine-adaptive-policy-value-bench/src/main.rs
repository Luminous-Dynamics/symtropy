use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const EXPECTED_SCENARIOS: usize = 4;
const PROBABILITY_BPS: u32 = 10_000;
type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Policy {
    Static,
    TwoStage,
    RollingHorizon,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Route {
    LotA,
    LotB,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Action {
    RequestAssayA,
    RequestAssayB,
    ChooseA,
    ChooseB,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    evidence_class: String,
    claim_ceiling: String,
    probability_basis_points: u32,
    utility_model: UtilityModel,
    scenarios: Vec<Scenario>,
    expected_aggregate: ExpectedAggregate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UtilityModel {
    a_high_grade_utility_units: i64,
    a_low_grade_utility_units: i64,
    b_high_grade_utility_units: i64,
    b_low_grade_utility_units: i64,
    assay_cost_units: i64,
    assay_delivery_delay_epochs: u32,
    prior_knowledge: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    probability_basis_points: u32,
    a_high_grade: bool,
    b_high_grade: bool,
    expected: ExpectedScores,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedScores {
    static_net_units: i64,
    two_stage_net_units: i64,
    rolling_horizon_net_units: i64,
    clairvoyant_upper_bound_units: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedAggregate {
    static_weighted_mean_net_units: i64,
    two_stage_weighted_mean_net_units: i64,
    rolling_horizon_weighted_mean_net_units: i64,
    clairvoyant_weighted_mean_upper_bound_units: i64,
    static_mean_regret_units: i64,
    two_stage_mean_regret_units: i64,
    rolling_horizon_mean_regret_units: i64,
    trace_violations: usize,
    nonanticipativity_violations: usize,
    rolling_regret_reduction_vs_two_stage_basis_points: u32,
    rolling_regret_reduction_vs_static_basis_points: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Observation {
    event_id: String,
    lot_id: String,
    observed_at_epoch: u32,
    delivered_at_epoch: u32,
    high_grade: bool,
    source_record_present: bool,
    qc_pass: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DecisionTrace {
    decision_id: String,
    epoch: u32,
    action: Action,
    visible_event_ids: Vec<String>,
    used_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PolicyOutcome {
    policy: Policy,
    selected_route: Route,
    gross_utility_units: i64,
    assay_count: u32,
    assay_cost_units: i64,
    net_utility_units: i64,
    clairvoyant_regret_units: i64,
    trace_valid: bool,
    decisions: Vec<DecisionTrace>,
    delivered_observations: Vec<Observation>,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioReceipt {
    scenario_id: String,
    probability_basis_points: u32,
    static_policy: PolicyOutcome,
    two_stage: PolicyOutcome,
    rolling_horizon: PolicyOutcome,
    clairvoyant_upper_bound_units: i64,
    expected_control_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PolicyAggregate {
    policy: Policy,
    weighted_mean_net_units: i64,
    weighted_mean_regret_units: i64,
    weighted_mean_assay_count_milli: i64,
}

#[derive(Debug, Clone, Serialize)]
struct Receipt {
    result: String,
    suite_id: String,
    suite_version: String,
    evaluator_version: String,
    source_commit: String,
    corpus_sha256: String,
    scenario_count: usize,
    expected_controls_matched: usize,
    trace_violations: usize,
    nonanticipativity_violations: usize,
    two_stage_regret_reduction_vs_static_basis_points: u32,
    rolling_regret_reduction_vs_two_stage_basis_points: u32,
    rolling_regret_reduction_vs_static_basis_points: u32,
    policy_aggregates: Vec<PolicyAggregate>,
    scenarios: Vec<ScenarioReceipt>,
    claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct PolicyStep {
    action: Action,
    used_event_ids: Vec<String>,
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
    if corpus.suite_id != "MINE-ADAPTIVE-POLICY-VALUE-001"
        || corpus.suite_version != VERSION
        || corpus.evidence_class != "public_authored_synthetic_policy_tournament"
        || !nonempty(&corpus.claim_ceiling)
        || corpus.probability_basis_points != PROBABILITY_BPS
        || corpus.scenarios.len() != EXPECTED_SCENARIOS
        || corpus.utility_model.assay_cost_units < 0
        || corpus.utility_model.assay_delivery_delay_epochs != 1
        || !nonempty(&corpus.utility_model.prior_knowledge)
    {
        return Err("invalid suite identity, evidence class, probabilities, model, or frozen scenario count".to_owned());
    }
    for utility in [
        corpus.utility_model.a_high_grade_utility_units,
        corpus.utility_model.a_low_grade_utility_units,
        corpus.utility_model.b_high_grade_utility_units,
        corpus.utility_model.b_low_grade_utility_units,
    ] {
        if utility < 0 {
            return Err(
                "synthetic utility table must be non-negative in this v0.1 profile".to_owned(),
            );
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut total_probability = 0_u32;
    for scenario in &corpus.scenarios {
        if !nonempty(&scenario.scenario_id) || !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err("scenario IDs must be non-empty and unique".to_owned());
        }
        total_probability = total_probability
            .checked_add(scenario.probability_basis_points)
            .ok_or_else(|| "probability basis-point sum overflows".to_owned())?;
    }
    if total_probability != PROBABILITY_BPS
        || corpus
            .scenarios
            .iter()
            .any(|scenario| scenario.probability_basis_points == 0)
    {
        return Err(
            "scenario probabilities must be positive and sum to 10,000 basis points".to_owned(),
        );
    }
    Ok(())
}

fn prior_route(corpus: &Corpus) -> Check<Route> {
    let mut a_sum = 0_i128;
    let mut b_sum = 0_i128;
    for scenario in &corpus.scenarios {
        let a = if scenario.a_high_grade {
            corpus.utility_model.a_high_grade_utility_units
        } else {
            corpus.utility_model.a_low_grade_utility_units
        };
        let b = if scenario.b_high_grade {
            corpus.utility_model.b_high_grade_utility_units
        } else {
            corpus.utility_model.b_low_grade_utility_units
        };
        a_sum += i128::from(scenario.probability_basis_points) * i128::from(a);
        b_sum += i128::from(scenario.probability_basis_points) * i128::from(b);
    }
    Ok(if a_sum >= b_sum {
        Route::LotA
    } else {
        Route::LotB
    })
}

fn observation_by_id<'a>(visible: &'a [Observation], id: &str) -> Check<&'a Observation> {
    visible
        .iter()
        .find(|observation| observation.event_id == id)
        .ok_or_else(|| format!("policy requested unavailable observation {id}"))
}

fn policy_step(
    policy: Policy,
    epoch: u32,
    visible: &[Observation],
    prior_route: Route,
) -> Check<PolicyStep> {
    let step = match (policy, epoch) {
        (Policy::Static, 0) => PolicyStep {
            action: match prior_route {
                Route::LotA => Action::ChooseA,
                Route::LotB => Action::ChooseB,
            },
            used_event_ids: Vec::new(),
        },
        (Policy::Static, _) => {
            return Err("static policy should have terminated at epoch 0".to_owned())
        }
        (Policy::TwoStage, 0) | (Policy::RollingHorizon, 0) => PolicyStep {
            action: Action::RequestAssayA,
            used_event_ids: Vec::new(),
        },
        (Policy::TwoStage, 1) => {
            let a = observation_by_id(visible, "ASSAY_A")?;
            PolicyStep {
                action: if a.high_grade {
                    Action::ChooseA
                } else {
                    Action::ChooseB
                },
                used_event_ids: vec!["ASSAY_A".to_owned()],
            }
        }
        (Policy::TwoStage, _) => {
            return Err("two-stage policy must have chosen a route by epoch 1".to_owned());
        }
        (Policy::RollingHorizon, 1) => {
            let a = observation_by_id(visible, "ASSAY_A")?;
            PolicyStep {
                action: if a.high_grade {
                    Action::ChooseA
                } else {
                    Action::RequestAssayB
                },
                used_event_ids: vec!["ASSAY_A".to_owned()],
            }
        }
        (Policy::RollingHorizon, 2) => {
            let a = observation_by_id(visible, "ASSAY_A")?;
            let b = observation_by_id(visible, "ASSAY_B")?;
            PolicyStep {
                action: if a.high_grade || !b.high_grade {
                    Action::ChooseA
                } else {
                    Action::ChooseB
                },
                used_event_ids: vec!["ASSAY_A".to_owned(), "ASSAY_B".to_owned()],
            }
        }
        (Policy::RollingHorizon, _) => {
            return Err("rolling-horizon policy has no action at this epoch".to_owned());
        }
    };

    let mut used = BTreeSet::new();
    for event_id in &step.used_event_ids {
        if !used.insert(event_id) {
            return Err("policy returned duplicate event dependency".to_owned());
        }
        let observation = observation_by_id(visible, event_id)?;
        if observation.delivered_at_epoch > epoch {
            return Err("policy consumed an observation before it was delivered".to_owned());
        }
        if !observation.source_record_present || !observation.qc_pass {
            return Err("policy consumed an unqualified observation".to_owned());
        }
    }
    Ok(step)
}

fn utility_for_route(model: &UtilityModel, scenario: &Scenario, route: Route) -> i64 {
    match route {
        Route::LotA if scenario.a_high_grade => model.a_high_grade_utility_units,
        Route::LotA => model.a_low_grade_utility_units,
        Route::LotB if scenario.b_high_grade => model.b_high_grade_utility_units,
        Route::LotB => model.b_low_grade_utility_units,
    }
}

fn clairvoyant_upper_bound(model: &UtilityModel, scenario: &Scenario) -> i64 {
    utility_for_route(model, scenario, Route::LotA).max(utility_for_route(
        model,
        scenario,
        Route::LotB,
    ))
}

fn simulate_policy(
    policy: Policy,
    scenario: &Scenario,
    corpus: &Corpus,
    prior_route: Route,
) -> Check<PolicyOutcome> {
    let mut visible: Vec<Observation> = Vec::new();
    let mut pending: Vec<Observation> = Vec::new();
    let mut decisions: Vec<DecisionTrace> = Vec::new();
    let mut assays_run = 0_u32;
    let mut selected_route: Option<Route> = None;

    for epoch in 0..=2_u32 {
        let mut still_pending = Vec::new();
        for observation in pending.drain(..) {
            if observation.delivered_at_epoch <= epoch {
                visible.push(observation);
            } else {
                still_pending.push(observation);
            }
        }
        pending = still_pending;
        visible.sort_by(|a, b| a.event_id.cmp(&b.event_id));

        let step = policy_step(policy, epoch, &visible, prior_route)?;
        let visible_event_ids = visible
            .iter()
            .map(|observation| observation.event_id.clone())
            .collect::<Vec<_>>();
        decisions.push(DecisionTrace {
            decision_id: format!("{policy:?}-E{epoch}"),
            epoch,
            action: step.action,
            visible_event_ids,
            used_event_ids: step.used_event_ids.clone(),
        });

        match step.action {
            Action::RequestAssayA | Action::RequestAssayB => {
                let (event_id, lot_id, high_grade) = if step.action == Action::RequestAssayA {
                    ("ASSAY_A", "LOT_A", scenario.a_high_grade)
                } else {
                    ("ASSAY_B", "LOT_B", scenario.b_high_grade)
                };
                if visible
                    .iter()
                    .any(|observation| observation.event_id == event_id)
                    || pending
                        .iter()
                        .any(|observation| observation.event_id == event_id)
                {
                    return Err(format!("policy requested duplicate assay {event_id}"));
                }
                let delivered_at_epoch = epoch
                    .checked_add(corpus.utility_model.assay_delivery_delay_epochs)
                    .ok_or_else(|| "assay delivery epoch overflows".to_owned())?;
                pending.push(Observation {
                    event_id: event_id.to_owned(),
                    lot_id: lot_id.to_owned(),
                    observed_at_epoch: epoch,
                    delivered_at_epoch,
                    high_grade,
                    source_record_present: true,
                    qc_pass: true,
                });
                assays_run += 1;
            }
            Action::ChooseA => {
                selected_route = Some(Route::LotA);
                break;
            }
            Action::ChooseB => {
                selected_route = Some(Route::LotB);
                break;
            }
        }
    }

    let selected_route =
        selected_route.ok_or_else(|| format!("{policy:?} policy did not select a route"))?;
    let gross_utility_units = utility_for_route(&corpus.utility_model, scenario, selected_route);
    let assay_cost_units = i64::from(assays_run)
        .checked_mul(corpus.utility_model.assay_cost_units)
        .ok_or_else(|| "assay cost overflows".to_owned())?;
    let net_utility_units = gross_utility_units
        .checked_sub(assay_cost_units)
        .ok_or_else(|| "net utility overflows".to_owned())?;
    let upper_bound = clairvoyant_upper_bound(&corpus.utility_model, scenario);
    let trace_valid = verify_trace(&decisions, &visible, &pending).is_ok();

    Ok(PolicyOutcome {
        policy,
        selected_route,
        gross_utility_units,
        assay_count: assays_run,
        assay_cost_units,
        net_utility_units,
        clairvoyant_regret_units: upper_bound - net_utility_units,
        trace_valid,
        decisions,
        delivered_observations: visible,
    })
}

fn verify_trace(
    decisions: &[DecisionTrace],
    delivered_observations: &[Observation],
    pending_observations: &[Observation],
) -> Check<()> {
    let mut previous_epoch = None;
    let delivered: BTreeMap<&str, &Observation> = delivered_observations
        .iter()
        .map(|observation| (observation.event_id.as_str(), observation))
        .collect();
    if delivered.len() != delivered_observations.len() {
        return Err("duplicate delivered observation ID".to_owned());
    }
    let pending_ids: BTreeSet<&str> = pending_observations
        .iter()
        .map(|observation| observation.event_id.as_str())
        .collect();

    for decision in decisions {
        if previous_epoch.is_some_and(|previous| decision.epoch < previous) {
            return Err("decision trace is not chronological".to_owned());
        }
        previous_epoch = Some(decision.epoch);
        let visible: BTreeSet<&str> = decision
            .visible_event_ids
            .iter()
            .map(String::as_str)
            .collect();
        if visible.len() != decision.visible_event_ids.len() {
            return Err("visible-event snapshot has duplicate IDs".to_owned());
        }
        let expected_visible: BTreeSet<&str> = delivered_observations
            .iter()
            .filter(|observation| observation.delivered_at_epoch <= decision.epoch)
            .map(|observation| observation.event_id.as_str())
            .collect();
        if visible != expected_visible {
            return Err("visible-event snapshot does not match observations delivered by the decision epoch".to_owned());
        }
        let used: BTreeSet<&str> = decision.used_event_ids.iter().map(String::as_str).collect();
        if used.len() != decision.used_event_ids.len() {
            return Err("decision has duplicate event dependencies".to_owned());
        }
        for event_id in &used {
            if !visible.contains(event_id) {
                return Err(format!(
                    "decision dependency {event_id} is absent from its visible snapshot"
                ));
            }
            let observation = delivered
                .get(event_id)
                .ok_or_else(|| format!("decision dependency {event_id} was not delivered"))?;
            if observation.delivered_at_epoch > decision.epoch
                || !observation.source_record_present
                || !observation.qc_pass
            {
                return Err(format!(
                    "decision dependency {event_id} was not admissible at decision time"
                ));
            }
        }
        for event_id in &visible {
            if pending_ids.contains(event_id) {
                return Err(
                    "pending observation is incorrectly present in visible snapshot".to_owned(),
                );
            }
        }
    }
    Ok(())
}

fn clairvoyant_outcome(model: &UtilityModel, scenario: &Scenario) -> i64 {
    clairvoyant_upper_bound(model, scenario)
}

fn weighted_mean(scenarios: &[Scenario], values: &[i64]) -> Check<i64> {
    if scenarios.len() != values.len() {
        return Err("weighted mean value count does not match scenario count".to_owned());
    }
    let weighted_sum =
        scenarios
            .iter()
            .zip(values)
            .try_fold(0_i128, |sum, (scenario, value)| {
                sum.checked_add(i128::from(scenario.probability_basis_points) * i128::from(*value))
                    .ok_or_else(|| "weighted metric sum overflows".to_owned())
            })?;
    let divisor = i128::from(PROBABILITY_BPS);
    if weighted_sum % divisor != 0 {
        return Err(
            "weighted fixture mean is not an exact integer in declared utility units".to_owned(),
        );
    }
    i64::try_from(weighted_sum / divisor).map_err(|_| "weighted mean overflows i64".to_owned())
}

fn reduction_basis_points(baseline: i64, improved: i64) -> Check<u32> {
    if baseline <= 0 || improved < 0 || improved > baseline {
        return Err("regret reduction requires 0 <= improved <= positive baseline".to_owned());
    }
    let reduction = i128::from(baseline - improved) * 10_000 / i128::from(baseline);
    u32::try_from(reduction).map_err(|_| "regret reduction overflows u32".to_owned())
}

fn source_commit_from_args(args: &[String]) -> Check<String> {
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: mine-adaptive-policy-value-bench <corpus.json> --source-commit <40-hex-sha>"
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
    let prior_route = prior_route(&corpus)?;

    let mut scenario_receipts = Vec::new();
    for scenario in &corpus.scenarios {
        let static_outcome = simulate_policy(Policy::Static, scenario, &corpus, prior_route)?;
        let two_stage_outcome = simulate_policy(Policy::TwoStage, scenario, &corpus, prior_route)?;
        let rolling_outcome =
            simulate_policy(Policy::RollingHorizon, scenario, &corpus, prior_route)?;
        let upper_bound = clairvoyant_outcome(&corpus.utility_model, scenario);
        let expected = &scenario.expected;
        let expected_control_matched = static_outcome.net_utility_units
            == expected.static_net_units
            && two_stage_outcome.net_utility_units == expected.two_stage_net_units
            && rolling_outcome.net_utility_units == expected.rolling_horizon_net_units
            && upper_bound == expected.clairvoyant_upper_bound_units
            && static_outcome.trace_valid
            && two_stage_outcome.trace_valid
            && rolling_outcome.trace_valid;

        scenario_receipts.push(ScenarioReceipt {
            scenario_id: scenario.scenario_id.clone(),
            probability_basis_points: scenario.probability_basis_points,
            static_policy: static_outcome,
            two_stage: two_stage_outcome,
            rolling_horizon: rolling_outcome,
            clairvoyant_upper_bound_units: upper_bound,
            expected_control_matched,
        });
    }

    let static_mean = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| row.static_policy.net_utility_units)
            .collect::<Vec<_>>(),
    )?;
    let two_stage_mean = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| row.two_stage.net_utility_units)
            .collect::<Vec<_>>(),
    )?;
    let rolling_mean = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| row.rolling_horizon.net_utility_units)
            .collect::<Vec<_>>(),
    )?;
    let clairvoyant_mean = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| row.clairvoyant_upper_bound_units)
            .collect::<Vec<_>>(),
    )?;
    let static_regret = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| row.static_policy.clairvoyant_regret_units)
            .collect::<Vec<_>>(),
    )?;
    let two_stage_regret = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| row.two_stage.clairvoyant_regret_units)
            .collect::<Vec<_>>(),
    )?;
    let rolling_regret = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| row.rolling_horizon.clairvoyant_regret_units)
            .collect::<Vec<_>>(),
    )?;

    let trace_violations = scenario_receipts
        .iter()
        .flat_map(|row| [&row.static_policy, &row.two_stage, &row.rolling_horizon])
        .filter(|outcome| !outcome.trace_valid)
        .count();
    let nonanticipativity_violations = count_nonanticipativity_violations(&scenario_receipts);
    let static_assays = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| i64::from(row.static_policy.assay_count) * 1000)
            .collect::<Vec<_>>(),
    )?;
    let two_stage_assays = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| i64::from(row.two_stage.assay_count) * 1000)
            .collect::<Vec<_>>(),
    )?;
    let rolling_assays = weighted_mean(
        &corpus.scenarios,
        &scenario_receipts
            .iter()
            .map(|row| i64::from(row.rolling_horizon.assay_count) * 1000)
            .collect::<Vec<_>>(),
    )?;

    let rolling_vs_two_stage = reduction_basis_points(two_stage_regret, rolling_regret)?;
    let rolling_vs_static = reduction_basis_points(static_regret, rolling_regret)?;
    let all_controls_matched = scenario_receipts
        .iter()
        .all(|row| row.expected_control_matched)
        && static_mean == corpus.expected_aggregate.static_weighted_mean_net_units
        && two_stage_mean == corpus.expected_aggregate.two_stage_weighted_mean_net_units
        && rolling_mean
            == corpus
                .expected_aggregate
                .rolling_horizon_weighted_mean_net_units
        && clairvoyant_mean
            == corpus
                .expected_aggregate
                .clairvoyant_weighted_mean_upper_bound_units
        && static_regret == corpus.expected_aggregate.static_mean_regret_units
        && two_stage_regret == corpus.expected_aggregate.two_stage_mean_regret_units
        && rolling_regret == corpus.expected_aggregate.rolling_horizon_mean_regret_units
        && trace_violations == corpus.expected_aggregate.trace_violations
        && nonanticipativity_violations == corpus.expected_aggregate.nonanticipativity_violations
        && rolling_vs_two_stage
            == corpus
                .expected_aggregate
                .rolling_regret_reduction_vs_two_stage_basis_points
        && rolling_vs_static
            == corpus
                .expected_aggregate
                .rolling_regret_reduction_vs_static_basis_points;

    let expected_controls_matched = scenario_receipts
        .iter()
        .filter(|row| row.expected_control_matched)
        .count();
    let receipt = Receipt {
        result: if all_controls_matched {
            "validated"
        } else {
            "control_mismatch"
        }
        .to_owned(),
        suite_id: corpus.suite_id,
        suite_version: corpus.suite_version,
        evaluator_version: VERSION.to_owned(),
        source_commit,
        corpus_sha256: sha256(&bytes),
        scenario_count: scenario_receipts.len(),
        expected_controls_matched,
        trace_violations,
        nonanticipativity_violations,
        two_stage_regret_reduction_vs_static_basis_points: reduction_basis_points(
            static_regret,
            two_stage_regret,
        )?,
        rolling_regret_reduction_vs_two_stage_basis_points: rolling_vs_two_stage,
        rolling_regret_reduction_vs_static_basis_points: rolling_vs_static,
        policy_aggregates: vec![
            PolicyAggregate {
                policy: Policy::Static,
                weighted_mean_net_units: static_mean,
                weighted_mean_regret_units: static_regret,
                weighted_mean_assay_count_milli: static_assays,
            },
            PolicyAggregate {
                policy: Policy::TwoStage,
                weighted_mean_net_units: two_stage_mean,
                weighted_mean_regret_units: two_stage_regret,
                weighted_mean_assay_count_milli: two_stage_assays,
            },
            PolicyAggregate {
                policy: Policy::RollingHorizon,
                weighted_mean_net_units: rolling_mean,
                weighted_mean_regret_units: rolling_regret,
                weighted_mean_assay_count_milli: rolling_assays,
            },
        ],
        scenarios: scenario_receipts,
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

fn count_nonanticipativity_violations(scenarios: &[ScenarioReceipt]) -> usize {
    let mut seen: BTreeMap<(Policy, u32, String), Action> = BTreeMap::new();
    let mut violations = 0_usize;
    for scenario in scenarios {
        for outcome in [
            &scenario.static_policy,
            &scenario.two_stage,
            &scenario.rolling_horizon,
        ] {
            let observations: BTreeMap<String, &Observation> = outcome
                .delivered_observations
                .iter()
                .map(|observation| (observation.event_id.clone(), observation))
                .collect();
            for decision in &outcome.decisions {
                let signature = decision
                    .visible_event_ids
                    .iter()
                    .filter_map(|event_id| observations.get(event_id))
                    .map(|observation| {
                        format!("{}:{}", observation.event_id, observation.high_grade)
                    })
                    .collect::<Vec<_>>()
                    .join("|");
                let key = (outcome.policy, decision.epoch, signature);
                if let Some(previous) = seen.get(&key) {
                    if *previous != decision.action {
                        violations += 1;
                    }
                } else {
                    seen.insert(key, decision.action);
                }
            }
        }
    }
    violations
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
            "../../../docs/tech/mine-adaptive-policy-value-corpus-v0.1.json"
        ))
        .expect("frozen synthetic policy tournament should deserialize")
    }

    #[test]
    fn frozen_worlds_match_all_per_policy_known_answers() {
        let corpus = corpus();
        validate_corpus(&corpus).expect("corpus metadata should validate");
        let prior = prior_route(&corpus).expect("prior baseline is defined");
        assert_eq!(Route::LotA, prior);
        for scenario in &corpus.scenarios {
            let s =
                simulate_policy(Policy::Static, scenario, &corpus, prior).expect("static policy");
            let t = simulate_policy(Policy::TwoStage, scenario, &corpus, prior)
                .expect("two-stage policy");
            let r = simulate_policy(Policy::RollingHorizon, scenario, &corpus, prior)
                .expect("rolling policy");
            assert_eq!(
                scenario.expected.static_net_units, s.net_utility_units,
                "{}",
                scenario.scenario_id
            );
            assert_eq!(
                scenario.expected.two_stage_net_units, t.net_utility_units,
                "{}",
                scenario.scenario_id
            );
            assert_eq!(
                scenario.expected.rolling_horizon_net_units, r.net_utility_units,
                "{}",
                scenario.scenario_id
            );
            assert_eq!(
                scenario.expected.clairvoyant_upper_bound_units,
                clairvoyant_upper_bound(&corpus.utility_model, scenario),
                "{}",
                scenario.scenario_id
            );
            assert!(s.trace_valid && t.trace_valid && r.trace_valid);
        }
    }

    #[test]
    fn rolling_horizon_has_lower_regret_on_this_planted_counterexample() {
        let corpus = corpus();
        let prior = prior_route(&corpus).expect("prior baseline");
        let mut scores = Vec::new();
        for scenario in &corpus.scenarios {
            scores.push((
                scenario.probability_basis_points,
                simulate_policy(Policy::RollingHorizon, scenario, &corpus, prior)
                    .expect("rolling policy")
                    .net_utility_units,
                simulate_policy(Policy::TwoStage, scenario, &corpus, prior)
                    .expect("two-stage policy")
                    .net_utility_units,
            ));
        }
        let rolling = weighted_mean(
            &corpus.scenarios,
            &scores.iter().map(|entry| entry.1).collect::<Vec<_>>(),
        )
        .expect("integer weighted mean");
        let two_stage = weighted_mean(
            &corpus.scenarios,
            &scores.iter().map(|entry| entry.2).collect::<Vec<_>>(),
        )
        .expect("integer weighted mean");
        assert_eq!(107, rolling);
        assert_eq!(98, two_stage);
        assert_eq!(
            5294,
            reduction_basis_points(17, 8).expect("positive baseline")
        );
    }

    #[test]
    fn policies_receive_only_delivered_observations() {
        let corpus = corpus();
        let prior = prior_route(&corpus).expect("prior baseline");
        for scenario in &corpus.scenarios {
            for policy in [Policy::Static, Policy::TwoStage, Policy::RollingHorizon] {
                let outcome =
                    simulate_policy(policy, scenario, &corpus, prior).expect("policy execution");
                assert!(outcome.trace_valid);
                for decision in &outcome.decisions {
                    for event_id in &decision.used_event_ids {
                        assert!(decision.visible_event_ids.contains(event_id));
                        let observation = outcome
                            .delivered_observations
                            .iter()
                            .find(|observation| observation.event_id == *event_id)
                            .expect("used event must exist in delivered observation set");
                        assert!(observation.delivered_at_epoch <= decision.epoch);
                    }
                }
            }
        }
    }

    #[test]
    fn visible_snapshot_must_match_delivered_observations() {
        let corpus = corpus();
        let scenario = corpus
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "WORLD_A_LOW_B_HIGH")
            .expect("scenario exists");
        let prior = prior_route(&corpus).expect("prior baseline");
        let mut outcome = simulate_policy(Policy::RollingHorizon, scenario, &corpus, prior)
            .expect("rolling policy");
        outcome.decisions[0]
            .visible_event_ids
            .push("ASSAY_A".to_owned());
        assert!(verify_trace(&outcome.decisions, &outcome.delivered_observations, &[]).is_err());
    }

    #[test]
    fn counterfactual_scenarios_preserve_actions_until_observation_diverges() {
        let corpus = corpus();
        let prior = prior_route(&corpus).expect("prior baseline");
        let runs: Vec<_> = corpus
            .scenarios
            .iter()
            .map(|scenario| {
                (
                    scenario.scenario_id.clone(),
                    simulate_policy(Policy::RollingHorizon, scenario, &corpus, prior)
                        .expect("rolling policy"),
                )
            })
            .collect();
        let low_a_high_b = runs
            .iter()
            .find(|(id, _)| id == "WORLD_A_LOW_B_HIGH")
            .expect("world exists");
        let low_a_low_b = runs
            .iter()
            .find(|(id, _)| id == "WORLD_A_LOW_B_LOW")
            .expect("world exists");
        let a_decision = low_a_high_b
            .1
            .decisions
            .iter()
            .find(|d| d.epoch == 1)
            .expect("decision");
        let b_decision = low_a_low_b
            .1
            .decisions
            .iter()
            .find(|d| d.epoch == 1)
            .expect("decision");
        assert_eq!(a_decision.action, b_decision.action);
        assert_eq!(Action::RequestAssayB, a_decision.action);
        let a_late = low_a_high_b
            .1
            .decisions
            .iter()
            .find(|d| d.epoch == 2)
            .expect("late decision");
        let b_late = low_a_low_b
            .1
            .decisions
            .iter()
            .find(|d| d.epoch == 2)
            .expect("late decision");
        assert_ne!(a_late.action, b_late.action);
    }

    #[test]
    fn unknown_json_fields_fail_closed() {
        let raw = include_str!("../../../docs/tech/mine-adaptive-policy-value-corpus-v0.1.json")
            .replacen(
                "\"suite_id\":",
                "\"clairvoyant_authority\":true,\"suite_id\":",
                1,
            );
        assert!(serde_json::from_str::<Corpus>(&raw).is_err());
    }
}
