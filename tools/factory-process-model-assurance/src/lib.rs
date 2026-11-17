use std::collections::{BTreeMap, BTreeSet};

const TASKS: &str = include_str!("../tasks-v0.1.tsv");
const OBLIGATIONS: &str = include_str!("../obligations-v0.1.tsv");
const MUTATIONS: &str = include_str!("../mutations-v0.1.tsv");
const REFERENCE_TRACE: &str = include_str!("../reference-trace-v0.1.tsv");

#[derive(Clone, Debug)]
struct Task {
    id: String,
    required_obligations: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct Obligation {
    expected_constraint_family: String,
    criticality: String,
}

#[derive(Clone, Debug)]
struct Mutation {
    id: String,
    target_obligation_id: String,
    objective_preserving_candidate: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TraceEntry {
    disposition: String,
    constraint_family: String,
    constraint_ref: String,
    scope_ref: String,
    evidence_ref: String,
    evidence_state: String,
    disposition_reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskResult {
    pub task_id: String,
    pub required_obligation_count: usize,
    pub represented_obligation_count: usize,
    pub blocking_obligations: Vec<String>,
    pub trace_contract_clear: bool,
}

fn parse_rows(input: &str, columns: usize, source: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    for (index, line) in input.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
        if fields.len() != columns {
            return Err(format!(
                "{source}: line {} has {} columns; expected {columns}",
                index + 1,
                fields.len()
            ));
        }
        if fields.iter().any(|field| field.trim().is_empty()) {
            return Err(format!(
                "{source}: line {} contains a blank field",
                index + 1
            ));
        }
        rows.push(fields);
    }
    Ok(rows)
}

fn split_ids(value: &str) -> BTreeSet<String> {
    if value == "-" || value.is_empty() {
        return BTreeSet::new();
    }
    value.split(';').map(str::to_owned).collect()
}

fn parse_obligations(input: &str) -> Result<BTreeMap<String, Obligation>, String> {
    let rows = parse_rows(input, 8, "obligations-v0.1.tsv")?;
    let mut obligations = BTreeMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        if row[0] == "-" || row[7] == "-" {
            return Err(format!(
                "obligations-v0.1.tsv: line {line}: obligation identity and family are required"
            ));
        }
        if !matches!(row[4].as_str(), "CRITICAL" | "HIGH" | "MEDIUM" | "LOW") {
            return Err(format!(
                "obligations-v0.1.tsv: line {line}: invalid criticality"
            ));
        }
        let obligation = Obligation {
            expected_constraint_family: row[7].clone(),
            criticality: row[4].clone(),
        };
        if obligations.insert(row[0].clone(), obligation).is_some() {
            return Err(format!(
                "obligations-v0.1.tsv: duplicate obligation id {}",
                row[0]
            ));
        }
    }
    if obligations.is_empty() {
        return Err("obligations-v0.1.tsv: empty obligation manifest".to_owned());
    }
    Ok(obligations)
}

fn parse_tasks(
    input: &str,
    obligations: &BTreeMap<String, Obligation>,
) -> Result<BTreeMap<String, Task>, String> {
    let rows = parse_rows(input, 15, "tasks-v0.1.tsv")?;
    let mut tasks = BTreeMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        if !matches!(row[1].as_str(), "1" | "2" | "3") {
            return Err(format!("tasks-v0.1.tsv: line {line}: invalid task level"));
        }
        let required = split_ids(&row[10]);
        if required.is_empty() {
            return Err(format!(
                "tasks-v0.1.tsv: line {line}: task has no critical obligations"
            ));
        }
        for id in &required {
            if !obligations.contains_key(id) {
                return Err(format!(
                    "tasks-v0.1.tsv: line {line}: unknown obligation {id}"
                ));
            }
        }
        let task = Task {
            id: row[0].clone(),
            required_obligations: required,
        };
        if tasks.insert(row[0].clone(), task).is_some() {
            return Err(format!("tasks-v0.1.tsv: duplicate task id {}", row[0]));
        }
    }
    if tasks.is_empty() {
        return Err("tasks-v0.1.tsv: empty task manifest".to_owned());
    }
    Ok(tasks)
}

