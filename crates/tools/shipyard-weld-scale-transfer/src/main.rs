use std::env;
use std::fs;
use std::process::ExitCode;

use shipyard_weld_scale_transfer::{
    TransferDisposition, TransferManifest, UncertaintyComponents, evaluate_transfer,
    validate_corpus,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let corpus_path = args
        .next()
        .unwrap_or_else(|| "benchmarks/shipyard/scale-transfer/corpus-v0.1.json".to_owned());
    let reference_path = args
        .next()
        .unwrap_or_else(|| "benchmarks/shipyard/scale-transfer/reference-chain-v0.1.json".to_owned());

    let corpus_bytes = match fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read corpus {corpus_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let summary = match validate_corpus(&corpus_bytes) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("scale-transfer corpus rejected: {error}");
            return ExitCode::FAILURE;
        }
    };

    let reference_bytes = match fs::read(&reference_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read reference manifest {reference_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let reference: TransferManifest = match serde_json::from_slice(&reference_bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("reference manifest parse failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let decision = evaluate_transfer(&reference);
    let Some(path) = decision.selected_path else {
        eprintln!(
            "reference bridge-chain oracle rejected: {:?} / {:?}",
            decision.disposition, decision.reason
        );
        return ExitCode::FAILURE;
    };
    if decision.disposition != TransferDisposition::AdmittedWithBridgeChain
        || !path
            .edge_ids
            .iter()
            .map(String::as_str)
            .eq(["E01", "E12", "E23"])
        || !path
            .domain_ids
            .iter()
            .map(String::as_str)
            .eq(["D0", "D1", "D2", "D3"])
        || path.cumulative_uncertainty_units != 45
        || path.minimum_empirical_coverage_permille != 960
        || path.uncertainty_components
            != (UncertaintyComponents {
                measurement_uncertainty_units: 3,
                parameter_uncertainty_units: 6,
                boundary_uncertainty_units: 9,
                model_form_uncertainty_units: 12,
                transfer_uncertainty_units: 15,
            })
        || decision.candidate_path_count != 1
    {
        eprintln!("reference bridge-chain known answer changed: {decision:?}");
        return ExitCode::FAILURE;
    }

    println!(
        "PASS {}: {} cases ({} positive, {} negative), fnv1a64={}",
        summary.benchmark_id,
        summary.cases,
        summary.positive,
        summary.negative,
        summary.corpus_fingerprint_fnv1a64
    );
    println!(
        "PASS bridge oracle: {} edges, total uncertainty {}, components M/P/B/F/T={}/{}/{}/{}/{}, minimum coverage {}‰",
        path.edge_ids.len(),
        path.cumulative_uncertainty_units,
        path.uncertainty_components.measurement_uncertainty_units,
        path.uncertainty_components.parameter_uncertainty_units,
        path.uncertainty_components.boundary_uncertainty_units,
        path.uncertainty_components.model_form_uncertainty_units,
        path.uncertainty_components.transfer_uncertainty_units,
        path.minimum_empirical_coverage_permille
    );
    ExitCode::SUCCESS
}
