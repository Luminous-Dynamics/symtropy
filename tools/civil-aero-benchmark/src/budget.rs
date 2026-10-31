//! Strict cost accounting for matched-budget engineering experiment-selection benchmarks.
//!
//! Failed operations retain their incurred cost. A budget comparison audits cost fairness,
//! not aerodynamic validity or optimizer quality. Costs may be normalized units, provided the
//! mapping to actual compute/wall-time/engineering work is documented by the benchmark.

use std::collections::{BTreeMap, HashSet};

pub const LEDGER_INPUT_HEADER: &str = "task_id,replicate_id,policy_id,operation_id,operation_kind,fidelity,cost_units,budget_cap_units,wall_seconds,cpu_seconds,success,output_sha256,reason_code";
pub const LEDGER_OUTPUT_HEADER: &str = "task_id,replicate_id,policy_id,budget_cap_units,total_cost_units,remaining_budget_units,wall_seconds,cpu_seconds,operation_count,successful_operations,failed_operations,all_operation_costs_counted,evidence_level";

#[derive(Debug, Clone, PartialEq)]
pub struct LedgerOperation {
    pub task_id: String,
    pub replicate_id: String,
    pub policy_id: String,
    pub operation_id: String,
    pub operation_kind: String,
    pub fidelity: String,
    pub cost_units: f64,
    pub budget_cap_units: f64,
    pub wall_seconds: f64,
    pub cpu_seconds: f64,
    pub success: bool,
    pub output_sha256: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyBudgetSummary {
    pub task_id: String,
    pub replicate_id: String,
    pub policy_id: String,
    pub budget_cap_units: f64,
    pub total_cost_units: f64,
    pub remaining_budget_units: f64,
    pub wall_seconds: f64,
    pub cpu_seconds: f64,
    pub operation_count: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
}

pub fn parse_ledger_csv(input: &str) -> Result<Vec<LedgerOperation>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| "experiment ledger is empty".to_owned())?
        .trim_end_matches('\r');
    if header != LEDGER_INPUT_HEADER {
        return Err(format!(
            "unexpected ledger header; expected exactly: {LEDGER_INPUT_HEADER}"
        ));
    }

    let mut operations = Vec::new();
    let mut seen_ids = HashSet::new();
    for (line_index, raw_line) in lines.enumerate() {
        let line_number = line_index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if line.contains('"') {
            return Err(format!(
                "line {line_number}: quoted CSV fields are not supported"
            ));
        }
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() != LEDGER_INPUT_HEADER.split(',').count() {
            return Err(format!(
                "line {line_number}: expected {} columns, got {}",
                LEDGER_INPUT_HEADER.split(',').count(),
                fields.len()
            ));
        }
        let string_field = |index: usize, name: &str| -> Result<String, String> {
            if fields[index].is_empty() {
                Err(format!("line {line_number}: {name} must not be empty"))
            } else {
                Ok(fields[index].to_owned())
            }
        };
        let number = |index: usize, name: &str| -> Result<f64, String> {
            let value = fields[index]
                .parse::<f64>()
                .map_err(|_| format!("line {line_number}: {name} must be numeric"))?;
            if !value.is_finite() {
                return Err(format!("line {line_number}: {name} must be finite"));
            }
            Ok(value)
        };
        let success = match fields[10] {
            "true" => true,
            "false" => false,
            _ => return Err(format!("line {line_number}: success must be true or false")),
        };

        let operation = LedgerOperation {
            task_id: string_field(0, "task_id")?,
            replicate_id: string_field(1, "replicate_id")?,
            policy_id: string_field(2, "policy_id")?,
            operation_id: string_field(3, "operation_id")?,
            operation_kind: string_field(4, "operation_kind")?,
            fidelity: string_field(5, "fidelity")?,
            cost_units: number(6, "cost_units")?,
            budget_cap_units: number(7, "budget_cap_units")?,
            wall_seconds: number(8, "wall_seconds")?,
            cpu_seconds: number(9, "cpu_seconds")?,
            success,
            output_sha256: string_field(11, "output_sha256")?,
            reason_code: string_field(12, "reason_code")?,
        };
        validate_operation(&operation, line_number)?;

        let unique_key = (
            operation.task_id.clone(),
            operation.replicate_id.clone(),
            operation.policy_id.clone(),
            operation.operation_id.clone(),
        );
        if !seen_ids.insert(unique_key) {
            return Err(format!(
                "line {line_number}: duplicate operation_id {} within task/replicate/policy",
                operation.operation_id
            ));
        }
        operations.push(operation);
    }

    if operations.is_empty() {
        return Err("experiment ledger must contain at least one operation".to_owned());
    }
    Ok(operations)
}

