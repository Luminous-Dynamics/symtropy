// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const SIZES: [u64; 10] = [
    10, 50, 100, 500, 1_000, 5_000, 10_000, 100_000, 1_000_000, 10_000_000,
];
const CLASSES: [&str; 3] = ["MCAR", "MAR", "MNAR"];

#[derive(Clone, Copy)]
struct ExpectedExperiment {
    id: &'static str,
    environment: &'static str,
    mechanism: &'static str,
    states: u64,
    exceptions: usize,
}

const EXPECTED: [ExpectedExperiment; 5] = [
    ExpectedExperiment {
        id: "icu/smar",
        environment: "icu",
        mechanism: "simple_mar",
        states: 800,
        exceptions: 33,
    },
    ExpectedExperiment {
        id: "icu/mnar-ident",
        environment: "icu",
        mechanism: "identifiable_mnar",
        states: 800,
        exceptions: 36,
    },
    ExpectedExperiment {
        id: "icu/mnar-unident",
        environment: "icu",
        mechanism: "unidentifiable_mnar",
        states: 800,
        exceptions: 15,
    },
    ExpectedExperiment {
        id: "predator-factored/smar",
        environment: "predator",
        mechanism: "simple_mar",
        states: 626,
        exceptions: 0,
    },
    ExpectedExperiment {
        id: "predator-factored/mnar-unident",
        environment: "predator",
        mechanism: "unidentifiable_mnar",
        states: 626,
        exceptions: 39,
    },
];

#[derive(Debug)]
struct Contract {
    id: String,
    environment: String,
    mechanism: String,
    states: u64,
    repetitions: usize,
    size_count: usize,
    class_count: usize,
    learning_rows: usize,
    finished_rows: usize,
    exception_rows: usize,
    exception_classes: Vec<String>,
    known_belief_collapse_rows: usize,
    baseline_count: usize,
    baseline_failed: usize,
    mconst_count: usize,
    mconst_failed: usize,
    pomcp_count: usize,
    pomcp_failed: usize,
    baseline_ids: Vec<String>,
    algorithms: Vec<String>,
    oracle_mean: f64,
    random_mean: f64,
}

#[derive(Debug)]
struct Run {
    experiment_id: String,
    dataset_size: u64,
    seed: u64,
    class: String,
    algorithm: String,
    finished: bool,
    exception: bool,
    exception_kind: String,
    failure_reason_known: bool,
    mean_present: bool,
    atv_present: bool,
    wtv_present: bool,
    atv: f64,
    wtv: f64,
    mean: Option<f64>,
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid boolean: {value}")),
    }
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid {field}: {value}"))
}

fn parse_f64(value: &str, field: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("invalid {field}: {value}"))?;
    if !parsed.is_finite() {
        return Err(format!("non-finite {field}"));
    }
    Ok(parsed)
}

fn parse_usize(value: &str, field: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid {field}: {value}"))
}

fn parse_contract(fields: &[&str]) -> Result<Contract, String> {
    if fields.len() != 23 || fields[0] != "contract" {
        return Err(format!(
            "contract row must have 23 fields, got {}",
            fields.len()
        ));
    }
    let mut baseline_ids: Vec<String> = fields[19].split(';').map(str::to_owned).collect();
    baseline_ids.sort();
    let mut algorithms: Vec<String> = fields[20].split(';').map(str::to_owned).collect();
    algorithms.sort();
    Ok(Contract {
        id: fields[1].to_owned(),
        environment: fields[2].to_owned(),
        mechanism: fields[3].to_owned(),
        states: parse_u64(fields[4], "state count")?,
        repetitions: parse_usize(fields[5], "repetitions")?,
        size_count: parse_usize(fields[6], "dataset size count")?,
        class_count: parse_usize(fields[7], "class count")?,
        learning_rows: parse_usize(fields[8], "learning row count")?,
        finished_rows: parse_usize(fields[9], "finished row count")?,
        exception_rows: parse_usize(fields[10], "exception row count")?,
        exception_classes: fields[11]
            .split(';')
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .collect(),
        known_belief_collapse_rows: parse_usize(fields[12], "belief-collapse exception count")?,
        baseline_count: parse_usize(fields[13], "baseline count")?,
        baseline_failed: parse_usize(fields[14], "failed baseline count")?,
        mconst_count: parse_usize(fields[15], "M-constant count")?,
        mconst_failed: parse_usize(fields[16], "failed M-constant count")?,
        pomcp_count: parse_usize(fields[17], "POMCP count")?,
        pomcp_failed: parse_usize(fields[18], "failed POMCP count")?,
        baseline_ids,
        algorithms,
        oracle_mean: parse_f64(fields[21], "oracle policy value")?,
        random_mean: parse_f64(fields[22], "random policy value")?,
    })
}

