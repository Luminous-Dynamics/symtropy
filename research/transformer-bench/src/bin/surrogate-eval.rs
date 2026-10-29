//! Evaluation-only metrics for transformer core-loss surrogate experiments.
//! Synthetic fixtures validate metric arithmetic; they are not FE or physical evidence.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 8] = [
    "candidate_id",
    "geometry_family",
    "material_id",
    "reference_core_loss_w",
    "predicted_core_loss_w",
    "prediction_lower_w",
    "prediction_upper_w",
    "model_fea_queries",
];

#[derive(Clone, Debug)]
struct Observation {
    candidate_id: String,
    geometry_family: String,
    material_id: String,
    reference: f64,
    predicted: f64,
    lower: f64,
    upper: f64,
}

#[derive(Clone, Debug)]
struct Summary {
    scope_type: String,
    scope_id: String,
    n: usize,
    model_fea_queries: u64,
    candidate_universe_size: u64,
    full_pool_fea_queries: u64,
    mean_relative_error_pct: f64,
    median_relative_error_pct: f64,
    p95_relative_error_pct: f64,
    max_relative_error_pct: f64,
    spearman_rank_correlation: Option<f64>,
    top_k: usize,
    top_k_recall_pct: f64,
    best_candidate_regret_pct: f64,
    prediction_interval_coverage_pct: f64,
    mean_interval_width_pct: f64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("surrogate-eval: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 || args.len() > 6 {
        return Err(format!(
            "usage: {} <predictions.csv> <report.csv> <candidate_universe_size> <full_pool_fea_queries> [top_k=8]",
            args.first().map(String::as_str).unwrap_or("surrogate-eval")
        ));
    }
    let universe: u64 = args[3]
        .parse()
        .map_err(|_| "candidate_universe_size must be a positive integer".to_string())?;
    if universe == 0 {
        return Err("candidate_universe_size must be positive".to_string());
    }
    let full_pool_fea_queries: u64 = args[4]
        .parse()
        .map_err(|_| "full_pool_fea_queries must be a positive integer".to_string())?;
    if full_pool_fea_queries == 0 {
        return Err("full_pool_fea_queries must be positive".to_string());
    }
    let requested_k: usize = if args.len() == 6 {
        args[5].parse().map_err(|_| "top_k must be a positive integer".to_string())?
    } else {
        8
    };
    if requested_k == 0 {
        return Err("top_k must be positive".to_string());
    }

    let (observations, model_fea_queries) = read_observations(&args[1])?;
    let mut summaries = Vec::new();
    summaries.push(summarize(
        "overall",
        "all",
        &observations,
        model_fea_queries,
        universe,
        full_pool_fea_queries,
        requested_k,
    )?);

    let families: BTreeSet<String> =
        observations.iter().map(|row| row.geometry_family.clone()).collect();
    for family in families {
        let subset: Vec<Observation> = observations
            .iter()
            .filter(|row| row.geometry_family == family)
            .cloned()
            .collect();
        summaries.push(summarize(
            "geometry_family",
            &family,
            &subset,
            model_fea_queries,
            universe,
            full_pool_fea_queries,
            requested_k,
        )?);
    }

    let materials: BTreeSet<String> =
        observations.iter().map(|row| row.material_id.clone()).collect();
    for material in materials {
        let subset: Vec<Observation> = observations
            .iter()
            .filter(|row| row.material_id == material)
            .cloned()
            .collect();
        summaries.push(summarize(
            "material",
            &material,
            &subset,
            model_fea_queries,
            universe,
            requested_k,
        )?);
    }

    write_report(&args[2], &summaries)?;
    eprintln!(
        "surrogate report written: candidates={}, groups={}, model_fea_queries={}, candidate_universe={}, full_pool_fea_queries={}; this report is only as valid as its frozen reference dataset and split",
        observations.len(), summaries.len(), model_fea_queries, universe, full_pool_fea_queries
    );
    Ok(())
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{context}: empty value is not allowed"))
    } else {
        Ok(value.to_string())
    }
}

fn finite_number(value: &str, context: &str) -> Result<f64, String> {
    let number: f64 = value
        .trim()
        .parse()
        .map_err(|_| format!("{context}: expected a finite number, got {value:?}"))?;
    if !number.is_finite() {
        return Err(format!("{context}: non-finite values are not allowed"));
    }
    Ok(number)
}