fn parse_mutations(
    input: &str,
    obligations: &BTreeMap<String, Obligation>,
) -> Result<Vec<Mutation>, String> {
    let rows = parse_rows(input, 8, "mutations-v0.1.tsv")?;
    let mut mutations = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        if !ids.insert(row[0].clone()) {
            return Err(format!(
                "mutations-v0.1.tsv: duplicate mutation id {}",
                row[0]
            ));
        }
        if !obligations.contains_key(&row[2]) {
            return Err(format!(
                "mutations-v0.1.tsv: line {line}: unknown target obligation {}",
                row[2]
            ));
        }
        let objective_preserving_candidate = match row[5].as_str() {
            "true" => true,
            "false" => false,
            _ => {
                return Err(format!(
                    "mutations-v0.1.tsv: line {line}: objective-preserving flag must be true or false"
                ));
            }
        };
        mutations.push(Mutation {
            id: row[0].clone(),
            target_obligation_id: row[2].clone(),
            objective_preserving_candidate,
        });
    }
    if mutations.is_empty() {
        return Err("mutations-v0.1.tsv: empty mutation manifest".to_owned());
    }
    Ok(mutations)
}

fn parse_trace(
    input: &str,
    tasks: &BTreeMap<String, Task>,
    obligations: &BTreeMap<String, Obligation>,
) -> Result<BTreeMap<(String, String), TraceEntry>, String> {
    let rows = parse_rows(input, 9, "reference-trace-v0.1.tsv")?;
    let mut trace = BTreeMap::new();
    for (index, row) in rows.into_iter().enumerate() {
        let line = index + 2;
        let task = tasks.get(&row[0]).ok_or_else(|| {
            format!(
                "reference-trace-v0.1.tsv: line {line}: unknown task {}",
                row[0]
            )
        })?;
        if !task.required_obligations.contains(&row[1]) {
            return Err(format!(
                "reference-trace-v0.1.tsv: line {line}: obligation {} is not required by task {}",
                row[1], row[0]
            ));
        }
        if !obligations.contains_key(&row[1]) {
            return Err(format!(
                "reference-trace-v0.1.tsv: line {line}: unknown obligation {}",
                row[1]
            ));
        }
        if !matches!(
            row[2].as_str(),
            "SATISFIED" | "NOT_APPLICABLE" | "UNSUPPORTED" | "UNKNOWN"
        ) {
            return Err(format!(
                "reference-trace-v0.1.tsv: line {line}: invalid disposition"
            ));
        }
        if !matches!(row[7].as_str(), "CURRENT" | "STALE" | "UNKNOWN") {
            return Err(format!(
                "reference-trace-v0.1.tsv: line {line}: invalid evidence state"
            ));
        }
        let entry = TraceEntry {
            disposition: row[2].clone(),
            constraint_family: row[3].clone(),
            constraint_ref: row[4].clone(),
            scope_ref: row[5].clone(),
            evidence_ref: row[6].clone(),
            evidence_state: row[7].clone(),
            disposition_reason: row[8].clone(),
        };
        let key = (row[0].clone(), row[1].clone());
        if trace.insert(key.clone(), entry).is_some() {
            return Err(format!(
                "reference-trace-v0.1.tsv: duplicate task/obligation pair {} / {}",
                key.0, key.1
            ));
        }
    }
    if trace.is_empty() {
        return Err("reference-trace-v0.1.tsv: empty trace".to_owned());
    }
    Ok(trace)
}