fn algorithm_for(class: &str) -> Option<&'static str> {
    match class {
        "MCAR" => Some("AsMAR (MCAR)"),
        "MAR" => Some("AsMAR (MAR)"),
        "MNAR" => Some("AIMI"),
        _ => None,
    }
}

fn parse_run(fields: &[&str]) -> Result<Run, String> {
    if fields.len() != 16 || fields[0] != "run" {
        return Err(format!("run row must have 16 fields, got {}", fields.len()));
    }
    let mean = if fields[13].is_empty() {
        None
    } else {
        Some(
            fields[13]
                .parse::<f64>()
                .map_err(|_| "invalid mean".to_owned())?,
        )
    };
    let atv = fields[11]
        .parse::<f64>()
        .map_err(|_| "invalid ATV".to_owned())?;
    let wtv = fields[12]
        .parse::<f64>()
        .map_err(|_| "invalid WTV".to_owned())?;
    if !atv.is_finite() || !(0.0..=1.0).contains(&atv) {
        return Err(format!("ATV outside [0,1]: {atv}"));
    }
    if !wtv.is_finite() || !(0.0..=1.0).contains(&wtv) {
        return Err(format!("WTV outside [0,1]: {wtv}"));
    }
    if let Some(value) = mean
        && !value.is_finite()
    {
        return Err("non-finite policy mean".to_owned());
    }
    Ok(Run {
        experiment_id: fields[1].to_owned(),
        dataset_size: parse_u64(fields[2], "dataset size")?,
        seed: parse_u64(fields[3], "seed")?,
        class: fields[4].to_owned(),
        algorithm: fields[5].to_owned(),
        finished: parse_bool(fields[6])?,
        exception: parse_bool(fields[7])?,
        exception_kind: fields[14].to_owned(),
        failure_reason_known: parse_bool(fields[15])?,
        mean_present: parse_bool(fields[8])?,
        atv_present: parse_bool(fields[9])?,
        wtv_present: parse_bool(fields[10])?,
        atv,
        wtv,
        mean,
    })
}

