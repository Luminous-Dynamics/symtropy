// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

use thermo_temporal_fairness::{evaluate, AgentSeries, AllocationTrace, Fraction};

const MAX_TICKS: u64 = 1_000_000;

#[derive(Clone, Debug)]
struct Observation {
    tick: u64,
    agent_id: u64,
    demand: u64,
    eligible: bool,
    allocated: u64,
}

fn parse_u64(value: &str, line: usize, field: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("line {line}: {field} must be an unsigned 64-bit integer"))
}

fn parse_bool(value: &str, line: usize, field: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("line {line}: {field} must be true or false")),
    }
}

fn parse_trace(input: &str) -> Result<AllocationTrace, String> {
    let mut lines = input.lines().enumerate().filter_map(|(index, raw)| {
        let trimmed = raw.trim();
        (!trimmed.is_empty() && !trimmed.starts_with('#')).then_some((index + 1, raw))
    });
    let Some((header_line, header)) = lines.next() else {
        return Err("trace file is empty".to_owned());
    };
    if header != "THERMO-TEMPORAL-TRACE\t1" {
        return Err(format!(
            "line {header_line}: expected THERMO-TEMPORAL-TRACE<TAB>1 header"
        ));
    }

    let mut case_id = None;
    let mut observations = Vec::new();
    for (line_number, raw) in lines {
        let fields: Vec<&str> = raw.split('\t').collect();
        match fields.first().copied().unwrap_or("") {
            "CASE" if fields.len() == 2 => {
                if case_id.replace(fields[1].to_owned()).is_some() {
                    return Err(format!("line {line_number}: duplicate CASE row"));
                }
            }
            "OBS" if fields.len() == 6 => observations.push(Observation {
                tick: parse_u64(fields[1], line_number, "tick")?,
                agent_id: parse_u64(fields[2], line_number, "agent ID")?,
                demand: parse_u64(fields[3], line_number, "demand")?,
                eligible: parse_bool(fields[4], line_number, "eligibility")?,
                allocated: parse_u64(fields[5], line_number, "allocation")?,
            }),
            _ => {
                return Err(format!(
                    "line {line_number}: unknown row or wrong field count: {raw}"
                ));
            }
        }
    }

    let case_id = case_id.ok_or_else(|| "missing CASE row".to_owned())?;
    if observations.is_empty() {
        return Err("trace must contain at least one OBS row".to_owned());
    }

    let ids: BTreeSet<u64> = observations.iter().map(|row| row.agent_id).collect();
    let max_tick = observations.iter().map(|row| row.tick).max().unwrap_or(0);
    if max_tick >= MAX_TICKS {
        return Err(format!("trace exceeds supported tick bound {MAX_TICKS}"));
    }
    let tick_count = usize::try_from(max_tick + 1)
        .map_err(|_| "tick count does not fit this platform".to_owned())?;
    if tick_count > observations.len() / ids.len() {
        return Err("trace has missing tick/agent observations".to_owned());
    }

    let mut by_agent: BTreeMap<u64, Vec<Option<Observation>>> = ids
        .iter()
        .map(|id| (*id, vec![None; tick_count]))
        .collect();
    for row in observations {
        let slot = by_agent
            .get_mut(&row.agent_id)
            .expect("agent IDs were collected from the observation set");
        let index = usize::try_from(row.tick).map_err(|_| "tick index conversion failed".to_owned())?;
        if slot[index].replace(row).is_some() {
            return Err(format!("duplicate observation for tick {index}"));
        }
    }

    let mut agents = Vec::with_capacity(ids.len());
    for (agent_id, rows) in by_agent {
        let mut demand = Vec::with_capacity(tick_count);
        let mut eligible = Vec::with_capacity(tick_count);
        let mut allocated = Vec::with_capacity(tick_count);
        for (tick, row) in rows.into_iter().enumerate() {
            let row = row.ok_or_else(|| format!("agent {agent_id} has no observation for tick {tick}"))?;
            demand.push(row.demand);
            eligible.push(row.eligible);
            allocated.push(row.allocated);
        }
        agents.push(AgentSeries {
            id: agent_id,
            demand,
            eligible,
            allocated,
        });
    }

    Ok(AllocationTrace { case_id, agents })
}

fn format_fraction(fraction: &Fraction) -> String {
    format!("{}/{}", fraction.numerator(), fraction.denominator())
}

fn format_optional_fraction(fraction: Option<&Fraction>) -> String {
    fraction.map(format_fraction).unwrap_or_else(|| "none".to_owned())
}

fn format_profile(profile: &[Fraction]) -> String {
    profile.iter().map(format_fraction).collect::<Vec<_>>().join(",")
}

fn usage() -> &'static str {
    "usage: thermo-temporal-fairness --audit <trace.tsv>"
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 3 || arguments[1] != "--audit" {
        eprintln!("{}", usage());
        process::exit(2);
    }
    let input = match fs::read_to_string(&arguments[2]) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("input_error: cannot read {}: {error}", arguments[2]);
            process::exit(2);
        }
    };
    let trace = match parse_trace(&input) {
        Ok(trace) => trace,
        Err(error) => {
            eprintln!("input_error: {error}");
            process::exit(2);
        }
    };

    let report = evaluate(&trace);
    println!("case_id={}", report.case_id);
    println!("valid_trace={}", report.valid_trace);
    println!("tick_count={}", report.tick_count);
    println!(
        "sorted_horizon_service_profile={}",
        format_profile(&report.sorted_horizon_service_profile)
    );
    println!(
        "worst_eligible_slot_service_ratio={}",
        format_optional_fraction(report.worst_eligible_slot_service_ratio.as_ref())
    );
    println!(
        "minimum_horizon_service_ratio={}",
        format_optional_fraction(report.minimum_horizon_service_ratio.as_ref())
    );
    println!(
        "maximum_horizon_service_ratio={}",
        format_optional_fraction(report.maximum_horizon_service_ratio.as_ref())
    );
    for metrics in &report.per_agent {
        println!(
            "agent={} demand={} allocated={} unmet={} horizon_ratio={} eligible_demand_ticks={} starved_ticks={} max_starvation_streak={} eligible_demand={} eligible_allocated={} eligible_unmet={}",
            metrics.agent_id,
            metrics.total_demand,
            metrics.total_allocated,
            metrics.total_unmet_demand,
            format_optional_fraction(metrics.horizon_service_ratio.as_ref()),
            metrics.eligible_demand_ticks,
            metrics.starved_ticks,
            metrics.maximum_consecutive_starvation_ticks,
            metrics.total_eligible_demand,
            metrics.total_eligible_allocated,
            metrics.total_eligible_unmet_demand
        );
    }
    for issue in &report.violations {
        println!("violation={} detail={}", issue.code, issue.detail);
    }

    if !report.valid_trace {
        process::exit(1);
    }
}
