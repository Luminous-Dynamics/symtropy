//! Joint-scenario Pareto analysis for transformer candidates.
//! Scenario rows preserve aligned demand, weather, tariff and material conditions.
//! CVaR95 is the weighted upper-tail mean over the worst 5% probability mass.
//! This is finite-scenario screening, not a solver or field validation.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 14] = [
    "candidate_id", "profile_id", "scenario_id", "scenario_weight",
    "currency_code", "price_year", "price_basis", "carbon_boundary_id",
    "lifecycle_cost", "no_load_loss_w", "load_loss_w", "max_hotspot_temp_c",
    "mass_kg", "embodied_carbon_kgco2e",
];
const OBJECTIVE_COUNT: usize = 6;
const TAIL_MASS: f64 = 0.05;
const WEIGHT_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug)]
struct ScenarioValue {
    weight: f64,
    objectives: [f64; OBJECTIVE_COUNT],
}

#[derive(Clone, Debug)]
struct Candidate {
    id: String,
    scenarios: BTreeMap<String, ScenarioValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProfileMetadata {
    profile_id: String,
    currency_code: String,
    price_year: i32,
    price_basis: String,
    carbon_boundary_id: String,
}

#[derive(Clone, Copy, Debug)]
struct Aggregate {
    expected: [f64; OBJECTIVE_COUNT],
    cvar95: [f64; OBJECTIVE_COUNT],
    worst: [f64; OBJECTIVE_COUNT],
}

#[derive(Clone, Debug)]
struct FrontierRow {
    candidate_id: String,
    aggregate: Aggregate,
    expected_pareto: bool,
    cvar95_pareto: bool,
    worst_case_pareto: bool,
    scenario_robust_pareto: bool,
    scenario_robust_dominators: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("scenario-frontier: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <candidate-scenarios.csv> <frontier-report.csv>",
            args.first().map(String::as_str).unwrap_or("scenario-frontier")
        ));
    }
    let (metadata, candidates, scenario_weights) = read_candidates(&args[1])?;
    let rows = classify(&candidates)?;
    write_report(&args[2], &metadata, scenario_weights.len(), &rows)?;
    eprintln!(
        "joint-scenario Pareto report written: candidates={}, scenarios={}, expected-frontier={}, CVaR95-frontier={}, worst-case-frontier={}, scenario-robust-frontier={}; finite declared scenario ensemble only",
        candidates.len(),
        scenario_weights.len(),
        rows.iter().filter(|row| row.expected_pareto).count(),
        rows.iter().filter(|row| row.cvar95_pareto).count(),
        rows.iter().filter(|row| row.worst_case_pareto).count(),
        rows.iter().filter(|row| row.scenario_robust_pareto).count()
    );
    Ok(())
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { Err(format!("{context}: empty value is not allowed")) }
    else { Ok(value.to_string()) }
}

fn finite_number(value: &str, context: &str) -> Result<f64, String> {
    let parsed: f64 = value.trim().parse()
        .map_err(|_| format!("{context}: expected a finite number, got {value:?}"))?;
    if !parsed.is_finite() { return Err(format!("{context}: non-finite values are not allowed")); }
    Ok(parsed)
}