fn validate_contract(contract: &Contract, expected: ExpectedExperiment) -> Result<(), String> {
    if contract.id != expected.id
        || contract.environment != expected.environment
        || contract.mechanism != expected.mechanism
        || contract.states != expected.states
    {
        return Err(format!("contract identity mismatch for {}", contract.id));
    }
    if contract.repetitions != 20
        || contract.size_count != SIZES.len()
        || contract.class_count != CLASSES.len()
    {
        return Err(format!("experiment schedule mismatch for {}", contract.id));
    }
    if contract.learning_rows != 600 || contract.finished_rows + contract.exception_rows != 600 {
        return Err(format!(
            "learning denominator/status mismatch for {}",
            contract.id
        ));
    }
    if contract.exception_rows != expected.exceptions {
        return Err(format!(
            "pinned exception count mismatch for {}: expected {}, got {}",
            contract.id, expected.exceptions, contract.exception_rows
        ));
    }
    if contract.baseline_count != 2
        || contract.baseline_failed != 0
        || contract.baseline_ids != ["random".to_owned(), "upper".to_owned()]
    {
        return Err(format!("baseline contract mismatch for {}", contract.id));
    }
    if contract.oracle_mean <= contract.random_mean {
        return Err(format!(
            "policy normalization anchors are not ordered for {}",
            contract.id
        ));
    }
    if contract.exception_rows > 0 {
        if contract.exception_classes != ["RemoteException".to_owned()]
            || contract.known_belief_collapse_rows != contract.exception_rows
        {
            return Err(format!("exception taxonomy mismatch for {}", contract.id));
        }
    } else if !contract.exception_classes.is_empty() || contract.known_belief_collapse_rows != 0 {
        return Err(format!("unexpected exception taxonomy in {}", contract.id));
    }
    if contract.mconst_count != 1 || contract.mconst_failed != 0 {
        return Err(format!("M-constant contract mismatch for {}", contract.id));
    }
    if contract.pomcp_count != 8 || contract.pomcp_failed != 0 {
        return Err(format!("POMCP contract mismatch for {}", contract.id));
    }
    let mut expected_algorithms = vec![
        "AIMI".to_owned(),
        "AsMAR (MCAR)".to_owned(),
        "AsMAR (MAR)".to_owned(),
    ];
    expected_algorithms.sort();
    if contract.algorithms != expected_algorithms {
        return Err(format!(
            "learning algorithm identities mismatch for {}",
            contract.id
        ));
    }
    Ok(())
}

fn validate_runs(contract: &Contract, runs: &[Run]) -> Result<(usize, usize), String> {
    if runs.len() != 600 {
        return Err(format!(
            "{} has {} learning rows, expected 600",
            contract.id,
            runs.len()
        ));
    }
    let mut seen = BTreeSet::new();
    let mut counts_by_size: BTreeMap<u64, usize> = BTreeMap::new();
    let mut finished = 0_usize;
    let mut exceptions = 0_usize;

    for run in runs {
        if run.experiment_id != contract.id {
            return Err(format!(
                "experiment id mismatch: {} != {}",
                run.experiment_id, contract.id
            ));
        }
        if !SIZES.contains(&run.dataset_size) {
            return Err(format!("unexpected dataset size {}", run.dataset_size));
        }
        if !(1..=20).contains(&run.seed) {
            return Err(format!("seed out of range: {}", run.seed));
        }
        if !CLASSES.contains(&run.class.as_str()) {
            return Err(format!("unknown missingness class {}", run.class));
        }
        let expected_algorithm = algorithm_for(run.class.as_str())
            .ok_or_else(|| format!("unknown missingness class {}", run.class))?;
        if run.algorithm != expected_algorithm {
            return Err(format!(
                "algorithm/class mismatch: {} / {}",
                run.algorithm, run.class
            ));
        }
        if !run.atv_present || !run.wtv_present {
            return Err("missingness-function TV absent from a learning row".to_owned());
        }
        if run.atv > run.wtv + 1e-9 {
            return Err(format!(
                "average TV exceeds worst TV for {} / seed {}",
                contract.id, run.seed
            ));
        }
        if run.finished {
            if run.exception
                || !run.mean_present
                || run.mean.is_none()
                || !run.exception_kind.is_empty()
                || run.failure_reason_known
            {
                return Err(
                    "finished learning row lacks a valid mean or carries exception metadata"
                        .to_owned(),
                );
            }
            finished += 1;
        } else {
            if !run.exception
                || run.mean_present
                || run.mean.is_some()
                || run.exception_kind != "RemoteException"
                || !run.failure_reason_known
            {
                return Err("unfinished learning row must retain the known RemoteException and omit policy mean".to_owned());
            }
            exceptions += 1;
        }
        let key = (run.dataset_size, run.seed, run.class.as_str());
        if !seen.insert(key) {
            return Err(format!(
                "duplicate run key: {} / {} / {} / {}",
                contract.id, run.dataset_size, run.seed, run.class
            ));
        }
        *counts_by_size.entry(run.dataset_size).or_default() += 1;
    }

    let expected_keys = SIZES.len() * 20 * CLASSES.len();
    if seen.len() != expected_keys {
        return Err(format!(
            "run-key denominator incomplete for {}",
            contract.id
        ));
    }
    for size in SIZES {
        if counts_by_size.get(&size).copied() != Some(60) {
            return Err(format!(
                "dataset size {size} does not have 60 assumption/seed rows"
            ));
        }
    }
    if finished != contract.finished_rows || exceptions != contract.exception_rows {
        return Err(format!(
            "run status counts disagree with contract for {}",
            contract.id
        ));
    }
    Ok((finished, exceptions))
}

