//! Evidence-aware, economics-only distribution-transformer screening harness.
//! The loss model is a declared first-order approximation, not a design solver.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HOURS_PER_YEAR: f64 = 8_760.0;
const HOURS_TOLERANCE: f64 = 1.0e-6;
const WEIGHT_TOLERANCE: f64 = 1.0e-9;
const CURRENT_MOMENT_REL_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug)]
struct Candidate { id: String, capex: f64, p0_w: f64, pk_w: f64 }
#[derive(Clone, Debug)]
struct Interval { hours: f64, current_pu_mean: f64, mean_square_current_pu2: f64, price_per_kwh: f64 }
#[derive(Clone, Debug)]
struct Scenario { weight: f64, intervals: Vec<Interval> }
#[derive(Clone, Debug)]
struct GateRecord { status: GateStatus, evidence_sha256: String }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GateStatus { Pass, Fail, Unknown }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManifestState { Complete, Rejected, Incomplete }

impl ManifestState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "MANIFEST_COMPLETE",
            Self::Rejected => "REJECTED",
            Self::Incomplete => "INCOMPLETE",
        }
    }
    fn eligible_for_manifest_frontier(self) -> bool { self == Self::Complete }
}

#[derive(Clone, Copy, Debug)]
struct ScenarioMetrics { annual_loss_kwh: f64, annual_loss_cost: f64 }
#[derive(Clone, Debug)]
struct ResultRow {
    id: String,
    manifest_state: ManifestState,
    capex: f64,
    expected_annual_loss_kwh: f64,
    expected_npv_loss_cost: f64,
    worst_case_npv_loss_cost: f64,
    expected_lifecycle_cost: f64,
    worst_case_lifecycle_cost: f64,
    economic_pareto: bool,
    manifest_complete_pareto: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("transformer-bench: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 6 || args.len() > 8 {
        return Err(format!(
            "usage: {} <candidates.csv> <scenario_profiles.csv> <required_gates.csv> <gate_records.csv> <report.csv> [life_years=30] [real_discount_rate=0.08]",
            args.first().map(String::as_str).unwrap_or("transformer-bench")
        ));
    }
    let life_years: u32 = if args.len() >= 7 {
        args[6].parse().map_err(|_| "life_years must be a positive integer".to_string())?
    } else { 30 };
    if !(1..=100).contains(&life_years) { return Err("life_years must be in 1..=100".to_string()); }
    let discount_rate = if args.len() >= 8 { parse_number(&args[7], "real_discount_rate")? } else { 0.08 };
    if !(0.0..=1.0).contains(&discount_rate) { return Err("real_discount_rate must be in [0, 1]".to_string()); }

    let candidates = read_candidates(&args[1])?;
    let scenarios = read_scenarios(&args[2])?;
    let required_gates = read_required_gates(&args[3])?;
    let gate_records = read_gate_records(&args[4], &candidates, &required_gates)?;
    let mut results = evaluate_all(&candidates, &scenarios, &required_gates, &gate_records, life_years, discount_rate);
    mark_frontiers(&mut results);
    write_report(&args[5], &results)?;
    eprintln!(
        "screening report written: candidates={}, scenarios={}, life_years={}, real_discount_rate={:.6}; not a design approval",
        results.len(), scenarios.len(), life_years, discount_rate
    );
    Ok(())
}

fn read_text(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))
}