fn read_candidates(
    path: &str,
) -> Result<(ProfileMetadata, BTreeMap<String, Candidate>, BTreeMap<String, f64>), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing CSV header"))?.trim_end_matches('\r');
    let columns: Vec<&str> = header.split(',').collect();
    if columns.as_slice() != HEADER.as_slice() {
        return Err(format!("{path}: expected header [{}], got [{}]", HEADER.join(","), columns.join(",")));
    }

    let mut metadata: Option<ProfileMetadata> = None;
    let mut candidates: BTreeMap<String, Candidate> = BTreeMap::new();
    let mut scenario_weights: BTreeMap<String, f64> = BTreeMap::new();

    for (index, raw_line) in lines.enumerate() {
        let line_no = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() { return Err(format!("{path}:{line_no}: blank rows are not allowed")); }
        if line.contains('"') { return Err(format!("{path}:{line_no}: quoted CSV fields are not supported")); }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != HEADER.len() {
            return Err(format!("{path}:{line_no}: expected {} fields, got {}", HEADER.len(), fields.len()));
        }

        let candidate_id = nonempty(fields[0], "candidate_id")?;
        let row_metadata = ProfileMetadata {
            profile_id: nonempty(fields[1], "profile_id")?,
            currency_code: nonempty(fields[4], "currency_code")?,
            price_year: fields[5].trim().parse().map_err(|_| format!("{path}:{line_no}: price_year must be an integer"))?,
            price_basis: nonempty(fields[6], "price_basis")?,
            carbon_boundary_id: nonempty(fields[7], "carbon_boundary_id")?,
        };
        if row_metadata.currency_code.len() != 3 || !row_metadata.currency_code.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(format!("{path}:{line_no}: currency_code must be three uppercase ASCII letters (syntax only)"));
        }
        if !(1900..=2200).contains(&row_metadata.price_year) { return Err(format!("{path}:{line_no}: price_year must be in 1900..=2200")); }
        if !matches!(row_metadata.price_basis.as_str(), "REAL" | "NOMINAL") { return Err(format!("{path}:{line_no}: price_basis must be REAL or NOMINAL")); }
        if let Some(first) = metadata.as_ref() {
            if first != &row_metadata {
                return Err(format!("{path}:{line_no}: profile, currency, price year/basis and carbon boundary must match across all rows"));
            }
        } else {
            metadata = Some(row_metadata);
        }

        let scenario_id = nonempty(fields[2], "scenario_id")?;
        let weight = finite_number(fields[3], "scenario_weight")?;
        if !(0.0..=1.0).contains(&weight) || weight == 0.0 {
            return Err(format!("{path}:{line_no}: scenario_weight must be in (0, 1]"));
        }
        match scenario_weights.get(&scenario_id) {
            Some(previous) if (previous - weight).abs() > WEIGHT_TOLERANCE => {
                return Err(format!("{path}:{line_no}: scenario weight differs across candidates for {scenario_id:?}"));
            }
            None => { scenario_weights.insert(scenario_id.clone(), weight); }
            _ => {}
        }

        let mut objectives = [0.0; OBJECTIVE_COUNT];
        for objective_index in 0..OBJECTIVE_COUNT {
            let value = finite_number(fields[8 + objective_index], HEADER[8 + objective_index])?;
            if value < 0.0 {
                return Err(format!("{path}:{line_no}: objective values must be non-negative"));
            }
            objectives[objective_index] = value;
        }

        let candidate = candidates.entry(candidate_id.clone()).or_insert_with(|| Candidate {
            id: candidate_id.clone(),
            scenarios: BTreeMap::new(),
        });
        if candidate.scenarios.insert(scenario_id.clone(), ScenarioValue { weight, objectives }).is_some() {
            return Err(format!("{path}:{line_no}: duplicate candidate/scenario pair ({candidate_id},{scenario_id})"));
        }
    }

    if candidates.len() < 2 { return Err(format!("{path}: at least two candidates are required")); }
    if scenario_weights.len() < 2 { return Err(format!("{path}: at least two scenarios are required")); }
    let total_weight: f64 = scenario_weights.values().sum();
    if (total_weight - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(format!("{path}: scenario weights must sum to 1.0; got {total_weight:.12}"));
    }
    let expected_scenarios: BTreeSet<&str> = scenario_weights.keys().map(String::as_str).collect();
    for candidate in candidates.values() {
        let actual: BTreeSet<&str> = candidate.scenarios.keys().map(String::as_str).collect();
        if actual != expected_scenarios {
            return Err(format!("candidate {:?}: scenario set differs from the common scenario ensemble", candidate.id));
        }
        for (scenario_id, value) in &candidate.scenarios {
            let common_weight = scenario_weights[scenario_id];
            if (common_weight - value.weight).abs() > WEIGHT_TOLERANCE {
                return Err(format!("candidate {:?}: inconsistent weight for scenario {:?}", candidate.id, scenario_id));
            }
        }
    }

    Ok((
        metadata.ok_or_else(|| format!("{path}: no profile metadata"))?,
        candidates,
        scenario_weights,
    ))
}

fn aggregate(candidate: &Candidate) -> Result<Aggregate, String> {
    let mut expected = [0.0; OBJECTIVE_COUNT];
    let mut worst = [0.0; OBJECTIVE_COUNT];
    let mut cvar95 = [0.0; OBJECTIVE_COUNT];

    for value in candidate.scenarios.values() {
        for index in 0..OBJECTIVE_COUNT {
            expected[index] += value.weight * value.objectives[index];
            worst[index] = worst[index].max(value.objectives[index]);
        }
    }

    for objective_index in 0..OBJECTIVE_COUNT {
        let mut tail: Vec<(f64, f64)> = candidate.scenarios.values()
            .map(|value| (value.objectives[objective_index], value.weight))
            .collect();
        tail.sort_by(|left, right| right.0.total_cmp(&left.0));
        let mut remaining = TAIL_MASS;
        let mut weighted_tail = 0.0;
        for (value, weight) in tail {
            if remaining <= 0.0 { break; }
            let selected_weight = weight.min(remaining);
            weighted_tail += selected_weight * value;
            remaining -= selected_weight;
        }
        cvar95[objective_index] = weighted_tail / TAIL_MASS;
    }

    if expected.iter().chain(cvar95.iter()).chain(worst.iter()).any(|value| !value.is_finite()) {
        return Err(format!("candidate {:?}: derived scenario aggregate is non-finite", candidate.id));
    }
    Ok(Aggregate { expected, cvar95, worst })
}

