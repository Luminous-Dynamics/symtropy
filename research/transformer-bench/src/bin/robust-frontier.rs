//! Conservative Pareto screening for transformer candidates with declared objective bounds.
//! Bounds are deterministic input ranges; they are not probability distributions or confidence intervals.
//! All six objectives are minimized. Profile/currency/price/carbon-boundary metadata must match.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 18] = [
    "candidate_id", "profile_id", "currency_code", "price_year", "price_basis",
    "carbon_boundary_id", "lifecycle_cost_lower", "lifecycle_cost_upper",
    "no_load_loss_w_lower", "no_load_loss_w_upper", "load_loss_w_lower",
    "load_loss_w_upper", "max_hotspot_temp_c_lower", "max_hotspot_temp_c_upper",
    "mass_kg_lower", "mass_kg_upper", "embodied_carbon_kgco2e_lower",
    "embodied_carbon_kgco2e_upper",
];
const OBJECTIVE_COUNT: usize = 6;

#[derive(Clone, Copy, Debug)]
struct Bounds { lower: f64, upper: f64 }
impl Bounds {
    fn midpoint(self) -> f64 { self.lower + (self.upper - self.lower) / 2.0 }
}

#[derive(Clone, Debug)]
struct Candidate { id: String, objectives: [Bounds; OBJECTIVE_COUNT] }

#[derive(Clone, Debug)]
struct ProfileMetadata {
    profile_id: String,
    currency_code: String,
    price_year: i32,
    price_basis: String,
    carbon_boundary_id: String,
}

#[derive(Clone, Debug)]
struct FrontierRow {
    candidate_id: String,
    nominal_pareto: bool,
    robust_pareto: bool,
    nominal_dominators: Vec<String>,
    certain_dominators: Vec<String>,
    possible_dominators: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("robust-frontier: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <candidate-bounds.csv> <frontier-report.csv>",
            args.first().map(String::as_str).unwrap_or("robust-frontier")
        ));
    }
    let (metadata, candidates) = read_candidates(&args[1])?;
    let frontier = classify(&candidates);
    write_report(&args[2], &metadata, &frontier)?;
    let nominal_count = frontier.iter().filter(|row| row.nominal_pareto).count();
    let robust_count = frontier.iter().filter(|row| row.robust_pareto).count();
    eprintln!(
        "uncertainty-aware Pareto report written: candidates={}, nominal_frontier={}, robust_frontier={}, profile={}, currency={}/{} {}; all objectives minimized, bounds are caller-declared ranges",
        candidates.len(), nominal_count, robust_count, metadata.profile_id,
        metadata.currency_code, metadata.price_year, metadata.price_basis
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

fn read_candidates(path: &str) -> Result<(ProfileMetadata, Vec<Candidate>), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing CSV header"))?.trim_end_matches('\r');
    let columns: Vec<&str> = header.split(',').collect();
    if columns.as_slice() != HEADER.as_slice() {
        return Err(format!("{path}: expected header [{}], got [{}]", HEADER.join(","), columns.join(",")));
    }

    let mut metadata: Option<ProfileMetadata> = None;
    let mut candidates = Vec::new();
    let mut ids = BTreeSet::new();

    for (index, raw_line) in lines.enumerate() {
        let line_no = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() { return Err(format!("{path}:{line_no}: blank rows are not allowed")); }
        if line.contains('"') { return Err(format!("{path}:{line_no}: quoted CSV fields are not supported")); }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != HEADER.len() {
            return Err(format!("{path}:{line_no}: expected {} fields, got {}", HEADER.len(), fields.len()));
        }

        let id = nonempty(fields[0], "candidate_id")?;
        if !ids.insert(id.clone()) { return Err(format!("{path}:{line_no}: duplicate candidate_id {id:?}")); }
        let row_metadata = ProfileMetadata {
            profile_id: nonempty(fields[1], "profile_id")?,
            currency_code: nonempty(fields[2], "currency_code")?,
            price_year: fields[3].trim().parse().map_err(|_| format!("{path}:{line_no}: price_year must be an integer"))?,
            price_basis: nonempty(fields[4], "price_basis")?,
            carbon_boundary_id: nonempty(fields[5], "carbon_boundary_id")?,
        };
        if row_metadata.currency_code.len() != 3 || !row_metadata.currency_code.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(format!("{path}:{line_no}: currency_code must be three uppercase ASCII letters (syntax only)"));
        }
        if !(1900..=2200).contains(&row_metadata.price_year) { return Err(format!("{path}:{line_no}: price_year must be in 1900..=2200")); }
        if !matches!(row_metadata.price_basis.as_str(), "REAL" | "NOMINAL") { return Err(format!("{path}:{line_no}: price_basis must be REAL or NOMINAL")); }
        match &metadata {
            None => metadata = Some(row_metadata),
            Some(first) if first.profile_id != row_metadata.profile_id
                || first.currency_code != row_metadata.currency_code
                || first.price_year != row_metadata.price_year
                || first.price_basis != row_metadata.price_basis
                || first.carbon_boundary_id != row_metadata.carbon_boundary_id => {
                    return Err(format!("{path}:{line_no}: profile, currency, price year/basis and carbon boundary must be identical across candidates"));
                }
            _ => {}
        }

        let mut objectives = [Bounds { lower: 0.0, upper: 0.0 }; OBJECTIVE_COUNT];
        for objective_index in 0..OBJECTIVE_COUNT {
            let lower = finite_number(fields[6 + objective_index * 2], HEADER[6 + objective_index * 2])?;
            let upper = finite_number(fields[7 + objective_index * 2], HEADER[7 + objective_index * 2])?;
            if lower < 0.0 || upper < 0.0 || lower > upper {
                return Err(format!("{path}:{line_no}: objective {} requires 0 <= lower <= upper", objective_index + 1));
            }
            objectives[objective_index] = Bounds { lower, upper };
        }
        candidates.push(Candidate { id, objectives });
    }

    if candidates.len() < 2 { return Err(format!("{path}: at least two candidates are required")); }
    Ok((metadata.ok_or_else(|| format!("{path}: no profile metadata"))?, candidates))
}