fn validate_operation(operation: &LedgerOperation, line_number: usize) -> Result<(), String> {
    const KINDS: &[&str] = &[
        "sampling",
        "low_fidelity",
        "high_fidelity",
        "tuning",
        "orchestration",
        "audit",
        "reference_evaluation",
    ];
    const FIDELITIES: &[&str] = &["none", "low", "medium", "high"];

    if !KINDS.contains(&operation.operation_kind.as_str()) {
        return Err(format!(
            "line {line_number}: unsupported operation_kind {}",
            operation.operation_kind
        ));
    }
    if !FIDELITIES.contains(&operation.fidelity.as_str()) {
        return Err(format!(
            "line {line_number}: fidelity must be none, low, medium, or high"
        ));
    }
    for (name, value) in [
        ("cost_units", operation.cost_units),
        ("budget_cap_units", operation.budget_cap_units),
        ("wall_seconds", operation.wall_seconds),
        ("cpu_seconds", operation.cpu_seconds),
    ] {
        if !value.is_finite() {
            return Err(format!("line {line_number}: {name} must be finite"));
        }
    }
    if operation.cost_units < 0.0 {
        return Err(format!(
            "line {line_number}: cost_units must be non-negative"
        ));
    }
    if operation.budget_cap_units <= 0.0 {
        return Err(format!(
            "line {line_number}: budget_cap_units must be greater than zero"
        ));
    }
    if operation.wall_seconds < 0.0 || operation.cpu_seconds < 0.0 {
        return Err(format!(
            "line {line_number}: time costs must be non-negative"
        ));
    }
    if operation.success {
        if !is_sha256_hex(&operation.output_sha256) {
            return Err(format!(
                "line {line_number}: successful operation needs a 64-character hexadecimal output_sha256"
            ));
        }
        if operation.reason_code != "-" {
            return Err(format!(
                "line {line_number}: successful operation must use '-' for reason_code"
            ));
        }
    } else if operation.output_sha256 != "-" || operation.reason_code == "-" {
        return Err(format!(
            "line {line_number}: failed operation must use '-' output_sha256 and provide a reason_code"
        ));
    }
    Ok(())
}

pub fn summarize_matched_budget(
    operations: &[LedgerOperation],
) -> Result<Vec<PolicyBudgetSummary>, String> {
    if operations.is_empty() {
        return Err("matched-budget audit requires at least one operation".to_owned());
    }

    #[derive(Default)]
    struct Accumulator {
        cap: Option<f64>,
        total_cost: f64,
        wall_seconds: f64,
        cpu_seconds: f64,
        count: usize,
        successful: usize,
        failed: usize,
    }

    let mut groups = BTreeMap::<(String, String, String), Accumulator>::new();
    let mut policy_counts = BTreeMap::<(String, String), HashSet<String>>::new();

    for operation in operations {
        validate_operation(operation, 0)?;
        let key = (
            operation.task_id.clone(),
            operation.replicate_id.clone(),
            operation.policy_id.clone(),
        );
        let acc = groups.entry(key.clone()).or_default();
        if let Some(cap) = acc.cap {
            if (cap - operation.budget_cap_units).abs() > 1e-9 {
                return Err(format!(
                    "budget cap differs within task {} replicate {} policy {}",
                    operation.task_id, operation.replicate_id, operation.policy_id
                ));
            }
        } else {
            acc.cap = Some(operation.budget_cap_units);
        }
        acc.total_cost += operation.cost_units;
        acc.wall_seconds += operation.wall_seconds;
        acc.cpu_seconds += operation.cpu_seconds;
        acc.count += 1;
        if operation.success {
            acc.successful += 1;
        } else {
            // Failed/aborted operations still consumed budget and remain in total_cost.
            acc.failed += 1;
        }
        policy_counts
            .entry((operation.task_id.clone(), operation.replicate_id.clone()))
            .or_default()
            .insert(operation.policy_id.clone());
    }

    for ((task_id, replicate_id, policy_id), acc) in &groups {
        let cap = acc.cap.expect("cap is set for every populated group");
        if acc.total_cost > cap + 1e-9 {
            return Err(format!(
                "policy {policy_id} exceeded budget cap for task {task_id} replicate {replicate_id}: {} > {cap}",
                acc.total_cost
            ));
        }
    }

    for ((task_id, replicate_id), policies) in &policy_counts {
        if policies.len() < 2 {
            return Err(format!(
                "matched-budget comparison requires at least two policies for task {task_id} replicate {replicate_id}"
            ));
        }
        let policy_caps: Vec<f64> = groups
            .iter()
            .filter_map(|((task, replicate, _), acc)| {
                if task == task_id && replicate == replicate_id {
                    acc.cap
                } else {
                    None
                }
            })
            .collect();
        if policy_caps
            .iter()
            .any(|cap| (cap - policy_caps[0]).abs() > 1e-9)
        {
            return Err(format!(
                "policies have different budget caps for task {task_id} replicate {replicate_id}"
            ));
        }
    }

    Ok(groups
        .into_iter()
        .map(|((task_id, replicate_id, policy_id), acc)| {
            let cap = acc.cap.expect("cap is set for every populated group");
            PolicyBudgetSummary {
                task_id,
                replicate_id,
                policy_id,
                budget_cap_units: cap,
                total_cost_units: acc.total_cost,
                remaining_budget_units: cap - acc.total_cost,
                wall_seconds: acc.wall_seconds,
                cpu_seconds: acc.cpu_seconds,
                operation_count: acc.count,
                successful_operations: acc.successful,
                failed_operations: acc.failed,
            }
        })
        .collect())
}

