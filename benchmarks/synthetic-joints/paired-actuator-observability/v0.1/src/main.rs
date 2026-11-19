use paired_actuator_source_replay::{json_quote, CsvTable, RunLedgerSummary, SpeedTraceSummary};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

#[derive(Debug)]
struct TraceInput {
    label: String,
    path: PathBuf,
}

#[derive(Debug)]
struct Options {
    runlog: PathBuf,
    traces: Vec<TraceInput>,
    q_window_deg: f64,
    minimum_points: usize,
}

fn usage() -> &'static str {
    "Usage:
  paired-actuator-source-replay --runlog PATH --trace LABEL=PATH [--trace LABEL=PATH ...]
      [--q-window-deg DEGREES] [--min-points COUNT]

Options:
  --runlog PATH              Source run-ledger CSV (requires run_id, experiment, valid).
  --trace LABEL=PATH         Raw encoder CSV (requires rep, t_s, q_deg). Repeatable.
  --q-window-deg DEGREES     Symmetric joint-angle window; default: 8.
  --min-points COUNT         Minimum samples per repetition; default: 15.
  -h, --help                 Show this help.

The tool reconstructs source summaries from supplied files. It does not establish
independent physical replication or infer per-actuator mechanical torque sharing."
}

fn parse_options() -> Result<Option<Options>, String> {
    let mut args = env::args().skip(1);
    let mut runlog: Option<PathBuf> = None;
    let mut traces = Vec::new();
    let mut q_window_deg = 8.0_f64;
    let mut minimum_points = 15_usize;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(None),
            "--runlog" => {
                let value = args.next().ok_or("--runlog requires a path")?;
                if runlog.replace(PathBuf::from(value)).is_some() {
                    return Err("--runlog may only be supplied once".to_owned());
                }
            }
            "--trace" => {
                let value = args.next().ok_or("--trace requires LABEL=PATH")?;
                let (label, path) = value
                    .split_once('=')
                    .ok_or("--trace value must have the form LABEL=PATH")?;
                if label.trim().is_empty() || path.trim().is_empty() {
                    return Err("--trace label and path must both be nonempty".to_owned());
                }
                if traces.iter().any(|trace: &TraceInput| trace.label == label) {
                    return Err(format!("duplicate trace label {label:?}"));
                }
                traces.push(TraceInput {
                    label: label.to_owned(),
                    path: PathBuf::from(path),
                });
            }
            "--q-window-deg" => {
                let value = args.next().ok_or("--q-window-deg requires a number")?;
                q_window_deg = value
                    .parse::<f64>()
                    .map_err(|error| format!("invalid --q-window-deg {value:?}: {error}"))?;
                if !q_window_deg.is_finite() || q_window_deg <= 0.0 || q_window_deg > 180.0 {
                    return Err("--q-window-deg must be finite and in (0, 180]".to_owned());
                }
            }
            "--min-points" => {
                let value = args.next().ok_or("--min-points requires an integer")?;
                minimum_points = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --min-points {value:?}: {error}"))?;
                if minimum_points < 3 {
                    return Err("--min-points must be at least 3".to_owned());
                }
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }

    let runlog = runlog.ok_or("--runlog is required")?;
    if traces.is_empty() {
        return Err("at least one --trace LABEL=PATH is required".to_owned());
    }

    Ok(Some(Options {
        runlog,
        traces,
        q_window_deg,
        minimum_points,
    }))
}

fn read_csv(path: &PathBuf, kind: &str) -> Result<CsvTable, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {kind} {}: {error}", path.display()))?;
    CsvTable::parse(&content)
        .map_err(|error| format!("failed to parse {kind} {}: {error}", path.display()))
}

fn run(options: Options) -> Result<String, String> {
    let ledger_table = read_csv(&options.runlog, "run ledger")?;
    let ledger = RunLedgerSummary::from_csv(&ledger_table)?;

    let mut trace_json = Vec::with_capacity(options.traces.len());
    for trace in &options.traces {
        let table = read_csv(&trace.path, "speed trace")?;
        let summary = SpeedTraceSummary::from_csv(
            &table,
            options.q_window_deg,
            options.minimum_points,
        )
        .map_err(|error| format!("trace {}: {error}", trace.label))?;
        trace_json.push(summary.to_json(&trace.label));
    }

    let traces = trace_json.join(",");
    Ok(format!(
        "{{\"run_ledger\":{},\"speed_traces\":[{}],\"interpretation\":{}}}",
        ledger.to_json(),
        traces,
        json_quote("Source-data reconstruction only. Speed is encoder-derived OLS slope within the declared angle window. Ledger inclusion is not task success. No independent physical replication or per-actuator mechanical torque share is inferred.")
    ))
}

fn main() {
    match parse_options() {
        Ok(None) => {
            println!("{}", usage());
        }
        Err(error) => {
            eprintln!("error: {error}\n\n{}", usage());
            process::exit(2);
        }
        Ok(Some(options)) => match run(options) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("error: {error}");
                process::exit(1);
            }
        },
    }
}
