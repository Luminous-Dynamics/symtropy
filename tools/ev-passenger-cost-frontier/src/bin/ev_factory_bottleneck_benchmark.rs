use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const DEFAULT_FIXTURE: &str =
    "tools/ev-passenger-cost-frontier/fixtures/ev_bottleneck_diagnostic_benchmark_v0_1.json";
const CLAIM_CEILING: &str = "Synthetic, hand-authored method-scoring harness only. Results do not measure a real plant, reproduce a published diagnostic algorithm, prove SOTA, calibrate a digital twin, or authorize capital spending.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    benchmark_id: String,
    evidence_class: EvidenceClass,
    scenarios: Vec<Scenario>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    operating_regime: String,
    ground_truth_constraint_id: String,
    labels_source: String,
    signals: Vec<StationSignals>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct StationSignals {
    station_id: String,
    utilization_fraction: f64,
    active_period_fraction: f64,
    blocked_fraction: f64,
    starved_fraction: f64,
    down_fraction: f64,
    interdeparture_cv: f64,
    predicted_marginal_good_output_gain_fraction: f64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Method {
    Utilization,
    ActivePeriod,
    InterdepartureCv,
    FlowPressure,
    PredictedMarginalGain,
}

impl Method {
    const ALL: [Self; 5] = [
        Self::Utilization,
        Self::ActivePeriod,
        Self::InterdepartureCv,
        Self::FlowPressure,
        Self::PredictedMarginalGain,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Utilization => "utilization_max",
            Self::ActivePeriod => "active_period_max",
            Self::InterdepartureCv => "interdeparture_cv_max_heuristic",
            Self::FlowPressure => "flow_pressure_heuristic",
            Self::PredictedMarginalGain => "predicted_marginal_gain_max",
        }
    }

    fn score(self, row: &StationSignals) -> f64 {
        match self {
            Self::Utilization => row.utilization_fraction,
            Self::ActivePeriod => row.active_period_fraction,
            Self::InterdepartureCv => row.interdeparture_cv,
            Self::FlowPressure => {
                row.utilization_fraction + 0.5 * row.blocked_fraction
                    - row.starved_fraction
                    - row.down_fraction
            }
            Self::PredictedMarginalGain => row.predicted_marginal_good_output_gain_fraction,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Prediction {
    method: String,
    predicted_constraint_id: Option<String>,
    correct: bool,
    abstained_on_tie: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ScenarioResult {
    scenario_id: String,
    operating_regime: String,
    ground_truth_constraint_id: String,
    predictions: Vec<Prediction>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ClassMetrics {
    station_id: String,
    true_positives: u64,
    false_positives: u64,
    false_negatives: u64,
    precision_fraction: f64,
    recall_fraction: f64,
    f1_fraction: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct MethodMetrics {
    method: String,
    case_count: usize,
    covered_cases: usize,
    abstentions: usize,
    exact_accuracy_fraction: f64,
    macro_f1_fraction: f64,
    per_class: Vec<ClassMetrics>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    evidence_class: EvidenceClass,
    result_count: usize,
    method_count: usize,
    results: Vec<ScenarioResult>,
    method_metrics: Vec<MethodMetrics>,
    scores_are_field_performance_claims: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_FIXTURE.to_string());
    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let catalog: Catalog = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid bottleneck benchmark catalog: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("invalid bottleneck benchmark catalog: {error}");
        process::exit(2);
    }
    let report = evaluate_catalog(&catalog);
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize bottleneck benchmark report: {error}");
            process::exit(2);
        }
    }
}

fn valid_fraction(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name}_must_be_finite_and_in_range_0_to_1"));
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0"
        || catalog.benchmark_id != "EV-FACTORY-BENCH-001"
        || catalog.claim_ceiling.trim().is_empty()
        || catalog.scenarios.is_empty()
    {
        return Err("invalid_header_or_empty_scenario_catalog".into());
    }
    if catalog.evidence_class != EvidenceClass::Synthetic {
        return Err("v0_1_hand_authored_fixture_requires_synthetic_evidence_class".into());
    }
    let mut scenario_ids = BTreeSet::new();
    for scenario in &catalog.scenarios {
        if scenario.scenario_id.trim().is_empty()
            || scenario.operating_regime.trim().is_empty()
            || scenario.ground_truth_constraint_id.trim().is_empty()
            || scenario.labels_source.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.clone())
        {
            return Err("scenario_metadata_must_be_nonempty_and_ids_unique".into());
        }
        if scenario.signals.len() < 2 {
            return Err(format!(
                "scenario_{}_requires_at_least_two_candidates",
                scenario.scenario_id
            ));
        }
        let mut station_ids = BTreeSet::new();
        for row in &scenario.signals {
            if row.station_id.trim().is_empty() || !station_ids.insert(row.station_id.clone()) {
                return Err(format!(
                    "scenario_{}_station_ids_must_be_unique",
                    scenario.scenario_id
                ));
            }
            valid_fraction("utilization_fraction", row.utilization_fraction)?;
            valid_fraction("active_period_fraction", row.active_period_fraction)?;
            valid_fraction("blocked_fraction", row.blocked_fraction)?;
            valid_fraction("starved_fraction", row.starved_fraction)?;
            valid_fraction("down_fraction", row.down_fraction)?;
            valid_fraction(
                "predicted_marginal_good_output_gain_fraction",
                row.predicted_marginal_good_output_gain_fraction,
            )?;
            if !row.interdeparture_cv.is_finite() || row.interdeparture_cv < 0.0 {
                return Err("interdeparture_cv_must_be_finite_and_nonnegative".into());
            }
        }
        if !station_ids.contains(&scenario.ground_truth_constraint_id) {
            return Err(format!(
                "scenario_{}_ground_truth_not_in_candidates",
                scenario.scenario_id
            ));
        }
    }
    Ok(())
}

