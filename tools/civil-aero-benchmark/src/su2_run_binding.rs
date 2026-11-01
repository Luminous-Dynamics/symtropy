//! Binds one SU2 process receipt to its staged discipline inputs and typed history/screen outputs.
//!
//! This is an unsigned run-lineage integrity receipt. It does not attest source provenance,
//! host environment, resource telemetry, grid convergence, or experimental validation.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use crate::external_solver::{
    ExternalSolverReceipt, read_external_solver_receipt, verify_external_solver_receipt,
};
use crate::integrity::sha256_hex;
use crate::su2_convergence_screen::{ConvergenceScreen, verify_convergence_screen};
use crate::su2_history::{Su2HistoryReport, verify_su2_history_import};

pub const SU2_RUN_LINEAGE_SCHEMA: &str = "su2-run-lineage-binding/0.1.0";
pub const SU2_RUN_LINEAGE_EVIDENCE_LEVEL: &str = "replayed_process_receipt_bound_to_history_screen_and_staged_inputs_unsigned_not_build_source_attested_not_grid_admitted_not_experimentally_validated";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2RunInputBindings {
    pub physics_configuration_path: String,
    pub geometry_path: String,
    pub mesh_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2BoundArtifact {
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Su2RunLineageBinding {
    pub schema: String,
    pub run_id: String,
    pub solver_id: String,
    pub solver_version_claim: String,
    pub executable_sha256: String,
    pub runner_receipt_sha256: String,
    pub physics_configuration: Su2BoundArtifact,
    pub geometry: Su2BoundArtifact,
    pub mesh: Su2BoundArtifact,
    pub history: Su2BoundArtifact,
    pub history_import_sha256: String,
    pub convergence_screen_sha256: String,
    pub convergence_criterion_count: usize,
    pub convergence_passed_count: usize,
    pub additional_outputs: Vec<Su2BoundArtifact>,
    pub binding_sha256: String,
    pub evidence_level: String,
}

fn validate_relative_path(value: &str) -> Result<(), String> {
    if value.trim().is_empty()
        || value.contains('|')
        || value.contains(',')
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\t')
        || value.contains('\\')
    {
        return Err(
            "run-lineage path is empty or contains forbidden separators/control characters"
                .to_owned(),
        );
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!(
            "run-lineage path is not a safe relative path: {value}"
        ));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn staged_input<'a>(
    receipt: &'a ExternalSolverReceipt,
    relative_path: &str,
) -> Result<&'a crate::external_solver::StagedInputReceipt, String> {
    validate_relative_path(relative_path)?;
    let matches = receipt
        .staged_inputs
        .iter()
        .filter(|row| row.relative_path == relative_path)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "expected one staged input {relative_path}, found {}",
            matches.len()
        ));
    }
    let row = matches[0];
    if row.byte_length == 0 || !valid_sha256(&row.sha256) {
        return Err(format!(
            "staged input {relative_path} has malformed byte evidence"
        ));
    }
    Ok(row)
}

fn bound_output(
    receipt: &ExternalSolverReceipt,
    relative_path: &str,
) -> Result<Su2BoundArtifact, String> {
    validate_relative_path(relative_path)?;
    let matches = receipt
        .outputs
        .iter()
        .filter(|row| row.relative_path == relative_path)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "expected one declared output {relative_path}, found {}",
            matches.len()
        ));
    }
    let row = matches[0];
    if !row.present {
        return Err(format!("declared output {relative_path} is missing"));
    }
    let byte_length = row
        .byte_length
        .ok_or_else(|| format!("output {relative_path} has no byte length"))?;
    let sha256 = row
        .sha256
        .clone()
        .ok_or_else(|| format!("output {relative_path} has no digest"))?;
    if byte_length == 0 || !valid_sha256(&sha256) {
        return Err(format!(
            "output {relative_path} has malformed byte evidence"
        ));
    }
    Ok(Su2BoundArtifact {
        relative_path: relative_path.to_owned(),
        byte_length,
        sha256,
    })
}