fn dominates(left: &[f64; OBJECTIVE_COUNT], right: &[f64; OBJECTIVE_COUNT]) -> bool {
    let mut strictly_better = false;
    for index in 0..OBJECTIVE_COUNT {
        if left[index] > right[index] { return false; }
        if left[index] < right[index] { strictly_better = true; }
    }
    strictly_better
}

fn scenario_robustly_dominates(left: &Candidate, right: &Candidate) -> bool {
    let mut strictly_better = false;
    for (scenario_id, left_value) in &left.scenarios {
        let right_value = &right.scenarios[scenario_id];
        for index in 0..OBJECTIVE_COUNT {
            if left_value.objectives[index] > right_value.objectives[index] {
                return false;
            }
            if left_value.objectives[index] < right_value.objectives[index] {
                strictly_better = true;
            }
        }
    }
    strictly_better
}

fn classify(candidates: &BTreeMap<String, Candidate>) -> Result<Vec<FrontierRow>, String> {
    let aggregates: BTreeMap<String, Aggregate> = candidates.iter()
        .map(|(id, candidate)| Ok((id.clone(), aggregate(candidate)?)))
        .collect::<Result<_, String>>()?;

    Ok(candidates.iter().map(|(id, candidate)| {
        let mut expected_dominated = false;
        let mut cvar_dominated = false;
        let mut worst_dominated = false;
        let mut scenario_robust_dominators = Vec::new();
        for (other_id, _) in candidates {
            if id == other_id { continue; }
            let other = aggregates[other_id];
            let current = aggregates[id];
            expected_dominated |= dominates(&other.expected, &current.expected);
            cvar_dominated |= dominates(&other.cvar95, &current.cvar95);
            worst_dominated |= dominates(&other.worst, &current.worst);
            if scenario_robustly_dominates(&candidates[other_id], candidate) {
                scenario_robust_dominators.push(other_id.clone());
            }
        }
        FrontierRow {
            candidate_id: id.clone(),
            aggregate: aggregates[id],
            expected_pareto: !expected_dominated,
            cvar95_pareto: !cvar_dominated,
            worst_case_pareto: !worst_dominated,
            scenario_robust_pareto: scenario_robust_dominators.is_empty(),
            scenario_robust_dominators,
        }
    }).collect())
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else { value.to_string() }
}