fn predict(scenario: &Scenario, method: Method) -> Prediction {
    let mut best_score = f64::NEG_INFINITY;
    let mut best_id: Option<&str> = None;
    let mut tied = false;
    for row in &scenario.signals {
        let score = method.score(row);
        if score > best_score + 1e-9 {
            best_score = score;
            best_id = Some(&row.station_id);
            tied = false;
        } else if (score - best_score).abs() <= 1e-9 {
            tied = true;
        }
    }
    let winner = if tied {
        None
    } else {
        best_id.map(str::to_string)
    };
    Prediction {
        method: method.name().to_string(),
        correct: winner.as_deref() == Some(scenario.ground_truth_constraint_id.as_str()),
        abstained_on_tie: winner.is_none(),
        predicted_constraint_id: winner,
    }
}

fn ratio(n: u64, d: u64) -> f64 {
    if d == 0 { 0.0 } else { n as f64 / d as f64 }
}

fn evaluate_method(
    catalog: &Catalog,
    station_ids: &[String],
    results: &[ScenarioResult],
    method: Method,
) -> MethodMetrics {
    let mut tp = BTreeMap::<String, u64>::new();
    let mut fp = BTreeMap::<String, u64>::new();
    let mut fn_ = BTreeMap::<String, u64>::new();
    let mut covered = 0_usize;
    let mut correct = 0_u64;
    for scenario_result in results {
        let predicted = scenario_result
            .predictions
            .iter()
            .find(|prediction| prediction.method == method.name())
            .and_then(|prediction| prediction.predicted_constraint_id.as_ref());
        if predicted.is_some() {
            covered += 1;
        }
        if predicted.is_some_and(|value| value == &scenario_result.ground_truth_constraint_id) {
            correct += 1;
        }
        if let Some(value) = predicted {
            if value == &scenario_result.ground_truth_constraint_id {
                *tp.entry(value.clone()).or_default() += 1;
            } else {
                *fp.entry(value.clone()).or_default() += 1;
                *fn_.entry(scenario_result.ground_truth_constraint_id.clone())
                    .or_default() += 1;
            }
        } else {
            *fn_.entry(scenario_result.ground_truth_constraint_id.clone())
                .or_default() += 1;
        }
    }

    let per_class: Vec<ClassMetrics> = station_ids
        .iter()
        .map(|station_id| {
            let true_positives = *tp.get(station_id).unwrap_or(&0);
            let false_positives = *fp.get(station_id).unwrap_or(&0);
            let false_negatives = *fn_.get(station_id).unwrap_or(&0);
            let precision = ratio(true_positives, true_positives + false_positives);
            let recall = ratio(true_positives, true_positives + false_negatives);
            let f1 = if precision + recall > 0.0 {
                2.0 * precision * recall / (precision + recall)
            } else {
                0.0
            };
            ClassMetrics {
                station_id: station_id.clone(),
                true_positives,
                false_positives,
                false_negatives,
                precision_fraction: precision,
                recall_fraction: recall,
                f1_fraction: f1,
            }
        })
        .collect();
    let macro_f1 = if per_class.is_empty() {
        0.0
    } else {
        per_class.iter().map(|item| item.f1_fraction).sum::<f64>() / per_class.len() as f64
    };
    MethodMetrics {
        method: method.name().to_string(),
        case_count: catalog.scenarios.len(),
        covered_cases: covered,
        abstentions: catalog.scenarios.len() - covered,
        exact_accuracy_fraction: ratio(correct, catalog.scenarios.len() as u64),
        macro_f1_fraction: macro_f1,
        per_class,
    }
}

