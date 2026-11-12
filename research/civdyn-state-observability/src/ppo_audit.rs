// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

#[derive(Clone, Copy)]
struct ExpectedGroup {
    id: &'static str,
    stacked: &'static str,
    model: &'static str,
    pattern: &'static str,
    expected_runs: usize,
}

const EXPECTED_GROUPS: [ExpectedGroup; 12] = [
    ExpectedGroup {
        id: "experiments/logs_PPO/icu/mnar-ident",
        stacked: "no",
        model: "icu",
        pattern: "mnar-ident",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO/icu/mnar-unident",
        stacked: "no",
        model: "icu",
        pattern: "mnar-unident",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO/icu/smar",
        stacked: "no",
        model: "icu",
        pattern: "smar",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO/predator/mcar_search_big",
        stacked: "no",
        model: "predator",
        pattern: "mcar_search_big",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO/predator/mnar-unident",
        stacked: "no",
        model: "predator",
        pattern: "mnar-unident",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO/predator/smar",
        stacked: "no",
        model: "predator",
        pattern: "smar",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO_stacked/icu/mnar-ident",
        stacked: "yes",
        model: "icu",
        pattern: "mnar-ident",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO_stacked/icu/mnar-unident",
        stacked: "yes",
        model: "icu",
        pattern: "mnar-unident",
        expected_runs: 19,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO_stacked/icu/smar",
        stacked: "yes",
        model: "icu",
        pattern: "smar",
        expected_runs: 23,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO_stacked/predator/mcar_search_big",
        stacked: "yes",
        model: "pred",
        pattern: "mcar_search_big",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO_stacked/predator/mnar-unident",
        stacked: "yes",
        model: "pred",
        pattern: "mnar-unident",
        expected_runs: 20,
    },
    ExpectedGroup {
        id: "experiments/logs_PPO_stacked/predator/smar",
        stacked: "yes",
        model: "pred",
        pattern: "smar",
        expected_runs: 20,
    },
];

