//! Strict, provenance-preserving import of ShapeBench's public results.csv interchange.
//!
//! This module does not execute ShapeBench, validate its solver, or promote surrogate rewards
//! into physical/aerodynamic evidence. It validates only the integrity and internal consistency
//! of a captured optimizer-result trajectory.

use crate::integrity::sha256_hex;

pub const SHAPEBENCH_RESULTS_SCHEMA: &str = "shapebench-results-import/0.1.0";
pub const SHAPEBENCH_RESULTS_CSV_HEADER: &str = "iteration,design,reward,best_reward";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardDirection {
    Maximize,
    Minimize,
}

impl RewardDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Maximize => "maximize",
            Self::Minimize => "minimize",
        }
    }

    fn best(self, previous: Option<f64>, reward: f64) -> f64 {
        match (self, previous) {
            (Self::Maximize, Some(value)) => value.max(reward),
            (Self::Minimize, Some(value)) => value.min(reward),
            (_, None) => reward,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeBenchRunContext {
    pub task_id: String,
    pub environment_id: String,
    pub method_id: String,
    /// Caller-supplied full 40-character revision claim, not a branch or tag label.
    pub source_commit: String,
    /// Caller-supplied digest reference; the importer validates shape, not artifact bytes.
    pub task_config_sha256: String,
    /// Caller-supplied digest reference; a separate runner/replay step must verify the manifest.
    pub run_manifest_sha256: String,
    /// Optional digest of ShapeBench's companion results.json artifact.
    pub results_json_sha256: Option<String>,
    /// Optional digest of a separate complete evaluation-cost ledger.
    pub cost_ledger_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeBenchResultRow {
    pub iteration: u64,
    /// Preserve the exact CSV-decoded design payload; do not normalize/round geometry here.
    pub design_payload: String,
    pub design_payload_sha256: String,
    pub reward: f64,
    pub best_reward: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShapeBenchImportReport {
    pub schema: String,
    pub context: ShapeBenchRunContext,
    pub reward_direction: RewardDirection,
    pub rows: Vec<ShapeBenchResultRow>,
    pub import_sha256: String,
    pub evidence_level: String,
}

impl ShapeBenchImportReport {
    pub fn cost_ledger_reference_present(&self) -> bool {
        self.context.cost_ledger_sha256.is_some()
    }

    pub fn to_csv(&self) -> String {
        let mut output = String::from(
            "task_id,environment_id,method_id,source_commit,task_config_sha256,run_manifest_sha256,reward_direction,iteration,design_payload_sha256,results_json_sha256,cost_ledger_sha256,reward,best_reward,import_sha256,evidence_level\n",
        );
        for row in &self.rows {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{:.17},{:.17},{},{}\n",
                csv_escape(&self.context.task_id),
                csv_escape(&self.context.environment_id),
                csv_escape(&self.context.method_id),
                self.context.source_commit,
                self.context.task_config_sha256,
                self.context.run_manifest_sha256,
                self.reward_direction.as_str(),
                row.iteration,
                row.design_payload_sha256,
                self.context.results_json_sha256.as_deref().unwrap_or(""),
                self.context.cost_ledger_sha256.as_deref().unwrap_or(""),
                row.reward,
                row.best_reward,
                self.import_sha256,
                self.evidence_level
            ));
        }
        output
    }
}

fn validate_token(value: &str, name: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\0')
        || value.contains(',')
    {
        return Err(format!("{name} must be non-empty and single-line"));
    }
    Ok(())
}

fn is_hex(value: &str, expected_length: usize) -> bool {
    value.len() == expected_length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_context(context: &ShapeBenchRunContext) -> Result<(), String> {
    validate_token(&context.task_id, "task_id")?;
    validate_token(&context.environment_id, "environment_id")?;
    validate_token(&context.method_id, "method_id")?;
    if !is_hex(&context.source_commit, 40) {
        return Err("source_commit must be a full 40-character hexadecimal revision".to_owned());
    }
    if !is_hex(&context.task_config_sha256, 64) {
        return Err("task_config_sha256 must be a 64-character SHA-256 digest".to_owned());
    }
    if !is_hex(&context.run_manifest_sha256, 64) {
        return Err("run_manifest_sha256 must be a 64-character SHA-256 digest".to_owned());
    }
    for (name, digest) in [
        (
            "results_json_sha256",
            context.results_json_sha256.as_deref(),
        ),
        ("cost_ledger_sha256", context.cost_ledger_sha256.as_deref()),
    ] {
        if let Some(value) = digest
            && !is_hex(value, 64)
        {
            return Err(format!("{name} must be a 64-character SHA-256 digest"));
        }
    }
    Ok(())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Parse one RFC-4180-style record. Quoted fields support commas and doubled quotes.
/// Multiline fields are deliberately not accepted by this line-oriented import contract.
fn parse_csv_record(line: &str) -> Result<Vec<String>, String> {
    let characters = line.chars().collect::<Vec<_>>();
    let mut fields = Vec::new();
    let mut index = 0;
    loop {
        let mut field = String::new();
        if characters.get(index) == Some(&'"') {
            index += 1;
            let mut closed = false;
            while index < characters.len() {
                if characters[index] == '"' {
                    if characters.get(index + 1) == Some(&'"') {
                        field.push('"');
                        index += 2;
                    } else {
                        index += 1;
                        closed = true;
                        break;
                    }
                } else {
                    field.push(characters[index]);
                    index += 1;
                }
            }
            if !closed {
                return Err("CSV contains an unterminated quoted field".to_owned());
            }
            if index < characters.len() && characters[index] != ',' {
                return Err("CSV contains data after a closing quote".to_owned());
            }
        } else {
            while index < characters.len() && characters[index] != ',' {
                if characters[index] == '"' {
                    return Err("CSV contains a quote inside an unquoted field".to_owned());
                }
                field.push(characters[index]);
                index += 1;
            }
        }
        fields.push(field);
        if index == characters.len() {
            break;
        }
        if characters[index] != ',' {
            return Err("CSV field delimiter is malformed".to_owned());
        }
        index += 1;
        if index == characters.len() {
            fields.push(String::new());
            break;
        }
    }
    Ok(fields)
}

fn canonical_import_bytes(
    context: &ShapeBenchRunContext,
    direction: RewardDirection,
    rows: &[ShapeBenchResultRow],
) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        SHAPEBENCH_RESULTS_SCHEMA,
        context.task_id.as_str(),
        context.environment_id.as_str(),
        context.method_id.as_str(),
        context.source_commit.as_str(),
        context.task_config_sha256.as_str(),
        context.run_manifest_sha256.as_str(),
        context.results_json_sha256.as_deref().unwrap_or(""),
        context.cost_ledger_sha256.as_deref().unwrap_or(""),
        direction.as_str(),
    ] {
        bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    bytes.extend_from_slice(&(rows.len() as u64).to_le_bytes());
    for row in rows {
        bytes.extend_from_slice(&row.iteration.to_le_bytes());
        bytes.extend_from_slice(&(row.design_payload.len() as u64).to_le_bytes());
        bytes.extend_from_slice(row.design_payload.as_bytes());
        bytes.extend_from_slice(&row.reward.to_bits().to_le_bytes());
        bytes.extend_from_slice(&row.best_reward.to_bits().to_le_bytes());
    }
    bytes
}

/// Import a ShapeBench `results.csv` trajectory. Costs are intentionally not inferred from
/// iteration numbers. A supplied ledger digest is only a reference; its records need a separate verifier.
pub fn import_shape_bench_results_csv(
    input: &str,
    context: ShapeBenchRunContext,
    reward_direction: RewardDirection,
) -> Result<ShapeBenchImportReport, String> {
    validate_context(&context)?;
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "ShapeBench results CSV is empty".to_owned())?
        .trim_end_matches('\r');
    if header != SHAPEBENCH_RESULTS_CSV_HEADER {
        return Err(format!(
            "ShapeBench results CSV header mismatch; expected {SHAPEBENCH_RESULTS_CSV_HEADER}"
        ));
    }

    let mut rows = Vec::new();
    let mut previous_iteration = None;
    let mut running_best = None;
    for (line_index, raw_line) in lines.enumerate() {
        let line_number = line_index + 2;
        if raw_line.trim().is_empty() {
            return Err(format!(
                "line {line_number}: blank CSV record is not permitted"
            ));
        }
        let fields = parse_csv_record(raw_line.trim_end_matches('\r'))
            .map_err(|error| format!("line {line_number}: {error}"))?;
        if fields.len() != 4 {
            return Err(format!(
                "line {line_number}: expected four columns, got {}",
                fields.len()
            ));
        }
        let iteration = fields[0]
            .parse::<u64>()
            .map_err(|error| format!("line {line_number}: invalid iteration: {error}"))?;
        if previous_iteration.is_some_and(|previous| iteration <= previous) {
            return Err(format!(
                "line {line_number}: iteration numbers must be strictly increasing"
            ));
        }
        if fields[1].is_empty()
            || fields[1].contains('\n')
            || fields[1].contains('\r')
            || fields[1].contains('\0')
        {
            return Err(format!(
                "line {line_number}: design payload must be non-empty and single-line"
            ));
        }
        let reward = fields[2]
            .trim()
            .parse::<f64>()
            .map_err(|error| format!("line {line_number}: invalid reward: {error}"))?;
        let best_reward = fields[3]
            .trim()
            .parse::<f64>()
            .map_err(|error| format!("line {line_number}: invalid best_reward: {error}"))?;
        if !reward.is_finite() || !best_reward.is_finite() {
            return Err(format!("line {line_number}: rewards must be finite"));
        }
        let expected_best = reward_direction.best(running_best, reward);
        let tolerance = 1.0e-12 * expected_best.abs().max(best_reward.abs()).max(1.0);
        if (best_reward - expected_best).abs() > tolerance {
            return Err(format!(
                "line {line_number}: best_reward does not equal the running {} reward",
                reward_direction.as_str()
            ));
        }
        let design_payload_sha256 = sha256_hex(fields[1].as_bytes())?;
        rows.push(ShapeBenchResultRow {
            iteration,
            design_payload: fields[1].clone(),
            design_payload_sha256,
            reward,
            best_reward,
        });
        previous_iteration = Some(iteration);
        running_best = Some(expected_best);
    }
    if rows.is_empty() {
        return Err("ShapeBench results CSV must contain at least one observation".to_owned());
    }

    let import_sha256 = sha256_hex(&canonical_import_bytes(&context, reward_direction, &rows))?;
    Ok(ShapeBenchImportReport {
        schema: SHAPEBENCH_RESULTS_SCHEMA.to_owned(),
        context,
        reward_direction,
        rows,
        import_sha256,
        evidence_level: "benchmark_output_import_only_not_solver_validation_or_physical_evidence"
            .to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> ShapeBenchRunContext {
        ShapeBenchRunContext {
            task_id: "superwing-sw001".to_owned(),
            environment_id: "SuperWing".to_owned(),
            method_id: "CMA-ES".to_owned(),
            source_commit: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            task_config_sha256: "1".repeat(64),
            run_manifest_sha256: "2".repeat(64),
            results_json_sha256: None,
            cost_ledger_sha256: None,
        }
    }

    const RESULTS: &str = concat!(
        "iteration,design,reward,best_reward\n",
        "1,\"shape-a,twist=0.1\",0.42,0.42\n",
        "2,\"shape-b,twist=0.2\",0.51,0.51\n",
        "3,\"shape-c,twist=0.15\",0.48,0.51\n"
    );

    #[test]
    fn imports_quoted_design_and_checks_running_best_without_inventing_cost() {
        let report = import_shape_bench_results_csv(RESULTS, context(), RewardDirection::Maximize)
            .expect("valid result trajectory");
        assert_eq!(report.rows.len(), 3);
        assert_eq!(report.rows[0].design_payload, "shape-a,twist=0.1");
        assert!(!report.cost_ledger_reference_present());
        assert_eq!(
            report.evidence_level,
            "benchmark_output_import_only_not_solver_validation_or_physical_evidence"
        );
        assert_eq!(report.to_csv().lines().count(), 4);
        let repeated =
            import_shape_bench_results_csv(RESULTS, context(), RewardDirection::Maximize)
                .expect("deterministic import");
        assert_eq!(report.import_sha256, repeated.import_sha256);
    }

    #[test]
    fn cost_comparison_requires_a_separate_pinned_ledger() {
        let mut run = context();
        run.cost_ledger_sha256 = Some("a".repeat(64));
        let report = import_shape_bench_results_csv(RESULTS, run, RewardDirection::Maximize)
            .expect("valid run");
        assert!(report.cost_ledger_reference_present());
    }

    #[test]
    fn rejects_duplicate_or_non_monotonic_iterations_and_bad_best_reward() {
        let duplicate = RESULTS.replace(
            "3,\"shape-c,twist=0.15\",0.48,0.51",
            "2,\"shape-c,twist=0.15\",0.48,0.51",
        );
        assert!(
            import_shape_bench_results_csv(&duplicate, context(), RewardDirection::Maximize)
                .expect_err("duplicate iteration")
                .contains("strictly increasing")
        );
        let false_best = RESULTS.replace(
            "3,\"shape-c,twist=0.15\",0.48,0.51",
            "3,\"shape-c,twist=0.15\",0.48,0.48",
        );
        assert!(
            import_shape_bench_results_csv(&false_best, context(), RewardDirection::Maximize)
                .expect_err("false best reward")
                .contains("running maximize reward")
        );
    }

    #[test]
    fn rejects_non_finite_rewards_wrong_schema_and_invalid_source_identity() {
        let non_finite = RESULTS.replace(
            "2,\"shape-b,twist=0.2\",0.51,0.51",
            "2,\"shape-b,twist=0.2\",NaN,0.51",
        );
        assert!(
            import_shape_bench_results_csv(&non_finite, context(), RewardDirection::Maximize)
                .expect_err("NaN rejected")
                .contains("finite")
        );
        assert!(
            import_shape_bench_results_csv(
                &RESULTS.replace("best_reward", "best"),
                context(),
                RewardDirection::Maximize
            )
            .is_err()
        );
        let mut bad_source = context();
        bad_source.source_commit = "main".to_owned();
        assert!(
            import_shape_bench_results_csv(RESULTS, bad_source, RewardDirection::Maximize).is_err()
        );
    }

    #[test]
    fn supports_minimization_without_misreading_best_so_far() {
        let input = concat!(
            "iteration,design,reward,best_reward\n",
            "0,\"shape-a\",1.0,1.0\n",
            "1,\"shape-b\",0.8,0.8\n",
            "2,\"shape-c\",0.9,0.8\n"
        );
        let report = import_shape_bench_results_csv(input, context(), RewardDirection::Minimize)
            .expect("valid minimization trajectory");
        assert_eq!(report.rows[2].best_reward, 0.8);
    }
}
