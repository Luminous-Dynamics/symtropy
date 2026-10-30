//! Validate provenance envelopes for robust experiment acquisition metadata.
//! Scores and eligibility are sourced elsewhere; this tool validates contract shape,
//! bounds and lineage. It does NOT recompute scores, authenticate bytes or authorize release.
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 25] = [
    "candidate_id", "product_family", "product_id", "component_lot_id",
    "reference_population_id", "reference_population_sha256", "candidate_descriptor_sha256",
    "representativeness_method", "representativeness_score",
    "representativeness_validation_sha256", "reference_holdout_lots", "reference_group_disjoint",
    "amplification_method", "amplification_risk", "amplification_audit_sha256",
    "discrepancy_scenario_set_sha256", "amplification_holdout_lots", "amplification_group_disjoint",
    "acquisition_model_version", "amplification_model_version", "acquisition_cost", "cost_unit",
    "observation_std_uncertainty", "eligibility_state", "eligibility_receipt_sha256",
];
const OUTPUT_HEADER: &str = "candidate_id,representativeness_score,amplification_risk,observation_std_uncertainty,acquisition_cost,cost_unit,eligible,eligibility_state,eligibility_receipt_sha256\n";

#[derive(Clone, Debug)]
struct Row {
    candidate_id: String,
    product_id: String,
    representative: f64,
    amplification_risk: f64,
    acquisition_cost: f64,
    cost_unit: String,
    observation_std_uncertainty: f64,
    eligibility_state: String,
    eligibility_receipt: String,
    acceptance_eligible: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("manufacturing-robustness-evidence: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(format!(
            "usage: {} <manufacturing-acceptance-report.csv> <evidence-envelope.csv> <symthaea-adapter.csv>",
            args.first().map(String::as_str).unwrap_or("manufacturing-robustness-evidence")
        ));
    }
    let acceptance = read_acceptance_report(&args[1])?;
    let mut rows = parse(&args[2])?;
    validate_eligibility(&mut rows, &acceptance)?;
    write_adapter(&args[3], &rows)?;
    let eligible = rows.iter().filter(|r| r.eligibility_state == "ELIGIBLE" && r.acceptance_eligible).count();
    eprintln!(
        "robustness evidence envelope valid: candidates={}, eligible={}, blocked={}; score values were not recomputed and artifact bytes were not authenticated",
        rows.len(), eligible, rows.len() - eligible
    );
    Ok(())
}

fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn finite(value: &str, name: &str, row: usize) -> Result<f64, String> {
    let parsed: f64 = value.parse().map_err(|_| format!("row {row}: {name} is not numeric"))?;
    if !parsed.is_finite() {
        return Err(format!("row {row}: {name} must be finite"));
    }
    Ok(parsed)
}

fn integer(value: &str, name: &str, row: usize) -> Result<u32, String> {
    value.parse().map_err(|_| format!("row {row}: {name} must be a non-negative integer"))
}

