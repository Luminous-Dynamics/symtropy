use std::{env, fs, process};

use haps_network_allocation::{
    evaluate_window, parse_trace_json, select_static_assignment, AllocationPolicy,
};

struct Cli {
    input_path: String,
    policy: String,
    beam_budget: usize,
    per_beam_capacity_mbps: f64,
    start_day: usize,
    days: Option<usize>,
    training_days: Option<usize>,
}

fn usage() -> &'static str {
    "Usage:
  haps-network-allocation <traffic-trace.json> --beam-budget N --beam-capacity-mbps M [options]

Options:
  --policy greedy|static-train   Default: greedy
  --start-day N                  Zero-based daily sample index; default: 0
  --days N                       Evaluation days; default: remainder of trace
  --training-days N              Required for static-train; selects fixed cells
                                 from days [0, N), which must end before start-day
  --help                         Print this message

This tool allocates abstract beams to critical excess only. It does not model
radio propagation, SINR, interference, spectrum, HAPS positioning, or flight."
}

fn parse_usize(flag: &str, value: Option<String>) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("{flag} requires a value"))?
        .parse::<usize>()
        .map_err(|_| format!("{flag} must be a non-negative integer"))
}

fn parse_f64(flag: &str, value: Option<String>) -> Result<f64, String> {
    let parsed = value
        .ok_or_else(|| format!("{flag} requires a value"))?
        .parse::<f64>()
        .map_err(|_| format!("{flag} must be a number"))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("{flag} must be finite and greater than zero"));
    }
    Ok(parsed)
}

fn parse_cli() -> Result<Option<Cli>, String> {
    let mut args = env::args().skip(1);
    let Some(input_path) = args.next() else {
        return Ok(None);
    };
    if input_path == "--help" || input_path == "-h" {
        return Ok(None);
    }

    let mut policy = "greedy".to_owned();
    let mut beam_budget = None;
    let mut per_beam_capacity_mbps = None;
    let mut start_day = 0usize;
    let mut days = None;
    let mut training_days = None;

    while let Some(flag) = args.next() {
        let value = args.next();
        match flag.as_str() {
            "--policy" => {
                policy = value.ok_or_else(|| "--policy requires a value".to_owned())?;
                if policy != "greedy" && policy != "static-train" {
                    return Err("--policy must be greedy or static-train".to_owned());
                }
            }
            "--beam-budget" => beam_budget = Some(parse_usize("--beam-budget", value)?),
            "--beam-capacity-mbps" => {
                per_beam_capacity_mbps = Some(parse_f64("--beam-capacity-mbps", value)?)
            }
            "--start-day" => start_day = parse_usize("--start-day", value)?,
            "--days" => days = Some(parse_usize("--days", value)?),
            "--training-days" => training_days = Some(parse_usize("--training-days", value)?),
            _ => return Err(format!("unknown option {flag}\n\n{}", usage())),
        }
    }

    Ok(Some(Cli {
        input_path,
        policy,
        beam_budget: beam_budget.ok_or_else(|| "--beam-budget is required".to_owned())?,
        per_beam_capacity_mbps: per_beam_capacity_mbps
            .ok_or_else(|| "--beam-capacity-mbps is required".to_owned())?,
        start_day,
        days,
        training_days,
    }))
}

fn run() -> Result<(), String> {
    let Some(cli) = parse_cli()? else {
        println!("{}", usage());
        return Ok(());
    };

    let bytes = fs::read(&cli.input_path)
        .map_err(|error| format!("could not read {}: {error}", cli.input_path))?;
    let trace = parse_trace_json(&bytes).map_err(|error| error.to_string())?;
    let days = cli
        .days
        .unwrap_or_else(|| trace.intervals.len().saturating_sub(cli.start_day));

    let policy = match cli.policy.as_str() {
        "greedy" => {
            if cli.training_days.is_some() {
                return Err("--training-days is only valid with --policy static-train".to_owned());
            }
            AllocationPolicy::CriticalExcessWeightedGreedy
        }
        "static-train" => {
            let training_days = cli
                .training_days
                .ok_or_else(|| "static-train requires --training-days".to_owned())?;
            if training_days == 0 || training_days > cli.start_day {
                return Err(
                    "--training-days must be positive and finish no later than start-day"
                        .to_owned(),
                );
            }
            let selected = select_static_assignment(&trace, training_days, cli.beam_budget)
                .map_err(|error| error.to_string())?;
            AllocationPolicy::StaticFixedAssignment(selected)
        }
        _ => return Err("unsupported policy".to_owned()),
    };

    let report = evaluate_window(
        &trace,
        cli.start_day,
        days,
        cli.beam_budget,
        cli.per_beam_capacity_mbps,
        policy,
    )
    .map_err(|error| error.to_string())?;

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("could not serialize report: {error}"))?;
    println!("{json}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("haps-network-allocation: {error}");
        process::exit(2);
    }
}