#[derive(Debug)]
struct PpoRun {
    group: String,
    stacked: String,
    model: String,
    pattern: String,
    id: u64,
    seed: u64,
    train_steps: u64,
    eval_episodes: u64,
    frames: u64,
    layers: u64,
    hidden: u64,
    return_mean: f64,
    return_std: f64,
    hparams: String,
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

fn parse_run(fields: &[&str]) -> Result<PpoRun, String> {
    if fields.len() != 15 || fields[0] != "run" {
        return Err(format!("PPO row must have 15 fields, got {}", fields.len()));
    }
    let run = PpoRun {
        group: fields[1].to_owned(),
        stacked: fields[2].to_owned(),
        model: fields[3].to_owned(),
        pattern: fields[4].to_owned(),
        id: parse_u64(fields[5], "run id")?,
        seed: parse_u64(fields[6], "seed")?,
        train_steps: parse_u64(fields[7], "training steps")?,
        eval_episodes: parse_u64(fields[8], "evaluation episodes")?,
        frames: parse_u64(fields[9], "frame count")?,
        layers: parse_u64(fields[10], "layer count")?,
        hidden: parse_u64(fields[11], "hidden width")?,
        return_mean: parse_f64(fields[12], "return mean")?,
        return_std: parse_f64(fields[13], "return std")?,
        hparams: fields[14].to_owned(),
    };
    if run.return_std < 0.0 {
        return Err("negative reported return standard deviation".to_owned());
    }
    if run.hparams.is_empty() {
        return Err("missing PPO hyperparameters".to_owned());
    }
    Ok(run)
}

#[cfg(test)]
fn expected_group(id: &str) -> Option<ExpectedGroup> {
    EXPECTED_GROUPS.iter().copied().find(|group| group.id == id)
}

fn model_matches(group: ExpectedGroup, actual: &str) -> bool {
    if group
        .id
        .starts_with("experiments/logs_PPO_stacked/predator/")
    {
        matches!(actual, "pred" | "predator")
    } else {
        actual == group.model
    }
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

fn summarize(group: ExpectedGroup, runs: &[PpoRun]) -> Result<String, String> {
    if runs.len() != group.expected_runs {
        return Err(format!(
            "{} has {} runs, expected {}",
            group.id,
            runs.len(),
            group.expected_runs
        ));
    }
    let mut ids = BTreeSet::new();
    let mut seeds = BTreeSet::new();
    let mut seed_counts: BTreeMap<u64, usize> = BTreeMap::new();
    let mut means = Vec::with_capacity(runs.len());
    let mut stds = Vec::with_capacity(runs.len());
    let mut frames = BTreeSet::new();
    let mut layers = BTreeSet::new();
    let mut hidden = BTreeSet::new();
    let mut hparams = BTreeSet::new();
    let mut observed_models = BTreeSet::new();

    for run in runs {
        if run.group != group.id
            || run.stacked != group.stacked
            || !model_matches(group, run.model.as_str())
            || run.pattern != group.pattern
        {
            return Err(format!("PPO group identity mismatch in {}", group.id));
        }
        if run.id == 0 || !ids.insert(run.id) {
            return Err(format!("invalid or duplicate PPO run id in {}", group.id));
        }
        if run.seed == 0 {
            return Err(format!("zero PPO seed in {}", group.id));
        }
        if run.train_steps != 1_000_000 || run.eval_episodes != 2_000 {
            return Err(format!(
                "training/evaluation budget mismatch in {}",
                group.id
            ));
        }
        if run.frames == 0 || run.layers == 0 || run.hidden == 0 {
            return Err(format!("invalid architecture dimensions in {}", group.id));
        }
        means.push(run.return_mean);
        stds.push(run.return_std);
        seeds.insert(run.seed);
        *seed_counts.entry(run.seed).or_default() += 1;
        frames.insert(run.frames);
        layers.insert(run.layers);
        hidden.insert(run.hidden);
        hparams.insert(run.hparams.as_str());
        observed_models.insert(run.model.as_str());
    }

    let mut sorted_means = means.clone();
    let median_return = median(&mut sorted_means);
    let mean_return = means.iter().sum::<f64>() / means.len() as f64;
    let mean_reported_std = stds.iter().sum::<f64>() / stds.len() as f64;
    let min_return = means.iter().copied().fold(f64::INFINITY, f64::min);
    let max_return = means.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let seed_coverage = seeds.len();
    let maximum_run_id = ids.iter().copied().max().unwrap_or(0);
    let observed_run_ids = ids.iter().map(u64::to_string).collect::<Vec<_>>().join(";");
    let missing_run_ids = (1..=maximum_run_id)
        .filter(|id| !ids.contains(id))
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(";");
    let duplicate_seed_values = seed_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(seed, count)| format!("{seed}x{count}"))
        .collect::<Vec<_>>()
        .join(";");

    Ok(format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{:.9},{:.9},{:.9},{:.9},{:.9},{},{},{},{},{},{},{}",
        group.id,
        group.stacked,
        group.model,
        group.pattern,
        runs.len(),
        ids.len(),
        seed_coverage,
        runs.iter().map(|run| run.seed).min().unwrap_or(0),
        runs.iter().map(|run| run.seed).max().unwrap_or(0),
        *frames.iter().next().unwrap_or(&0),
        *layers.iter().next().unwrap_or(&0),
        *hidden.iter().next().unwrap_or(&0),
        mean_return,
        median_return,
        min_return,
        max_return,
        mean_reported_std,
        hparams.len(),
        *frames.iter().next_back().unwrap_or(&0),
        *layers.iter().next_back().unwrap_or(&0),
        *hidden.iter().next_back().unwrap_or(&0),
        observed_models.into_iter().collect::<Vec<_>>().join(";"),
        observed_run_ids,
        missing_run_ids,
        duplicate_seed_values,
    ))
}