fn append_string(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn append_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_artifact(bytes: &mut Vec<u8>, artifact: &Su2BoundArtifact) {
    append_string(bytes, &artifact.relative_path);
    append_u64(bytes, artifact.byte_length);
    append_string(bytes, &artifact.sha256);
}

fn canonical_binding_bytes(binding: &Su2RunLineageBinding) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [
        binding.schema.as_str(),
        binding.run_id.as_str(),
        binding.solver_id.as_str(),
        binding.solver_version_claim.as_str(),
        binding.executable_sha256.as_str(),
        binding.runner_receipt_sha256.as_str(),
        binding.history_import_sha256.as_str(),
        binding.convergence_screen_sha256.as_str(),
        binding.evidence_level.as_str(),
    ] {
        append_string(&mut bytes, value);
    }
    for artifact in [
        &binding.physics_configuration,
        &binding.geometry,
        &binding.mesh,
        &binding.history,
    ] {
        append_artifact(&mut bytes, artifact);
    }
    append_u64(&mut bytes, binding.convergence_criterion_count as u64);
    append_u64(&mut bytes, binding.convergence_passed_count as u64);
    append_u64(&mut bytes, binding.additional_outputs.len() as u64);
    for artifact in &binding.additional_outputs {
        append_artifact(&mut bytes, artifact);
    }
    bytes
}

/// Bind a replayed SU2 process receipt, three staged discipline inputs, an imported history,
/// a fully passing declared steady screen, and additional force/field output files.
pub fn bind_su2_run_lineage(
    run_directory: &Path,
    history: &Su2HistoryReport,
    screen: &ConvergenceScreen,
    inputs: &Su2RunInputBindings,
    additional_output_paths: &[String],
) -> Result<Su2RunLineageBinding, String> {
    verify_su2_history_import(history)?;
    verify_convergence_screen(screen)?;
    let receipt = read_external_solver_receipt(run_directory)?;
    verify_external_solver_receipt(run_directory, &receipt)?;
    if !receipt.is_process_success() || receipt.solver_id != "SU2_CFD" {
        return Err("run receipt must identify a successful SU2_CFD process".to_owned());
    }
    if history.run_id() != receipt.run_id
        || history.solver_id() != receipt.solver_id
        || screen.run_id != receipt.run_id
        || screen.solver_id != receipt.solver_id
    {
        return Err("process, history and convergence screen identities differ".to_owned());
    }
    if history.runner_receipt_sha256() != receipt.receipt_sha256
        || screen.runner_receipt_sha256 != receipt.receipt_sha256
        || history.executable_sha256() != receipt.executable_sha256
        || history.history_sha256() != screen.history_sha256
        || history.import_sha256() != screen.history_import_sha256
    {
        return Err("process, history and convergence screen digests do not agree".to_owned());
    }
    if screen.status != "declared_thresholds_met"
        || screen.failed_count != 0
        || screen.insufficient_count != 0
        || screen.passed_count != screen.results.len()
    {
        return Err(
            "every declared convergence criterion must pass before lineage binding".to_owned(),
        );
    }
    if inputs.physics_configuration_path == inputs.geometry_path
        || inputs.physics_configuration_path == inputs.mesh_path
        || inputs.geometry_path == inputs.mesh_path
    {
        return Err(
            "configuration, geometry and mesh must be three distinct staged inputs".to_owned(),
        );
    }
    let config = staged_input(&receipt, &inputs.physics_configuration_path)?;
    let geometry = staged_input(&receipt, &inputs.geometry_path)?;
    let mesh = staged_input(&receipt, &inputs.mesh_path)?;
    let history_artifact = bound_output(&receipt, &history.history_relative_path)?;

    let mut unique_outputs = BTreeSet::new();
    unique_outputs.insert(history.history_relative_path.as_str());
    if additional_output_paths.is_empty() {
        return Err("at least one additional force or field output must be bound".to_owned());
    }
    let mut additional_outputs = Vec::with_capacity(additional_output_paths.len());
    for path in additional_output_paths {
        if !unique_outputs.insert(path.as_str()) {
            return Err(format!(
                "duplicate or history path in additional outputs: {path}"
            ));
        }
        additional_outputs.push(bound_output(&receipt, path)?);
    }
    additional_outputs.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let mut binding = Su2RunLineageBinding {
        schema: SU2_RUN_LINEAGE_SCHEMA.to_owned(),
        run_id: receipt.run_id.clone(),
        solver_id: receipt.solver_id.clone(),
        solver_version_claim: receipt.solver_version_claim.clone(),
        executable_sha256: receipt.executable_sha256.clone(),
        runner_receipt_sha256: receipt.receipt_sha256.clone(),
        physics_configuration: Su2BoundArtifact {
            relative_path: inputs.physics_configuration_path.clone(),
            byte_length: config.byte_length,
            sha256: config.sha256.clone(),
        },
        geometry: Su2BoundArtifact {
            relative_path: inputs.geometry_path.clone(),
            byte_length: geometry.byte_length,
            sha256: geometry.sha256.clone(),
        },
        mesh: Su2BoundArtifact {
            relative_path: inputs.mesh_path.clone(),
            byte_length: mesh.byte_length,
            sha256: mesh.sha256.clone(),
        },
        history: history_artifact,
        history_import_sha256: history.import_sha256().to_owned(),
        convergence_screen_sha256: screen.screen_sha256.clone(),
        convergence_criterion_count: screen.results.len(),
        convergence_passed_count: screen.passed_count,
        additional_outputs,
        binding_sha256: String::new(),
        evidence_level: SU2_RUN_LINEAGE_EVIDENCE_LEVEL.to_owned(),
    };
    binding.binding_sha256 = sha256_hex(&canonical_binding_bytes(&binding))?;
    verify_su2_run_lineage_binding(
        run_directory,
        history,
        screen,
        inputs,
        additional_output_paths,
        &binding,
    )?;
    Ok(binding)
}

