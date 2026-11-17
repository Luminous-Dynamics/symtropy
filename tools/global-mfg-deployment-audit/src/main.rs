use global_mfg_deployment_audit::audit_schema_and_corpus;
use std::env;
use std::fs;

fn main() {
    if let Err(error) = run() {
        eprintln!("global_mfg_deployment_audit: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let schema_path = args.next().ok_or_else(|| {
        "usage: global_mfg_deployment_audit <schema.json> <corpus.json>".to_owned()
    })?;
    let corpus_path = args.next().ok_or_else(|| {
        "usage: global_mfg_deployment_audit <schema.json> <corpus.json>".to_owned()
    })?;
    if args.next().is_some() {
        return Err("unexpected extra arguments".to_owned());
    }

    let schema =
        fs::read_to_string(&schema_path).map_err(|e| format!("cannot read {schema_path}: {e}"))?;
    let corpus =
        fs::read_to_string(&corpus_path).map_err(|e| format!("cannot read {corpus_path}: {e}"))?;
    let report = audit_schema_and_corpus(&schema, &corpus)?;
    let output =
        serde_json::to_string_pretty(&report).map_err(|e| format!("cannot encode report: {e}"))?;
    println!("{output}");
    Ok(())
}
