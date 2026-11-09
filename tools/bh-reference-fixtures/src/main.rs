use std::{env, fs, process};

use bh_reference_fixtures::continuation::{
    KerrQnmSpinContinuationTrace, QnmExtremalDampingAnalysisConfig,
    QnmExtremalDampingBehavior, analyze_extremal_damping,
};
use bh_reference_fixtures::qnm::{
    QnmFamilyAnalysisConfig, QnmFamilyAsymptoticTargets, QnmFamilyClassification,
    analyze_qnm_family_sequence,
};
use bh_reference_fixtures::{
    KerrQnmSequenceFixture, RadialComparisonConfig, RadialComparisonStatus, ReferenceFixture,
    canonical_payload_sha256, compare_radial_fixtures,
};
use serde::Deserialize;
use serde_json::json;

fn read_radial_fixture(path: &str) -> Result<ReferenceFixture, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read radial fixture {path}: {error}"))?;
    let fixture: ReferenceFixture = serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse radial fixture {path}: {error}"))?;
    fixture
        .validate()
        .map_err(|error| format!("invalid radial fixture {path}: {error}"))?;
    Ok(fixture)
}

fn read_qnm_sequence(path: &str) -> Result<KerrQnmSequenceFixture, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read QNM sequence {path}: {error}"))?;
    let sequence: KerrQnmSequenceFixture = serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse QNM sequence {path}: {error}"))?;
    sequence
        .validate()
        .map_err(|error| format!("invalid QNM sequence {path}: {error}"))?;
    Ok(sequence)
}

fn read_qnm_continuation(path: &str) -> Result<KerrQnmSpinContinuationTrace, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read QNM continuation trace {path}: {error}"))?;
    let trace: KerrQnmSpinContinuationTrace = serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse QNM continuation trace {path}: {error}"))?;
    trace
        .validate()
        .map_err(|error| format!("invalid QNM continuation trace {path}: {error}"))?;
    Ok(trace)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QnmAnalysisRequest {
    targets: QnmFamilyAsymptoticTargets,
    #[serde(default)]
    config: QnmFamilyAnalysisConfig,
}

fn read_qnm_analysis_request(path: &str) -> Result<QnmAnalysisRequest, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read QNM analysis request {path}: {error}"))?;
    serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse QNM analysis request {path}: {error}"))
}

fn print_usage() {
    eprintln!(
        "Usage:\n  bh-reference-fixtures validate-radial <fixture.json>\n  \
         bh-reference-fixtures validate-qnm <sequence.json>\n  \
         bh-reference-fixtures validate-continuation <trace.json>\n  \
         bh-reference-fixtures analyze-continuation <trace.json>\n  \
         bh-reference-fixtures analyze-qnm <sequence.json> <analysis.json>\n  \
         bh-reference-fixtures compare-radial <left.json> <right.json> <abs-tol> <rel-tol>"
    );
}