fn parse(path: &str) -> Result<Vec<Row>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing header"))?.trim_end_matches('\r');
    if header != HEADER.join(",") {
        return Err(format!("{path}: unexpected header; expected {}", HEADER.join(",")));
    }

    let mut rows = Vec::new();
    let mut ids = BTreeSet::new();
    let mut common_cost_unit: Option<String> = None;
    for (index, raw) in lines.enumerate() {
        let row_no = index + 2;
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{path}:{row_no}: blank row"));
        }
        if line.contains('"') {
            return Err(format!("{path}:{row_no}: quoted CSV fields are unsupported"));
        }
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        if f.len() != HEADER.len() {
            return Err(format!("{path}:{row_no}: expected {} columns, got {}", HEADER.len(), f.len()));
        }

        let candidate_id = f[0].to_string();
        if candidate_id.is_empty() || !ids.insert(candidate_id.clone()) {
            return Err(format!("{path}:{row_no}: empty or duplicate candidate_id {candidate_id:?}"));
        }
        for (column, name) in [
            (1, "product_family"), (2, "product_id"), (3, "component_lot_id"),
            (4, "reference_population_id"), (7, "representativeness_method"),
            (12, "amplification_method"), (18, "acquisition_model_version"),
            (19, "amplification_model_version"), (21, "cost_unit"),
        ] {
            if f[column].is_empty() {
                return Err(format!("{path}:{row_no}: {name} is empty"));
            }
        }
        for (column, name) in [
            (5, "reference_population_sha256"), (6, "candidate_descriptor_sha256"),
            (9, "representativeness_validation_sha256"), (14, "amplification_audit_sha256"),
            (15, "discrepancy_scenario_set_sha256"), (24, "eligibility_receipt_sha256"),
        ] {
            if !sha256(f[column]) {
                return Err(format!("{path}:{row_no}: {name} is not a 64-character SHA-256"));
            }
        }
        if !matches!(f[7], "KNN_REFERENCE_ENVELOPE" | "DENSITY_RATIO_CALIBRATED") {
            return Err(format!("{path}:{row_no}: unsupported representativeness_method"));
        }
        if !matches!(f[12], "FROZEN_DISCREPANCY_REPLAY" | "PAIRED_HOLDOUT_ERROR_AUDIT") {
            return Err(format!("{path}:{row_no}: unsupported amplification_method"));
        }

        let representative = finite(f[8], "representativeness_score", row_no)?;
        let amplification_risk = finite(f[13], "amplification_risk", row_no)?;
        if !(0.0..=1.0).contains(&representative) {
            return Err(format!("{path}:{row_no}: representativeness_score must be in [0,1]"));
        }
        if !(0.0..=1.0).contains(&amplification_risk) {
            return Err(format!("{path}:{row_no}: amplification_risk must be in [0,1]"));
        }
        let reference_lots = integer(f[10], "reference_holdout_lots", row_no)?;
        let amplification_lots = integer(f[16], "amplification_holdout_lots", row_no)?;
        if reference_lots < 5 || amplification_lots < 5 {
            return Err(format!("{path}:{row_no}: at least five independent holdout lots are required for each audit"));
        }
        if f[11] != "true" || f[17] != "true" {
            return Err(format!("{path}:{row_no}: reference and amplification audit groups must be disjoint"));
        }
        if f[18] == f[19] {
            return Err(format!("{path}:{row_no}: acquisition and amplification audit models must be independently versioned"));
        }
        let acquisition_cost = finite(f[20], "acquisition_cost", row_no)?;
        let observation_std_uncertainty = finite(f[22], "observation_std_uncertainty", row_no)?;
        if acquisition_cost <= 0.0 || observation_std_uncertainty <= 0.0 {
            return Err(format!("{path}:{row_no}: cost and standard uncertainty must be positive"));
        }
        let cost_unit = f[21].to_string();
        if let Some(unit) = &common_cost_unit {
            if unit != &cost_unit {
                return Err(format!("{path}:{row_no}: cost unit {cost_unit:?} differs from comparison unit {unit:?}"));
            }
        } else {
            common_cost_unit = Some(cost_unit.clone());
        }
        let eligibility_state = f[23].to_string();
        if !matches!(eligibility_state.as_str(), "ELIGIBLE" | "INELIGIBLE" | "UNKNOWN") {
            return Err(format!("{path}:{row_no}: eligibility_state must be ELIGIBLE, INELIGIBLE or UNKNOWN"));
        }

        rows.push(Row {
            candidate_id,
            product_id: f[2].to_string(),
            representative,
            amplification_risk,
            acquisition_cost,
            cost_unit,
            observation_std_uncertainty,
            eligibility_state,
            eligibility_receipt: f[24].to_string(),
            acceptance_eligible: false,
        });
    }
    if rows.is_empty() {
        return Err(format!("{path}: no candidate rows"));
    }
    Ok(rows)
}

