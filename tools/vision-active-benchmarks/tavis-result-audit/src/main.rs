use std::env;
use std::fs;
use std::path::PathBuf;

use tavis_result_audit::{audit_run, cross_run_pairing_ready, fmt_opt_pct, tsv};

#[derive(Debug)]
struct Args {
    runs: Vec<(String, PathBuf)>,
    expected_tasks: Vec<String>,
    expected_modes: Vec<String>,
    expected_episodes: usize,
    out_dir: PathBuf,
    allow_unmanifested: bool,
}

fn usage() -> &'static str {
    "TAVIS native result audit\n\
     Usage:\n\
       tavis-result-audit --run LABEL=DIR [--run LABEL=DIR ...]\n\
         --expected-tasks task1,task2,... [--expected-modes id,ood_spatial]\n\
         [--expected-episodes 96] [--out-dir tavis-audit] [--allow-unmanifested]\n\
\n\
     Each run directory must contain official TAVIS per-(task, eval_mode) JSON files.\n\
     Strict mode requires audit-manifest.json with pinned upstream/checkpoint/assets/environment hashes.\n\
     Use --allow-unmanifested only for smoke tests; it does not waive missing cells or incomplete episodes."
}

fn parse_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_args() -> Result<Args, String> {
    let mut args = env::args().skip(1);
    let mut runs = Vec::new();
    let mut expected_tasks = Vec::new();
    let mut expected_modes = vec!["id".to_string(), "ood_spatial".to_string()];
    let mut expected_episodes = 96_usize;
    let mut out_dir = PathBuf::from("tavis-audit");
    let mut allow_unmanifested = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--run" => {
                let raw = args.next().ok_or("--run requires LABEL=DIR")?;
                let (label, dir) = raw.split_once('=').ok_or("--run requires LABEL=DIR")?;
                if label.trim().is_empty() || dir.trim().is_empty() {
                    return Err("--run requires non-empty LABEL and DIR".into());
                }
                runs.push((label.trim().to_string(), PathBuf::from(dir.trim())));
            }
            "--expected-tasks" => {
                expected_tasks = parse_csv(&args.next().ok_or("--expected-tasks requires CSV")?);
            }
            "--expected-modes" => {
                expected_modes = parse_csv(&args.next().ok_or("--expected-modes requires CSV")?);
            }
            "--expected-episodes" => {
                expected_episodes = args
                    .next()
                    .ok_or("--expected-episodes requires an integer")?
                    .parse()
                    .map_err(|_| "--expected-episodes must be an integer")?;
            }
            "--out-dir" => out_dir = PathBuf::from(args.next().ok_or("--out-dir requires PATH")?),
            "--allow-unmanifested" => allow_unmanifested = true,
            "-h" | "--help" => return Err(usage().to_string()),
            other => return Err(format!("unknown argument: {other}\n\n{}", usage())),
        }
    }

    if runs.is_empty() {
        return Err(format!("at least one --run is required\n\n{}", usage()));
    }
    if expected_tasks.is_empty() {
        return Err(format!("--expected-tasks is required\n\n{}", usage()));
    }
    if expected_modes.is_empty() {
        return Err("--expected-modes may not be empty".into());
    }
    if expected_episodes == 0 {
        return Err("--expected-episodes must be positive".into());
    }
    let mut labels = std::collections::BTreeSet::new();
    for (label, _) in &runs {
        if !labels.insert(label.clone()) {
            return Err(format!("duplicate run label: {label}"));
        }
    }
    Ok(Args {
        runs,
        expected_tasks,
        expected_modes,
        expected_episodes,
        out_dir,
        allow_unmanifested,
    })
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let mut runs = Vec::new();
    let mut fatal = Vec::new();
    for (label, dir) in &args.runs {
        match audit_run(
            label,
            dir,
            &args.expected_tasks,
            &args.expected_modes,
            args.expected_episodes,
        ) {
            Ok(run) => {
                for error in &run.status.errors {
                    fatal.push(format!("{label}: {error}"));
                }
                if !run.status.manifest_valid && !args.allow_unmanifested {
                    fatal.push(format!("{label}: provenance manifest is absent or invalid (use --allow-unmanifested only for smoke tests)"));
                }
                runs.push(run);
            }
            Err(errors) => {
                fatal.extend(errors.into_iter().map(|error| format!("{label}: {error}")))
            }
        }
    }

    if runs.len() > 1 && !cross_run_pairing_ready(&runs) {
        eprintln!(
            "PAIRING STATUS: unavailable. No paired episode effect may be computed because one or more runs lack stable episode IDs/seeds or their sets differ."
        );
    }

    if let Err(error) = fs::create_dir_all(&args.out_dir) {
        eprintln!(
            "cannot create output directory {}: {error}",
            args.out_dir.display()
        );
        std::process::exit(2);
    }

    let mut per_task = String::from(
        "run\tmodel\tcamera\trobot\tcheckpoint\ttask\teval_mode\tepisodes\tsuccesses\tsuccess_rate\twilson95_low\twilson95_high\tgalt_valid_n\tgalt_mean_s\tgalt_median_s\tgalt_reason_counts\tepisode_ids_present\tseeds_present\tcell_status\n",
    );
    let mut summaries = String::from(
        "run\tmodel\tcamera\trobot\teval_mode\tcells_present\tepisodes\tsuccesses\tofficial_pooled_success_rate\twilson95_low\twilson95_high\ttask_macro_success_rate\tpaired_episode_analysis_ready\n",
    );
    let mut audit = String::from(
        "run\trun_dir\tresult_files\tvalid_cells\texpected_cells\tmissing_cells\tunexpected_cells\tprovenance_manifest_valid\tepisode_ids_present\tseeds_present\tcross_run_pairing_ready\terrors\n",
    );

    let pair_ready = cross_run_pairing_ready(&runs);
    for run in &runs {
        let cells_present = run.cells.len();
        for cell in &run.cells {
            let reason_counts = cell
                .galt
                .reason_counts
                .iter()
                .map(|(reason, count)| format!("{reason}:{count}"))
                .collect::<Vec<_>>()
                .join(";");
            let cell_status = if cell.errors.is_empty() {
                "ok"
            } else {
                "error"
            };
            per_task.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                tsv(&run.label),
                tsv(cell.meta.model.as_deref().unwrap_or("NA")),
                tsv(cell.meta.camera.as_deref().unwrap_or("NA")),
                tsv(cell.meta.robot.as_deref().unwrap_or("NA")),
                tsv(cell.meta.checkpoint.as_deref().unwrap_or("NA")),
                tsv(&cell.meta.task),
                tsv(&cell.meta.eval_mode),
                cell.proportion.total,
                cell.proportion.successes,
                fmt_opt_pct(cell.proportion.rate),
                fmt_opt_pct(cell.proportion.low_95),
                fmt_opt_pct(cell.proportion.high_95),
                cell.galt.valid_n,
                fmt_opt_pct(cell.galt.mean_s),
                fmt_opt_pct(cell.galt.median_s),
                tsv(&reason_counts),
                cell.episode_ids_present,
                cell.seeds_present,
                cell_status,
            ));
        }

        for mode in &args.expected_modes {
            let proportion = run.official_pooled_by_mode.get(mode);
            let success = proportion.and_then(|p| p.rate);
            let low = proportion.and_then(|p| p.low_95);
            let high = proportion.and_then(|p| p.high_95);
            let successes = proportion.map(|p| p.successes).unwrap_or(0);
            let episodes = proportion.map(|p| p.total).unwrap_or(0);
            let mode_cells = run
                .cells
                .iter()
                .filter(|cell| cell.meta.eval_mode == *mode)
                .count();
            summaries.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                tsv(&run.label),
                tsv(run
                    .cells
                    .first()
                    .and_then(|c| c.meta.model.as_deref())
                    .unwrap_or("NA")),
                tsv(run
                    .cells
                    .first()
                    .and_then(|c| c.meta.camera.as_deref())
                    .unwrap_or("NA")),
                tsv(run
                    .cells
                    .first()
                    .and_then(|c| c.meta.robot.as_deref())
                    .unwrap_or("NA")),
                tsv(mode),
                mode_cells,
                episodes,
                successes,
                fmt_opt_pct(success),
                fmt_opt_pct(low),
                fmt_opt_pct(high),
                fmt_opt_pct(run.task_macro_by_mode.get(mode).copied().flatten()),
                pair_ready,
            ));
        }

        let status_errors = if run.status.errors.is_empty() {
            "none".to_string()
        } else {
            run.status.errors.join("; ")
        };
        audit.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            tsv(&run.label),
            tsv(&run.dir.display().to_string()),
            run.status.result_files,
            run.status.valid_cells,
            args.expected_tasks.len() * args.expected_modes.len(),
            tsv(&run.status.missing_cells.join(",")),
            tsv(&run.status.unexpected_cells.join(",")),
            run.status.manifest_valid,
            run.status.episode_ids_present,
            run.status.seeds_present,
            pair_ready,
            tsv(&status_errors),
        ));
    }

    for (name, content) in [
        ("per-task.tsv", per_task),
        ("summary.tsv", summaries),
        ("audit.tsv", audit),
    ] {
        if let Err(error) = fs::write(args.out_dir.join(name), content) {
            eprintln!(
                "cannot write {}: {error}",
                args.out_dir.join(name).display()
            );
            std::process::exit(2);
        }
    }

    if !fatal.is_empty() {
        for error in fatal {
            eprintln!("AUDIT FAILURE: {error}");
        }
        std::process::exit(2);
    }
    println!("TAVIS audit reports written to {}", args.out_dir.display());
}