/// Offline replay: re-check the process receipt, staged inputs, output digests, typed history,
/// convergence screen and final canonical binding digest.
pub fn verify_su2_run_lineage_binding(
    run_directory: &Path,
    history: &Su2HistoryReport,
    screen: &ConvergenceScreen,
    inputs: &Su2RunInputBindings,
    additional_output_paths: &[String],
    binding: &Su2RunLineageBinding,
) -> Result<(), String> {
    if binding.schema != SU2_RUN_LINEAGE_SCHEMA
        || binding.evidence_level != SU2_RUN_LINEAGE_EVIDENCE_LEVEL
    {
        return Err("unsupported run-lineage schema or evidence ceiling".to_owned());
    }
    let root = fs::canonicalize(run_directory)
        .map_err(|error| format!("cannot resolve run directory: {error}"))?;
    let receipt = read_external_solver_receipt(&root)?;
    verify_external_solver_receipt(&root, &receipt)?;
    if !receipt.is_process_success()
        || receipt.run_id != binding.run_id
        || receipt.solver_id != binding.solver_id
        || receipt.solver_version_claim != binding.solver_version_claim
        || receipt.executable_sha256 != binding.executable_sha256
        || receipt.receipt_sha256 != binding.runner_receipt_sha256
    {
        return Err("replayed process receipt differs from run-lineage binding".to_owned());
    }
    if history.run_id() != binding.run_id
        || history.solver_id() != binding.solver_id
        || history.executable_sha256() != binding.executable_sha256
        || history.runner_receipt_sha256() != binding.runner_receipt_sha256
        || history.history_sha256() != binding.history.sha256
        || history.import_sha256() != binding.history_import_sha256
    {
        return Err("history import differs from run-lineage binding".to_owned());
    }
    if screen.run_id != binding.run_id
        || screen.solver_id != binding.solver_id
        || screen.runner_receipt_sha256 != binding.runner_receipt_sha256
        || screen.history_sha256 != binding.history.sha256
        || screen.history_import_sha256 != binding.history_import_sha256
        || screen.screen_sha256 != binding.convergence_screen_sha256
        || screen.results.len() != binding.convergence_criterion_count
        || screen.passed_count != binding.convergence_passed_count
        || screen.status != "declared_thresholds_met"
    {
        return Err("convergence screen differs from run-lineage binding".to_owned());
    }
    verify_su2_history_import(history)?;
    verify_convergence_screen(screen)?;

    for (path, bound) in [
        (
            &inputs.physics_configuration_path,
            &binding.physics_configuration,
        ),
        (&inputs.geometry_path, &binding.geometry),
        (&inputs.mesh_path, &binding.mesh),
    ] {
        let actual = staged_input(&receipt, path)?;
        if actual.relative_path != bound.relative_path
            || actual.byte_length != bound.byte_length
            || actual.sha256 != bound.sha256
        {
            return Err(format!("staged input {path} differs from binding"));
        }
    }
    if bound_output(&receipt, &history.history_relative_path)? != binding.history {
        return Err("history output differs from run-lineage binding".to_owned());
    }
    let mut expected_paths = additional_output_paths.to_vec();
    expected_paths.sort();
    expected_paths.dedup();
    if expected_paths.len() != additional_output_paths.len()
        || expected_paths
            .iter()
            .any(|path| path == &history.history_relative_path)
    {
        return Err("additional output paths are duplicated or include history".to_owned());
    }
    let mut observed_outputs = binding.additional_outputs.clone();
    observed_outputs.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if observed_outputs.len() != expected_paths.len()
        || observed_outputs
            .iter()
            .zip(&expected_paths)
            .any(|(artifact, path)| artifact.relative_path != *path)
    {
        return Err("additional outputs differ from run-lineage binding".to_owned());
    }
    for artifact in &observed_outputs {
        if bound_output(&receipt, &artifact.relative_path)? != *artifact {
            return Err(format!(
                "output {} differs from run-lineage binding",
                artifact.relative_path
            ));
        }
    }
    if sha256_hex(&canonical_binding_bytes(binding))? != binding.binding_sha256 {
        return Err("run-lineage canonical binding digest mismatch".to_owned());
    }
    Ok(())
}

