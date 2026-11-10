use std::env;
use std::fs;

use resource_normalized_driving_evaluator::{
    compare_metric, pareto_frontier, MetricDirection, PairComparison, ParetoFrontier, ResultCard,
};
use serde::Serialize;

#[derive(Serialize)]
struct EvaluationOutput {
    native_metric: String,
    direction: String,
    comparisons: Vec<PairComparison>,
    pareto_frontier: ParetoFrontier,
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 4 {
        return Err(
            "usage: capability-frontier <native-metric> <maximize|minimize> <baseline.json> <candidate.json> [additional.json ...]"
                .into(),
        );
    }

    let metric = &args[0];
    let direction = MetricDirection::parse(&args[1])?;
    let mut cards = Vec::new();

    for path in &args[2..] {
        let input = fs::read_to_string(path)
            .map_err(|error| format!("cannot read {path:?}: {error}"))?;
        let card = ResultCard::from_json(&input)
            .map_err(|error| format!("cannot parse {path:?}: {error}"))?;
        card.validate()
            .map_err(|errors| format!("invalid result card {path:?}: {}", errors.join("; ")))?;
        cards.push(card);
    }

    let baseline = &cards[0];
    let mut comparisons = Vec::new();
    for candidate in cards.iter().skip(1) {
        comparisons.push(compare_metric(
            baseline,
            candidate,
            metric,
            direction,
        )?);
    }

    let frontier = pareto_frontier(&cards, metric, direction)?;
    let output = EvaluationOutput {
        native_metric: metric.clone(),
        direction: match direction {
            MetricDirection::HigherIsBetter => "maximize".to_string(),
            MetricDirection::LowerIsBetter => "minimize".to_string(),
        },
        comparisons,
        pareto_frontier: frontier,
    };
    let rendered = serde_json::to_string_pretty(&output)
        .map_err(|error| format!("cannot serialize evaluation output: {error}"))?;
    println!("{rendered}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("capability-frontier: {error}");
        std::process::exit(2);
    }
}
