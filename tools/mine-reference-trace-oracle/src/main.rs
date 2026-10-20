use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use std::{env, fs, process::ExitCode};

const EXPECTED_REPOSITORY: &str = "creator-xi/Conformal-Optimistic-Prediction";
const EXPECTED_COMMIT: &str = "a41c0dcadc10979d8521ab9b5b8f3d460807d883";
const COP_SCALE: f64 = 0.5;

type Check<T> = Result<T, String>;

#[derive(Debug, Clone, Deserialize)]
struct Inputs {
    schema_version: u32,
    reference_repository: String,
    reference_commit: String,
    cases: Vec<Case>,
}

#[derive(Debug, Clone, Deserialize)]
struct Case {
    name: String,
    method: String,
    scores: Vec<f64>,
    alpha: f64,
    learning_rate: f64,
    #[serde(default)]
    window_length: usize,
    #[serde(default)]
    burnin: usize,
    ahead: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Trace {
    name: String,
    method: String,
    quantiles: Vec<Value>,
    coverage: Vec<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    adaptive_alpha: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize)]
struct Output {
    schema_version: u32,
    reference_repository: String,
    reference_commit: String,
    implementation: String,
    cases: Vec<Trace>,
}

fn validate_inputs(inputs: &Inputs) -> Check<()> {
    if inputs.schema_version != 1 {
        return Err("unsupported trace-input schema version".to_owned());
    }
    if inputs.reference_repository != EXPECTED_REPOSITORY
        || inputs.reference_commit != EXPECTED_COMMIT
    {
        return Err("reference repository/commit does not match the pinned oracle".to_owned());
    }
    if inputs.cases.is_empty() {
        return Err("trace input must contain at least one case".to_owned());
    }
    for case in &inputs.cases {
        if case.name.trim().is_empty() || case.name.trim() != case.name {
            return Err("case names must be non-empty and unpadded".to_owned());
        }
        if case.scores.is_empty() || case.scores.iter().any(|score| !score.is_finite()) {
            return Err(format!("{}: scores must be finite and non-empty", case.name));
        }
        if !(0.0..1.0).contains(&case.alpha) {
            return Err(format!("{}: alpha must be strictly between 0 and 1", case.name));
        }
        if !case.learning_rate.is_finite() || case.learning_rate < 0.0 {
            return Err(format!("{}: learning rate must be finite and nonnegative", case.name));
        }
        if case.ahead == 0 {
            return Err(format!("{}: ahead must be at least one", case.name));
        }
        if matches!(case.method.as_str(), "aci" | "aci_clipped") && case.window_length == 0 {
            return Err(format!("{}: ACI window length must be nonzero", case.name));
        }
        if case.method == "cop" && case.burnin == 0 {
            return Err(format!("{}: COP burn-in must be nonzero", case.name));
        }
        if !matches!(
            case.method.as_str(),
            "aci" | "aci_clipped" | "cop" | "ogd" | "sf_ogd" | "decay_ogd"
        ) {
            return Err(format!("{}: unsupported reference method {}", case.name, case.method));
        }
    }
    Ok(())
}