/// This evaluation CSV intentionally disallows quoted/comma-containing fields.
/// IDs are machine identifiers; rejecting quotes is safer than misparsing them.
fn read_observations(path: &str) -> Result<(Vec<Observation>, u64), String> {
    let content =
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let mut lines = content.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{path}: missing header"))?
        .trim_end_matches('\r');
    let header_columns: Vec<&str> = header.split(',').collect();
    if header_columns.as_slice() != HEADER.as_slice() {
        return Err(format!(
            "{path}: expected header [{}], got [{}]",
            HEADER.join(","),
            header_columns.join(",")
        ));
    }

    let mut observations = Vec::new();
    let mut ids = BTreeSet::new();
    let mut declared_queries: Option<u64> = None;
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{path}:{line_number}: blank rows are not allowed"));
        }
        if line.contains('"') {
            return Err(format!("{path}:{line_number}: quoted fields are not supported"));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != HEADER.len() {
            return Err(format!(
                "{path}:{line_number}: expected {} fields, found {}",
                HEADER.len(),
                fields.len()
            ));
        }
        let candidate_id = nonempty(fields[0], "candidate_id")?;
        if !ids.insert(candidate_id.clone()) {
            return Err(format!("{path}:{line_number}: duplicate candidate_id {candidate_id:?}"));
        }
        let geometry_family = nonempty(fields[1], "geometry_family")?;
        let material_id = nonempty(fields[2], "material_id")?;
        let reference = finite_number(fields[3], "reference_core_loss_w")?;
        let predicted = finite_number(fields[4], "predicted_core_loss_w")?;
        let lower = finite_number(fields[5], "prediction_lower_w")?;
        let upper = finite_number(fields[6], "prediction_upper_w")?;
        let calls: u64 = fields[7]
            .trim()
            .parse()
            .map_err(|_| format!("{path}:{line_number}: model_fea_queries must be an integer"))?;

        if reference <= 0.0 {
            return Err(format!("{path}:{line_number}: reference_core_loss_w must be positive"));
        }
        if predicted < 0.0 || lower < 0.0 || upper < 0.0 {
            return Err(format!("{path}:{line_number}: predicted loss and interval bounds must be non-negative"));
        }
        if lower > predicted || predicted > upper {
            return Err(format!("{path}:{line_number}: predicted_core_loss_w must lie inside its prediction interval"));
        }
        match declared_queries {
            None => declared_queries = Some(calls),
            Some(previous) if previous != calls => {
                return Err(format!("{path}:{line_number}: model_fea_queries must be constant across rows"));
            }
            _ => {}
        }
        observations.push(Observation {
            candidate_id,
            geometry_family,
            material_id,
            reference,
            predicted,
            lower,
            upper,
        });
    }
    if observations.is_empty() {
        return Err(format!("{path}: at least one observation is required"));
    }
    Ok((observations, declared_queries.unwrap_or(0)))
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn quantile_nearest_rank(sorted: &[f64], q: f64) -> f64 {
    let rank = ((q * sorted.len() as f64).ceil() as usize).max(1);
    sorted[rank.min(sorted.len()) - 1]
}

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

fn rank(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut ranks = vec![0.0; values.len()];
    let mut begin = 0;
    while begin < order.len() {
        let mut end = begin + 1;
        while end < order.len() && values[order[end]] == values[order[begin]] {
            end += 1;
        }
        // One-based average rank for an exact tie.
        let average_rank = ((begin + 1) as f64 + end as f64) / 2.0;
        for position in begin..end {
            ranks[order[position]] = average_rank;
        }
        begin = end;
    }
    ranks
}

fn spearman(reference: &[f64], predicted: &[f64]) -> Option<f64> {
    if reference.len() < 2 || reference.len() != predicted.len() {
        return None;
    }
    let x = rank(reference);
    let y = rank(predicted);
    let x_mean = mean(&x);
    let y_mean = mean(&y);
    let mut covariance = 0.0;
    let mut x_variance = 0.0;
    let mut y_variance = 0.0;
    for i in 0..x.len() {
        let dx = x[i] - x_mean;
        let dy = y[i] - y_mean;
        covariance += dx * dy;
        x_variance += dx * dx;
        y_variance += dy * dy;
    }
    if x_variance <= 0.0 || y_variance <= 0.0 {
        None
    } else {
        Some(covariance / (x_variance * y_variance).sqrt())
    }
}