fn trace_entry_blockers(entry: &TraceEntry, obligation: &Obligation) -> Vec<String> {
    let mut blockers = Vec::new();
    match entry.disposition.as_str() {
        "SATISFIED" => {
            if entry.constraint_family != obligation.expected_constraint_family {
                blockers.push("CONSTRAINT_FAMILY_MISMATCH".to_owned());
            }
            if entry.constraint_ref == "-" {
                blockers.push("MISSING_CONSTRAINT_REFERENCE".to_owned());
            }
            if entry.scope_ref == "-" {
                blockers.push("MISSING_SCOPE_REFERENCE".to_owned());
            }
            if entry.evidence_ref == "-" {
                blockers.push("MISSING_EVIDENCE_REFERENCE".to_owned());
            }
            if entry.evidence_state != "CURRENT" {
                blockers.push("EVIDENCE_NOT_CURRENT".to_owned());
            }
            if entry.disposition_reason != "-" {
                blockers.push("SATISFIED_WITH_UNEXPECTED_REASON".to_owned());
            }
        }
        "NOT_APPLICABLE" => {
            if entry.disposition_reason == "-" {
                blockers.push("NOT_APPLICABLE_WITHOUT_REASON".to_owned());
            }
            if entry.constraint_ref != "-" || entry.constraint_family != "-" {
                blockers.push("NOT_APPLICABLE_HAS_CONSTRAINT".to_owned());
            }
            if entry.evidence_state != "CURRENT" || entry.evidence_ref == "-" {
                blockers.push("NOT_APPLICABLE_WITHOUT_CURRENT_EVIDENCE".to_owned());
            }
        }
        "UNSUPPORTED" => blockers.push("UNSUPPORTED_OBLIGATION".to_owned()),
        "UNKNOWN" => blockers.push("UNKNOWN_OBLIGATION".to_owned()),
        _ => blockers.push("INVALID_DISPOSITION".to_owned()),
    }
    if obligation.criticality == "CRITICAL" && entry.disposition != "SATISFIED" {
        blockers.push("CRITICAL_OBLIGATION_NOT_SATISFIED".to_owned());
    }
    blockers
}

fn evaluate(
    tasks: &BTreeMap<String, Task>,
    obligations: &BTreeMap<String, Obligation>,
    trace: &BTreeMap<(String, String), TraceEntry>,
) -> Vec<TaskResult> {
    let mut results = Vec::new();
    for task in tasks.values() {
        let mut blockers = Vec::new();
        let mut represented = 0;
        for obligation_id in &task.required_obligations {
            let key = (task.id.clone(), obligation_id.clone());
            match trace.get(&key) {
                None => blockers.push(format!("{obligation_id}:MISSING_TRACE")),
                Some(entry) => {
                    represented += 1;
                    let obligation = obligations
                        .get(obligation_id)
                        .expect("task parser validated obligation");
                    for blocker in trace_entry_blockers(entry, obligation) {
                        blockers.push(format!("{obligation_id}:{blocker}"));
                    }
                }
            }
        }
        blockers.sort();
        results.push(TaskResult {
            task_id: task.id.clone(),
            required_obligation_count: task.required_obligations.len(),
            represented_obligation_count: represented,
            trace_contract_clear: blockers.is_empty(),
            blocking_obligations: blockers,
        });
    }
    results
}

fn load_reference() -> Result<
    (
        BTreeMap<String, Task>,
        BTreeMap<String, Obligation>,
        Vec<Mutation>,
        BTreeMap<(String, String), TraceEntry>,
    ),
    String,
> {
    let obligations = parse_obligations(OBLIGATIONS)?;
    let tasks = parse_tasks(TASKS, &obligations)?;
    let mutations = parse_mutations(MUTATIONS, &obligations)?;
    let trace = parse_trace(REFERENCE_TRACE, &tasks, &obligations)?;
    Ok((tasks, obligations, mutations, trace))
}

pub fn evaluate_reference_trace() -> Result<Vec<TaskResult>, String> {
    let (tasks, obligations, _mutations, trace) = load_reference()?;
    Ok(evaluate(&tasks, &obligations, &trace))
}