/// Supports quoted CSV fields and escaped quotes; rejects malformed quoting.
fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut after_quote = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if matches!(chars.peek(), Some(&'"')) { chars.next(); field.push('"'); }
                else { in_quotes = false; after_quote = true; }
            } else { field.push(ch); }
            continue;
        }
        if after_quote {
            match ch {
                ',' => { row.push(std::mem::take(&mut field)); after_quote = false; }
                '\n' => { row.push(std::mem::take(&mut field)); push_nonblank_row(&mut rows, &mut row); after_quote = false; }
                '\r' if matches!(chars.peek(), Some(&'\n')) => {}
                _ => return Err("unexpected character after closing CSV quote".to_string()),
            }
            continue;
        }
        match ch {
            '"' if field.is_empty() => in_quotes = true,
            '"' => return Err("quote inside an unquoted CSV field".to_string()),
            ',' => row.push(std::mem::take(&mut field)),
            '\n' => { row.push(std::mem::take(&mut field)); push_nonblank_row(&mut rows, &mut row); }
            '\r' if matches!(chars.peek(), Some(&'\n')) => {}
            _ => field.push(ch),
        }
    }
    if in_quotes { return Err("unterminated quoted CSV field".to_string()); }
    if after_quote || !field.is_empty() || !row.is_empty() {
        row.push(field);
        push_nonblank_row(&mut rows, &mut row);
    }
    Ok(rows)
}

fn push_nonblank_row(rows: &mut Vec<Vec<String>>, row: &mut Vec<String>) {
    if row.len() == 1 && row[0].trim().is_empty() { row.clear(); } else { rows.push(std::mem::take(row)); }
}

fn read_table(path: &str, expected_header: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let mut rows = parse_csv(&read_text(path)?).map_err(|error| format!("{path}: {error}"))?;
    if rows.is_empty() { return Err(format!("{path}: missing CSV header")); }
    let header: Vec<&str> = rows[0].iter().map(|value| value.trim()).collect();
    if header.as_slice() != expected_header {
        return Err(format!("{path}: expected header [{}], got [{}]", expected_header.join(","), header.join(",")));
    }
    rows.remove(0);
    for (index, row) in rows.iter().enumerate() {
        if row.len() != expected_header.len() {
            return Err(format!("{path}: row {} has {} fields; expected {}", index + 2, row.len(), expected_header.len()));
        }
    }
    Ok(rows)
}

fn parse_number(value: &str, context: &str) -> Result<f64, String> {
    let parsed: f64 = value.trim().parse().map_err(|_| format!("{context}: expected a finite number, got {value:?}"))?;
    if !parsed.is_finite() { return Err(format!("{context}: non-finite value is not allowed")); }
    Ok(parsed)
}
fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() { Err(format!("{context}: empty value is not allowed")) } else { Ok(trimmed.to_string()) }
}

fn read_candidates(path: &str) -> Result<Vec<Candidate>, String> {
    let rows = read_table(path, &["candidate_id", "capex", "p0_w", "pk_w"])?;
    if rows.is_empty() { return Err(format!("{path}: at least one candidate is required")); }
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let line = index + 2;
        let id = nonempty(&row[0], &format!("{path}:{line}: candidate_id"))?;
        if !seen.insert(id.clone()) { return Err(format!("{path}:{line}: duplicate candidate_id {id:?}")); }
        let capex = parse_number(&row[1], &format!("{path}:{line}: capex"))?;
        let p0_w = parse_number(&row[2], &format!("{path}:{line}: p0_w"))?;
        let pk_w = parse_number(&row[3], &format!("{path}:{line}: pk_w"))?;
        if capex < 0.0 || p0_w < 0.0 || pk_w < 0.0 { return Err(format!("{path}:{line}: capex, p0_w and pk_w must be non-negative")); }
        candidates.push(Candidate { id, capex, p0_w, pk_w });
    }
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(candidates)
}

