// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const SIZES: [u64; 10] = [10, 50, 100, 500, 1_000, 5_000, 10_000, 100_000, 1_000_000, 10_000_000];
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
    ExpectedExperiment { id: "icu/smar", environment: "icu", mechanism: "simple_mar", states: 800, exceptions: 33 },
    ExpectedExperiment { id: "icu/mnar-ident", environment: "icu", mechanism: "identifiable_mnar", states: 800, exceptions: 36 },
    ExpectedExperiment { id: "icu/mnar-unident", environment: "icu", mechanism: "unidentifiable_mnar", states: 800, exceptions: 15 },
    ExpectedExperiment { id: "predator-factored/smar", environment: "predator", mechanism: "simple_mar", states: 626, exceptions: 0 },
    ExpectedExperiment { id: "predator-factored/mnar-unident", environment: "predator", mechanism: "unidentifiable_mnar", states: 626, exceptions: 39 },
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
    baseline_count: usize,
    baseline_failed: usize,
    mconst_count: usize,
    mconst_failed: usize,
    pomcp_count: usize,
    pomcp_failed: usize,
    baseline_ids: Vec<String>,
    algorithms: Vec<String>,
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
    value.parse::<u64>().map_err(|_| format!("invalid {field}: {value}"))
}

fn parse_usize(value: &str, field: &str) -> Result<usize, String> {
    value.parse::<usize>().map_err(|_| format!("invalid {field}: {value}"))
}

fn parse_contract(fields: &[&str]) -> Result<Contract, String> {
    if fields.len() != 19 || fields[0] != "contract" {
        return Err(format!("contract row must have 19 fields, got {}", fields.len()));
    }
    let mut baseline_ids: Vec<String> = fields[17].split(';').map(str::to_owned).collect();
    baseline_ids.sort();
    let mut algorithms: Vec<String> = fields[18].split(';').map(str::to_owned).collect();
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
        baseline_count: parse_usize(fields[11], "baseline count")?,
        baseline_failed: parse_usize(fields[12], "failed baseline count")?,
        mconst_count: parse_usize(fields[13], "M-constant count")?,
        mconst_failed: parse_usize(fields[14], "failed M-constant count")?,
        pomcp_count: parse_usize(fields[15], "POMCP count")?,
        pomcp_failed: parse_usize(fields[16], "failed POMCP count")?,
        baseline_ids,
        algorithms,
    })
}

fn parse_run(fields: &[&str]) -> Result<Run, String> {
    if fields.len() != 14 || fields[0] != "run" {
        return Err(format!("run row must have 14 fields, got {}", fields.len()));
    }
    let mean = if fields[13].is_empty() {
        None
    } else {
        Some(fields[13].parse::<f64>().map_err(|_| "invalid mean".to_owned())?)
    };
    let atv = fields[11].parse::<f64>().map_err(|_| "invalid ATV".to_owned())?;
    let wtv = fields[12].parse::<f64>().map_err(|_| "invalid WTV".to_owned())?;
    if !atv.is_finite() || !(0.0..=1.0).contains(&atv) {
        return Err(format!("ATV outside [0,1]: {atv}"));
    }
    if !wtv.is_finite() || !(0.0..=1.0).contains(&wtv) {
        return Err(format!("WTV outside [0,1]: {wtv}"));
    }
    if let Some(value) = mean {
        if !value.is_finite() {
            return Err("non-finite policy mean".to_owned());
        }
    }
    Ok(Run {
        experiment_id: fields[1].to_owned(),
        dataset_size: parse_u64(fields[2], "dataset size")?,
        seed: parse_u64(fields[3], "seed")?,
        class: fields[4].to_owned(),
        algorithm: fields[5].to_owned(),
        finished: parse_bool(fields[6])?,
        exception: parse_bool(fields[7])?,
        mean_present: parse_bool(fields[8])?,
        atv_present: parse_bool(fields[9])?,
        wtv_present: parse_bool(fields[10])?,
        atv,
        wtv,
        mean,
    })
}