fn read_acceptance_report(path: &str) -> Result<std::collections::BTreeMap<String, bool>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read acceptance report {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: empty acceptance report"))?.trim_end_matches('\\r');
    if header != ACCEPTANCE_REPORT_HEADER {
        return Err(format!("{path}: unexpected acceptance report header"));
    }
    let mut output = std::collections::BTreeMap::new();
    for (index, raw) in lines.enumerate() {
        let row_no = index + 2;
        let line = raw.trim_end_matches('\\r');
        if line.trim().is_empty() || line.contains('"') {
            return Err(format!("{path}:{row_no}: blank or quoted acceptance report row"));
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != 15 || fields[0].is_empty() {
            return Err(format!("{path}:{row_no}: expected 15 fields and a product_id"));
        }
        let eligible = match fields[13] {
            "true" => true,
            "false" => false,
            _ => return Err(format!("{path}:{row_no}: malformed eligible_for_authorized_review")),
        };
        if output.insert(fields[0].to_string(), eligible).is_some() {
            return Err(format!("{path}:{row_no}: duplicate product_id in acceptance report"));
        }
    }
    if output.is_empty() {
        return Err(format!("{path}: acceptance report has no product rows"));
    }
    Ok(output)
}

fn validate_eligibility(
    rows: &mut [Row],
    acceptance: &std::collections::BTreeMap<String, bool>,
) -> Result<(), String> {
    for row in rows {
        let accepted = *acceptance.get(&row.product_id).ok_or_else(|| {
            format!("candidate {:?}: product {:?} is absent from manufacturing acceptance report", row.candidate_id, row.product_id)
        })?;
        row.acceptance_eligible = accepted;
        if row.eligibility_state == "ELIGIBLE" && !accepted {
            return Err(format!(
                "candidate {:?}: evidence envelope claims ELIGIBLE but manufacturing acceptance report blocks product {:?}",
                row.candidate_id, row.product_id
            ));
        }
    }
    Ok(())
}

fn write_adapter(path: &str, rows: &[Row]) -> Result<(), String> {
    let mut output = String::from(OUTPUT_HEADER);
    for row in rows {
        let eligible = row.eligibility_state == "ELIGIBLE"
            && row.acceptance_eligible
            && sha256(&row.eligibility_receipt);
        output.push_str(&format!(
            "{},{:.8},{:.8},{:.8},{:.8},{},{},{},{}\n",
            row.candidate_id,
            row.representative,
            row.amplification_risk,
            row.observation_std_uncertainty,
            row.acquisition_cost,
            row.cost_unit,
            eligible,
            row.eligibility_state,
            row.eligibility_receipt
        ));
    }
    fs::write(path, output).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        let digest_a = "a".repeat(64);
        let digest_b = "b".repeat(64);
        format!(
            "{}\nTX-ROBUST-01,POWER_TRANSFORMER,TX-001,LOT-001,REF-PWR-2026,{digest_a},{digest_b},KNN_REFERENCE_ENVELOPE,0.95,{digest_a},8,true,FROZEN_DISCREPANCY_REPLAY,0.05,{digest_b},{digest_a},8,true,thermal-surrogate-v3,independent-audit-v2,125.0,USD_2026,0.002,ELIGIBLE,{digest_b}",
            HEADER.join(",")
        )
    }

    #[test]
    fn accepts_complete_independently_versioned_robustness_envelope() {
        let mut path = std::env::temp_dir();
        path.push(format!("robustness-evidence-{}.csv", std::process::id()));
        fs::write(&path, fixture()).unwrap();
        let parsed = parse(path.to_str().unwrap()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].representative, 0.95);
        assert_eq!(parsed[0].amplification_risk, 0.05);
        assert_eq!(parsed[0].eligibility_state, "ELIGIBLE");
        let _ = fs::remove_file(path);
    }

    fn acceptance_report(eligible: bool) -> String {
        format!(
            "{}\\nTX-001,SERIAL-001,ROUTE-A,true,0,0,4,4,0,0,0,0,0,{},NONE",
            ACCEPTANCE_REPORT_HEADER, eligible
        )
    }

    #[test]
    fn eligibility_requires_the_independent_manufacturing_acceptance_report() {
        let mut path = std::env::temp_dir();
        path.push(format!("acceptance-report-{}.csv", std::process::id()));
        fs::write(&path, acceptance_report(true)).unwrap();
        let acceptance = read_acceptance_report(path.to_str().unwrap()).unwrap();
        let mut rows = parse_from_content(&fixture());
        validate_eligibility(&mut rows, &acceptance).unwrap();
        assert!(rows[0].acceptance_eligible);

        fs::write(&path, acceptance_report(false)).unwrap();
        let blocked = read_acceptance_report(path.to_str().unwrap()).unwrap();
        let mut rows = parse_from_content(&fixture());
        assert!(validate_eligibility(&mut rows, &blocked).unwrap_err().contains("manufacturing acceptance report blocks"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn export_preserves_hard_eligibility_state() {
        let mut path = std::env::temp_dir();
        path.push(format!("robustness-evidence-export-{}.csv", std::process::id()));
        let row = parse_from_content(&fixture());
        let output = path.to_str().unwrap();
        write_adapter(output, &row).unwrap();
        let text = fs::read_to_string(output).unwrap();
        assert!(text.contains("TX-ROBUST-01,0.95000000,0.05000000,0.00200000,125.00000000,USD_2026,true,ELIGIBLE,"));
        let _ = fs::remove_file(path);
    }

    fn parse_from_content(content: &str) -> Vec<Row> {
        let mut path = std::env::temp_dir();
        path.push(format!("robustness-evidence-content-{}.csv", std::process::id()));
        fs::write(&path, content).unwrap();
        let parsed = parse(path.to_str().unwrap()).unwrap();
        let _ = fs::remove_file(path);
        parsed
    }

    #[test]
    fn rejects_non_disjoint_or_too_small_holdout_lots() {
        let bad = fixture().replace(",8,true,FROZEN_DISCREPANCY_REPLAY", ",4,true,FROZEN_DISCREPANCY_REPLAY");
        let mut path = std::env::temp_dir();
        path.push(format!("robustness-evidence-bad-{}.csv", std::process::id()));
        fs::write(&path, bad).unwrap();
        assert!(parse(path.to_str().unwrap()).unwrap_err().contains("five independent"));
        let bad = fixture().replace(",8,true,FROZEN_DISCREPANCY_REPLAY", ",8,false,FROZEN_DISCREPANCY_REPLAY");
        fs::write(&path, bad).unwrap();
        assert!(parse(path.to_str().unwrap()).unwrap_err().contains("disjoint"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_model_self_scoring_and_out_of_range_scores() {
        let same_model = fixture().replace("independent-audit-v2", "thermal-surrogate-v3");
        let mut path = std::env::temp_dir();
        path.push(format!("robustness-evidence-model-{}.csv", std::process::id()));
        fs::write(&path, same_model).unwrap();
        assert!(parse(path.to_str().unwrap()).unwrap_err().contains("independently versioned"));
        let bad_score = fixture().replace(",0.95,", ",1.01,");
        fs::write(&path, bad_score).unwrap();
        assert!(parse(path.to_str().unwrap()).unwrap_err().contains("representativeness_score"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_mixed_cost_units_and_duplicate_candidate_ids() {
        let row = fixture().lines().nth(1).unwrap().to_string();
        let duplicate = format!("{}\n{}\n{}", HEADER.join(","), row, row);
        let mut path = std::env::temp_dir();
        path.push(format!("robustness-evidence-duplicate-{}.csv", std::process::id()));
        fs::write(&path, duplicate).unwrap();
        assert!(parse(path.to_str().unwrap()).unwrap_err().contains("duplicate candidate_id"));
        let mixed = format!("{}\n{}\n{}", HEADER.join(","), row, row.replace("TX-ROBUST-01", "TX-ROBUST-02").replace(",USD_2026,", ",EUR_2026,"));
        fs::write(&path, mixed).unwrap();
        assert!(parse(path.to_str().unwrap()).unwrap_err().contains("cost unit"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_unknown_eligibility_state() {
        let bad = fixture().replace(",ELIGIBLE,", ",UNKNOWNISH,");
        let mut path = std::env::temp_dir();
        path.push(format!("robustness-evidence-eligibility-{}.csv", std::process::id()));
        fs::write(&path, bad).unwrap();
        assert!(parse(path.to_str().unwrap()).unwrap_err().contains("eligibility_state"));
        let _ = fs::remove_file(path);
    }
}
