//! Byte-bound artifact manifests for civil CFD run evidence.
//!
//! This module verifies actual bytes under a caller-selected root. It proves that the files
//! match the manifest digests; it does not prove who authored the manifest or that the solver
//! was run as claimed. Manifest signatures and execution-environment attestation are future work.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::cfd_admission::{
    CfdEvidenceAdmissionReport, CfdEvidenceStudy, evaluate_cfd_evidence_admission,
};
use crate::integrity::verify_source_file;

pub const ARTIFACT_MANIFEST_SCHEMA: &str = "civil-cfd-run-artifacts/0.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CfdArtifactRole {
    SolverBuild,
    PhysicsConfiguration,
    Geometry,
    Mesh,
    RunLog,
    ResidualHistory,
    ForceHistory,
    FieldOutput,
}

impl CfdArtifactRole {
    pub const REQUIRED: [Self; 8] = [
        Self::SolverBuild,
        Self::PhysicsConfiguration,
        Self::Geometry,
        Self::Mesh,
        Self::RunLog,
        Self::ResidualHistory,
        Self::ForceHistory,
        Self::FieldOutput,
    ];

    pub fn parse_role(value: &str) -> Option<Self> {
        match value {
            "solver_build" => Some(Self::SolverBuild),
            "physics_configuration" => Some(Self::PhysicsConfiguration),
            "geometry" => Some(Self::Geometry),
            "mesh" => Some(Self::Mesh),
            "run_log" => Some(Self::RunLog),
            "residual_history" => Some(Self::ResidualHistory),
            "force_history" => Some(Self::ForceHistory),
            "field_output" => Some(Self::FieldOutput),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SolverBuild => "solver_build",
            Self::PhysicsConfiguration => "physics_configuration",
            Self::Geometry => "geometry",
            Self::Mesh => "mesh",
            Self::RunLog => "run_log",
            Self::ResidualHistory => "residual_history",
            Self::ForceHistory => "force_history",
            Self::FieldOutput => "field_output",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfdArtifactRecord {
    pub role: CfdArtifactRole,
    /// Path relative to the artifact root; absolute paths and traversal are rejected.
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfdArtifactManifest {
    pub schema: String,
    pub run_id: String,
    pub artifacts: Vec<CfdArtifactRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCfdArtifact {
    pub role: CfdArtifactRole,
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedCfdArtifactManifest {
    pub run_id: String,
    pub canonical_manifest_sha256: String,
    pub artifacts: Vec<VerifiedCfdArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfdRunArtifactAudit {
    pub run_id: String,
    pub canonical_manifest_sha256: String,
    pub artifact_count: usize,
    pub solver_build_sha256: String,
    pub physics_config_sha256: String,
    pub geometry_sha256: String,
    pub mesh_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfdStudyArtifactAudit {
    pub study_id: String,
    pub status: &'static str,
    pub run_audits: Vec<CfdRunArtifactAudit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CfdEvidenceChainReceipt {
    pub case_id: String,
    pub quantity: String,
    pub run_count: usize,
    pub artifact_bundle_sha256: String,
    pub numerical_admission_sha256: String,
    pub receipt_sha256: String,
    pub evidence_level: &'static str,
}

pub const CFD_EVIDENCE_CHAIN_RECEIPT_CSV_HEADER: &str =
    "case_id,quantity,run_count,artifact_bundle_sha256,numerical_admission_sha256,receipt_sha256,evidence_level";

pub fn receipt_to_csv(receipt: &CfdEvidenceChainReceipt) -> String {
    format!(
        "{CFD_EVIDENCE_CHAIN_RECEIPT_CSV_HEADER}\n{},{},{},{},{},{},{}\n",
        csv_escape(&receipt.case_id),
        csv_escape(&receipt.quantity),
        receipt.run_count,
        receipt.artifact_bundle_sha256,
        receipt.numerical_admission_sha256,
        receipt.receipt_sha256,
        receipt.evidence_level
    )
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn verify_artifact_manifest(
    artifact_root: &Path,
    manifest: &CfdArtifactManifest,
) -> Result<VerifiedCfdArtifactManifest, String> {
    if manifest.schema != ARTIFACT_MANIFEST_SCHEMA {
        return Err(format!(
            "unsupported artifact manifest schema: {}",
            manifest.schema
        ));
    }
    if manifest.run_id.trim().is_empty()
        || manifest.run_id.contains('\n')
        || manifest.run_id.contains('\r')
        || manifest.run_id.contains('\t')
        || manifest.run_id.contains('|')
    {
        return Err("artifact manifest run_id must be non-empty and single-line".to_owned());
    }

    let root = fs::canonicalize(artifact_root)
        .map_err(|error| format!("cannot resolve artifact root: {error}"))?;
    if !root.is_dir() {
        return Err("artifact root must be a directory".to_owned());
    }
    if manifest.artifacts.len() != CfdArtifactRole::REQUIRED.len() {
        return Err(format!(
            "artifact manifest must contain exactly {} required roles",
            CfdArtifactRole::REQUIRED.len()
        ));
    }

    let mut roles = HashSet::new();
    let mut paths = HashSet::new();
    let mut verified = Vec::with_capacity(manifest.artifacts.len());
    for record in &manifest.artifacts {
        if !roles.insert(record.role) {
            return Err(format!("duplicate artifact role {}", record.role.as_str()));
        }
        let relative_path = validate_relative_path(&record.relative_path)?;
        if !paths.insert(relative_path.clone()) {
            return Err(format!("duplicate artifact path {}", record.relative_path));
        }

        let joined = root.join(&relative_path);
        let canonical_path = fs::canonicalize(&joined).map_err(|error| {
            format!(
                "cannot resolve artifact {} for role {}: {error}",
                record.relative_path,
                record.role.as_str()
            )
        })?;
        if !canonical_path.starts_with(&root) {
            return Err(format!(
                "artifact {} for role {} resolves outside the artifact root",
                record.relative_path,
                record.role.as_str()
            ));
        }
        verify_source_file(
            &canonical_path,
            record.role.as_str(),
            record.byte_length,
            &record.sha256,
        )?;
        verified.push(VerifiedCfdArtifact {
            role: record.role,
            relative_path: record.relative_path.clone(),
            byte_length: record.byte_length,
            sha256: record.sha256.to_ascii_lowercase(),
        });
    }

    for required_role in CfdArtifactRole::REQUIRED {
        if !roles.contains(&required_role) {
            return Err(format!(
                "artifact manifest is missing required role {}",
                required_role.as_str()
            ));
        }
    }
    verified.sort_by_key(|artifact| artifact.role);

    let manifest_bytes = canonical_manifest_bytes(manifest, &verified);
    let canonical_manifest_sha256 = crate::integrity::sha256_hex(&manifest_bytes)?;
    Ok(VerifiedCfdArtifactManifest {
        run_id: manifest.run_id.clone(),
        canonical_manifest_sha256,
        artifacts: verified,
    })
}

/// Re-verify every artifact byte, recompute numerical admission from the study inputs,
/// and bind both outputs into one deterministic receipt digest. This is an integrity binding,
/// not a signature, trusted-runner attestation, source-authorship proof or experimental validation.
pub fn create_cfd_evidence_chain_receipt(
    study: &CfdEvidenceStudy,
    artifact_root: &Path,
    manifests: &[CfdArtifactManifest],
) -> Result<CfdEvidenceChainReceipt, String> {
    let artifact_audit = verify_cfd_study_artifacts(study, artifact_root, manifests)?;
    let admission = evaluate_cfd_evidence_admission(study)?;
    if artifact_audit.study_id != admission.case_id || artifact_audit.study_id != study.case_id {
        return Err("artifact and numerical-admission case identities differ".to_owned());
    }
    if artifact_audit.run_audits.len() != admission.grid_run_audits.len()
        || artifact_audit.run_audits.len() != study.runs.len()
    {
        return Err("artifact and numerical-admission run counts differ".to_owned());
    }
    if artifact_audit.status
        != "artifact_file_bytes_sha256_verified_canonical_manifest_unsigned_not_source_authenticated_not_experimentally_validated"
        || admission.status
            != "numerically_admitted_under_declared_criteria_caller_hashes_unverified_not_experimentally_validated"
    {
        return Err("unexpected evidence status in composite receipt inputs".to_owned());
    }

    let mut artifact_bundle = Vec::new();
    append_string(&mut artifact_bundle, "civil-cfd-artifact-bundle/0.1.0");
    append_string(&mut artifact_bundle, &artifact_audit.study_id);
    for ((artifact, numeric), run) in artifact_audit
        .run_audits
        .iter()
        .zip(&admission.grid_run_audits)
        .zip(&study.runs)
    {
        if artifact.run_id != numeric.run_id || artifact.run_id != run.run_id {
            return Err(format!(
                "run identity mismatch while composing receipt for {}",
                run.run_id
            ));
        }
        if artifact.artifact_count != CfdArtifactRole::REQUIRED.len() {
            return Err(format!(
                "run {} has incomplete verified artifact roles",
                run.run_id
            ));
        }
        if !is_sha256_hex(&artifact.canonical_manifest_sha256)
            || !is_sha256_hex(&artifact.solver_build_sha256)
            || !is_sha256_hex(&artifact.physics_config_sha256)
            || !is_sha256_hex(&artifact.geometry_sha256)
            || !is_sha256_hex(&artifact.mesh_sha256)
        {
            return Err(format!(
                "run {} has malformed artifact digests in composite receipt",
                run.run_id
            ));
        }
        if artifact.solver_build_sha256 != admission.solver_build_sha256
            || artifact.physics_config_sha256 != admission.physics_config_sha256
            || artifact.geometry_sha256 != admission.geometry_sha256
            || artifact.mesh_sha256 != numeric.mesh_sha256
            || artifact.mesh_sha256 != run.mesh_sha256.to_ascii_lowercase()
        {
            return Err(format!(
                "run {} artifact hashes disagree with the admitted numerical record",
                run.run_id
            ));
        }
        append_string(&mut artifact_bundle, &artifact.run_id);
        append_string(&mut artifact_bundle, &artifact.canonical_manifest_sha256);
        append_u64(&mut artifact_bundle, artifact.artifact_count as u64);
        append_string(&mut artifact_bundle, &artifact.solver_build_sha256);
        append_string(&mut artifact_bundle, &artifact.physics_config_sha256);
        append_string(&mut artifact_bundle, &artifact.geometry_sha256);
        append_string(&mut artifact_bundle, &artifact.mesh_sha256);
    }
    let artifact_bundle_sha256 = crate::integrity::sha256_hex(&artifact_bundle)?;

    let mut numerical_bytes = Vec::new();
    append_admission_report(&mut numerical_bytes, &admission);
    let numerical_admission_sha256 = crate::integrity::sha256_hex(&numerical_bytes)?;

    let mut receipt_bytes = Vec::new();
    append_string(&mut receipt_bytes, "civil-cfd-evidence-chain-receipt/0.1.0");
    append_string(&mut receipt_bytes, &admission.case_id);
    append_string(&mut receipt_bytes, &admission.quantity);
    append_string(&mut receipt_bytes, &artifact_bundle_sha256);
    append_string(&mut receipt_bytes, &numerical_admission_sha256);
    let receipt_sha256 = crate::integrity::sha256_hex(&receipt_bytes)?;

    Ok(CfdEvidenceChainReceipt {
        case_id: admission.case_id,
        quantity: admission.quantity,
        run_count: admission.grid_run_audits.len(),
        artifact_bundle_sha256,
        numerical_admission_sha256,
        receipt_sha256,
        evidence_level: "byte_verified_artifacts_bound_to_recomputed_numerical_admission_unsigned_not_source_authenticated_not_experimentally_validated",
    })
}

fn append_string(target: &mut Vec<u8>, value: &str) {
    append_u64(target, value.len() as u64);
    target.extend_from_slice(value.as_bytes());
}

fn append_u64(target: &mut Vec<u8>, value: u64) {
    target.extend_from_slice(&value.to_le_bytes());
}

fn append_i32(target: &mut Vec<u8>, value: i32) {
    target.extend_from_slice(&value.to_le_bytes());
}

fn append_f64(target: &mut Vec<u8>, value: f64) {
    target.extend_from_slice(&value.to_bits().to_le_bytes());
}

fn append_optional_f64(target: &mut Vec<u8>, value: Option<f64>) {
    match value {
        Some(number) => {
            target.push(1);
            append_f64(target, number);
        }
        None => target.push(0),
    }
}

fn append_admission_report(target: &mut Vec<u8>, report: &CfdEvidenceAdmissionReport) {
    append_string(target, "civil-cfd-numerical-admission/0.1.0");
    for value in [
        report.case_id.as_str(),
        report.quantity.as_str(),
        report.source_reference.as_str(),
        report.spacing_definition.as_str(),
        report.solver_build_sha256.as_str(),
        report.physics_config_sha256.as_str(),
        report.geometry_sha256.as_str(),
        report.grid_family_kind.as_str(),
        report.status,
        report.gci_report.evidence_level,
        report.gci_report.quantity.as_str(),
        report.gci_report.source_reference.as_str(),
        report.gci_report.spacing_definition.as_str(),
        report.gci_report.grid_family_kind.as_str(),
    ] {
        append_string(target, value);
    }
    append_f64(target, report.safety_factor);
    append_f64(target, report.gci_report.safety_factor);
    for value in [
        report.criteria.residual_final_max,
        report.criteria.residual_reduction_ratio_max,
        report.criteria.quantity_history_drift_absolute_max,
        report.criteria.mass_imbalance_fraction_max,
        report.criteria.energy_imbalance_fraction_max,
        report.criteria.observed_order_min,
        report.criteria.observed_order_max,
        report.criteria.asymptotic_ratio_min,
        report.criteria.asymptotic_ratio_max,
    ] {
        append_f64(target, value);
    }
    append_u64(target, report.grid_run_audits.len() as u64);
    for run in &report.grid_run_audits {
        append_string(target, &run.run_id);
        append_string(target, &run.grid_id);
        append_u64(target, u64::from(run.grid_level));
        append_f64(target, run.characteristic_spacing);
        append_string(target, &run.mesh_sha256);
        append_u64(target, run.mesh_node_count);
        append_i32(target, run.solver_exit_code);
        append_u64(target, run.iterations_completed);
        append_f64(target, run.quantity_value);
        append_f64(target, run.residual_final);
        append_f64(target, run.residual_reduction_ratio);
        append_f64(target, run.quantity_history_drift_absolute);
        append_f64(target, run.mass_imbalance_fraction);
        append_f64(target, run.energy_imbalance_fraction);
        append_f64(target, run.cpu_core_hours);
        append_optional_f64(target, run.gpu_hours);
        append_f64(target, run.wall_clock_seconds);
        append_u64(target, run.peak_memory_bytes);
        append_optional_f64(target, run.energy_consumed_joules);
    }
    append_f64(target, report.total_cpu_core_hours);
    append_optional_f64(target, report.total_gpu_hours_measured);
    append_u64(target, report.gpu_measured_run_count as u64);
    append_f64(target, report.total_wall_clock_seconds);
    append_u64(target, report.peak_memory_bytes);
    append_optional_f64(target, report.total_energy_joules_measured);
    append_u64(target, report.energy_measured_run_count as u64);
    append_u64(target, report.gci_report.triplets.len() as u64);
    for triplet in &report.gci_report.triplets {
        append_string(target, &triplet.coarse_grid_id);
        append_string(target, &triplet.medium_grid_id);
        append_string(target, &triplet.fine_grid_id);
        for value in [
            triplet.h_coarse,
            triplet.h_medium,
            triplet.h_fine,
            triplet.phi_coarse,
            triplet.phi_medium,
            triplet.phi_fine,
            triplet.refinement_ratio_coarse_medium,
            triplet.refinement_ratio_medium_fine,
            triplet.observed_order,
            triplet.extrapolated_value,
            triplet.medium_gci_absolute,
        ] {
            append_f64(target, value);
        }
        append_optional_f64(target, triplet.medium_gci_relative_percent);
        append_f64(target, triplet.fine_gci_absolute);
        append_optional_f64(target, triplet.fine_gci_relative_percent);
        append_f64(target, triplet.asymptotic_ratio);
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Verify one complete artifact manifest for every grid run in a CFD study, then bind the
/// four principal artifact hashes back to the numerical run summary.
pub fn verify_cfd_study_artifacts(
    study: &CfdEvidenceStudy,
    artifact_root: &Path,
    manifests: &[CfdArtifactManifest],
) -> Result<CfdStudyArtifactAudit, String> {
    if manifests.len() != study.runs.len() {
        return Err(format!(
            "artifact manifest count {} does not match study run count {}",
            manifests.len(),
            study.runs.len()
        ));
    }

    let mut manifests_by_run = std::collections::BTreeMap::new();
    for manifest in manifests {
        if manifests_by_run
            .insert(manifest.run_id.as_str(), manifest)
            .is_some()
        {
            return Err(format!(
                "duplicate artifact manifest for run {}",
                manifest.run_id
            ));
        }
    }

    let mut run_audits = Vec::with_capacity(study.runs.len());
    for run in &study.runs {
        let manifest = manifests_by_run
            .get(run.run_id.as_str())
            .ok_or_else(|| format!("missing artifact manifest for run {}", run.run_id))?;
        let verified = verify_artifact_manifest(artifact_root, manifest)?;
        for (role, expected_sha256) in [
            (CfdArtifactRole::SolverBuild, &run.solver_build_sha256),
            (
                CfdArtifactRole::PhysicsConfiguration,
                &run.physics_config_sha256,
            ),
            (CfdArtifactRole::Geometry, &run.geometry_sha256),
            (CfdArtifactRole::Mesh, &run.mesh_sha256),
        ] {
            let actual = verified
                .artifacts
                .iter()
                .find(|artifact| artifact.role == role)
                .ok_or_else(|| format!("verified manifest lacks role {}", role.as_str()))?;
            if !actual.sha256.eq_ignore_ascii_case(expected_sha256) {
                return Err(format!(
                    "run {} {} artifact digest differs from numerical run record",
                    run.run_id,
                    role.as_str()
                ));
            }
        }
        run_audits.push(CfdRunArtifactAudit {
            run_id: run.run_id.clone(),
            canonical_manifest_sha256: verified.canonical_manifest_sha256,
            artifact_count: verified.artifacts.len(),
            solver_build_sha256: run.solver_build_sha256.to_ascii_lowercase(),
            physics_config_sha256: run.physics_config_sha256.to_ascii_lowercase(),
            geometry_sha256: run.geometry_sha256.to_ascii_lowercase(),
            mesh_sha256: run.mesh_sha256.to_ascii_lowercase(),
        });
    }

    Ok(CfdStudyArtifactAudit {
        study_id: study.case_id.clone(),
        status: "artifact_file_bytes_sha256_verified_canonical_manifest_unsigned_not_source_authenticated_not_experimentally_validated",
        run_audits,
    })
}

fn validate_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty()
        || value.contains('|')
        || value.contains(',')
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('\t')
        || value.contains('\\')
    {
        return Err(
            "artifact path must be non-empty and contain no control or separator characters"
                .to_owned(),
        );
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "artifact path must be a safe root-relative path: {value}"
        ));
    }
    Ok(path.to_path_buf())
}

fn canonical_manifest_bytes(
    manifest: &CfdArtifactManifest,
    artifacts: &[VerifiedCfdArtifact],
) -> Vec<u8> {
    let mut text = format!("schema={}\nrun_id={}\n", manifest.schema, manifest.run_id);
    for artifact in artifacts {
        text.push_str(&format!(
            "{}|{}|{}|{}\n",
            artifact.role.as_str(),
            artifact.relative_path,
            artifact.byte_length,
            artifact.sha256
        ));
    }
    text.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfd_admission::{AdmissionCriteria, CfdEvidenceStudy, CfdGridRun, GridFamilyKind};
    use crate::integrity::sha256_hex;
    use std::fs;

    fn fixture_study() -> CfdEvidenceStudy {
        CfdEvidenceStudy {
            case_id: "artifact-fixture-study".to_owned(),
            quantity: "CD".to_owned(),
            source_reference: "synthetic-artifact-known-answer".to_owned(),
            spacing_definition: "explicit test spacings".to_owned(),
            grid_family_kind: GridFamilyKind::Structured,
            safety_factor: 1.25,
            criteria: AdmissionCriteria {
                residual_final_max: 1.0e-8,
                residual_reduction_ratio_max: 1.0e-7,
                quantity_history_drift_absolute_max: 1.0e-9,
                mass_imbalance_fraction_max: 1.0e-6,
                energy_imbalance_fraction_max: 1.0e-6,
                observed_order_min: 1.0,
                observed_order_max: 3.0,
                asymptotic_ratio_min: 0.9,
                asymptotic_ratio_max: 1.1,
            },
            runs: (1..=3)
                .map(|index| CfdGridRun {
                    run_id: format!("run-{index}"),
                    grid_id: format!("grid-{index}"),
                    grid_level: index,
                    characteristic_spacing: 1.0 / f64::from(index),
                    mesh_sha256: "0".repeat(64),
                    solver_build_sha256: "0".repeat(64),
                    physics_config_sha256: "0".repeat(64),
                    geometry_sha256: "0".repeat(64),
                    solver_exit_code: 0,
                    iterations_completed: 100,
                    mesh_node_count: 1000 * u64::from(index),
                    cpu_core_hours: 0.1,
                    gpu_hours: None,
                    wall_clock_seconds: 10.0,
                    peak_memory_bytes: 1024,
                    energy_consumed_joules: None,
                    residual_initial: 1.0,
                    residual_final: 1.0e-9,
                    quantity_history: vec![1.0; 6],
                    mass_imbalance_fraction: 1.0e-9,
                    energy_imbalance_fraction: 1.0e-9,
                    quantity_value: 1.0 + f64::from(index).powi(2) * 0.1,
                })
                .collect(),
        }
    }

    fn fixture_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "civil-cfd-artifact-manifest-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        root
    }

    fn build_manifests(root: &Path, study: &mut CfdEvidenceStudy) -> Vec<CfdArtifactManifest> {
        let shared = [
            (
                CfdArtifactRole::SolverBuild,
                b"synthetic solver build bytes".to_vec(),
            ),
            (
                CfdArtifactRole::PhysicsConfiguration,
                b"synthetic physics config bytes".to_vec(),
            ),
            (
                CfdArtifactRole::Geometry,
                b"synthetic geometry bytes".to_vec(),
            ),
        ];
        let mut manifests = Vec::new();
        for run in &mut study.runs {
            let run_dir = run.run_id.clone();
            fs::create_dir_all(root.join(&run_dir)).expect("create per-run directory");
            let mut records = Vec::new();
            for role in CfdArtifactRole::REQUIRED {
                let bytes = match role {
                    CfdArtifactRole::SolverBuild
                    | CfdArtifactRole::PhysicsConfiguration
                    | CfdArtifactRole::Geometry => shared
                        .iter()
                        .find(|(shared_role, _)| *shared_role == role)
                        .expect("shared artifact role")
                        .1
                        .clone(),
                    CfdArtifactRole::Mesh => format!("mesh bytes for {}", run.run_id).into_bytes(),
                    CfdArtifactRole::RunLog => format!("run log for {}", run.run_id).into_bytes(),
                    CfdArtifactRole::ResidualHistory => {
                        format!("residual history for {}", run.run_id).into_bytes()
                    }
                    CfdArtifactRole::ForceHistory => {
                        format!("force history for {}", run.run_id).into_bytes()
                    }
                    CfdArtifactRole::FieldOutput => {
                        format!("field output for {}", run.run_id).into_bytes()
                    }
                };
                let relative_path = format!("{run_dir}/{}.dat", role.as_str());
                fs::write(root.join(&relative_path), &bytes).expect("write artifact fixture");
                let record = CfdArtifactRecord {
                    role,
                    relative_path,
                    byte_length: u64::try_from(bytes.len()).expect("fixture size fits u64"),
                    sha256: sha256_hex(&bytes).expect("fixture digest"),
                };
                match role {
                    CfdArtifactRole::SolverBuild => run.solver_build_sha256 = record.sha256.clone(),
                    CfdArtifactRole::PhysicsConfiguration => {
                        run.physics_config_sha256 = record.sha256.clone()
                    }
                    CfdArtifactRole::Geometry => run.geometry_sha256 = record.sha256.clone(),
                    CfdArtifactRole::Mesh => run.mesh_sha256 = record.sha256.clone(),
                    _ => {}
                }
                records.push(record);
            }
            manifests.push(CfdArtifactManifest {
                schema: ARTIFACT_MANIFEST_SCHEMA.to_owned(),
                run_id: run.run_id.clone(),
                artifacts: records,
            });
        }
        manifests
    }

    #[test]
    fn creates_deterministic_receipt_binding_artifacts_to_numerical_admission() {
        let root = fixture_root("composite-receipt");
        let mut study = fixture_study();
        for run in &mut study.runs {
            run.quantity_value = 2.0 + 0.8 * run.characteristic_spacing.powi(2);
        }
        let manifests = build_manifests(&root, &mut study);
        let first = create_cfd_evidence_chain_receipt(&study, &root, &manifests)
            .expect("bytes verify and numerical admission recomputes");
        let second = create_cfd_evidence_chain_receipt(&study, &root, &manifests)
            .expect("same evidence produces same receipt");
        assert_eq!(first, second);
        assert_eq!(first.run_count, 3);
        assert!(is_sha256_hex(&first.artifact_bundle_sha256));
        assert!(is_sha256_hex(&first.numerical_admission_sha256));
        assert!(is_sha256_hex(&first.receipt_sha256));
        assert!(first.evidence_level.contains("unsigned_not_source_authenticated"));
        assert!(first.evidence_level.contains("not_experimentally_validated"));
        let first_csv = receipt_to_csv(&first);
        assert_eq!(first_csv, receipt_to_csv(&second));
        assert_eq!(first_csv.lines().count(), 2);
        assert!(first_csv.contains(&first.receipt_sha256));
        fs::remove_dir_all(&root).expect("remove composite receipt fixture");
    }

    #[test]
    fn composite_receipt_changes_when_admitted_numerical_evidence_changes() {
        let root = fixture_root("composite-numeric-mutation");
        let mut study = fixture_study();
        for run in &mut study.runs {
            run.quantity_value = 2.0 + 0.8 * run.characteristic_spacing.powi(2);
        }
        let manifests = build_manifests(&root, &mut study);
        let original = create_cfd_evidence_chain_receipt(&study, &root, &manifests)
            .expect("original evidence admitted");
        let mut changed_study = study.clone();
        changed_study.runs[0].quantity_value += 0.01;
        let changed = create_cfd_evidence_chain_receipt(&changed_study, &root, &manifests)
            .expect("small monotone numeric change remains admitted");
        assert_ne!(original.numerical_admission_sha256, changed.numerical_admission_sha256);
        assert_ne!(original.receipt_sha256, changed.receipt_sha256);
        fs::remove_dir_all(&root).expect("remove numeric mutation fixture");
    }

    #[test]
    fn composite_receipt_rejects_artifact_or_numerical_identity_mismatch() {
        let root = fixture_root("composite-mismatch");
        let mut study = fixture_study();
        for run in &mut study.runs {
            run.quantity_value = 2.0 + 0.8 * run.characteristic_spacing.powi(2);
        }
        let manifests = build_manifests(&root, &mut study);
        let mut changed_manifests = manifests.clone();
        changed_manifests[0].run_id = "wrong-run-id".to_owned();
        assert!(
            create_cfd_evidence_chain_receipt(&study, &root, &changed_manifests)
                .expect_err("manifest/run identity mismatch rejected")
                .contains("missing artifact manifest")
        );
        fs::remove_dir_all(&root).expect("remove composite mismatch fixture");
    }

    #[test]
    fn verifies_each_run_artifact_bundle_and_binds_principal_digests() {
        let root = fixture_root("positive");
        let mut study = fixture_study();
        let manifests = build_manifests(&root, &mut study);
        let report =
            verify_cfd_study_artifacts(&study, &root, &manifests).expect("all actual bytes match");
        assert_eq!(report.run_audits.len(), 3);
        assert_eq!(
            report.status,
            "artifact_file_bytes_sha256_verified_canonical_manifest_unsigned_not_source_authenticated_not_experimentally_validated"
        );
        assert!(report.run_audits.iter().all(|run| run.artifact_count == 8));
        assert_ne!(
            report.run_audits[0].canonical_manifest_sha256,
            report.run_audits[1].canonical_manifest_sha256
        );
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn same_length_byte_mutation_fails_actual_file_hash_check() {
        let root = fixture_root("tamper");
        let mut study = fixture_study();
        let manifests = build_manifests(&root, &mut study);
        let record = manifests[0]
            .artifacts
            .iter()
            .find(|record| record.role == CfdArtifactRole::FieldOutput)
            .expect("field artifact");
        let file = root.join(&record.relative_path);
        let mut bytes = fs::read(&file).expect("read field file");
        bytes[0] ^= 1;
        fs::write(&file, &bytes).expect("mutate field file without changing length");
        let error = verify_cfd_study_artifacts(&study, &root, &manifests)
            .expect_err("one-byte mutation must be rejected");
        assert!(error.contains("SHA-256 mismatch"));
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[test]
    fn rejects_manifest_role_loss_path_traversal_and_run_mismatch() {
        let root = fixture_root("negative");
        let mut study = fixture_study();
        let manifests = build_manifests(&root, &mut study);

        let mut missing = manifests.clone();
        missing[0].artifacts.pop();
        assert!(
            verify_cfd_study_artifacts(&study, &root, &missing)
                .expect_err("missing artifact role rejected")
                .contains("exactly 8 required roles")
        );

        let mut traversal = manifests.clone();
        traversal[0].artifacts[0].relative_path = "../escape.bin".to_owned();
        assert!(
            verify_cfd_study_artifacts(&study, &root, &traversal)
                .expect_err("path traversal rejected")
                .contains("safe root-relative")
        );

        let mut wrong_run = manifests.clone();
        wrong_run[0].run_id = "unmatched-run".to_owned();
        assert!(
            verify_cfd_study_artifacts(&study, &root, &wrong_run)
                .expect_err("manifest/run ID mismatch rejected")
                .contains("missing artifact manifest")
        );
        fs::remove_dir_all(root).expect("remove fixture root");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_that_resolves_outside_artifact_root() {
        use std::os::unix::fs::symlink;

        let root = fixture_root("symlink-escape");
        let outside = fixture_root("symlink-outside");
        let mut study = fixture_study();
        let mut manifests = build_manifests(&root, &mut study);
        let outside_file = outside.join("outside-field-output.dat");
        fs::write(&outside_file, b"outside root bytes").expect("write outside artifact");
        let link_path = root.join("run-1/escape-link.dat");
        symlink(&outside_file, &link_path).expect("create outside symlink");
        let record = manifests[0]
            .artifacts
            .iter_mut()
            .find(|record| record.role == CfdArtifactRole::FieldOutput)
            .expect("field artifact");
        record.relative_path = "run-1/escape-link.dat".to_owned();
        record.byte_length = u64::try_from(fs::metadata(&outside_file).expect("metadata").len())
            .expect("file length fits u64");
        record.sha256 = sha256_hex(&fs::read(&outside_file).expect("read outside artifact"))
            .expect("outside digest");
        let error = verify_cfd_study_artifacts(&study, &root, &manifests)
            .expect_err("outside symlink must be rejected");
        assert!(error.contains("resolves outside the artifact root"));
        fs::remove_dir_all(root).expect("remove fixture root");
        fs::remove_dir_all(outside).expect("remove outside fixture root");
    }

    #[test]
    fn rejects_manifest_hash_that_does_not_match_run_mesh_identity() {
        let root = fixture_root("mesh-mismatch");
        let mut study = fixture_study();
        let mut manifests = build_manifests(&root, &mut study);
        let record = manifests[0]
            .artifacts
            .iter_mut()
            .find(|record| record.role == CfdArtifactRole::Mesh)
            .expect("mesh artifact");
        let replacement = b"replacement mesh bytes".to_vec();
        let path = root.join(&record.relative_path);
        fs::write(&path, &replacement).expect("replace mesh bytes");
        record.byte_length = u64::try_from(replacement.len()).expect("fixture size fits u64");
        record.sha256 = sha256_hex(&replacement).expect("replacement digest");
        let error = verify_cfd_study_artifacts(&study, &root, &manifests)
            .expect_err("mesh digest differing from numerical record rejected");
        assert!(error.contains("mesh artifact digest differs"));
        fs::remove_dir_all(root).expect("remove fixture root");
    }
}
