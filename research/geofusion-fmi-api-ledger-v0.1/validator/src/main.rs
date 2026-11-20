use geofusion_fmi_api_ledger::audit_file;
use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(ledger_path) = args.next() else {
        eprintln!("usage: geofusion-fmi-api-ledger <ledger.json> [artifact-root]");
        return ExitCode::from(2);
    };
    let ledger_path = PathBuf::from(ledger_path);
    let root = args.next().map(PathBuf::from).unwrap_or_else(|| {
        ledger_path.parent()
            .and_then(|path| path.parent())
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf()
    });
    match audit_file(&ledger_path, &root) {
        Ok(report) => {
            println!("GeoFusion FMI API ledger audit: PASS");
            println!("federates={} events={} do_step_attempts={} exchanged_values={} barriers={}",
                report.federates, report.events, report.do_step_attempts,
                report.exchanged_values, report.barriers);
            println!("artifacts_verified={} synthetic={} state_checkpointing={}",
                report.artifacts_verified, report.synthetic, report.state_checkpointing);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("GeoFusion FMI API ledger audit: FAIL: {error}");
            ExitCode::FAILURE
        }
    }
}