fn run_audit(input: &str) -> Result<String, String> {
    let mut contracts = BTreeMap::new();
    let mut run_groups: BTreeMap<String, Vec<Run>> = BTreeMap::new();

    for (line_number, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied() {
            Some("contract") => {
                let contract = parse_contract(&fields)
                    .map_err(|error| format!("line {}: {error}", line_number + 1))?;
                if contracts.insert(contract.id.clone(), contract).is_some() {
                    return Err(format!("duplicate contract on line {}", line_number + 1));
                }
            }
            Some("run") => {
                let run = parse_run(&fields)
                    .map_err(|error| format!("line {}: {error}", line_number + 1))?;
                run_groups
                    .entry(run.experiment_id.clone())
                    .or_default()
                    .push(run);
            }
            _ => return Err(format!("unknown record on line {}", line_number + 1)),
        }
    }

    if contracts.len() != EXPECTED.len() {
        return Err(format!(
            "expected {} experiment contracts, got {}",
            EXPECTED.len(),
            contracts.len()
        ));
    }
    if run_groups.len() != EXPECTED.len() {
        return Err(format!(
            "expected {} run groups, got {}",
            EXPECTED.len(),
            run_groups.len()
        ));
    }

    let mut total_finished = 0_usize;
    let mut total_exceptions = 0_usize;
    let mut output = String::from(
        "experiment_id,environment,true_mechanism,pomdp_states,learning_rows,finished_rows,exception_rows,learning_success_rate,exceptions_n10,exceptions_n50,exceptions_other,baseline_count,pomcp_count,exception_class_count,known_belief_collapse_exception_rows\n",
    );

    for expected in EXPECTED {
        let contract = contracts
            .get(expected.id)
            .ok_or_else(|| format!("missing contract {}", expected.id))?;
        validate_contract(contract, expected)?;
        let runs = run_groups
            .get(expected.id)
            .ok_or_else(|| format!("missing learning rows for {}", expected.id))?;
        let (finished, exceptions) = validate_runs(contract, runs)?;

        let exceptions_n10 = runs
            .iter()
            .filter(|run| !run.finished && run.dataset_size == 10)
            .count();
        let exceptions_n50 = runs
            .iter()
            .filter(|run| !run.finished && run.dataset_size == 50)
            .count();
        let exceptions_other = exceptions - exceptions_n10 - exceptions_n50;
        let expected_failure_split = match expected.id {
            "icu/smar" => (33, 0, 0),
            "icu/mnar-ident" => (33, 3, 0),
            "icu/mnar-unident" => (15, 0, 0),
            "predator-factored/smar" => (0, 0, 0),
            "predator-factored/mnar-unident" => (39, 0, 0),
            _ => unreachable!(),
        };
        if (exceptions_n10, exceptions_n50, exceptions_other) != expected_failure_split {
            return Err(format!(
                "pinned failure-size distribution mismatch for {}",
                expected.id
            ));
        }

        total_finished += finished;
        total_exceptions += exceptions;
        let success_rate = finished as f64 / 600.0;
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.6},{},{},{},{},{},{},{}\n",
            contract.id,
            contract.environment,
            contract.mechanism,
            contract.states,
            runs.len(),
            finished,
            exceptions,
            success_rate,
            exceptions_n10,
            exceptions_n50,
            exceptions_other,
            contract.baseline_count,
            contract.pomcp_count,
            contract.exception_classes.len(),
            contract.known_belief_collapse_rows
        ));
    }

    if total_finished != 2_877 || total_exceptions != 123 {
        return Err(format!(
            "aggregate status mismatch: finished={total_finished}, exceptions={total_exceptions}"
        ));
    }
    output.push_str(&format!(
        "TOTAL,all,all,NA,3000,{total_finished},{total_exceptions},{:.6},120,3,0,10,40,1,123\n",
        total_finished as f64 / 3000.0
    ));
    Ok(output)
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn normalized_policy_value(value: f64, oracle: f64, random: f64) -> Option<f64> {
    let gap = oracle - random;
    if !value.is_finite() || !oracle.is_finite() || !random.is_finite() || gap <= f64::EPSILON {
        None
    } else {
        Some((value - random) / gap)
    }
}