fn expected_for(id: &str) -> Option<ExpectedExperiment> {
    EXPECTED.iter().copied().find(|expected| expected.id == id)
}

fn validate_contract(contract: &Contract, expected: ExpectedExperiment) -> Result<(), String> {
    if contract.id != expected.id
        || contract.environment != expected.environment
        || contract.mechanism != expected.mechanism
        || contract.states != expected.states
    {
        return Err(format!("contract identity mismatch for {}", contract.id));
    }
    if contract.repetitions != 20 || contract.size_count != SIZES.len() || contract.class_count != CLASSES.len() {
        return Err(format!("experiment schedule mismatch for {}", contract.id));
    }
    if contract.learning_rows != 600 || contract.finished_rows + contract.exception_rows != 600 {
        return Err(format!("learning denominator/status mismatch for {}", contract.id));
    }
    if contract.exception_rows != expected.exceptions {
        return Err(format!("pinned exception count mismatch for {}: expected {}, got {}", contract.id, expected.exceptions, contract.exception_rows));
    }
    if contract.baseline_count != 2 || contract.baseline_failed != 0
        || contract.baseline_ids != ["random".to_owned(), "upper".to_owned()]
    {
        return Err(format!("baseline contract mismatch for {}", contract.id));
    }
    if contract.mconst_count != 1 || contract.mconst_failed != 0 {
        return Err(format!("M-constant contract mismatch for {}", contract.id));
    }
    if contract.pomcp_count != 8 || contract.pomcp_failed != 0 {
        return Err(format!("POMCP contract mismatch for {}", contract.id));
    }
    let expected_algorithms = vec!["AIMI".to_owned(), "AsMAR (MCAR)".to_owned(), "AsMAR (MAR)".to_owned()];
    if contract.algorithms != expected_algorithms {
        return Err(format!("learning algorithm identities mismatch for {}", contract.id));
    }
    Ok(())
}