fn json_float(value: f64) -> Value {
    if value == f64::INFINITY {
        Value::String("Infinity".to_owned())
    } else if value == f64::NEG_INFINITY {
        Value::String("-Infinity".to_owned())
    } else if value.is_nan() {
        Value::String("NaN".to_owned())
    } else {
        Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

fn higher_quantile(values: &[f64], probability: f64) -> f64 {
    if values.is_empty() || !(0.0..=1.0).contains(&probability) {
        return f64::INFINITY;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let position = probability * (sorted.len() - 1) as f64;
    let index = position.ceil() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn linear_quantile(values: &[f64], probability: f64) -> f64 {
    if values.is_empty() || !(0.0..=1.0).contains(&probability) {
        return f64::INFINITY;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] + fraction * (sorted[upper] - sorted[lower])
    }
}

fn coverage_trace(quantiles: &[f64], scores: &[f64]) -> Vec<bool> {
    quantiles
        .iter()
        .zip(scores)
        .map(|(quantile, score)| quantile >= score)
        .collect()
}

fn run_aci(case: &Case, clipped: bool) -> Trace {
    let count = case.scores.len();
    let mut quantiles = vec![0.0_f64; count];
    let mut adaptive_alpha = vec![case.alpha; count];
    let mut covered = vec![false; count];
    let mut alpha_t = case.alpha;
    let burnin = case.burnin as isize;
    let window_length = case.window_length as isize;
    let ahead = case.ahead as isize;

    for t in 0..count {
        let prediction_index = t as isize - ahead + 1;
        if prediction_index < 0 {
            continue;
        }
        let pred = prediction_index as usize;
        let window_start = (prediction_index - window_length).max(0) as usize;
        let previous_scores = &case.scores[window_start..pred];

        if prediction_index > burnin {
            let q = if alpha_t <= 1.0 / (prediction_index + 1) as f64 {
                f64::INFINITY
            } else {
                higher_quantile(previous_scores, 1.0 - alpha_t.clamp(0.0, 1.0))
            };
            quantiles[t] = q;
            covered[t] = q >= case.scores[t];

            let gradient = if covered[pred] {
                -case.alpha
            } else {
                1.0 - case.alpha
            };
            alpha_t -= case.learning_rate * gradient;
            if t < count - 1 {
                adaptive_alpha[t + 1] = alpha_t;
            }
        } else {
            quantiles[t] = if prediction_index as f64 > (1.0 / case.alpha).ceil() {
                linear_quantile(&case.scores[..pred], 1.0 - case.alpha)
            } else {
                f64::INFINITY
            };
        }

        if clipped && quantiles[t] == f64::INFINITY {
            let clip_value = if prediction_index > 0 {
                previous_scores.iter().copied().reduce(f64::max).unwrap_or(f64::INFINITY)
            } else {
                f64::INFINITY
            };
            quantiles[t] = clip_value;
        }
    }

    Trace {
        name: case.name.clone(),
        method: case.method.clone(),
        quantiles: quantiles.iter().map(|value| json_float(*value)).collect(),
        coverage: coverage_trace(&quantiles, &case.scores),
        adaptive_alpha: if case.method == "aci" || case.method == "aci_clipped" {
            Some(adaptive_alpha)
        } else {
            None
        },
    }
}

fn run_cop(case: &Case) -> Trace {
    let count = case.scores.len();
    let mut quantiles = vec![0.0_f64; count];
    let mut base_quantiles = vec![0.0_f64; count];
    let mut integrators = vec![0.0_f64; count];
    let mut covered = vec![false; count];
    let ahead = case.ahead as isize;
    let burnin = case.burnin as isize;

    for t in 0..count {
        let recent_start = t.saturating_sub(case.burnin);
        let learning_rate = if t > 0 {
            let recent = &case.scores[recent_start..t];
            let max = recent.iter().copied().reduce(f64::max).unwrap_or(0.0);
            let min = recent.iter().copied().reduce(f64::min).unwrap_or(0.0);
            case.learning_rate * (max - min)
        } else {
            case.learning_rate
        };

        let prediction_index = t as isize - ahead + 1;
        if prediction_index < 0 {
            continue;
        }
        let pred = prediction_index as usize;

        covered[t] = quantiles[t] >= case.scores[t];
        let gradient = if covered[pred] {
            case.alpha
        } else {
            -(1.0 - case.alpha)
        };

        let distribution_gradient = if prediction_index < burnin {
            0.0
        } else {
            let start = (prediction_index - burnin) as usize;
            let window = &case.scores[start..pred];
            if window.is_empty() {
                return Trace {
                    name: case.name.clone(),
                    method: case.method.clone(),
                    quantiles: vec![json!("ERROR: empty COP ECDF window")],
                    coverage: Vec::new(),
                    adaptive_alpha: None,
                };
            }
            let threshold = base_quantiles[t] - learning_rate * gradient;
            let empirical_cdf =
                window.iter().filter(|score| **score <= threshold).count() as f64
                    / window.len() as f64;
            empirical_cdf - (1.0 - case.alpha)
        };

        let integrator = -COP_SCALE * distribution_gradient;
        if t < count - 1 {
            base_quantiles[t + 1] = base_quantiles[t] - learning_rate * gradient;
            integrators[t + 1] = learning_rate * integrator;
            quantiles[t + 1] = base_quantiles[t + 1] + integrators[t + 1];
        }
    }

    Trace {
        name: case.name.clone(),
        method: case.method.clone(),
        quantiles: quantiles.iter().map(|value| json_float(*value)).collect(),
        coverage: coverage_trace(&quantiles, &case.scores),
        adaptive_alpha: None,
    }
}

fn run_ogd_family(case: &Case) -> Trace {
    let count = case.scores.len();
    let mut quantiles = vec![0.0_f64; count];
    let mut covered = vec![false; count];
    let ahead = case.ahead as isize;

    for t in 0..count {
        let prediction_index = t as isize - ahead + 1;
        if prediction_index < 0 {
            continue;
        }
        let pred = prediction_index as usize;
        covered[t] = quantiles[t] >= case.scores[t];
        let gradient = if covered[pred] {
            case.alpha
        } else {
            -(1.0 - case.alpha)
        };

        let step_size = match case.method.as_str() {
            "ogd" => case.learning_rate,
            "sf_ogd" => {
                let gradient_squares = covered[..=t]
                    .iter()
                    .map(|is_covered| {
                        let value = case.alpha - (*is_covered as u8 as f64 - 1.0);
                        value * value
                    })
                    .sum::<f64>();
                case.learning_rate / gradient_squares.sqrt()
            }
            "decay_ogd" => {
                if t == 0 {
                    case.learning_rate
                } else {
                    case.learning_rate * (t as f64).powf(-0.5 - 0.1)
                }
            }
            _ => unreachable!("validated method"),
        };

        if t < count - 1 {
            quantiles[t + 1] = quantiles[t] - step_size * gradient;
        }
    }

    Trace {
        name: case.name.clone(),
        method: case.method.clone(),
        quantiles: quantiles.iter().map(|value| json_float(*value)).collect(),
        coverage: coverage_trace(&quantiles, &case.scores),
        adaptive_alpha: None,
    }
}

fn run_trace(case: &Case) -> Trace {
    match case.method.as_str() {
        "aci" => run_aci(case, false),
        "aci_clipped" => run_aci(case, true),
        "cop" => run_cop(case),
        "ogd" | "sf_ogd" | "decay_ogd" => run_ogd_family(case),
        _ => unreachable!("validated method"),
    }
}

fn run() -> Check<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--inputs" {
        return Err("usage: mine-reference-trace-oracle --inputs <path>".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|error| format!("cannot read inputs: {error}"))?;
    let inputs: Inputs = serde_json::from_slice(&bytes)
        .map_err(|error| format!("cannot parse input JSON: {error}"))?;
    validate_inputs(&inputs)?;
    let cases = inputs.cases.iter().map(run_trace).collect();
    let output = Output {
        schema_version: 1,
        reference_repository: inputs.reference_repository,
        reference_commit: inputs.reference_commit,
        implementation: "rust-port-under-test".to_owned(),
        cases,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .map_err(|error| format!("cannot serialize traces: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numpy_higher_quantile_uses_rounded_up_interpolation_index() {
        assert_eq!(0.08, higher_quantile(&[0.04, 0.08], 0.725));
        assert_eq!(0.04, higher_quantile(&[0.04, 0.08], 0.0));
        assert_eq!(0.08, higher_quantile(&[0.04, 0.08], 1.0));
    }

    #[test]
    fn ogd_trace_matches_hand_computed_reference_equation() {
        let case = Case {
            name: "ogd".to_owned(),
            method: "ogd".to_owned(),
            scores: vec![0.0, 0.02, 0.04, 0.08, 0.06, 0.10],
            alpha: 0.25,
            learning_rate: 0.1,
            window_length: 0,
            burnin: 0,
            ahead: 1,
        };
        let trace = run_ogd_family(&case);
        let expected = [0.0, -0.025, 0.05, 0.025, 0.1, 0.075];
        let actual = trace
            .quantiles
            .iter()
            .map(|value| value.as_f64().expect("finite quantile"))
            .collect::<Vec<_>>();
        for (actual, expected) in actual.iter().zip(expected) {
            assert!((actual - expected).abs() <= 1e-12);
        }
    }

    #[test]
    fn input_gate_rejects_wrong_reference_commit() {
        let inputs = Inputs {
            schema_version: 1,
            reference_repository: EXPECTED_REPOSITORY.to_owned(),
            reference_commit: "not-pinned".to_owned(),
            cases: vec![Case {
                name: "bad".to_owned(),
                method: "ogd".to_owned(),
                scores: vec![0.0],
                alpha: 0.25,
                learning_rate: 0.1,
                window_length: 0,
                burnin: 0,
                ahead: 1,
            }],
        };
        assert!(validate_inputs(&inputs).is_err());
    }
}