fn run_policy_summary(input: &str) -> Result<String, String> {
    let mut contracts = BTreeMap::new();
    let mut run_groups: BTreeMap<String, Vec<Run>> = BTreeMap::new();

    for (line_number, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied() {
            Some("contract") => {
                let contract = parse_contract(&fields)
                    .map_err(|error| format!("line {}: {error}", line_number + 1))?;
                if contracts.insert(contract.id.clone(), contract).is_some() {
                    return Err(format!("duplicate contract on line {}", line_number + 1));
                }
            }
            Some("run") => {
                let run = parse_run(&fields)
                    .map_err(|error| format!("line {}: {error}", line_number + 1))?;
                run_groups
                    .entry(run.experiment_id.clone())
                    .or_default()
                    .push(run);
            }
            _ => return Err(format!("unknown record on line {}", line_number + 1)),
        }
    }

    if contracts.len() != EXPECTED.len() || run_groups.len() != EXPECTED.len() {
        return Err(
            "policy-summary input does not contain the complete five-experiment matrix".to_owned(),
        );
    }

    let mut output = String::from(
        "experiment_id,dataset_size,assumed_missingness,algorithm,finished,exceptions,mean_return,median_return,minimum_return,maximum_return,mean_average_tv,mean_worst_tv,normalized_policy_value,oracle_anchor,random_anchor\n",
    );

    for expected in EXPECTED {
        let contract = contracts
            .get(expected.id)
            .ok_or_else(|| format!("missing contract {}", expected.id))?;
        validate_contract(contract, expected)?;
        let runs = run_groups
            .get(expected.id)
            .ok_or_else(|| format!("missing learning rows for {}", expected.id))?;
        validate_runs(contract, runs)?;

        for size in SIZES {
            for class in CLASSES {
                let cell: Vec<&Run> = runs
                    .iter()
                    .filter(|run| run.dataset_size == size && run.class == class)
                    .collect();
                if cell.len() != 20 {
                    return Err(format!(
                        "{} / {} / {} has {} runs, expected 20",
                        expected.id,
                        size,
                        class,
                        cell.len()
                    ));
                }
                let finished: Vec<&Run> = cell.iter().copied().filter(|run| run.finished).collect();
                let exceptions = cell.len() - finished.len();
                let algorithm = algorithm_for(class)
                    .ok_or_else(|| format!("unknown missingness class {class}"))?;
                if finished.is_empty() {
                    let row = [
                        expected.id.to_owned(),
                        size.to_string(),
                        class.to_owned(),
                        algorithm.to_owned(),
                        "0".to_owned(),
                        exceptions.to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        format!("{:.9}", contract.oracle_mean),
                        format!("{:.9}", contract.random_mean),
                    ];
                    output.push_str(&row.join(","));
                    output.push('\n');
                    continue;
                }

                let mut returns: Vec<f64> = finished
                    .iter()
                    .map(|run| run.mean.expect("finished row has mean"))
                    .collect();
                let median_return = median(&mut returns);
                let mean_return = finished
                    .iter()
                    .map(|run| run.mean.expect("finished row has mean"))
                    .sum::<f64>()
                    / finished.len() as f64;
                let minimum_return = finished
                    .iter()
                    .map(|run| run.mean.expect("finished row has mean"))
                    .fold(f64::INFINITY, f64::min);
                let maximum_return = finished
                    .iter()
                    .map(|run| run.mean.expect("finished row has mean"))
                    .fold(f64::NEG_INFINITY, f64::max);
                let mean_average_tv =
                    finished.iter().map(|run| run.atv).sum::<f64>() / finished.len() as f64;
                let mean_worst_tv =
                    finished.iter().map(|run| run.wtv).sum::<f64>() / finished.len() as f64;
                let normalized = normalized_policy_value(
                    mean_return,
                    contract.oracle_mean,
                    contract.random_mean,
                )
                .map_or_else(String::new, |value| format!("{value:.9}"));

                output.push_str(&format!(
                    "{},{},{},{},{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{},{:.9},{:.9}\n",
                    expected.id,
                    size,
                    class,
                    algorithm,
                    finished.len(),
                    exceptions,
                    mean_return,
                    median_return,
                    minimum_return,
                    maximum_return,
                    mean_average_tv,
                    mean_worst_tv,
                    normalized,
                    contract.oracle_mean,
                    contract.random_mean
                ));
            }
        }
    }
    Ok(output)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let policy_summary = args
        .first()
        .is_some_and(|argument| argument == "--policy-summary");
    let path_index = if policy_summary { 1 } else { 0 };
    let path = args.get(path_index).unwrap_or_else(|| {
        eprintln!("usage: miss-mdp-audit [--policy-summary] <normalized-upstream-results.tsv>");
        process::exit(2);
    });
    let input = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("failed to read audit input: {error}");
        process::exit(2);
    });
    let result = if policy_summary {
        run_policy_summary(&input)
    } else {
        run_audit(&input)
    };
    match result {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("reference audit failed closed: {error}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_parser_preserves_known_belief_collapse_fingerprint() {
        let line = "run\ticu/smar\t10\t1\tMCAR\tAsMAR (MCAR)\tfalse\ttrue\tfalse\ttrue\ttrue\t0.3\t0.4\t\tRemoteException\ttrue";
        let run = parse_run(&line.split('\t').collect::<Vec<_>>()).expect("exception row");
        assert_eq!(run.exception_kind, "RemoteException");
        assert!(run.failure_reason_known);
        assert!(!run.finished);
        assert!(run.mean.is_none());
    }

    #[test]
    fn normalized_policy_value_uses_per_experiment_oracle_and_random_anchors() {
        assert_eq!(normalized_policy_value(-10.0, -10.0, -30.0), Some(1.0));
        assert_eq!(normalized_policy_value(-30.0, -10.0, -30.0), Some(0.0));
        assert_eq!(normalized_policy_value(-20.0, -10.0, -30.0), Some(0.5));
        assert_eq!(normalized_policy_value(-20.0, -20.0, -20.0), None);
    }

    #[test]
    fn boolean_parser_is_strict() {
        assert_eq!(parse_bool("true"), Ok(true));
        assert_eq!(parse_bool("false"), Ok(false));
        assert!(parse_bool("yes").is_err());
    }

    #[test]
    fn expected_pinned_artifact_set_has_five_distinct_experiments() {
        let ids: BTreeSet<&str> = EXPECTED.iter().map(|item| item.id).collect();
        assert_eq!(ids.len(), 5);
        assert_eq!(
            EXPECTED.iter().map(|item| item.exceptions).sum::<usize>(),
            123
        );
    }

    #[test]
    fn known_sample_schedule_is_ordered_and_unique() {
        let sizes: BTreeSet<u64> = SIZES.into_iter().collect();
        assert_eq!(sizes.len(), 10);
        assert!(SIZES.windows(2).all(|window| window[0] < window[1]));
    }

    #[test]
    fn assumption_to_algorithm_mapping_is_explicit() {
        assert_eq!(algorithm_for("MCAR"), Some("AsMAR (MCAR)"));
        assert_eq!(algorithm_for("MAR"), Some("AsMAR (MAR)"));
        assert_eq!(algorithm_for("MNAR"), Some("AIMI"));
        assert_eq!(algorithm_for("unknown"), None);
    }
}