pub fn summary_csv_row(summary: &PolicyBudgetSummary) -> String {
    format!(
        "{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},{},{},true,ledger_audit_only",
        summary.task_id,
        summary.replicate_id,
        summary.policy_id,
        summary.budget_cap_units,
        summary.total_cost_units,
        summary.remaining_budget_units,
        summary.wall_seconds,
        summary.cpu_seconds,
        summary.operation_count,
        summary.successful_operations,
        summary.failed_operations
    )
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn operation(
        policy: &str,
        op_id: &str,
        kind: &str,
        cost: f64,
        success: bool,
        cap: f64,
    ) -> LedgerOperation {
        LedgerOperation {
            task_id: "naca-smoke".to_owned(),
            replicate_id: "seed-1".to_owned(),
            policy_id: policy.to_owned(),
            operation_id: op_id.to_owned(),
            operation_kind: kind.to_owned(),
            fidelity: "high".to_owned(),
            cost_units: cost,
            budget_cap_units: cap,
            wall_seconds: 0.5 * cost,
            cpu_seconds: 0.4 * cost,
            success,
            output_sha256: if success { digest('a') } else { "-".to_owned() },
            reason_code: if success {
                "-".to_owned()
            } else {
                "solver_nonconverged".to_owned()
            },
        }
    }

    #[test]
    fn failed_solver_cost_is_counted_and_policy_caps_are_matched() {
        let rows = vec![
            operation("random", "r1", "high_fidelity", 1.0, true, 5.0),
            operation("random", "r2", "high_fidelity", 1.5, false, 5.0),
            operation("voi", "v1", "high_fidelity", 2.0, true, 5.0),
            operation("voi", "v2", "audit", 0.5, true, 5.0),
        ];
        let summary = summarize_matched_budget(&rows).expect("valid matched budget");
        let random = summary
            .iter()
            .find(|s| s.policy_id == "random")
            .expect("random");
        let voi = summary.iter().find(|s| s.policy_id == "voi").expect("voi");
        assert!((random.total_cost_units - 2.5).abs() < 1e-12);
        assert_eq!(random.failed_operations, 1);
        assert_eq!(random.remaining_budget_units, 2.5);
        assert!((voi.total_cost_units - 2.5).abs() < 1e-12);
        assert_eq!(voi.failed_operations, 0);
    }

    #[test]
    fn rejects_budget_mismatch_overrun_and_single_policy_groups() {
        let a = operation("random", "r1", "high_fidelity", 1.0, true, 5.0);
        let b = operation("voi", "v1", "high_fidelity", 1.0, true, 6.0);
        assert!(summarize_matched_budget(&[a.clone(), b]).is_err());

        let over_a = operation("random", "r1", "high_fidelity", 6.0, true, 5.0);
        let over_b = operation("voi", "v1", "high_fidelity", 1.0, true, 5.0);
        assert!(summarize_matched_budget(&[over_a, over_b]).is_err());
        assert!(summarize_matched_budget(&[a]).is_err());
    }

    #[test]
    fn rejects_unhashed_success_failed_op_without_reason_and_duplicate_id() {
        let mut bad_hash = operation("random", "r1", "high_fidelity", 1.0, true, 5.0);
        bad_hash.output_sha256 = "-".to_owned();
        assert!(validate_operation(&bad_hash, 1).is_err());

        let mut bad_failure = operation("random", "r1", "high_fidelity", 1.0, false, 5.0);
        bad_failure.reason_code = "-".to_owned();
        assert!(validate_operation(&bad_failure, 1).is_err());

        let duplicate_a = operation("random", "same", "high_fidelity", 1.0, true, 5.0);
        let duplicate_b = duplicate_a.clone();
        assert!(parse_ledger_csv(&serialize(&[duplicate_a, duplicate_b])).is_err());
    }

    #[test]
    fn parser_rejects_malformed_rows_and_nonfinite_costs() {
        assert!(parse_ledger_csv("task_id,wrong\nx,y\n").is_err());
        let mut bad = operation("random", "r1", "high_fidelity", 1.0, true, 5.0);
        bad.cost_units = f64::INFINITY;
        assert!(validate_operation(&bad, 1).is_err());
    }

    fn serialize(operations: &[LedgerOperation]) -> String {
        let mut output = format!("{LEDGER_INPUT_HEADER}\n");
        for operation in operations {
            output.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                operation.task_id,
                operation.replicate_id,
                operation.policy_id,
                operation.operation_id,
                operation.operation_kind,
                operation.fidelity,
                operation.cost_units,
                operation.budget_cap_units,
                operation.wall_seconds,
                operation.cpu_seconds,
                operation.success,
                operation.output_sha256,
                operation.reason_code
            ));
        }
        output
    }
}
