//! Byte-bound artifact manifests for civil CFD run evidence.
//!
//! This module verifies actual bytes under a caller-selected root. It proves that the files
//! match the manifest digests; it does not prove who authored the manifest or that the solver
//! was run as claimed. Manifest signatures and execution-environment attestation are future work.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::cfd_admission::CfdEvidenceStudy;
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
    use crate::cfd_admission::{AdmissionCriteria, CfdEvidenceStudy, CfdGridRun};
    use crate::integrity::sha256_hex;
    use std::fs;

    fn fixture_study() -> CfdEvidenceStudy {
        CfdEvidenceStudy {
            case_id: "artifact-fixture-study".to_owned(),
            quantity: "CD".to_owned(),
            source_reference: "synthetic-artifact-known-answer".to_owned(),
            spacing_definition: "explicit test spacings".to_owned(),
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