fn nominally_dominates(left: &Candidate, right: &Candidate) -> bool {
    let mut strictly_better = false;
    for index in 0..OBJECTIVE_COUNT {
        let a = left.objectives[index].midpoint();
        let b = right.objectives[index].midpoint();
        if a > b { return false; }
        if a < b { strictly_better = true; }
    }
    strictly_better
}

// left is guaranteed no worse than right for every value in the declared independent interval
// boxes, with at least one objective guaranteed strictly better.
fn certainly_dominates(left: &Candidate, right: &Candidate) -> bool {
    let mut strictly_better = false;
    for index in 0..OBJECTIVE_COUNT {
        let a = left.objectives[index];
        let b = right.objectives[index];
        if a.upper > b.lower { return false; }
        if a.upper < b.lower { strictly_better = true; }
    }
    strictly_better
}

// left dominates right for at least one assignment within independent interval boxes.
fn possibly_dominates(left: &Candidate, right: &Candidate) -> bool {
    let mut strictly_better = false;
    for index in 0..OBJECTIVE_COUNT {
        let a = left.objectives[index];
        let b = right.objectives[index];
        if a.lower > b.upper { return false; }
        if a.lower < b.upper { strictly_better = true; }
    }
    strictly_better
}

fn classify(candidates: &[Candidate]) -> Vec<FrontierRow> {
    candidates.iter().enumerate().map(|(index, candidate)| {
        let mut nominal_dominators = Vec::new();
        let mut certain_dominators = Vec::new();
        let mut possible_dominators = Vec::new();
        for (other_index, other) in candidates.iter().enumerate() {
            if index == other_index { continue; }
            if nominally_dominates(other, candidate) { nominal_dominators.push(other.id.clone()); }
            if certainly_dominates(other, candidate) { certain_dominators.push(other.id.clone()); }
            if possibly_dominates(other, candidate) { possible_dominators.push(other.id.clone()); }
        }
        nominal_dominators.sort();
        certain_dominators.sort();
        possible_dominators.sort();
        FrontierRow {
            candidate_id: candidate.id.clone(),
            nominal_pareto: nominal_dominators.is_empty(),
            robust_pareto: certain_dominators.is_empty(),
            nominal_dominators, certain_dominators, possible_dominators,
        }
    }).collect()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else { value.to_string() }
}

