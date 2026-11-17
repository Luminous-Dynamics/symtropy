use global_mfg_deployment_audit::audit_corpus;
use std::env;
use std::fs;

fn main() {
    if let Err(error) = run() {
        eprintln!("global_mfg_deployment_audit: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: global_mfg_deployment_audit <corpus.json>".to_owned())?;
    let input = fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let report = audit_corpus(&input)?;
    let output =
        serde_json::to_string_pretty(&report).map_err(|e| format!("cannot encode report: {e}"))?;
    println!("{output}");
    Ok(())
}