fn read_scenarios(path: &str) -> Result<BTreeMap<String, Scenario>, String> {
    let rows = read_table(path, &["scenario_id", "weight", "interval_id", "hours", "current_pu_mean", "mean_square_current_pu2", "price_per_kwh"])?;
    if rows.is_empty() { return Err(format!("{path}: at least one scenario interval is required")); }
    let mut scenarios: BTreeMap<String, Scenario> = BTreeMap::new();
    let mut interval_ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let line = index + 2;
        let scenario_id = nonempty(&row[0], &format!("{path}:{line}: scenario_id"))?;
        let weight = parse_number(&row[1], &format!("{path}:{line}: weight"))?;
        let interval_id = nonempty(&row[2], &format!("{path}:{line}: interval_id"))?;
        let hours = parse_number(&row[3], &format!("{path}:{line}: hours"))?;
        let current_pu_mean = parse_number(&row[4], &format!("{path}:{line}: current_pu_mean"))?;
        let mean_square_current_pu2 = parse_number(&row[5], &format!("{path}:{line}: mean_square_current_pu2"))?;
        let price_per_kwh = parse_number(&row[6], &format!("{path}:{line}: price_per_kwh"))?;
        if !(0.0..=1.0).contains(&weight) || weight == 0.0 { return Err(format!("{path}:{line}: weight must be in (0, 1]")); }
        if hours <= 0.0 || current_pu_mean < 0.0 || mean_square_current_pu2 < 0.0 || price_per_kwh < 0.0 {
            return Err(format!("{path}:{line}: hours must be positive; current moments and price must be non-negative"));
        }
        let squared_mean = current_pu_mean * current_pu_mean;
        if !squared_mean.is_finite() {
            return Err(format!("{path}:{line}: current_pu_mean is too large to square safely"));
        }
        let moment_tolerance = CURRENT_MOMENT_REL_TOLERANCE * squared_mean.max(mean_square_current_pu2).max(1.0);
        if mean_square_current_pu2 + moment_tolerance < squared_mean {
            return Err(format!("{path}:{line}: mean_square_current_pu2 must be at least current_pu_mean squared (within relative tolerance)"));
        }
        if let Some(existing) = scenarios.get_mut(&scenario_id) {
            if (existing.weight - weight).abs() > WEIGHT_TOLERANCE { return Err(format!("{path}:{line}: inconsistent weight for scenario {scenario_id:?}")); }
            existing.intervals.push(Interval { hours, current_pu_mean, mean_square_current_pu2, price_per_kwh });
        } else {
            scenarios.insert(scenario_id.clone(), Scenario { weight, intervals: vec![Interval { hours, current_pu_mean, mean_square_current_pu2, price_per_kwh }] });
        }
        if !interval_ids.entry(scenario_id.clone()).or_default().insert(interval_id.clone()) {
            return Err(format!("{path}:{line}: duplicate interval_id {interval_id:?} in scenario {scenario_id:?}"));
        }
    }
    let total_weight: f64 = scenarios.values().map(|scenario| scenario.weight).sum();
    if (total_weight - 1.0).abs() > WEIGHT_TOLERANCE { return Err(format!("{path}: scenario weights must sum to 1.0; got {total_weight:.12}")); }
    for (id, scenario) in &scenarios {
        let total_hours: f64 = scenario.intervals.iter().map(|interval| interval.hours).sum();
        if (total_hours - HOURS_PER_YEAR).abs() > HOURS_TOLERANCE {
            return Err(format!("{path}: scenario {id:?} covers {total_hours:.9} hours, expected {HOURS_PER_YEAR:.0}"));
        }
    }
    Ok(scenarios)
}

fn read_required_gates(path: &str) -> Result<Vec<String>, String> {
    let rows = read_table(path, &["gate_id"])?;
    if rows.is_empty() { return Err(format!("{path}: at least one required gate is required")); }
    let mut gates = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        let id = nonempty(&row[0], &format!("{path}:{}: gate_id", index + 2))?;
        if !gates.insert(id.clone()) { return Err(format!("{path}: duplicate required gate {id:?}")); }
    }
    Ok(gates.into_iter().collect())
}

fn parse_gate_status(value: &str, context: &str) -> Result<GateStatus, String> {
    match value.trim() {
        "PASS" => Ok(GateStatus::Pass), "FAIL" => Ok(GateStatus::Fail), "UNKNOWN" => Ok(GateStatus::Unknown),
        other => Err(format!("{context}: status must be PASS, FAIL or UNKNOWN; got {other:?}")),
    }
}
fn is_valid_sha256(value: &str) -> bool { value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()) }