fn write_report(path: &str, metadata: &ProfileMetadata, rows: &[FrontierRow]) -> Result<(), String> {
    let mut output = String::from(
        "candidate_id,profile_id,currency_code,price_year,price_basis,carbon_boundary_id,nominal_pareto,robust_pareto,nominal_dominators,certain_dominators,possible_dominators,possible_dominator_count\n",
    );
    for row in rows {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(&row.candidate_id), csv_escape(&metadata.profile_id),
            metadata.currency_code, metadata.price_year, metadata.price_basis,
            csv_escape(&metadata.carbon_boundary_id), row.nominal_pareto, row.robust_pareto,
            csv_escape(&row.nominal_dominators.join(";")),
            csv_escape(&row.certain_dominators.join(";")),
            csv_escape(&row.possible_dominators.join(";")),
            row.possible_dominators.len()
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(id: &str, values: [(f64, f64); OBJECTIVE_COUNT]) -> Candidate {
        Candidate { id: id.to_string(), objectives: values.map(|(lower, upper)| Bounds { lower, upper }) }
    }
    fn a() -> Candidate {
        candidate("A", [(990.0,1010.0),(990.0,1010.0),(4900.0,5100.0),(89.0,91.0),(495.0,505.0),(1990.0,2010.0)])
    }
    fn c() -> Candidate {
        candidate("C", [(1490.0,1510.0),(1190.0,1210.0),(5900.0,6100.0),(99.0,101.0),(595.0,605.0),(2190.0,2210.0)])
    }

    #[test]
    fn certain_dominance_requires_non_overlapping_bounds_on_every_objective() {
        assert!(certainly_dominates(&a(), &c()));
        assert!(!certainly_dominates(&c(), &a()));
    }

    #[test]
    fn midpoint_frontier_can_be_smaller_than_conservative_robust_frontier() {
        let uncertain = candidate("D", [(950.0,1150.0),(950.0,1150.0),(4800.0,5400.0),(88.0,92.0),(480.0,520.0),(1900.0,2100.0)]);
        assert!(nominally_dominates(&a(), &uncertain));
        assert!(!certainly_dominates(&a(), &uncertain));
        let rows = classify(&[a(), uncertain]);
        assert!(!rows[1].nominal_pareto);
        assert!(rows[1].robust_pareto);
    }

    #[test]
    fn tradeoff_candidates_are_not_dominated_by_a_single_scalar_score() {
        let efficient = candidate("B", [(1190.0,1210.0),(690.0,710.0),(3900.0,4100.0),(84.0,86.0),(515.0,525.0),(2080.0,2120.0)]);
        let rows = classify(&[a(), efficient]);
        assert!(rows.iter().all(|row| row.robust_pareto));
        assert!(rows.iter().all(|row| row.nominal_pareto));
    }

    #[test]
    fn exactly_equal_degenerate_points_do_not_dominate_each_other() {
        let x = candidate("X", [(1.0,1.0); OBJECTIVE_COUNT]);
        let y = candidate("Y", [(1.0,1.0); OBJECTIVE_COUNT]);
        assert!(!certainly_dominates(&x, &y));
        assert!(!nominally_dominates(&x, &y));
    }

    #[test]
    fn parser_rejects_mixed_currency_and_reversed_bounds() {
        let path = env::temp_dir().join("transformer-robust-frontier-test.csv");
        let path = path.to_string_lossy().into_owned();
        let header = HEADER.join(",");
        let row = "a,SA-UTILITY,ZAR,2026,REAL,cradle-to-gate,1,2,1,2,1,2,1,2,1,2,1,2";
        let second = "b,SA-UTILITY,USD,2026,REAL,cradle-to-gate,1,2,1,2,1,2,1,2,1,2,1,2";
        fs::write(&path, format!("{header}\n{row}\n{second}\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("must be identical"));
        let reversed = "b,SA-UTILITY,ZAR,2026,REAL,cradle-to-gate,2,1,1,2,1,2,1,2,1,2,1,2";
        fs::write(&path, format!("{header}\n{row}\n{reversed}\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("lower <= upper"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_duplicate_candidate_ids() {
        let path = env::temp_dir().join("transformer-robust-frontier-duplicate.csv");
        let path = path.to_string_lossy().into_owned();
        let header = HEADER.join(",");
        let row = "a,SA-UTILITY,ZAR,2026,REAL,cradle-to-gate,1,2,1,2,1,2,1,2,1,2,1,2";
        fs::write(&path, format!("{header}\n{row}\n{row}\n")).unwrap();
        assert!(read_candidates(&path).unwrap_err().contains("duplicate candidate_id"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn checked_in_fixture_has_a_certainly_dominated_and_uncertain_candidate() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/robust-frontier-synthetic.csv");
        let (_, candidates) = read_candidates(path.to_str().unwrap()).unwrap();
        let rows = classify(&candidates);
        let dominated = rows.iter().find(|row| row.candidate_id == "C").unwrap();
        let uncertain = rows.iter().find(|row| row.candidate_id == "D").unwrap();
        assert!(!dominated.robust_pareto);
        assert!(dominated.certain_dominators.contains(&"A".to_string()));
        assert!(!uncertain.nominal_pareto);
        assert!(uncertain.robust_pareto);
    }
}
