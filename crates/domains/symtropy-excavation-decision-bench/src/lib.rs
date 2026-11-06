//! Evidence-bounded excavation strategy eligibility and robust Pareto comparison.
//!
//! This is an offline decision benchmark, not a physical model, safety case,
//! site qualification authority, statistical inference engine, or controller.
//! It compares only eligible strategies under one declared decision profile.
//! Outcome intervals are treated conservatively: overlapping intervals do
//! not establish dominance.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Eligibility {
    Eligible,
    Ineligible,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DecisionState {
    ParetoFrontierAvailable,
    AbstainNoEligibleStrategy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DecisionErrorCode {
    RootMustBeObject,
    UnsupportedSchemaVersion,
    MissingCaseId,
    MissingProfile,
    MissingStrategies,
    EmptyStrategies,
    MissingProfileField,
    EmptyObjectiveSet,
    DuplicateObjectiveId,
    DuplicateStrategyId,
    MissingStrategyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DecisionError {
    pub code: DecisionErrorCode,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Maximize,
    Minimize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Relation {
    AtLeast,
    AtMost,
}

#[derive(Clone, Debug, PartialEq)]
struct Objective {
    metric_id: String,
    unit: String,
    direction: Direction,
}

#[derive(Clone, Debug, PartialEq)]
struct Constraint {
    metric_id: String,
    unit: String,
    relation: Relation,
    threshold: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecisionContext {
    profile_id: String,
    task_profile_ref: String,
    material_cohort_ref: String,
    acceptance_profile_ref: String,
    site_context_ref: String,
    energy_boundary_ref: String,
}

#[derive(Clone, Debug, PartialEq)]
struct MetricInterval {
    lower: f64,
    upper: f64,
    unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StrategyAssessment {
    pub strategy_id: String,
    pub eligibility: Eligibility,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionReport {
    pub case_id: String,
    pub state: DecisionState,
    pub assessments: Vec<StrategyAssessment>,
    pub eligible_strategy_ids: Vec<String>,
    pub ineligible_strategy_ids: Vec<String>,
    pub indeterminate_strategy_ids: Vec<String>,
    pub pareto_frontier_ids: Vec<String>,
    pub robustly_dominated_ids: Vec<String>,
    pub experiment_option_count: usize,
    /// v0.1 deliberately does not compute EVSI/EVPI without a declared utility
    /// function, prior, observation model, experiment-cost model and stopping rule.
    pub value_of_information_computed: bool,
}

#[derive(Clone, Debug)]
struct EvaluatedStrategy {
    assessment: StrategyAssessment,
    outcomes: BTreeMap<String, MetricInterval>,
}

#[derive(Default)]
struct StatusBuilder {
    ineligible: bool,
    indeterminate: bool,
    reasons: BTreeSet<String>,
}

impl StatusBuilder {
    fn ineligible(&mut self, reason: impl Into<String>) {
        self.ineligible = true;
        self.reasons.insert(reason.into());
    }

    fn indeterminate(&mut self, reason: impl Into<String>) {
        self.indeterminate = true;
        self.reasons.insert(reason.into());
    }

    fn finish(self, strategy_id: String) -> StrategyAssessment {
        let eligibility = if self.ineligible {
            Eligibility::Ineligible
        } else if self.indeterminate {
            Eligibility::Indeterminate
        } else {
            Eligibility::Eligible
        };
        StrategyAssessment {
            strategy_id,
            eligibility,
            reasons: self.reasons.into_iter().collect(),
        }
    }
}

/// Parse JSON and evaluate a decision case.
pub fn evaluate_case_json(source: &str) -> Result<DecisionReport, String> {
    let value: Value = serde_json::from_str(source).map_err(|error| error.to_string())?;
    evaluate_case(&value).map_err(|errors| {
        errors
            .into_iter()
            .map(|error| format!("{:?}: {}", error.code, error.detail))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

/// Evaluate strategy eligibility and robust Pareto dominance.
///
/// This evaluator intentionally does not create a single weighted score. It
/// requires a common profile, units, energy boundary, independent primary
/// outcomes (when declared), and evidence-qualified support. Unknown evidence
/// produces Indeterminate; an explicit failed support/qualification gate
/// produces Ineligible.
pub fn evaluate_case(case: &Value) -> Result<DecisionReport, Vec<DecisionError>> {
    let mut errors = Vec::new();
    if !case.is_object() {
        return Err(vec![DecisionError {
            code: DecisionErrorCode::RootMustBeObject,
            detail: "case root must be a JSON object".to_owned(),
        }]);
    }
    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        errors.push(DecisionError {
            code: DecisionErrorCode::UnsupportedSchemaVersion,
            detail: "schema_version must equal 1".to_owned(),
        });
    }
    let case_id = non_empty_string(case, "case_id").map(ToOwned::to_owned);
    if case_id.is_none() {
        errors.push(DecisionError {
            code: DecisionErrorCode::MissingCaseId,
            detail: "case_id must be a non-empty string".to_owned(),
        });
    }
    let profile = case.get("profile");
    if profile.is_none() {
        errors.push(DecisionError {
            code: DecisionErrorCode::MissingProfile,
            detail: "profile object is required".to_owned(),
        });
    }
    let Some(strategies) = case.get("strategies").and_then(Value::as_array) else {
        errors.push(DecisionError {
            code: DecisionErrorCode::MissingStrategies,
            detail: "strategies must be an array".to_owned(),
        });
        return Err(errors);
    };
    if strategies.is_empty() {
        errors.push(DecisionError {
            code: DecisionErrorCode::EmptyStrategies,
            detail: "strategies must contain at least one entry".to_owned(),
        });
    }
    let Some(profile) = profile else {
        return Err(errors);
    };

    let parsed_profile = parse_profile(profile, &mut errors);
    let mut ids = BTreeSet::new();
    for strategy in strategies {
        match non_empty_string(strategy, "strategy_id") {
            Some(id) => {
                if !ids.insert(id.to_owned()) {
                    errors.push(DecisionError {
                        code: DecisionErrorCode::DuplicateStrategyId,
                        detail: format!("duplicate strategy_id: {id}"),
                    });
                }
            }
            None => errors.push(DecisionError {
                code: DecisionErrorCode::MissingStrategyId,
                detail: "every strategy requires a non-empty strategy_id".to_owned(),
            }),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let (objectives, constraints, min_tier, context, require_independent) =
        parsed_profile.expect("profile is valid when errors is empty");
    let case_id = case_id.expect("case_id was validated");

    let mut evaluated = strategies
        .iter()
        .map(|strategy| {
            evaluate_strategy(
                strategy,
                &objectives,
                &constraints,
                min_tier,
                &context,
                require_independent,
            )
        })
        .collect::<Vec<_>>();
    evaluated.sort_by(|left, right| {
        left.assessment
            .strategy_id
            .cmp(&right.assessment.strategy_id)
    });

    let eligible = evaluated
        .iter()
        .filter(|entry| entry.assessment.eligibility == Eligibility::Eligible)
        .collect::<Vec<_>>();
    let eligible_ids = eligible
        .iter()
        .map(|entry| entry.assessment.strategy_id.clone())
        .collect::<Vec<_>>();

    let mut dominated = BTreeSet::new();
    for candidate in &eligible {
        for alternative in &eligible {
            if candidate.assessment.strategy_id != alternative.assessment.strategy_id
                && robustly_dominates(alternative, candidate, &objectives)
            {
                dominated.insert(candidate.assessment.strategy_id.clone());
                break;
            }
        }
    }

    let mut frontier = eligible_ids
        .iter()
        .filter(|id| !dominated.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    frontier.sort();
    let mut dominated_ids = dominated.into_iter().collect::<Vec<_>>();
    dominated_ids.sort();

    let assessments = evaluated
        .iter()
        .map(|entry| entry.assessment.clone())
        .collect::<Vec<_>>();
    let ineligible_ids = assessments
        .iter()
        .filter(|assessment| assessment.eligibility == Eligibility::Ineligible)
        .map(|assessment| assessment.strategy_id.clone())
        .collect::<Vec<_>>();
    let indeterminate_ids = assessments
        .iter()
        .filter(|assessment| assessment.eligibility == Eligibility::Indeterminate)
        .map(|assessment| assessment.strategy_id.clone())
        .collect::<Vec<_>>();
    let experiment_option_count = case
        .get("experiment_options")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    Ok(DecisionReport {
        case_id,
        state: if eligible_ids.is_empty() {
            DecisionState::AbstainNoEligibleStrategy
        } else {
            DecisionState::ParetoFrontierAvailable
        },
        assessments,
        eligible_strategy_ids: eligible_ids,
        ineligible_strategy_ids: ineligible_ids,
        indeterminate_strategy_ids: indeterminate_ids,
        pareto_frontier_ids: frontier,
        robustly_dominated_ids: dominated_ids,
        experiment_option_count,
        value_of_information_computed: false,
    })
}

fn parse_profile(
    profile: &Value,
    errors: &mut Vec<DecisionError>,
) -> Option<(Vec<Objective>, Vec<Constraint>, u8, DecisionContext, bool)> {
    let required_strings = [
        "profile_id",
        "task_profile_ref",
        "material_cohort_ref",
        "acceptance_profile_ref",
        "site_context_ref",
        "energy_boundary_ref",
        "minimum_evidence_tier",
    ];
    let mut valid = true;
    for field in required_strings {
        if non_empty_string(profile, field).is_none() {
            errors.push(DecisionError {
                code: DecisionErrorCode::MissingProfileField,
                detail: format!("profile.{field} must be a non-empty string"),
            });
            valid = false;
        }
    }
    let min_tier = non_empty_string(profile, "minimum_evidence_tier")
        .and_then(tier_rank);
    if min_tier.is_none() {
        errors.push(DecisionError {
            code: DecisionErrorCode::MissingProfileField,
            detail: "profile.minimum_evidence_tier must be R0..R5".to_owned(),
        });
        valid = false;
    }
    let require_independent = profile
        .get("primary_outcomes_require_independent_measurements")
        .and_then(Value::as_bool);
    if require_independent.is_none() {
        errors.push(DecisionError {
            code: DecisionErrorCode::MissingProfileField,
            detail: "profile.primary_outcomes_require_independent_measurements must be boolean".to_owned(),
        });
        valid = false;
    }

    let Some(objective_values) = profile.get("objectives").and_then(Value::as_array) else {
        errors.push(DecisionError {
            code: DecisionErrorCode::EmptyObjectiveSet,
            detail: "profile.objectives must be a non-empty array".to_owned(),
        });
        return None;
    };
    if objective_values.is_empty() {
        errors.push(DecisionError {
            code: DecisionErrorCode::EmptyObjectiveSet,
            detail: "profile.objectives must contain at least one objective".to_owned(),
        });
        valid = false;
    }

    let mut objective_ids = BTreeSet::new();
    let mut objectives = Vec::new();
    for item in objective_values {
        let id = non_empty_string(item, "metric_id");
        let unit = non_empty_string(item, "unit");
        let direction = match non_empty_string(item, "direction") {
            Some("MAXIMIZE") => Some(Direction::Maximize),
            Some("MINIMIZE") => Some(Direction::Minimize),
            _ => None,
        };
        if let (Some(id), Some(unit), Some(direction)) = (id, unit, direction) {
            if !objective_ids.insert(id.to_owned()) {
                errors.push(DecisionError {
                    code: DecisionErrorCode::DuplicateObjectiveId,
                    detail: format!("duplicate objective metric_id: {id}"),
                });
                valid = false;
            }
            objectives.push(Objective {
                metric_id: id.to_owned(),
                unit: unit.to_owned(),
                direction,
            });
        } else {
            errors.push(DecisionError {
                code: DecisionErrorCode::MissingProfileField,
                detail: "every objective requires metric_id, unit and direction".to_owned(),
            });
            valid = false;
        }
    }

    let mut constraints = Vec::new();
    if let Some(constraint_values) = profile.get("constraints").and_then(Value::as_array) {
        for item in constraint_values {
            let relation = match non_empty_string(item, "relation") {
                Some("AT_LEAST") => Some(Relation::AtLeast),
                Some("AT_MOST") => Some(Relation::AtMost),
                _ => None,
            };
            let threshold = finite_number(item, "threshold");
            if let (
                Some(metric_id),
                Some(unit),
                Some(relation),
                Some(threshold),
            ) = (
                non_empty_string(item, "metric_id"),
                non_empty_string(item, "unit"),
                relation,
                threshold,
            ) {
                constraints.push(Constraint {
                    metric_id: metric_id.to_owned(),
                    unit: unit.to_owned(),
                    relation,
                    threshold,
                });
            } else {
                errors.push(DecisionError {
                    code: DecisionErrorCode::MissingProfileField,
                    detail: "every constraint requires metric_id, unit, relation and finite threshold".to_owned(),
                });
                valid = false;
            }
        }
    } else {
        errors.push(DecisionError {
            code: DecisionErrorCode::MissingProfileField,
            detail: "profile.constraints must be an array".to_owned(),
        });
        valid = false;
    }

    if !valid {
        return None;
    }
    Some((
        objectives,
        constraints,
        min_tier.expect("validated tier"),
        DecisionContext {
            profile_id: non_empty_string(profile, "profile_id").expect("validated profile id").to_owned(),
            task_profile_ref: non_empty_string(profile, "task_profile_ref").expect("validated task ref").to_owned(),
            material_cohort_ref: non_empty_string(profile, "material_cohort_ref").expect("validated material cohort").to_owned(),
            acceptance_profile_ref: non_empty_string(profile, "acceptance_profile_ref").expect("validated acceptance profile").to_owned(),
            site_context_ref: non_empty_string(profile, "site_context_ref").expect("validated site context").to_owned(),
            energy_boundary_ref: non_empty_string(profile, "energy_boundary_ref").expect("validated boundary").to_owned(),
        },
        require_independent.expect("validated boolean"),
    ))
}

fn evaluate_strategy(
    strategy: &Value,
    objectives: &[Objective],
    constraints: &[Constraint],
    min_tier: u8,
    context: &DecisionContext,
    require_independent: bool,
) -> EvaluatedStrategy {
    let strategy_id = non_empty_string(strategy, "strategy_id")
        .unwrap_or("<missing-strategy-id>")
        .to_owned();
    let mut status = StatusBuilder::default();

    match non_empty_string(strategy, "evidence_tier").and_then(tier_rank) {
        Some(rank) if rank < min_tier => status.ineligible(format!(
            "EVIDENCE_BELOW_MINIMUM: strategy tier is below profile minimum R{}",
            min_tier
        )),
        Some(_) => {}
        None => status.indeterminate("EVIDENCE_TIER_UNKNOWN_OR_MISSING"),
    }

    check_support(
        strategy,
        "geology_support",
        "GEOLOGY",
        &mut status,
    );
    check_support(
        strategy,
        "machine_support",
        "MACHINE",
        &mut status,
    );

    match non_empty_string(strategy, "engineering_qualification_status") {
        Some("QUALIFIED_FOR_DECLARED_TASK")
            if non_empty_string(strategy, "engineering_qualification_ref").is_some() => {}
        Some("QUALIFIED_FOR_DECLARED_TASK") => {
            status.indeterminate("ENGINEERING_QUALIFICATION_REFERENCE_MISSING")
        }
        Some("NOT_QUALIFIED") => {
            status.ineligible("ENGINEERING_QUALIFICATION_EXPLICITLY_FAILED")
        }
        Some("UNKNOWN") | None => {
            status.indeterminate("ENGINEERING_QUALIFICATION_UNKNOWN")
        }
        _ => status.indeterminate("ENGINEERING_QUALIFICATION_STATUS_INVALID"),
    }

    match non_empty_string(strategy, "resource_boundary_status") {
        Some("COMPLETE") => {
            if non_empty_string(strategy, "resource_ledger_ref").is_none() {
                status.indeterminate("RESOURCE_LEDGER_REFERENCE_MISSING");
            }
        }
        Some("INCOMPLETE") => status.indeterminate("RESOURCE_BOUNDARY_INCOMPLETE"),
        Some("UNKNOWN") | None => status.indeterminate("RESOURCE_BOUNDARY_UNKNOWN"),
        _ => status.indeterminate("RESOURCE_BOUNDARY_STATUS_INVALID"),
    }

    for (field, expected, label) in [
        ("decision_profile_ref", context.profile_id.as_str(), "DECISION_PROFILE"),
        ("task_profile_ref", context.task_profile_ref.as_str(), "TASK_PROFILE"),
        ("material_cohort_ref", context.material_cohort_ref.as_str(), "MATERIAL_COHORT"),
        ("acceptance_profile_ref", context.acceptance_profile_ref.as_str(), "ACCEPTANCE_PROFILE"),
        ("site_context_ref", context.site_context_ref.as_str(), "SITE_CONTEXT"),
        ("energy_boundary_ref", context.energy_boundary_ref.as_str(), "ENERGY_BOUNDARY"),
    ] {
        match non_empty_string(strategy, field) {
            Some(actual) if actual == expected => {}
            Some(_) => status.indeterminate(format!("{label}_MISMATCH")),
            None => status.indeterminate(format!("{label}_REFERENCE_MISSING")),
        }
    }

    for field in ["configuration_ref"] {
        if non_empty_string(strategy, field).is_none() {
            status.indeterminate(format!("CONFIGURATION_REFERENCE_MISSING: {field}"));
        }
    }

    let mut outcomes = BTreeMap::new();
    let mut duplicate_metrics = BTreeSet::new();
    if let Some(values) = strategy.get("outcomes").and_then(Value::as_array) {
        for outcome in values {
            let Some(metric_id) = non_empty_string(outcome, "metric_id") else {
                status.indeterminate("OUTCOME_METRIC_ID_MISSING");
                continue;
            };
            if !duplicate_metrics.insert(metric_id.to_owned()) {
                status.indeterminate(format!("DUPLICATE_OUTCOME_METRIC: {metric_id}"));
                continue;
            }
            let (Some(lower), Some(upper), Some(unit)) = (
                finite_number(outcome, "lower"),
                finite_number(outcome, "upper"),
                non_empty_string(outcome, "unit"),
            ) else {
                status.indeterminate(format!("OUTCOME_INTERVAL_INVALID: {metric_id}"));
                continue;
            };
            if lower > upper {
                status.indeterminate(format!("OUTCOME_INTERVAL_REVERSED: {metric_id}"));
                continue;
            }
            let role = non_empty_string(outcome, "evidence_role");
            if require_independent {
                match role {
                    Some("IndependentMeasurement") => {
                        if non_empty_string(outcome, "calibration_ref").is_none() {
                            status.indeterminate(format!("INDEPENDENT_OUTCOME_CALIBRATION_MISSING: {metric_id}"));
                        }
                    }
                    Some("DerivedFromIndependentMeasurements") => {
                        if non_empty_string(outcome, "lineage_ref").is_none() {
                            status.indeterminate(format!("DERIVED_OUTCOME_LINEAGE_MISSING: {metric_id}"));
                        }
                        if non_empty_string(outcome, "calibration_ref").is_none() {
                            status.indeterminate(format!("DERIVED_OUTCOME_CALIBRATION_MISSING: {metric_id}"));
                        }
                    }
                    Some("ModelEstimate" | "Simulation" | "OperatorReported" | "ProcessSensor") => {
                        status.indeterminate(format!("PRIMARY_OUTCOME_NOT_INDEPENDENT: {metric_id}"))
                    }
                    _ => status.indeterminate(format!("OUTCOME_EVIDENCE_ROLE_UNKNOWN: {metric_id}")),
                }
            }
            outcomes.insert(
                metric_id.to_owned(),
                MetricInterval {
                    lower,
                    upper,
                    unit: unit.to_owned(),
                },
            );
        }
    } else {
        status.indeterminate("OUTCOMES_ARRAY_MISSING");
    }

    for objective in objectives {
        match outcomes.get(&objective.metric_id) {
            Some(interval) if interval.unit == objective.unit => {}
            Some(_) => status.indeterminate(format!(
                "OBJECTIVE_UNIT_MISMATCH: {}",
                objective.metric_id
            )),
            None => status.indeterminate(format!(
                "OBJECTIVE_OUTCOME_MISSING: {}",
                objective.metric_id
            )),
        }
    }

    for constraint in constraints {
        match outcomes.get(&constraint.metric_id) {
            Some(interval) if interval.unit != constraint.unit => status.indeterminate(format!(
                "CONSTRAINT_UNIT_MISMATCH: {}",
                constraint.metric_id
            )),
            Some(interval) => match constraint.relation {
                Relation::AtLeast if interval.upper < constraint.threshold => status
                    .ineligible(format!(
                        "HARD_CONSTRAINT_FAILED: {} is below required threshold",
                        constraint.metric_id
                    )),
                Relation::AtLeast if interval.lower < constraint.threshold => status
                    .indeterminate(format!(
                        "HARD_CONSTRAINT_INTERVAL_CROSSES: {}",
                        constraint.metric_id
                    )),
                Relation::AtMost if interval.lower > constraint.threshold => status
                    .ineligible(format!(
                        "HARD_CONSTRAINT_FAILED: {} is above maximum threshold",
                        constraint.metric_id
                    )),
                Relation::AtMost if interval.upper > constraint.threshold => status
                    .indeterminate(format!(
                        "HARD_CONSTRAINT_INTERVAL_CROSSES: {}",
                        constraint.metric_id
                    )),
                _ => {}
            },
            None => status.indeterminate(format!(
                "CONSTRAINT_OUTCOME_MISSING: {}",
                constraint.metric_id
            )),
        }
    }

    EvaluatedStrategy {
        assessment: status.finish(strategy_id),
        outcomes,
    }
}

fn check_support(
    strategy: &Value,
    field: &str,
    label: &str,
    status: &mut StatusBuilder,
) {
    match non_empty_string(strategy, field) {
        Some("SUPPORTED") => {}
        Some("UNSUPPORTED") => status.ineligible(format!("{label}_DOMAIN_UNSUPPORTED")),
        Some("UNKNOWN") | None => status.indeterminate(format!("{label}_SUPPORT_UNKNOWN")),
        _ => status.indeterminate(format!("{label}_SUPPORT_STATUS_INVALID")),
    }
}

fn robustly_dominates(
    candidate: &EvaluatedStrategy,
    other: &EvaluatedStrategy,
    objectives: &[Objective],
) -> bool {
    let mut strictly_better = false;
    for objective in objectives {
        let (Some(candidate_interval), Some(other_interval)) = (
            candidate.outcomes.get(&objective.metric_id),
            other.outcomes.get(&objective.metric_id),
        ) else {
            return false;
        };
        if candidate_interval.unit != objective.unit || other_interval.unit != objective.unit {
            return false;
        }
        match objective.direction {
            Direction::Maximize => {
                if candidate_interval.lower < other_interval.upper {
                    return false;
                }
                if candidate_interval.lower > other_interval.upper {
                    strictly_better = true;
                }
            }
            Direction::Minimize => {
                if candidate_interval.upper > other_interval.lower {
                    return false;
                }
                if candidate_interval.upper < other_interval.lower {
                    strictly_better = true;
                }
            }
        }
    }
    strictly_better
}

fn tier_rank(value: &str) -> Option<u8> {
    match value {
        "R0" => Some(0),
        "R1" => Some(1),
        "R2" => Some(2),
        "R3" => Some(3),
        "R4" => Some(4),
        "R5" => Some(5),
        _ => None,
    }
}

fn finite_number(object: &Value, field: &str) -> Option<f64> {
    object
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn non_empty_string<'a>(object: &'a Value, field: &str) -> Option<&'a str> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-excavation-decision-case.v0_1.json"
        ))
        .expect("synthetic decision fixture must parse")
    }

    fn assessment<'a>(
        report: &'a DecisionReport,
        strategy_id: &str,
    ) -> &'a StrategyAssessment {
        report
            .assessments
            .iter()
            .find(|item| item.strategy_id == strategy_id)
            .expect("strategy assessment exists")
    }

    fn make_two_strategy_case(
        candidate_lower: f64,
        candidate_upper: f64,
        baseline_lower: f64,
        baseline_upper: f64,
    ) -> Value {
        let mut data = fixture();
        let strategies = data["strategies"].as_array_mut().unwrap();
        strategies.truncate(2);
        strategies[0]["outcomes"][0]["lower"] = json!(baseline_lower);
        strategies[0]["outcomes"][0]["upper"] = json!(baseline_upper);
        strategies[1]["outcomes"][0]["lower"] = json!(candidate_lower);
        strategies[1]["outcomes"][0]["upper"] = json!(candidate_upper);
        data
    }

    #[test]
    fn synthetic_case_returns_expected_frontier_and_abstentions() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(report.state, DecisionState::ParetoFrontierAvailable);
        assert_eq!(
            report.pareto_frontier_ids,
            vec!["HPWJ-DISC-HYBRID", "OSCILLATING-DISC"]
        );
        assert_eq!(
            report.robustly_dominated_ids,
            vec!["MECHANICAL-BASELINE"]
        );
        assert_eq!(assessment(&report, "ATCP-HIGH-TEMP").eligibility, Eligibility::Indeterminate);
        assert_eq!(assessment(&report, "LASER-PDC-UNQUALIFIED").eligibility, Eligibility::Ineligible);
        assert_eq!(report.value_of_information_computed, false);
        assert_eq!(report.experiment_option_count, 2);
    }

    #[test]
    fn no_eligible_strategy_returns_explicit_abstention() {
        let mut data = fixture();
        for strategy in data["strategies"].as_array_mut().unwrap() {
            strategy["engineering_qualification_status"] = json!("NOT_QUALIFIED");
        }
        let report = evaluate_case(&data).unwrap();
        assert_eq!(report.state, DecisionState::AbstainNoEligibleStrategy);
        assert!(report.eligible_strategy_ids.is_empty());
        assert!(report.pareto_frontier_ids.is_empty());
    }

    #[test]
    fn task_profile_mismatch_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["task_profile_ref"] = json!("DIFFERENT-TASK");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn material_cohort_mismatch_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["material_cohort_ref"] = json!("DIFFERENT-COHORT");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn acceptance_profile_mismatch_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["acceptance_profile_ref"] = json!("DIFFERENT-ACCEPTANCE");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn site_context_mismatch_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["site_context_ref"] = json!("DIFFERENT-SITE");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn complete_resource_boundary_requires_ledger_reference() {
        let mut data = fixture();
        data["strategies"][0].as_object_mut().unwrap().remove("resource_ledger_ref");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn evidence_below_minimum_is_ineligible() {
        let mut data = fixture();
        data["strategies"][0]["evidence_tier"] = json!("R1");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Ineligible);
    }

    #[test]
    fn unsupported_geology_is_ineligible() {
        let mut data = fixture();
        data["strategies"][0]["geology_support"] = json!("UNSUPPORTED");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Ineligible);
    }

    #[test]
    fn unknown_geology_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["geology_support"] = json!("UNKNOWN");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn unqualified_engineering_status_is_ineligible() {
        let mut data = fixture();
        data["strategies"][0]["engineering_qualification_status"] = json!("NOT_QUALIFIED");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Ineligible);
    }

    #[test]
    fn unknown_engineering_status_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["engineering_qualification_status"] = json!("UNKNOWN");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn incomplete_resource_boundary_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["resource_boundary_status"] = json!("INCOMPLETE");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn energy_boundary_mismatch_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["energy_boundary_ref"] = json!("DIFFERENT-BOUNDARY");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn model_estimate_cannot_support_independent_primary_outcome() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][0]["evidence_role"] = json!("ModelEstimate");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn derived_primary_outcome_requires_lineage() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][0]["evidence_role"] = json!("DerivedFromIndependentMeasurements");
        data["strategies"][0]["outcomes"][0].as_object_mut().unwrap().remove("lineage_ref");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn independent_primary_outcome_requires_calibration_reference() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][0].as_object_mut().unwrap().remove("calibration_ref");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn derived_primary_outcome_requires_calibration_reference() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][2].as_object_mut().unwrap().remove("calibration_ref");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn missing_objective_outcome_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"]
            .as_array_mut()
            .unwrap()
            .retain(|outcome| outcome["metric_id"] != "cutter_wear_intensity");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn outcome_unit_mismatch_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][0]["unit"] = json!("km/h");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn reversed_interval_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][0]["lower"] = json!(12.0);
        data["strategies"][0]["outcomes"][0]["upper"] = json!(8.0);
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn hard_constraint_definitely_failed_is_ineligible() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][3]["lower"] = json!(0.06);
        data["strategies"][0]["outcomes"][3]["upper"] = json!(0.08);
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Ineligible);
    }

    #[test]
    fn hard_constraint_interval_crossing_is_indeterminate() {
        let mut data = fixture();
        data["strategies"][0]["outcomes"][3]["lower"] = json!(0.04);
        data["strategies"][0]["outcomes"][3]["upper"] = json!(0.06);
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Indeterminate);
    }

    #[test]
    fn overlapping_intervals_do_not_establish_dominance() {
        let mut data = make_two_strategy_case(9.0, 11.0, 8.0, 10.0);
        data["strategies"][1]["outcomes"][1]["lower"] = json!(12.0);
        data["strategies"][1]["outcomes"][1]["upper"] = json!(14.0);
        data["strategies"][1]["outcomes"][2]["lower"] = json!(1.4);
        data["strategies"][1]["outcomes"][2]["upper"] = json!(1.8);
        let report = evaluate_case(&data).unwrap();
        assert!(report.pareto_frontier_ids.contains(&"MECHANICAL-BASELINE".to_owned()));
        assert!(report.pareto_frontier_ids.contains(&"HPWJ-DISC-HYBRID".to_owned()));
    }

    #[test]
    fn robust_interval_separation_can_establish_dominance() {
        let mut data = make_two_strategy_case(10.5, 12.0, 8.0, 10.0);
        data["strategies"][1]["outcomes"][1]["lower"] = json!(8.0);
        data["strategies"][1]["outcomes"][1]["upper"] = json!(10.0);
        data["strategies"][1]["outcomes"][2]["lower"] = json!(1.0);
        data["strategies"][1]["outcomes"][2]["upper"] = json!(1.2);
        let report = evaluate_case(&data).unwrap();
        assert_eq!(report.robustly_dominated_ids, vec!["MECHANICAL-BASELINE"]);
    }

    #[test]
    fn tradeoff_preserves_both_strategies_on_frontier() {
        let mut data = make_two_strategy_case(10.5, 12.0, 8.0, 10.0);
        data["strategies"][1]["outcomes"][1]["lower"] = json!(14.0);
        data["strategies"][1]["outcomes"][1]["upper"] = json!(16.0);
        data["strategies"][1]["outcomes"][2]["lower"] = json!(1.8);
        data["strategies"][1]["outcomes"][2]["upper"] = json!(2.2);
        let report = evaluate_case(&data).unwrap();
        assert_eq!(report.pareto_frontier_ids.len(), 2);
        assert!(report.robustly_dominated_ids.is_empty());
    }

    #[test]
    fn unsupported_strategy_does_not_dominate_eligible_strategy() {
        let mut data = fixture();
        data["strategies"][3]["geology_support"] = json!("UNSUPPORTED");
        data["strategies"][3]["outcomes"][0]["lower"] = json!(100.0);
        data["strategies"][3]["outcomes"][0]["upper"] = json!(100.0);
        let report = evaluate_case(&data).unwrap();
        assert!(!report.pareto_frontier_ids.contains(&"ATCP-HIGH-TEMP".to_owned()));
        assert_eq!(assessment(&report, "ATCP-HIGH-TEMP").eligibility, Eligibility::Ineligible);
    }

    #[test]
    fn eligibility_and_frontier_output_order_is_deterministic() {
        let data = fixture();
        let first = evaluate_case(&data).unwrap();
        let second = evaluate_case(&data).unwrap();
        assert_eq!(first, second);
        let mut ids = first.eligible_strategy_ids.clone();
        let original = ids.clone();
        ids.sort();
        assert_eq!(ids, original);
    }

    #[test]
    fn duplicate_strategy_identity_is_a_case_error() {
        let mut data = fixture();
        let duplicate_id = data["strategies"][0]["strategy_id"].clone();
        data["strategies"][1]["strategy_id"] = duplicate_id;
        let result = evaluate_case(&data);
        assert!(result.is_err());
    }

    #[test]
    fn wrong_schema_version_is_a_case_error() {
        let mut data = fixture();
        data["schema_version"] = json!(7);
        assert!(evaluate_case(&data).is_err());
    }

    #[test]
    fn no_assist_strategy_type_is_eligible_when_evidence_supports_it() {
        let mut data = fixture();
        data["strategies"][0]["strategy_type"] = json!("NO_ASSIST");
        let report = evaluate_case(&data).unwrap();
        assert_eq!(assessment(&report, "MECHANICAL-BASELINE").eligibility, Eligibility::Eligible);
    }

    #[test]
    fn experiment_options_are_retained_but_voi_is_not_fabricated() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(report.experiment_option_count, 2);
        assert!(!report.value_of_information_computed);
    }

    #[test]
    fn json_entrypoint_returns_parse_errors() {
        assert!(evaluate_case_json("{not-json").is_err());
    }
}