fn validate_runs(contract: &Contract, runs: &[Run]) -> Result<(usize, usize), String> {
    if runs.len() != 600 {
        return Err(format!("{} has {} learning rows, expected 600", contract.id, runs.len()));
    }
    let mut seen = BTreeSet::new();
    let mut counts_by_size: BTreeMap<u64, usize> = BTreeMap::new();
    let mut finished = 0_usize;
    let mut exceptions = 0_usize;

    for run in runs {
        if run.experiment_id != contract.id {
            return Err(format!("experiment id mismatch: {} != {}", run.experiment_id, contract.id));
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
        let expected_algorithm = match run.class.as_str() {
            "MCAR" => "AsMAR (MCAR)",
            "MAR" => "AsMAR (MAR)",
            "MNAR" => "AIMI",
            _ => unreachable!(),
        };
        if run.algorithm != expected_algorithm {
            return Err(format!("algorithm/class mismatch: {} / {}", run.algorithm, run.class));
        }
        if !run.atv_present || !run.wtv_present {
            return Err("missingness-function TV absent from a learning row".to_owned());
        }
        if run.atv > run.wtv + 1e-9 {
            return Err(format!("average TV exceeds worst TV for {} / seed {}", contract.id, run.seed));
        }
        if run.finished {
            if run.exception || !run.mean_present || run.mean.is_none() {
                return Err("finished learning row lacks a valid mean or carries exception".to_owned());
            }
            finished += 1;
        } else {
            if !run.exception {
                return Err("unfinished learning row has no recorded exception".to_owned());
            }
            exceptions += 1;
        }
        let key = (run.dataset_size, run.seed, run.class.as_str());
        if !seen.insert(key) {
            return Err(format!("duplicate run key: {} / {} / {} / {}", contract.id, run.dataset_size, run.seed, run.class));
        }
        *counts_by_size.entry(run.dataset_size).or_default() += 1;
    }

    let expected_keys = SIZES.len() * 20 * CLASSES.len();
    if seen.len() != expected_keys {
        return Err(format!("run-key denominator incomplete for {}", contract.id));
    }
    for size in SIZES {
        if counts_by_size.get(&size).copied() != Some(60) {
            return Err(format!("dataset size {size} does not have 60 assumption/seed rows"));
        }
    }
    if finished != contract.finished_rows || exceptions != contract.exception_rows {
        return Err(format!("run status counts disagree with contract for {}", contract.id));
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
                run_groups.entry(run.experiment_id.clone()).or_default().push(run);
            }
            _ => return Err(format!("unknown record on line {}", line_number + 1)),
        }
    }

    if contracts.len() != EXPECTED.len() {
        return Err(format!("expected {} experiment contracts, got {}", EXPECTED.len(), contracts.len()));
    }
    if run_groups.len() != EXPECTED.len() {
        return Err(format!("expected {} run groups, got {}", EXPECTED.len(), run_groups.len()));
    }

    let mut total_finished = 0_usize;
    let mut total_exceptions = 0_usize;
    let mut output = String::from("experiment_id,environment,true_mechanism,pomdp_states,learning_rows,finished_rows,exception_rows,learning_success_rate,exceptions_n10,exceptions_n50,exceptions_other,baseline_count,pomcp_count\n");

    for expected in EXPECTED {
        let contract = contracts
            .get(expected.id)
            .ok_or_else(|| format!("missing contract {}", expected.id))?;
        validate_contract(contract, expected)?;
        let runs = run_groups
            .get(expected.id)
            .ok_or_else(|| format!("missing learning rows for {}", expected.id))?;
        let (finished, exceptions) = validate_runs(contract, runs)?;

        let exceptions_n10 = runs.iter().filter(|run| !run.finished && run.dataset_size == 10).count();
        let exceptions_n50 = runs.iter().filter(|run| !run.finished && run.dataset_size == 50).count();
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
            return Err(format!("pinned failure-size distribution mismatch for {}", expected.id));
        }

        total_finished += finished;
        total_exceptions += exceptions;
        let success_rate = finished as f64 / 600.0;
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.6},{},{},{},{},{}\n",
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
            contract.pomcp_count
        ));
    }

    if total_finished != 2_877 || total_exceptions != 123 {
        return Err(format!("aggregate status mismatch: finished={total_finished}, exceptions={total_exceptions}"));
    }
    output.push_str(&format!(
        "TOTAL,all,all,NA,3000,{total_finished},{total_exceptions},{:.6},120,3,0,10,40\n",
        total_finished as f64 / 3000.0
    ));
    Ok(output)
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: miss-mdp-audit <normalized-upstream-results.tsv>");
        process::exit(2);
    });
    let input = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("failed to read audit input: {error}");
        process::exit(2);
    });
    match run_audit(&input) {
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
    fn boolean_parser_is_strict() {
        assert_eq!(parse_bool("true"), Ok(true));
        assert_eq!(parse_bool("false"), Ok(false));
        assert!(parse_bool("yes").is_err());
    }

    #[test]
    fn expected_pinned_artifact_set_has_five_distinct_experiments() {
        let ids: BTreeSet<&str> = EXPECTED.iter().map(|item| item.id).collect();
        assert_eq!(ids.len(), 5);
        assert_eq!(EXPECTED.iter().map(|item| item.exceptions).sum::<usize>(), 123);
    }

    #[test]
    fn known_sample_schedule_is_ordered_and_unique() {
        let sizes: BTreeSet<u64> = SIZES.into_iter().collect();
        assert_eq!(sizes.len(), 10);
        assert!(SIZES.windows(2).all(|window| window[0] < window[1]));
    }

    #[test]
    fn assumption_to_algorithm_mapping_is_explicit() {
        assert_eq!(match "MCAR" { "MCAR" => "AsMAR (MCAR)", "MAR" => "AsMAR (MAR)", _ => "AIMI" }, "AsMAR (MCAR)");
        assert_eq!(match "MAR" { "MCAR" => "AsMAR (MCAR)", "MAR" => "AsMAR (MAR)", _ => "AIMI" }, "AsMAR (MAR)");
        assert_eq!(match "MNAR" { "MCAR" => "AsMAR (MCAR)", "MAR" => "AsMAR (MAR)", _ => "AIMI" }, "AIMI");
    }
}