pub fn render_tsv(results: &[TaskResult]) -> String {
    let mut output = String::from(
        "task_id\trequired_obligation_count\trepresented_obligation_count\tblocking_obligations\ttrace_contract_clear\n",
    );
    for result in results {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            result.task_id,
            result.required_obligation_count,
            result.represented_obligation_count,
            if result.blocking_obligations.is_empty() {
                "-".to_owned()
            } else {
                result.blocking_obligations.join(";")
            },
            result.trace_contract_clear
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_trace_covers_all_task_obligations() {
        let (tasks, obligations, mutations, trace) =
            load_reference().expect("reference data parses");
        let results = evaluate(&tasks, &obligations, &trace);
        assert_eq!(results.len(), 18);
        assert_eq!(trace.len(), 40);
        assert!(results.iter().all(|result| result.trace_contract_clear));
        assert_eq!(mutations.len(), 16);
    }

    #[test]
    fn every_mutation_target_is_a_task_obligation() {
        let (tasks, obligations, mutations, trace) =
            load_reference().expect("reference data parses");
        for mutation in mutations {
            assert!(!mutation.id.is_empty());
            assert!(obligations.contains_key(&mutation.target_obligation_id));
            assert!(tasks.values().any(|task| {
                task.required_obligations
                    .contains(&mutation.target_obligation_id)
            }));
            assert!(
                trace
                    .keys()
                    .any(|(_, obligation)| obligation == &mutation.target_obligation_id)
            );
        }
    }

    #[test]
    fn missing_critical_trace_blocks_task() {
        let (tasks, obligations, _mutations, mut trace) =
            load_reference().expect("reference data parses");
        let key = ("E01".to_owned(), "OB-MACHINE-AVAIL".to_owned());
        trace.remove(&key);
        let results = evaluate(&tasks, &obligations, &trace);
        let result = results
            .iter()
            .find(|result| result.task_id == "E01")
            .unwrap();
        assert!(!result.trace_contract_clear);
        assert!(
            result
                .blocking_obligations
                .iter()
                .any(|item| item.contains("MISSING_TRACE"))
        );
    }

    #[test]
    fn stale_evidence_blocks_satisfied_obligation() {
        let (tasks, obligations, _mutations, mut trace) =
            load_reference().expect("reference data parses");
        let key = ("E01".to_owned(), "OB-MACHINE-AVAIL".to_owned());
        trace.get_mut(&key).unwrap().evidence_state = "STALE".to_owned();
        let result = evaluate(&tasks, &obligations, &trace)
            .into_iter()
            .find(|result| result.task_id == "E01")
            .unwrap();
        assert!(!result.trace_contract_clear);
        assert!(
            result
                .blocking_obligations
                .iter()
                .any(|item| item.contains("EVIDENCE_NOT_CURRENT"))
        );
    }

    #[test]
    fn unknown_obligation_blocks_critical_task() {
        let (tasks, obligations, _mutations, mut trace) =
            load_reference().expect("reference data parses");
        let key = ("E01".to_owned(), "OB-MACHINE-AVAIL".to_owned());
        trace.get_mut(&key).unwrap().disposition = "UNKNOWN".to_owned();
        let result = evaluate(&tasks, &obligations, &trace)
            .into_iter()
            .find(|result| result.task_id == "E01")
            .unwrap();
        assert!(!result.trace_contract_clear);
        assert!(
            result
                .blocking_obligations
                .iter()
                .any(|item| item.contains("UNKNOWN_OBLIGATION"))
        );
    }

    #[test]
    fn constraint_family_mismatch_blocks_task() {
        let (tasks, obligations, _mutations, mut trace) =
            load_reference().expect("reference data parses");
        let key = ("E01".to_owned(), "OB-MACHINE-AVAIL".to_owned());
        trace.get_mut(&key).unwrap().constraint_family = "machine_preference".to_owned();
        let result = evaluate(&tasks, &obligations, &trace)
            .into_iter()
            .find(|result| result.task_id == "E01")
            .unwrap();
        assert!(!result.trace_contract_clear);
        assert!(
            result
                .blocking_obligations
                .iter()
                .any(|item| item.contains("CONSTRAINT_FAMILY_MISMATCH"))
        );
    }

    #[test]
    fn objective_preserving_flags_are_candidates_not_results() {
        let (_, _, mutations, _) = load_reference().expect("reference data parses");
        assert!(
            mutations
                .iter()
                .any(|mutation| mutation.objective_preserving_candidate)
        );
        // The trace checker does not solve mutated models or confirm objective preservation.
        // That requires the later paired exact-solver + semantic-oracle campaign.
    }
}