fn read_gate_records(path: &str, candidates: &[Candidate], required_gates: &[String]) -> Result<BTreeMap<(String, String), GateRecord>, String> {
    let rows = read_table(path, &["candidate_id", "gate_id", "status", "evidence_sha256"])?;
    let candidate_ids: BTreeSet<&str> = candidates.iter().map(|candidate| candidate.id.as_str()).collect();
    let gate_ids: BTreeSet<&str> = required_gates.iter().map(String::as_str).collect();
    let mut records = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let line = index + 2;
        let candidate_id = nonempty(&row[0], &format!("{path}:{line}: candidate_id"))?;
        let gate_id = nonempty(&row[1], &format!("{path}:{line}: gate_id"))?;
        if !candidate_ids.contains(candidate_id.as_str()) { return Err(format!("{path}:{line}: unknown candidate_id {candidate_id:?}")); }
        if !gate_ids.contains(gate_id.as_str()) { return Err(format!("{path}:{line}: gate {gate_id:?} is not in the required-gate manifest")); }
        let status = parse_gate_status(&row[2], &format!("{path}:{line}"))?;
        let evidence_sha256 = row[3].trim().to_string();
        if !evidence_sha256.is_empty() && !is_valid_sha256(&evidence_sha256) {
            return Err(format!("{path}:{line}: evidence_sha256 must be empty or exactly 64 hexadecimal characters"));
        }
        let key = (candidate_id.clone(), gate_id.clone());
        if records.insert(key, GateRecord { status, evidence_sha256 }).is_some() {
            return Err(format!("{path}:{line}: duplicate gate record for candidate {candidate_id:?}, gate {gate_id:?}"));
        }
    }
    Ok(records)
}

fn manifest_state(candidate_id: &str, required_gates: &[String], records: &BTreeMap<(String, String), GateRecord>) -> ManifestState {
    let mut has_failure = false;
    let mut complete = true;
    for gate_id in required_gates {
        match records.get(&(candidate_id.to_string(), gate_id.clone())) {
            Some(record) if record.status == GateStatus::Fail => has_failure = true,
            Some(record) if record.status == GateStatus::Pass && is_valid_sha256(&record.evidence_sha256) => {},
            _ => complete = false,
        }
    }
    if has_failure { ManifestState::Rejected } else if complete { ManifestState::Complete } else { ManifestState::Incomplete }
}

fn evaluate_scenario(candidate: &Candidate, scenario: &Scenario) -> ScenarioMetrics {
    let mut annual_loss_kwh = 0.0;
    let mut annual_loss_cost = 0.0;
    for interval in &scenario.intervals {
        // The load-loss term depends on mean-square per-unit current, not the
        // square of an interval's arithmetic mean current.
        let load_loss_w = candidate.pk_w * interval.mean_square_current_pu2;
        let loss_kwh = (candidate.p0_w + load_loss_w) * interval.hours / 1_000.0;
        annual_loss_kwh += loss_kwh;
        annual_loss_cost += loss_kwh * interval.price_per_kwh;
    }
    ScenarioMetrics { annual_loss_kwh, annual_loss_cost }
}

fn annuity_factor(life_years: u32, real_discount_rate: f64) -> f64 {
    (1..=life_years).map(|year| 1.0 / (1.0 + real_discount_rate).powi(year as i32)).sum()
}