pub const SU2_RUN_LINEAGE_CSV_HEADER: &str = "schema,run_id,solver_id,solver_version_claim,executable_sha256,runner_receipt_sha256,artifact_role,relative_path,byte_length,artifact_sha256,history_import_sha256,convergence_screen_sha256,convergence_criterion_count,convergence_passed_count,binding_sha256,evidence_level";

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

/// Deterministic row-per-artifact export for independent offline review.
pub fn binding_to_csv(binding: &Su2RunLineageBinding) -> String {
    let mut output = format!("{SU2_RUN_LINEAGE_CSV_HEADER}\n");
    let artifacts = [
        ("physics_configuration", &binding.physics_configuration),
        ("geometry", &binding.geometry),
        ("mesh", &binding.mesh),
        ("history", &binding.history),
    ];
    let mut rows = artifacts
        .into_iter()
        .map(|(role, artifact)| (role, artifact))
        .chain(
            binding
                .additional_outputs
                .iter()
                .map(|artifact| ("additional_output", artifact)),
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        left.0
            .cmp(right.0)
            .then_with(|| left.1.relative_path.cmp(&right.1.relative_path))
    });
    for (role, artifact) in rows {
        let fields = [
            csv_escape(&binding.schema),
            csv_escape(&binding.run_id),
            csv_escape(&binding.solver_id),
            csv_escape(&binding.solver_version_claim),
            binding.executable_sha256.clone(),
            binding.runner_receipt_sha256.clone(),
            role.to_owned(),
            csv_escape(&artifact.relative_path),
            artifact.byte_length.to_string(),
            artifact.sha256.clone(),
            binding.history_import_sha256.clone(),
            binding.convergence_screen_sha256.clone(),
            binding.convergence_criterion_count.to_string(),
            binding.convergence_passed_count.to_string(),
            binding.binding_sha256.clone(),
            csv_escape(&binding.evidence_level),
        ];
        output.push_str(&fields.join(","));
        output.push('\n');
    }
    output
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::external_solver::{ExternalSolverRequest, StagedInputSpec, execute_external_solver};
    use crate::integrity::{sha256_file, sha256_hex};
    use crate::su2_convergence_screen::{
        ConvergenceCriteria, MonitorCriterion, ResidualAcceptance, ResidualCriterion,
        screen_su2_history,
    };
    use crate::su2_history::{Su2HistoryField, Su2HistorySchema, import_su2_history};
    use std::collections::BTreeMap;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::time::Duration;

    fn temp_root() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("su2-run-lineage-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp root");
        path
    }

    fn history_fixture() -> &'static str {
        "Iteration,CL,CD,RMS_DENSITY,RMS_MOMENTUM-X\n\
         0,0.4100,0.0210,-1,-1\n\
         10,0.4200,0.0200,-2,-2\n\
         20,0.4230,0.01820,-6.5,-6.5\n\
         30,0.42305,0.01815,-6.8,-6.8\n\
         40,0.42310,0.01810,-7.1,-7.1\n\
         50,0.42310,0.01808,-7.4,-7.4\n\
         60,0.42315,0.01806,-7.7,-7.7\n\
         70,0.42316,0.01805,-8,-8\n"
    }

    fn execute_fixture(
        root: &Path,
    ) -> (
        PathBuf,
        Su2HistoryReport,
        ConvergenceScreen,
        Su2RunInputBindings,
    ) {
        fs::create_dir_all(root).expect("create fixture root");
        let config = root.join("config.cfg");
        let geometry = root.join("geometry.dat");
        let mesh = root.join("mesh.su2");
        fs::write(&config, "SOLVER= RANS\nMESH_FILENAME= mesh.su2\n").expect("write config");
        fs::write(&geometry, "synthetic geometry fixture\n").expect("write geometry");
        fs::write(&mesh, "synthetic mesh fixture\n").expect("write mesh");
        let executable = root.join("fake-su2-cfd.sh");
        let script = format!(
            "#!/bin/sh\nset -eu\nprintf '%s' '{}' > history.csv\nprintf 'force fixture\\n' > forces_breakdown.dat\nprintf 'surface fixture\\n' > surface_flow.csv\n",
            history_fixture()
        );
        fs::write(&executable, script).expect("write fake solver");
        let mut permissions = fs::metadata(&executable)
            .expect("fake solver metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("make executable");

        let staged_inputs = [
            (&config, "config.cfg"),
            (&geometry, "geometry.dat"),
            (&mesh, "mesh.su2"),
        ]
        .iter()
        .map(|(path, relative)| {
            let bytes = fs::read(path).expect("read input");
            StagedInputSpec {
                source_path: (*path).clone(),
                relative_path: (*relative).to_owned(),
                expected_byte_length: bytes.len() as u64,
                expected_sha256: sha256_hex(&bytes).expect("input digest"),
                source_reference: format!("synthetic_fixture:{relative}"),
            }
        })
        .collect::<Vec<_>>();
        let run_directory = root.join("run");
        let request = ExternalSolverRequest {
            run_id: "su2-lineage-ci-fixture".to_owned(),
            solver_id: "SU2_CFD".to_owned(),
            solver_version_claim: "synthetic-fixture-not-SU2".to_owned(),
            executable_path: executable.clone(),
            run_directory: run_directory.clone(),
            arguments: vec!["-f".to_owned(), "config.cfg".to_owned()],
            environment: BTreeMap::new(),
            staged_inputs,
            expected_outputs: vec![
                "history.csv".to_owned(),
                "forces_breakdown.dat".to_owned(),
                "surface_flow.csv".to_owned(),
            ],
            timeout: Duration::from_secs(2),
            max_log_bytes_per_stream: 16 * 1024,
        };
        let receipt = execute_external_solver(&request).expect("execute fixture");
        assert!(receipt.is_process_success());
        assert_eq!(
            receipt.executable_sha256,
            sha256_file(&executable).expect("hash executable")
        );

        let schema = Su2HistorySchema {
            history_relative_path: "history.csv".to_owned(),
            expected_header: vec![
                "Iteration".to_owned(),
                "CL".to_owned(),
                "CD".to_owned(),
                "RMS_DENSITY".to_owned(),
                "RMS_MOMENTUM-X".to_owned(),
            ],
            iteration_column: "Iteration".to_owned(),
            numeric_fields: vec![
                Su2HistoryField {
                    column: "CL".to_owned(),
                    unit: "dimensionless".to_owned(),
                },
                Su2HistoryField {
                    column: "CD".to_owned(),
                    unit: "dimensionless".to_owned(),
                },
                Su2HistoryField {
                    column: "RMS_DENSITY".to_owned(),
                    unit: "log10(residual)".to_owned(),
                },
                Su2HistoryField {
                    column: "RMS_MOMENTUM-X".to_owned(),
                    unit: "log10(residual)".to_owned(),
                },
            ],
            minimum_rows: 8,
        };
        let imported = import_su2_history(&run_directory, &schema).expect("import history");
        let criteria = ConvergenceCriteria {
            residuals: vec![
                ResidualCriterion {
                    column: "RMS_DENSITY".to_owned(),
                    unit: "log10(residual)".to_owned(),
                    terminal_window_rows: 6,
                    acceptance: ResidualAcceptance::Log10 {
                        maximum_final_log10_residual: -8.0,
                        minimum_log10_reduction: 6.0,
                        maximum_terminal_span_log10: 2.0,
                    },
                },
                ResidualCriterion {
                    column: "RMS_MOMENTUM-X".to_owned(),
                    unit: "log10(residual)".to_owned(),
                    terminal_window_rows: 6,
                    acceptance: ResidualAcceptance::Log10 {
                        maximum_final_log10_residual: -8.0,
                        minimum_log10_reduction: 6.0,
                        maximum_terminal_span_log10: 2.0,
                    },
                },
            ],
            monitors: vec![
                MonitorCriterion {
                    column: "CL".to_owned(),
                    unit: "dimensionless".to_owned(),
                    trailing_window_rows: 6,
                    maximum_absolute_range: 0.0004,
                    maximum_absolute_mean_drift: 0.0003,
                    maximum_absolute_slope_per_1000_iterations: 0.02,
                },
                MonitorCriterion {
                    column: "CD".to_owned(),
                    unit: "dimensionless".to_owned(),
                    trailing_window_rows: 6,
                    maximum_absolute_range: 0.0004,
                    maximum_absolute_mean_drift: 0.0003,
                    maximum_absolute_slope_per_1000_iterations: 0.02,
                },
            ],
        };
        let screen = screen_su2_history(&imported, &criteria).expect("screen history");
        let inputs = Su2RunInputBindings {
            physics_configuration_path: "config.cfg".to_owned(),
            geometry_path: "geometry.dat".to_owned(),
            mesh_path: "mesh.su2".to_owned(),
        };
        (run_directory, imported, screen, inputs)
    }

    #[test]
    fn binds_process_history_screen_inputs_and_force_field_outputs() {
        let root = temp_root();
        let (run, history, screen, inputs) = execute_fixture(&root);
        let outputs = vec![
            "forces_breakdown.dat".to_owned(),
            "surface_flow.csv".to_owned(),
        ];
        let binding = bind_su2_run_lineage(&run, &history, &screen, &inputs, &outputs)
            .expect("bind exact run lineage");
        assert_eq!(binding.run_id, "su2-lineage-ci-fixture");
        assert_eq!(binding.additional_outputs.len(), 2);
        verify_su2_run_lineage_binding(&run, &history, &screen, &inputs, &outputs, &binding)
            .expect("offline replay");
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn rejects_cross_run_identity_and_tampered_staged_mesh() {
        let root = temp_root();
        let (run, history, screen, inputs) = execute_fixture(&root.join("first"));
        let mut wrong_screen = screen.clone();
        wrong_screen.run_id = "other-run".to_owned();
        wrong_screen.screen_sha256 = sha256_hex(wrong_screen.screen_sha256.as_bytes())
            .expect("recompute non-matching digest");
        assert!(
            bind_su2_run_lineage(
                &run,
                &history,
                &wrong_screen,
                &inputs,
                &["forces_breakdown.dat".to_owned()]
            )
            .is_err()
        );

        let (run2, history2, screen2, inputs2) = execute_fixture(&root.join("second"));
        fs::write(run2.join("mesh.su2"), "tampered mesh\n").expect("tamper mesh");
        assert!(
            bind_su2_run_lineage(
                &run2,
                &history2,
                &screen2,
                &inputs2,
                &["forces_breakdown.dat".to_owned()]
            )
            .is_err()
        );
        fs::remove_dir_all(root).expect("remove fixture root");
    }
}