fn evaluate_catalog(catalog: &Catalog) -> Report {
    let results: Vec<ScenarioResult> = catalog
        .scenarios
        .iter()
        .map(|scenario| ScenarioResult {
            scenario_id: scenario.scenario_id.clone(),
            operating_regime: scenario.operating_regime.clone(),
            ground_truth_constraint_id: scenario.ground_truth_constraint_id.clone(),
            predictions: Method::ALL
                .iter()
                .copied()
                .map(|method| predict(scenario, method))
                .collect(),
        })
        .collect();
    let station_ids: Vec<String> = catalog
        .scenarios
        .iter()
        .flat_map(|scenario| scenario.signals.iter().map(|row| row.station_id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let method_metrics = Method::ALL
        .iter()
        .copied()
        .map(|method| evaluate_method(catalog, &station_ids, &results, method))
        .collect();
    Report {
        schema_version: catalog.schema_version.clone(),
        benchmark_id: catalog.benchmark_id.clone(),
        evidence_class: catalog.evidence_class,
        result_count: results.len(),
        method_count: Method::ALL.len(),
        results,
        method_metrics,
        scores_are_field_performance_claims: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Catalog {
        serde_json::from_str(include_str!(
            "../../fixtures/ev_bottleneck_diagnostic_benchmark_v0_1.json"
        ))
        .expect("fixture parses")
    }

    #[test]
    fn synthetic_benchmark_fixture_is_well_formed() {
        validate_catalog(&fixture()).expect("fixture valid");
        assert_eq!(fixture().scenarios.len(), 8);
    }

    #[test]
    fn report_is_deterministic_and_never_claims_field_sota() {
        let catalog = fixture();
        let a = serde_json::to_vec(&evaluate_catalog(&catalog)).unwrap();
        let b = serde_json::to_vec(&evaluate_catalog(&catalog)).unwrap();
        assert_eq!(a, b);
        let report = evaluate_catalog(&catalog);
        assert!(!report.scores_are_field_performance_claims);
        assert!(!report.production_release_authorized);
        assert_eq!(report.method_count, 5);
    }

    #[test]
    fn known_counterexample_defeats_utilization_only() {
        let catalog = fixture();
        let scenario = &catalog.scenarios[0];
        let utilization = predict(scenario, Method::Utilization);
        let gain = predict(scenario, Method::PredictedMarginalGain);
        assert_ne!(
            utilization.predicted_constraint_id,
            Some(scenario.ground_truth_constraint_id.clone())
        );
        assert_eq!(
            gain.predicted_constraint_id,
            Some(scenario.ground_truth_constraint_id.clone())
        );
    }

    #[test]
    fn ties_abstain_instead_of_using_incidental_station_order() {
        let mut scenario = fixture().scenarios.remove(0);
        for station in &mut scenario.signals {
            station.utilization_fraction = 0.8;
        }
        let prediction = predict(&scenario, Method::Utilization);
        assert!(prediction.abstained_on_tie);
        assert!(prediction.predicted_constraint_id.is_none());
    }

    #[test]
    fn invalid_fractions_and_duplicate_scenario_ids_fail_closed() {
        let mut catalog = fixture();
        catalog.scenarios[0].signals[0].blocked_fraction = 1.1;
        assert!(validate_catalog(&catalog).is_err());

        let mut catalog = fixture();
        catalog.scenarios[1].scenario_id = catalog.scenarios[0].scenario_id.clone();
        assert!(validate_catalog(&catalog).is_err());
    }
}