fn evaluate_all(candidates: &[Candidate], scenarios: &BTreeMap<String, Scenario>, required_gates: &[String], gate_records: &BTreeMap<(String, String), GateRecord>, life_years: u32, real_discount_rate: f64) -> Vec<ResultRow> {
    let factor = annuity_factor(life_years, real_discount_rate);
    let mut results = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let mut expected_annual_loss_kwh = 0.0;
        let mut expected_npv_loss_cost = 0.0;
        let mut worst_case_npv_loss_cost: f64 = 0.0;
        for scenario in scenarios.values() {
            let metrics = evaluate_scenario(candidate, scenario);
            let npv_loss_cost = metrics.annual_loss_cost * factor;
            expected_annual_loss_kwh += scenario.weight * metrics.annual_loss_kwh;
            expected_npv_loss_cost += scenario.weight * npv_loss_cost;
            worst_case_npv_loss_cost = worst_case_npv_loss_cost.max(npv_loss_cost);
        }
        results.push(ResultRow {
            id: candidate.id.clone(), manifest_state: manifest_state(&candidate.id, required_gates, gate_records), capex: candidate.capex,
            expected_annual_loss_kwh, expected_npv_loss_cost, worst_case_npv_loss_cost,
            expected_lifecycle_cost: candidate.capex + expected_npv_loss_cost,
            worst_case_lifecycle_cost: candidate.capex + worst_case_npv_loss_cost,
            economic_pareto: false, manifest_complete_pareto: false,
        });
    }
    results
}

fn dominates(a: &ResultRow, b: &ResultRow) -> bool {
    let a_values = [a.capex, a.expected_npv_loss_cost, a.worst_case_npv_loss_cost];
    let b_values = [b.capex, b.expected_npv_loss_cost, b.worst_case_npv_loss_cost];
    a_values.iter().zip(b_values.iter()).all(|(left, right)| left <= right)
        && a_values.iter().zip(b_values.iter()).any(|(left, right)| left < right)
}