fn run(args: &[String]) -> Result<i32, String> {
    match args {
        [command, path] if command == "validate-radial" => {
            let fixture = read_radial_fixture(path)?;
            let payload_sha256 = canonical_payload_sha256(&fixture)
                .map_err(|error| format!("cannot canonicalize radial fixture {path}: {error}"))?;
            println!(
                "{}",
                json!({
                    "status": "valid",
                    "schema_version": fixture.schema_version,
                    "fixture_id": fixture.fixture_id,
                    "payload_sha256": payload_sha256,
                    "radial_samples": fixture.data.radial_samples.len()
                })
            );
            Ok(0)
        }
        [command, path] if command == "validate-qnm" => {
            let sequence = read_qnm_sequence(path)?;
            let payload_sha256 = canonical_payload_sha256(&sequence)
                .map_err(|error| format!("cannot canonicalize QNM sequence {path}: {error}"))?;
            let mode_summaries = sequence
                .modes
                .iter()
                .map(|mode| {
                    json!({
                        "sequence_position": mode.sequence_position,
                        "overtone_index_n": mode.overtone_index,
                        "wkb_quantization_index_N": mode.wkb_quantization_index,
                        "solver_method": mode.solver_method,
                        "method_applicability": mode.method_applicability,
                        "initial_guess_source": mode.initial_guess_source,
                        "angular_branch_id": mode.angular_branch_id,
                        "normal_prolate_wkb_small_spin_parameter": sequence
                            .normal_prolate_wkb_small_spin_parameter(mode)
                    })
                })
                .collect::<Vec<_>>();
            println!(
                "{}",
                json!({
                    "status": "valid",
                    "schema_version": sequence.schema_version,
                    "sequence_id": sequence.sequence_id,
                    "payload_sha256": payload_sha256,
                    "family_classification": sequence.family_classification,
                    "spin_over_mass": sequence.spin_over_mass,
                    "angular_eigenvalue_convention": sequence.angular_eigenvalue_convention,
                    "normal_prolate_oscillator_level": sequence.normal_prolate_oscillator_level,
                    "modes": mode_summaries
                })
            );
            Ok(0)
        }
        [command, path] if command == "validate-continuation" => {
            let trace = read_qnm_continuation(path)?;
            let payload_sha256 = canonical_payload_sha256(&trace)
                .map_err(|error| format!("cannot canonicalize QNM continuation {path}: {error}"))?;
            let minimum_spin = trace
                .samples
                .iter()
                .map(|sample| sample.spin_over_mass)
                .fold(f64::INFINITY, f64::min);
            let maximum_spin = trace
                .samples
                .iter()
                .map(|sample| sample.spin_over_mass)
                .fold(f64::NEG_INFINITY, f64::max);
            let has_exact_extremal_sample = trace
                .samples
                .iter()
                .any(|sample| sample.spin_over_mass.abs() == 1.0);
            println!(
                "{}",
                json!({
                    "status": "valid",
                    "schema_version": trace.schema_version,
                    "trace_id": trace.trace_id,
                    "mode_lineage_id": trace.mode_lineage_id,
                    "payload_sha256": payload_sha256,
                    "riemann_sheet_kind": trace.riemann_sheet_kind,
                    "riemann_sheet_id": trace.riemann_sheet_id,
                    "continuation_class": trace.continuation_class,
                    "frequency_cut_relation": trace.frequency_cut_relation,
                    "extremal_damping_behavior": trace.extremal_damping_behavior,
                    "minimum_spin": minimum_spin,
                    "maximum_spin": maximum_spin,
                    "has_exact_extremal_sample": has_exact_extremal_sample,
                    "samples": trace.samples.len()
                })
            );
            match trace.extremal_damping_behavior {
                QnmExtremalDampingBehavior::FiniteDamping
                | QnmExtremalDampingBehavior::ZeroDamping => Ok(0),
                QnmExtremalDampingBehavior::Unresolved | QnmExtremalDampingBehavior::Ambiguous => {
                    Ok(6)
                }
            }
        }
        [command, path] if command == "analyze-continuation" => {
            let trace = read_qnm_continuation(path)?;
            let payload_sha256 = canonical_payload_sha256(&trace)
                .map_err(|error| format!("cannot canonicalize QNM continuation {path}: {error}"))?;
            let analysis = analyze_extremal_damping(
                &trace,
                QnmExtremalDampingAnalysisConfig::default(),
            )
            .map_err(|error| error.to_string())?;
            println!(
                "{}",
                json!({
                    "status": "analyzed",
                    "trace_id": trace.trace_id,
                    "mode_lineage_id": trace.mode_lineage_id,
                    "payload_sha256": payload_sha256,
                    "producer_damping_label": trace.extremal_damping_behavior,
                    "computed_damping_classification": analysis.classification,
                    "producer_label_matches_computed": analysis.producer_label_matches_analysis,
                    "analysis": analysis
                })
            );
            match analysis.classification {
                QnmExtremalDampingBehavior::FiniteDamping
                | QnmExtremalDampingBehavior::ZeroDamping => Ok(0),
                QnmExtremalDampingBehavior::Unresolved
                | QnmExtremalDampingBehavior::Ambiguous => Ok(6),
            }
        }
        [command, sequence_path, analysis_path] if command == "analyze-qnm" => {
            let sequence = read_qnm_sequence(sequence_path)?;
            let payload_sha256 = canonical_payload_sha256(&sequence).map_err(|error| {
                format!("cannot canonicalize QNM sequence {sequence_path}: {error}")
            })?;
            let request = read_qnm_analysis_request(analysis_path)?;
            let analysis = analyze_qnm_family_sequence(&sequence, request.targets, request.config)
                .map_err(|error| error.to_string())?;
            let producer_label_matches_computed =
                sequence.family_classification == analysis.classification;
            println!(
                "{}",
                json!({
                    "status": "analyzed",
                    "sequence_id": sequence.sequence_id,
                    "payload_sha256": payload_sha256,
                    "producer_family_label": sequence.family_classification,
                    "computed_family_classification": analysis.classification,
                    "producer_label_matches_computed": producer_label_matches_computed,
                    "analysis": analysis
                })
            );
            match analysis.classification {
                QnmFamilyClassification::Prolate | QnmFamilyClassification::Oblate => Ok(0),
                QnmFamilyClassification::Unresolved | QnmFamilyClassification::Ambiguous => Ok(6),
            }
        }
        [command, left_path, right_path, abs_tol, rel_tol] if command == "compare-radial" => {
            let left = read_radial_fixture(left_path)?;
            let right = read_radial_fixture(right_path)?;
            let left_payload_sha256 = canonical_payload_sha256(&left).map_err(|error| {
                format!("cannot canonicalize left fixture {left_path}: {error}")
            })?;
            let right_payload_sha256 = canonical_payload_sha256(&right).map_err(|error| {
                format!("cannot canonicalize right fixture {right_path}: {error}")
            })?;
            let absolute_tolerance = abs_tol
                .parse::<f64>()
                .map_err(|error| format!("invalid absolute tolerance {abs_tol}: {error}"))?;
            let relative_tolerance = rel_tol
                .parse::<f64>()
                .map_err(|error| format!("invalid relative tolerance {rel_tol}: {error}"))?;
            let comparison = compare_radial_fixtures(
                &left,
                &right,
                RadialComparisonConfig {
                    absolute_tolerance,
                    relative_tolerance,
                },
            )
            .map_err(|error| error.to_string())?;
            let status = match comparison.status {
                RadialComparisonStatus::Unmatched => "unmatched",
                RadialComparisonStatus::Unresolved => "unresolved",
                RadialComparisonStatus::NumericallyAgreeing => "numerically_agreeing",
                RadialComparisonStatus::NumericallyDisagreeing => "numerically_disagreeing",
            };
            println!(
                "{}",
                json!({
                    "status": status,
                    "left_payload_sha256": left_payload_sha256,
                    "right_payload_sha256": right_payload_sha256,
                    "comparison": comparison,
                    "interpolation_performed": false
                })
            );
            match comparison.status {
                RadialComparisonStatus::NumericallyAgreeing => Ok(0),
                RadialComparisonStatus::Unmatched => Ok(4),
                RadialComparisonStatus::NumericallyDisagreeing => Ok(5),
                RadialComparisonStatus::Unresolved => Ok(6),
            }
        }
        _ => {
            print_usage();
            Err("invalid command or argument count".to_owned())
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            2
        }
    };
    process::exit(code);
}