fn run_audit(input: &str) -> Result<String, String> {
    let mut grouped: BTreeMap<String, Vec<PpoRun>> = BTreeMap::new();
    for (line_index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let run =
            parse_run(&fields).map_err(|error| format!("line {}: {error}", line_index + 1))?;
        grouped.entry(run.group.clone()).or_default().push(run);
    }

    if grouped.len() != EXPECTED_GROUPS.len() {
        return Err(format!(
            "expected {} PPO groups, got {}",
            EXPECTED_GROUPS.len(),
            grouped.len()
        ));
    }
    let expected_total: usize = EXPECTED_GROUPS
        .iter()
        .map(|group| group.expected_runs)
        .sum();
    let actual_total: usize = grouped.values().map(Vec::len).sum();
    if actual_total != expected_total || actual_total != 242 {
        return Err(format!(
            "PPO artifact total mismatch: {actual_total}, expected {expected_total}"
        ));
    }

    let mut output = String::from(
        "group,is_stacked,model,pattern,run_count,unique_run_ids,unique_seeds,minimum_seed,maximum_seed,minimum_frames,minimum_layers,minimum_hidden,mean_return,median_return,minimum_return,maximum_return,mean_reported_return_std,unique_hyperparameter_sets,maximum_frames,maximum_layers,maximum_hidden,observed_model_values,observed_run_ids,missing_run_ids,duplicate_seed_values\n",
    );
    for group in EXPECTED_GROUPS {
        let runs = grouped
            .get(group.id)
            .ok_or_else(|| format!("missing PPO group {}", group.id))?;
        let row = summarize(group, runs)?;
        output.push_str(&row);
        output.push('\n');
    }
    let all: Vec<&PpoRun> = grouped.values().flat_map(|runs| runs.iter()).collect();
    let unique_seeds = all
        .iter()
        .map(|run| run.seed)
        .collect::<BTreeSet<_>>()
        .len();
    let minimum_seed = all.iter().map(|run| run.seed).min().unwrap_or(0);
    let maximum_seed = all.iter().map(|run| run.seed).max().unwrap_or(0);
    // ICU and Predator returns have different task semantics/scales; do not
    // pool them into a cross-environment headline value.
    output.push_str(&format!(
        "TOTAL,all,all,all,{actual_total},{actual_total},{unique_seeds},{minimum_seed},{maximum_seed},NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA,NA\\n"
    ));
    Ok(output)
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: miss-mdp-ppo-audit <normalized-ppo-results.tsv>");
        process::exit(2);
    });
    let input = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("failed to read PPO audit input: {error}");
        process::exit(2);
    });
    match run_audit(&input) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("PPO reference audit failed closed: {error}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_group_counts_sum_to_242() {
        assert_eq!(EXPECTED_GROUPS.len(), 12);
        assert_eq!(
            EXPECTED_GROUPS
                .iter()
                .map(|group| group.expected_runs)
                .sum::<usize>(),
            242
        );
        assert_eq!(
            expected_group("experiments/logs_PPO_stacked/icu/mnar-unident")
                .unwrap()
                .expected_runs,
            19
        );
        assert_eq!(
            expected_group("experiments/logs_PPO_stacked/icu/smar")
                .unwrap()
                .expected_runs,
            23
        );
    }

    #[test]
    fn non_contiguous_artifact_ids_are_preserved_as_gaps_not_renumbered() {
        let ids = [
            1_u64, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ]
        .into_iter()
        .collect::<BTreeSet<_>>();
        let gaps = (1..=ids.iter().copied().max().unwrap())
            .filter(|id| !ids.contains(id))
            .collect::<Vec<_>>();
        assert_eq!(gaps, vec![7]);
    }

    #[test]
    fn duplicate_seed_receipts_are_reported_without_dropping_runs() {
        let mut counts = BTreeMap::new();
        for seed in [520_u64, 421, 422, 420, 421, 422] {
            *counts.entry(seed).or_insert(0_usize) += 1;
        }
        let duplicates = counts
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(seed, count)| format!("{seed}x{count}"))
            .collect::<Vec<_>>()
            .join(";");
        assert_eq!(duplicates, "421x2;422x2");
    }

    #[test]
    fn stacked_predator_model_alias_is_preserved_and_scoped() {
        let stacked_predator =
            expected_group("experiments/logs_PPO_stacked/predator/mcar_search_big").unwrap();
        assert!(model_matches(stacked_predator, "pred"));
        assert!(model_matches(stacked_predator, "predator"));
        assert!(!model_matches(stacked_predator, "icu"));
        let unstacked_predator =
            expected_group("experiments/logs_PPO/predator/mcar_search_big").unwrap();
        assert!(model_matches(unstacked_predator, "predator"));
        assert!(!model_matches(unstacked_predator, "pred"));
    }

    #[test]
    fn group_lookup_rejects_unknown_group() {
        assert!(expected_group("experiments/logs_PPO/icu/not-a-pattern").is_none());
    }

    #[test]
    fn median_handles_even_and_odd_sample_sizes() {
        assert_eq!(median(&mut [1.0, 3.0][..]), 2.0);
        assert_eq!(median(&mut [5.0, 1.0, 3.0][..]), 3.0);
    }

    #[test]
    fn malformed_and_nonfinite_rows_fail() {
        assert!(parse_run(&["run"]).is_err());
        let line = "run\texperiments/logs_PPO/icu/mnar-ident\tno\ticu\tmnar-ident\t1\t720\t1000000\t2000\t1\t2\t32\tNaN\t1\t{}";
        assert!(parse_run(&line.split('\t').collect::<Vec<_>>()).is_err());
    }
}