fn mark_frontiers(results: &mut [ResultRow]) {
    for i in 0..results.len() {
        results[i].economic_pareto = !(0..results.len()).any(|j| i != j && dominates(&results[j], &results[i]));
        if !results[i].manifest_state.eligible_for_manifest_frontier() {
            results[i].manifest_complete_pareto = false;
        } else {
            results[i].manifest_complete_pareto = !(0..results.len()).any(|j| {
                i != j && results[j].manifest_state.eligible_for_manifest_frontier() && dominates(&results[j], &results[i])
            });
        }
    }
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else { value.to_string() }
}
fn write_report(path: &str, results: &[ResultRow]) -> Result<(), String> {
    let mut output = String::from("candidate_id,manifest_state,capex,expected_annual_loss_kwh,expected_npv_loss_cost,worst_case_npv_loss_cost,expected_lifecycle_cost,worst_case_lifecycle_cost,economic_pareto,manifest_complete_pareto\n");
    for result in results {
        output.push_str(&format!("{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{},{}\n",
            csv_escape(&result.id), result.manifest_state.as_str(), result.capex, result.expected_annual_loss_kwh,
            result.expected_npv_loss_cost, result.worst_case_npv_loss_cost, result.expected_lifecycle_cost,
            result.worst_case_lifecycle_cost, result.economic_pareto, result.manifest_complete_pareto));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn digest() -> String { "a".repeat(64) }
    fn candidate(id: &str, capex: f64, p0_w: f64, pk_w: f64) -> Candidate { Candidate { id: id.to_string(), capex, p0_w, pk_w } }
    fn row(id: &str, capex: f64, expected: f64, worst: f64, state: ManifestState) -> ResultRow {
        ResultRow { id: id.to_string(), manifest_state: state, capex, expected_annual_loss_kwh: 0.0,
            expected_npv_loss_cost: expected, worst_case_npv_loss_cost: worst, expected_lifecycle_cost: capex + expected,
            worst_case_lifecycle_cost: capex + worst, economic_pareto: false, manifest_complete_pareto: false }
    }

    #[test]
    fn loss_formula_matches_hand_calculation() {
        let c = candidate("hand", 10.0, 1_000.0, 4_000.0);
        let s = Scenario { weight: 1.0, intervals: vec![Interval { hours: 8_760.0, current_pu_mean: 0.5, mean_square_current_pu2: 0.25, price_per_kwh: 1.0 }] };
        let metrics = evaluate_scenario(&c, &s);
        assert!((metrics.annual_loss_kwh - 17_520.0).abs() < 1.0e-9);
        assert!((metrics.annual_loss_cost - 17_520.0).abs() < 1.0e-9);
    }


    #[test]
    fn mean_square_current_prevents_variance_loss_underestimation() {
        let c = candidate("moment", 0.0, 0.0, 4_000.0);
        let constant = Scenario {
            weight: 1.0,
            intervals: vec![Interval { hours: 8_760.0, current_pu_mean: 0.5, mean_square_current_pu2: 0.25, price_per_kwh: 1.0 }],
        };
        // A 50/50 trace at 0 pu and 1 pu has mean current 0.5 pu,
        // but mean-square current 0.5 pu².
        let variable = Scenario {
            weight: 1.0,
            intervals: vec![Interval { hours: 8_760.0, current_pu_mean: 0.5, mean_square_current_pu2: 0.5, price_per_kwh: 1.0 }],
        };
        let constant_metrics = evaluate_scenario(&c, &constant);
        let variable_metrics = evaluate_scenario(&c, &variable);
        assert!((constant_metrics.annual_loss_kwh - 8_760.0).abs() < 1.0e-9);
        assert!((variable_metrics.annual_loss_kwh - 17_520.0).abs() < 1.0e-9);
        assert!((variable_metrics.annual_loss_kwh / constant_metrics.annual_loss_kwh - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn splitting_intervals_preserves_loss_when_second_moment_and_tariff_are_preserved() {
        let c = candidate("split", 0.0, 700.0, 3_000.0);
        let whole = Scenario {
            weight: 1.0,
            intervals: vec![Interval { hours: 8.0, current_pu_mean: 0.575, mean_square_current_pu2: 0.34375, price_per_kwh: 2.0 }],
        };
        let split = Scenario {
            weight: 1.0,
            intervals: vec![
                Interval { hours: 3.0, current_pu_mean: 0.7, mean_square_current_pu2: 0.5, price_per_kwh: 2.0 },
                Interval { hours: 5.0, current_pu_mean: 0.5, mean_square_current_pu2: 0.25, price_per_kwh: 2.0 },
            ],
        };
        let a = evaluate_scenario(&c, &whole);
        let b = evaluate_scenario(&c, &split);
        assert!((a.annual_loss_kwh - b.annual_loss_kwh).abs() < 1.0e-12);
        assert!((a.annual_loss_cost - b.annual_loss_cost).abs() < 1.0e-12);
    }

    #[test]
    fn mean_square_current_cannot_be_lower_than_squared_mean() {
        let path = std::env::temp_dir().join("transformer-bench-moment-test.csv");
        let path = path.to_string_lossy().into_owned();
        let bad = "scenario_id,weight,interval_id,hours,current_pu_mean,mean_square_current_pu2,price_per_kwh\na,1.0,all,8760,0.5,0.2,2\n";
        fs::write(&path, bad).unwrap();
        assert!(read_scenarios(&path).is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn moment_schema_rejects_mean_only_profile() {
        let path = std::env::temp_dir().join("transformer-bench-mean-only-test.csv");
        let path = path.to_string_lossy().into_owned();
        let mean_only = "scenario_id,weight,interval_id,hours,load_pu,price_per_kwh\na,1.0,all,8760,0.5,2\n";
        fs::write(&path, mean_only).unwrap();
        assert!(read_scenarios(&path).is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn zero_discount_factor_equals_lifetime_years() { assert!((annuity_factor(30, 0.0) - 30.0).abs() < 1.0e-12); }

    #[test]
    fn scenario_weights_and_annual_hours_are_validated() {
        let path = std::env::temp_dir().join("transformer-bench-scenarios-test.csv");
        let path = path.to_string_lossy().into_owned();
        let valid = "scenario_id,weight,interval_id,hours,current_pu_mean,mean_square_current_pu2,price_per_kwh\na,0.7,all,8760,0.5,0.25,2\nb,0.3,all,8760,1.0,1.0,3\n";
        fs::write(&path, valid).unwrap();
        assert_eq!(read_scenarios(&path).unwrap().len(), 2);
        let bad_hours = "scenario_id,weight,interval_id,hours,current_pu_mean,mean_square_current_pu2,price_per_kwh\na,1.0,part,8000,0.5,0.25,2\n";
        fs::write(&path, bad_hours).unwrap();
        assert!(read_scenarios(&path).is_err());
        let bad_weights = "scenario_id,weight,interval_id,hours,current_pu_mean,mean_square_current_pu2,price_per_kwh\na,0.8,all,8760,0.5,0.25,2\nb,0.3,all,8760,1.0,1.0,3\n";
        fs::write(&path, bad_weights).unwrap();
        assert!(read_scenarios(&path).is_err());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_required_gate_is_incomplete() {
        let required = vec!["thermal".to_string(), "dielectric".to_string()];
        let mut records = BTreeMap::new();
        records.insert(("c1".to_string(), "thermal".to_string()), GateRecord { status: GateStatus::Pass, evidence_sha256: digest() });
        assert_eq!(manifest_state("c1", &required, &records), ManifestState::Incomplete);
    }

    #[test]
    fn failed_gate_is_rejected_even_if_another_gate_is_missing() {
        let required = vec!["thermal".to_string(), "dielectric".to_string()];
        let mut records = BTreeMap::new();
        records.insert(("c1".to_string(), "thermal".to_string()), GateRecord { status: GateStatus::Fail, evidence_sha256: String::new() });
        assert_eq!(manifest_state("c1", &required, &records), ManifestState::Rejected);
    }

    #[test]
    fn pass_without_digest_is_not_complete() {
        let required = vec!["thermal".to_string()];
        let mut records = BTreeMap::new();
        records.insert(("c1".to_string(), "thermal".to_string()), GateRecord { status: GateStatus::Pass, evidence_sha256: String::new() });
        assert_eq!(manifest_state("c1", &required, &records), ManifestState::Incomplete);
    }

    #[test]
    fn digest_syntax_is_fail_closed() {
        assert!(is_valid_sha256(&digest()));
        assert!(!is_valid_sha256("not-a-digest"));
        assert!(!is_valid_sha256(&"g".repeat(64)));
    }

    #[test]
    fn pareto_frontier_preserves_tradeoffs_and_marks_dominated_candidate() {
        let complete = ManifestState::Complete;
        let mut results = vec![row("a", 100.0, 20.0, 30.0, complete), row("b", 90.0, 35.0, 40.0, complete), row("c", 120.0, 25.0, 35.0, complete)];
        mark_frontiers(&mut results);
        assert!(results[0].economic_pareto);
        assert!(results[1].economic_pareto);
        assert!(!results[2].economic_pareto);
        assert!(!results[2].manifest_complete_pareto);
    }

    #[test]
    fn incomplete_candidate_never_enters_manifest_complete_frontier() {
        let mut results = vec![row("complete", 100.0, 20.0, 30.0, ManifestState::Complete), row("unknown", 1.0, 1.0, 1.0, ManifestState::Incomplete)];
        mark_frontiers(&mut results);
        assert!(results[0].manifest_complete_pareto);
        assert!(!results[1].manifest_complete_pareto);
    }

    #[test]
    fn csv_quotes_are_parsed_and_malformed_quote_is_rejected() {
        assert_eq!(parse_csv("\"a,b\",c\n").unwrap(), vec![vec!["a,b".to_string(), "c".to_string()]]);
        assert!(parse_csv("\"a,b,c\n").is_err());
        assert!(parse_csv("\"a\"x,b\n").is_err());
    }

    #[test]
    fn non_finite_numbers_are_rejected() {
        assert!(parse_number("NaN", "test").is_err());
        assert!(parse_number("inf", "test").is_err());
    }
}