fn ordered_indices(observations: &[Observation], use_prediction: bool) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..observations.len()).collect();
    indices.sort_by(|&a, &b| {
        let left = if use_prediction {
            observations[a].predicted
        } else {
            observations[a].reference
        };
        let right = if use_prediction {
            observations[b].predicted
        } else {
            observations[b].reference
        };
        left.total_cmp(&right)
            .then_with(|| observations[a].candidate_id.cmp(&observations[b].candidate_id))
    });
    indices
}

fn summarize(
    scope_type: &str,
    scope_id: &str,
    observations: &[Observation],
    model_fea_queries: u64,
    candidate_universe_size: u64,
    full_pool_fea_queries: u64,
    requested_k: usize,
) -> Result<Summary, String> {
    if observations.is_empty() {
        return Err(format!("scope {scope_type}={scope_id:?} is empty"));
    }
    let n = observations.len();
    let errors: Vec<f64> = observations
        .iter()
        .map(|row| (row.predicted - row.reference).abs() / row.reference * 100.0)
        .collect();
    if errors.iter().any(|value| !value.is_finite()) {
        return Err(format!("scope {scope_type}={scope_id:?}: derived relative error is non-finite"));
    }
    let mut sorted_errors = errors.clone();
    sorted_errors.sort_by(f64::total_cmp);

    let reference: Vec<f64> = observations.iter().map(|row| row.reference).collect();
    let predicted: Vec<f64> = observations.iter().map(|row| row.predicted).collect();
    let covered = observations
        .iter()
        .filter(|row| row.lower <= row.reference && row.reference <= row.upper)
        .count();
    let relative_widths: Vec<f64> = observations
        .iter()
        .map(|row| (row.upper - row.lower) / row.reference * 100.0)
        .collect();
    if relative_widths.iter().any(|value| !value.is_finite()) {
        return Err(format!("scope {scope_type}={scope_id:?}: derived interval width is non-finite"));
    }

    let k = requested_k.min(n);
    let true_order = ordered_indices(observations, false);
    let predicted_order = ordered_indices(observations, true);
    let true_top: BTreeSet<usize> = true_order.iter().take(k).copied().collect();
    let predicted_top: BTreeSet<usize> = predicted_order.iter().take(k).copied().collect();
    let overlap = true_top.intersection(&predicted_top).count();
    let true_best = observations[true_order[0]].reference;
    let predicted_best_true_loss = observations[predicted_order[0]].reference;
    let regret_pct = (predicted_best_true_loss - true_best).max(0.0) / true_best * 100.0;
    Ok(Summary {
        scope_type: scope_type.to_string(),
        scope_id: scope_id.to_string(),
        n,
        model_fea_queries,
        candidate_universe_size,
        full_pool_fea_queries,
        mean_relative_error_pct: mean(&errors),
        median_relative_error_pct: median(&sorted_errors),
        p95_relative_error_pct: quantile_nearest_rank(&sorted_errors, 0.95),
        max_relative_error_pct: sorted_errors[n - 1],
        spearman_rank_correlation: spearman(&reference, &predicted),
        top_k: k,
        top_k_recall_pct: overlap as f64 / k as f64 * 100.0,
        best_candidate_regret_pct: regret_pct,
        prediction_interval_coverage_pct: covered as f64 / n as f64 * 100.0,
        mean_interval_width_pct: mean(&relative_widths),
    })
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn write_report(path: &str, summaries: &[Summary]) -> Result<(), String> {
    let mut output = String::from(
        "scope_type,scope_id,n,model_fea_queries,candidate_universe_size,full_pool_fea_queries,fea_query_reduction_pct,mean_relative_error_pct,median_relative_error_pct,p95_relative_error_pct,max_relative_error_pct,spearman_rank_correlation,top_k,top_k_recall_pct,best_candidate_regret_pct,prediction_interval_coverage_pct,mean_interval_width_pct\n",
    );
    for row in summaries {
        let spearman = row
            .spearman_rank_correlation
            .map(|value| format!("{value:.8}"))
            .unwrap_or_default();
        let reduction =
            (1.0 - row.model_fea_queries as f64 / row.full_pool_fea_queries as f64) * 100.0;
        output.push_str(&format!(
            "{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{:.6},{:.6},{:.6},{:.6}\n",
            row.scope_type,
            csv_escape(&row.scope_id),
            row.n,
            row.model_fea_queries,
            row.candidate_universe_size,
            row.full_pool_fea_queries,
            reduction,
            row.mean_relative_error_pct,
            row.median_relative_error_pct,
            row.p95_relative_error_pct,
            row.max_relative_error_pct,
            spearman,
            row.top_k,
            row.top_k_recall_pct,
            row.best_candidate_regret_pct,
            row.prediction_interval_coverage_pct,
            row.mean_interval_width_pct
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(id: &str, reference: f64, predicted: f64, lower: f64, upper: f64) -> Observation {
        Observation {
            candidate_id: id.to_string(),
            geometry_family: "g1".to_string(),
            material_id: "m1".to_string(),
            reference,
            predicted,
            lower,
            upper,
        }
    }

    #[test]
    fn exact_predictions_have_zero_error_zero_regret_and_perfect_rank() {
        let rows = vec![
            obs("a", 10.0, 10.0, 9.0, 11.0),
            obs("b", 20.0, 20.0, 19.0, 21.0),
            obs("c", 30.0, 30.0, 29.0, 31.0),
        ];
        let s = summarize("overall", "all", &rows, 8, 3000, 3000, 2).unwrap();
        assert!(s.mean_relative_error_pct.abs() < 1.0e-12);
        assert!(s.best_candidate_regret_pct.abs() < 1.0e-12);
        assert!((s.spearman_rank_correlation.unwrap() - 1.0).abs() < 1.0e-12);
        assert!((s.top_k_recall_pct - 100.0).abs() < 1.0e-12);
        assert!((s.prediction_interval_coverage_pct - 100.0).abs() < 1.0e-12);
    }

    #[test]
    fn ranking_metrics_detect_wrong_selected_candidate_even_with_small_error() {
        let rows = vec![
            obs("a", 10.0, 10.05, 10.0, 10.4),
            obs("b", 10.1, 10.0, 9.8, 10.2),
            obs("c", 40.0, 40.1, 39.0, 41.0),
        ];
        let s = summarize("overall", "all", &rows, 2, 100, 100, 1).unwrap();
        assert!(s.mean_relative_error_pct < 1.0);
        assert!(s.best_candidate_regret_pct > 0.0);
        assert_eq!(s.top_k_recall_pct, 0.0);
    }

    #[test]
    fn query_reduction_is_computed_from_declared_model_calls() {
        let rows = vec![obs("a", 10.0, 10.0, 9.0, 11.0), obs("b", 20.0, 20.0, 19.0, 21.0)];
        let s = summarize("overall", "all", &rows, 8, 3000, 3000, 1).unwrap();
        assert!(((1.0 - s.model_fea_queries as f64 / s.full_pool_fea_queries as f64) * 100.0 - 99.7333333333).abs() < 1.0e-8);
    }

    #[test]
    fn tied_ranks_use_average_rank_and_constant_ranks_are_undefined() {
        assert_eq!(rank(&[1.0, 1.0, 3.0]), vec![1.5, 1.5, 3.0]);
        assert!(spearman(&[2.0, 2.0], &[1.0, 2.0]).is_none());
    }

    #[test]
    fn rejects_reference_zero_bad_interval_and_inconsistent_query_count() {
        let path = env::temp_dir().join("transformer-surrogate-eval-test.csv");
        let path = path.to_string_lossy().into_owned();
        let header = HEADER.join(",");
        fs::write(&path, format!("{header}\na,g1,m1,0,2,1,3,2\n")).unwrap();
        assert!(read_observations(&path).is_err());
        fs::write(&path, format!("{header}\na,g1,m1,10,2,3,1,2\n")).unwrap();
        assert!(read_observations(&path).is_err());
        fs::write(
            &path,
            format!("{header}\na,g1,m1,10,10,9,11,2\nb,g1,m1,20,20,19,21,3\n"),
        )
        .unwrap();
        assert!(read_observations(&path).is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn nearest_rank_p95_uses_upper_empirical_rank() {
        let mut x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        x.sort_by(f64::total_cmp);
        assert_eq!(quantile_nearest_rank(&x, 0.95), 5.0);
    }
}
