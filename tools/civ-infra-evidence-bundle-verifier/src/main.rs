use civ_infra_evidence_bundle_verifier::verify_bundle;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("civ-infra-evidence-bundle-verifier: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 || args.len() > 4 {
        return Err(
            "usage: civ-infra-evidence-bundle-verifier MANIFEST.json BUNDLE_ROOT [REPORT.json]"
                .to_owned(),
        );
    }

    let manifest_path = &args[1];
    let root = Path::new(&args[2]);
    let bytes = fs::read(manifest_path)
        .map_err(|error| format!("cannot read manifest {manifest_path:?}: {error}"))?;
    let report = verify_bundle(root, &bytes)?;
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("cannot serialize report: {error}"))?;

    if let Some(output_path) = args.get(3) {
        fs::write(output_path, &report_bytes)
            .map_err(|error| format!("cannot write report {output_path:?}: {error}"))?;
        println!(
            "report_written={} manifest_sha256={} artifacts={} verified={} failed={}",
            output_path,
            report.input_manifest_sha256,
            report.artifact_count,
            report.verified_artifact_count,
            report.failed_artifact_count
        );
    } else {
        println!(
            "{}",
            String::from_utf8(report_bytes)
                .map_err(|error| format!("report is not valid UTF-8: {error}"))?
        );
    }

    if report.qualified {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}