fn write_report(
    path: &str,
    metadata: &ProfileMetadata,
    scenario_count: usize,
    rows: &[FrontierRow],
) -> Result<(), String> {
    let mut output = String::from(
        "candidate_id,profile_id,currency_code,price_year,price_basis,carbon_boundary_id,scenario_count,expected_lifecycle_cost,cvar95_lifecycle_cost,worst_lifecycle_cost,expected_no_load_loss_w,cvar95_no_load_loss_w,worst_no_load_loss_w,expected_load_loss_w,cvar95_load_loss_w,worst_load_loss_w,expected_hotspot_temp_c,cvar95_hotspot_temp_c,worst_hotspot_temp_c,expected_mass_kg,cvar95_mass_kg,worst_mass_kg,expected_embodied_carbon_kgco2e,cvar95_embodied_carbon_kgco2e,worst_embodied_carbon_kgco2e,expected_pareto,cvar95_pareto,worst_case_pareto,scenario_robust_pareto,scenario_robust_dominators\n",
    );
    for row in rows {
        let aggregate = row.aggregate;
        let mut values = vec![
            csv_escape(&row.candidate_id), csv_escape(&metadata.profile_id),
            metadata.currency_code.clone(), metadata.price_year.to_string(),
            metadata.price_basis.clone(), csv_escape(&metadata.carbon_boundary_id),
            scenario_count.to_string(),
        ];
        for index in 0..OBJECTIVE_COUNT {
            values.push(format!("{:.9}", aggregate.expected[index]));
            values.push(format!("{:.9}", aggregate.cvar95[index]));
            values.push(format!("{:.9}", aggregate.worst[index]));
        }
        values.push(row.expected_pareto.to_string());
        values.push(row.cvar95_pareto.to_string());
        values.push(row.worst_case_pareto.to_string());
        values.push(row.scenario_robust_pareto.to_string());
        values.push(csv_escape(&row.scenario_robust_dominators.join(";")));
        output.push_str(&values.join(","));
        output.push('\n');
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario(weight: f64, values: [f64; OBJECTIVE_COUNT]) -> ScenarioValue {
        ScenarioValue { weight, objectives: values }
    }

    fn candidate(id: &str, rows: &[(&str, f64, [f64; OBJECTIVE_COUNT])]) -> Candidate {
        Candidate {
            id: id.to_string(),
            scenarios: rows.iter().map(|(name, weight, values)| {
                ((*name).to_string(), scenario(*weight, *values))
            }).collect(),
        }
    }

    #[test]
    fn cvar95_uses_exact_fraction_of_upper_tail_probability_mass() {
        let candidate = candidate("A", &[
            ("ordinary", 0.88, [10.0; OBJECTIVE_COUNT]),
            ("bad", 0.10, [20.0; OBJECTIVE_COUNT]),
            ("catastrophic", 0.02, [30.0; OBJECTIVE_COUNT]),
        ]);
        let aggregate = aggregate(&candidate).unwrap();
        assert!((aggregate.cvar95[0] - 24.0).abs() < 1.0e-12);
        assert_eq!(aggregate.worst[0], 30.0);
        assert!((aggregate.expected[0] - 11.4).abs() < 1.0e-12);
    }

    #[test]
    fn scenario_robust_dominance_requires_no_worse_values_in_every_paired_scenario() {
        let better = candidate("A", &[
            ("normal", 0.5, [10.0,10.0,10.0,10.0,10.0,10.0]),
            ("stress", 0.5, [20.0,20.0,20.0,20.0,20.0,20.0]),
        ]);
        let worse = candidate("B", &[
            ("normal", 0.5, [11.0,11.0,11.0,11.0,11.0,11.0]),
            ("stress", 0.5, [21.0,21.0,21.0,21.0,21.0,21.0]),
        ]);
        assert!(scenario_robustly_dominates(&better, &worse));
        assert!(!scenario_robustly_dominates(&worse, &better));
    }

    #[test]
    fn opposing_scenario_tradeoffs_prevent_false_robust_dominance() {
        let a = candidate("A", &[
            ("normal", 0.5, [10.0,20.0,10.0,20.0,10.0,20.0]),
            ("stress", 0.5, [20.0,10.0,20.0,10.0,20.0,10.0]),
        ]);
        let b = candidate("B", &[
            ("normal", 0.5, [20.0,10.0,20.0,10.0,20.0,10.0]),
            ("stress", 0.5, [10.0,20.0,10.0,20.0,10.0,20.0]),
        ]);
        assert!(!scenario_robustly_dominates(&a, &b));
        assert!(!scenario_robustly_dominates(&b, &a));
    }

    #[test]
    fn checked_in_fixture_contains_all_expected_candidates_and_scenario_rows() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/scenario-frontier-synthetic.csv");
        let (_, candidates, weights) = read_candidates(path.to_str().unwrap()).unwrap();
        assert_eq!(candidates.len(), 4);
        assert_eq!(weights.len(), 4);
        assert!(candidates.values().all(|candidate| candidate.scenarios.len() == 4));
        let rows = classify(&candidates).unwrap();
        let dominated = rows.iter().find(|row| row.candidate_id == "C").unwrap();
        assert!(dominated.scenario_robust_dominators.contains(&"A".to_string()));
        assert!(!dominated.scenario_robust_pareto);
    }

    #[test]
    fn parser_rejects_missing_scenario_and_inconsistent_scenario_weights() {
        let path = env::temp_dir().join("transformer-scenario-frontier-test.csv");
        let path = path.to_string_lossy().into_owned();
        let header = HEADER.join(",");
        let row_a = "A,P,normal,0.5,ZAR,2026,REAL,CB,1,1,1,1,1,1";
        let row_b = "B,P,normal,0.4,ZAR,2026,REAL,CB,2,2,2,2,2,2";
        let row_c = "B,P,stress,0.6,ZAR,2026,REAL,CB,2,2,2,2,2,2";
        fs::write(&path, format!("{header}\n{row_a}\n{row_b}\n{row_c}\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("scenario weight differs"));

        let row_a = "A,P,normal,0.5,ZAR,2026,REAL,CB,1,1,1,1,1,1";
        let row_b = "B,P,normal,0.5,ZAR,2026,REAL,CB,2,2,2,2,2,2";
        let row_c = "B,P,stress,0.5,ZAR,2026,REAL,CB,2,2,2,2,2,2";
        fs::write(&path, format!("{header}\n{row_a}\n{row_b}\n{row_c}\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("scenario set differs"));
        let _ = fs::remove_file(path);
    }
}
