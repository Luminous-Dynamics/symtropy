//! Offline, compile-only implementation of Engineering Scenario DSL v0.1.
//!
//! A successful type-check is not execution admission, solver validation, or
//! evidence that a numerical model describes the physical system.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const COMPILER_ID: &str = "engineering-scenario-compiler";
pub const COMPILER_VERSION: &str = "0.1.0";
pub const SCHEMA_VERSION: &str = "engineering-scenario-v0.1";
pub const REGISTRY_REVISION: &str = "naca0012-inc-rans-profile-registry-v0.1";
pub const MAX_SOURCE_BYTES: usize = 1_048_576;
pub const MAX_INSTANCES: usize = 128;
// Allows for the published decimal precision of the pinned NACA0012 fixture.
const REYNOLDS_RELATIVE_TOLERANCE: f64 = 2.0e-6;
const TOML_PARSER_PROFILE: &str = "toml-1.0/toml_edit-0.23.10+spec-1.0.0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition { Invalid, Blocked, Unsupported, PlanReady }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Resolution { SupportedExact, SupportedApproximation, Blocked, Unsupported }

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompileObligation { pub id: String, pub resolution: Resolution, pub detail: String }

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TypedQuantity { pub value: f64, pub dimension: String, pub unit: String }

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GeometryIr {
    pub name: String,
    pub chord: TypedQuantity,
    pub upstream_repository: String,
    pub upstream_ref_annotation: String,
    pub upstream_config_path: String,
    pub upstream_config_git_blob_oid: String,
    pub upstream_mesh_path: String,
    pub upstream_mesh_git_blob_oid: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlowIr {
    pub model_profile: String,
    pub turbulence_model: String,
    pub reynolds_number: TypedQuantity,
    pub density: TypedQuantity,
    pub dynamic_viscosity: TypedQuantity,
    pub mesh_normal_nodes: u32,
    pub mesh_tangential_nodes: u32,
    pub farfield_chords: TypedQuantity,
    pub wall_y_plus_target: TypedQuantity,
    pub wall_condition: String,
    pub farfield_condition: String,
    pub convective_scheme: String,
    pub time_integration: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InstanceIr {
    pub id: String,
    pub angle_of_attack: TypedQuantity,
    pub freestream_velocity: [TypedQuantity; 3],
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ObservableIr {
    pub pressure_coefficient: String,
    pub skin_friction_coefficient: String,
    pub lift_drag: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LimitationsIr {
    pub executable_digest: String,
    pub derived_config_digests: String,
    pub reference_data_digests: String,
    pub grid_convergence: String,
    pub result_status: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ScenarioIr {
    pub schema_version: String,
    pub case_id: String,
    pub case_revision: u32,
    pub source_status: String,
    pub parent_issue: String,
    pub runner_issue: String,
    pub geometry: GeometryIr,
    pub flow: FlowIr,
    pub instances: Vec<InstanceIr>,
    pub observables: ObservableIr,
    pub limitations: LimitationsIr,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CompilationReport {
    pub compiler_id: String,
    pub compiler_version: String,
    pub registry_revision: String,
    pub source_sha256: String,
    pub semantic_ir_sha256: Option<String>,
    pub disposition: Disposition,
    pub obligations: Vec<CompileObligation>,
    pub diagnostics: Vec<String>,
    pub scenario_ir: Option<ScenarioIr>,
    pub claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceScenario {
    case_id: String,
    case_revision: u32,
    status: String,
    parent_issue: String,
    runner_issue: String,
    geometry: SourceGeometry,
    flow: SourceFlow,
    instances: Vec<SourceInstance>,
    observables: SourceObservables,
    limitations: SourceLimitations,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceGeometry {
    name: String,
    chord_m: f64,
    upstream_repository: String,
    upstream_ref: String,
    upstream_config_path: String,
    upstream_config_git_blob_sha: String,
    upstream_mesh_path: String,
    upstream_mesh_git_blob_sha: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFlow {
    model: String,
    turbulence_model: String,
    reynolds_number_per_chord: f64,
    density_kg_m3: f64,
    dynamic_viscosity_kg_m_s: f64,
    mesh_normal_nodes: u32,
    mesh_tangential_nodes: u32,
    farfield_chords_approx: f64,
    wall_y_plus_target: f64,
    wall_condition: String,
    farfield_condition: String,
    convective_scheme: String,
    time_integration: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceInstance {
    id: String,
    angle_of_attack_deg: f64,
    freestream_velocity_m_s: [f64; 3],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceObservables {
    pressure_coefficient: String,
    skin_friction_coefficient: String,
    lift_drag: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceLimitations {
    executable_digest: String,
    derived_config_digests: String,
    reference_data_digests: String,
    grid_convergence: String,
    result_status: String,
}

fn sha256(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

fn quantity(value: f64, dimension: &str, unit: &str) -> TypedQuantity {
    TypedQuantity { value, dimension: dimension.to_owned(), unit: unit.to_owned() }
}

fn nonempty(value: &str) -> bool { !value.trim().is_empty() }

fn valid_git_blob_oid(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn claim_ceiling() -> String {
    "compile-time syntax, structural, typed-quantity and frozen-profile checks only; no solver execution, model validation, numerical verification, evidence authentication, physical truth, operational safety or execution authorization".to_owned()
}

fn invalid_report(source_sha256: String, diagnostic: String) -> CompilationReport {
    CompilationReport {
        compiler_id: COMPILER_ID.to_owned(),
        compiler_version: COMPILER_VERSION.to_owned(),
        registry_revision: REGISTRY_REVISION.to_owned(),
        source_sha256,
        semantic_ir_sha256: None,
        disposition: Disposition::Invalid,
        obligations: Vec::new(),
        diagnostics: vec![diagnostic],
        scenario_ir: None,
        claim_ceiling: claim_ceiling(),
    }
}

fn validate_source(source: &SourceScenario) -> Result<(), String> {
    for (name, value) in [
        ("case_id", source.case_id.as_str()),
        ("status", source.status.as_str()),
        ("parent_issue", source.parent_issue.as_str()),
        ("runner_issue", source.runner_issue.as_str()),
        ("geometry.name", source.geometry.name.as_str()),
        ("geometry.upstream_repository", source.geometry.upstream_repository.as_str()),
        ("geometry.upstream_ref", source.geometry.upstream_ref.as_str()),
        ("geometry.upstream_config_path", source.geometry.upstream_config_path.as_str()),
        ("geometry.upstream_mesh_path", source.geometry.upstream_mesh_path.as_str()),
        ("flow.model", source.flow.model.as_str()),
        ("flow.turbulence_model", source.flow.turbulence_model.as_str()),
        ("flow.wall_condition", source.flow.wall_condition.as_str()),
        ("flow.farfield_condition", source.flow.farfield_condition.as_str()),
        ("flow.convective_scheme", source.flow.convective_scheme.as_str()),
        ("flow.time_integration", source.flow.time_integration.as_str()),
        ("observables.pressure_coefficient", source.observables.pressure_coefficient.as_str()),
        ("observables.skin_friction_coefficient", source.observables.skin_friction_coefficient.as_str()),
        ("observables.lift_drag", source.observables.lift_drag.as_str()),
        ("limitations.executable_digest", source.limitations.executable_digest.as_str()),
        ("limitations.derived_config_digests", source.limitations.derived_config_digests.as_str()),
        ("limitations.reference_data_digests", source.limitations.reference_data_digests.as_str()),
        ("limitations.grid_convergence", source.limitations.grid_convergence.as_str()),
        ("limitations.result_status", source.limitations.result_status.as_str()),
    ] {
        if !nonempty(value) { return Err(format!("{name} must be a non-empty string")); }
    }
    if source.case_revision == 0 { return Err("case_revision must be greater than zero".to_owned()); }
    if !source.geometry.chord_m.is_finite() || source.geometry.chord_m <= 0.0 {
        return Err("geometry.chord_m must be finite and positive".to_owned());
    }
    let flow = &source.flow;
    for (name, value) in [
        ("flow.reynolds_number_per_chord", flow.reynolds_number_per_chord),
        ("flow.density_kg_m3", flow.density_kg_m3),
        ("flow.dynamic_viscosity_kg_m_s", flow.dynamic_viscosity_kg_m_s),
        ("flow.farfield_chords_approx", flow.farfield_chords_approx),
        ("flow.wall_y_plus_target", flow.wall_y_plus_target),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be finite and positive"));
        }
    }
    if flow.mesh_normal_nodes < 2 || flow.mesh_tangential_nodes < 2 {
        return Err("both mesh dimensions must be at least two nodes".to_owned());
    }
    if source.instances.is_empty() || source.instances.len() > MAX_INSTANCES {
        return Err(format!("instances must contain between 1 and {MAX_INSTANCES} entries"));
    }
    let mut instance_ids = BTreeSet::new();
    for instance in &source.instances {
        if !nonempty(&instance.id) || !instance_ids.insert(instance.id.as_str()) {
            return Err("instance IDs must be non-empty and unique".to_owned());
        }
        if !instance.angle_of_attack_deg.is_finite()
            || !(-90.0..=90.0).contains(&instance.angle_of_attack_deg) {
            return Err(format!("instance {} angle_of_attack_deg must be finite and within [-90, 90]", instance.id));
        }
        if instance.freestream_velocity_m_s.iter().any(|v| !v.is_finite())
            || instance.freestream_velocity_m_s[2] != 0.0
            || instance.freestream_velocity_m_s[0].hypot(instance.freestream_velocity_m_s[1]) <= 0.0 {
            return Err(format!("instance {} requires finite, non-zero, planar freestream velocity", instance.id));
        }
        let speed = instance.freestream_velocity_m_s[0]
            .hypot(instance.freestream_velocity_m_s[1]);
        let angle_radians = instance.angle_of_attack_deg.to_radians();
        let expected_x = speed * angle_radians.cos();
        let expected_y = speed * angle_radians.sin();
        let angle_vector_error = (instance.freestream_velocity_m_s[0] - expected_x)
            .hypot(instance.freestream_velocity_m_s[1] - expected_y);
        if angle_vector_error > speed * 1.0e-9 {
            return Err(format!(
                "instance {} freestream vector disagrees with angle_of_attack_deg",
                instance.id
            ));
        }
        let derived_reynolds = flow.density_kg_m3 * speed * source.geometry.chord_m
            / flow.dynamic_viscosity_kg_m_s;
        let relative_reynolds_error =
            (derived_reynolds - flow.reynolds_number_per_chord).abs()
                / flow.reynolds_number_per_chord;
        if !derived_reynolds.is_finite() || relative_reynolds_error > REYNOLDS_RELATIVE_TOLERANCE {
            return Err(format!(
                "instance {} velocity, geometry, density and viscosity disagree with the declared Reynolds number per chord",
                instance.id
            ));
        }
    }
    for (name, oid) in [
        ("geometry.upstream_config_git_blob_sha", source.geometry.upstream_config_git_blob_sha.as_str()),
        ("geometry.upstream_mesh_git_blob_sha", source.geometry.upstream_mesh_git_blob_sha.as_str()),
    ] {
        if !valid_git_blob_oid(oid) {
            return Err(format!("{name} must be a 40-character lowercase hexadecimal Git blob object ID"));
        }
    }
    Ok(())
}

fn build_ir(source: SourceScenario) -> ScenarioIr {
    let geometry = GeometryIr {
        name: source.geometry.name,
        chord: quantity(source.geometry.chord_m, "length", "m"),
        upstream_repository: source.geometry.upstream_repository,
        upstream_ref_annotation: source.geometry.upstream_ref,
        upstream_config_path: source.geometry.upstream_config_path,
        upstream_config_git_blob_oid: source.geometry.upstream_config_git_blob_sha,
        upstream_mesh_path: source.geometry.upstream_mesh_path,
        upstream_mesh_git_blob_oid: source.geometry.upstream_mesh_git_blob_sha,
    };
    let flow = FlowIr {
        model_profile: source.flow.model,
        turbulence_model: source.flow.turbulence_model,
        reynolds_number: quantity(source.flow.reynolds_number_per_chord, "dimensionless", "1"),
        density: quantity(source.flow.density_kg_m3, "mass_per_volume", "kg/m^3"),
        dynamic_viscosity: quantity(source.flow.dynamic_viscosity_kg_m_s, "dynamic_viscosity", "kg/(m*s)"),
        mesh_normal_nodes: source.flow.mesh_normal_nodes,
        mesh_tangential_nodes: source.flow.mesh_tangential_nodes,
        farfield_chords: quantity(source.flow.farfield_chords_approx, "dimensionless", "chord"),
        wall_y_plus_target: quantity(source.flow.wall_y_plus_target, "dimensionless", "1"),
        wall_condition: source.flow.wall_condition,
        farfield_condition: source.flow.farfield_condition,
        convective_scheme: source.flow.convective_scheme,
        time_integration: source.flow.time_integration,
    };
    let instances = source.instances.into_iter().map(|instance| InstanceIr {
        id: instance.id,
        angle_of_attack: quantity(instance.angle_of_attack_deg, "angle", "deg"),
        freestream_velocity: instance.freestream_velocity_m_s.map(|value| quantity(value, "velocity", "m/s")),
    }).collect();
    ScenarioIr {
        schema_version: SCHEMA_VERSION.to_owned(),
        case_id: source.case_id,
        case_revision: source.case_revision,
        source_status: source.status,
        parent_issue: source.parent_issue,
        runner_issue: source.runner_issue,
        geometry,
        flow,
        instances,
        observables: ObservableIr {
            pressure_coefficient: source.observables.pressure_coefficient,
            skin_friction_coefficient: source.observables.skin_friction_coefficient,
            lift_drag: source.observables.lift_drag,
        },
        limitations: LimitationsIr {
            executable_digest: source.limitations.executable_digest,
            derived_config_digests: source.limitations.derived_config_digests,
            reference_data_digests: source.limitations.reference_data_digests,
            grid_convergence: source.limitations.grid_convergence,
            result_status: source.limitations.result_status,
        },
    }
}

fn profile_is_supported(flow: &FlowIr) -> bool {
    flow.model_profile == "steady 2D incompressible RANS"
        && flow.turbulence_model == "Spalart-Allmaras"
        && flow.wall_condition == "no-slip adiabatic"
        && flow.farfield_condition == "far-field"
        && flow.convective_scheme == "FDS with MUSCL second order"
        && flow.time_integration == "implicit Euler"
}

#[derive(Serialize)]
struct SemanticEnvelope<'a> {
    compiler_version: &'static str,
    schema_version: &'static str,
    registry_revision: &'static str,
    parser_profile: &'static str,
    scenario: &'a ScenarioIr,
}

/// Compile one exact source byte sequence. This function never opens files,
/// resolves network references, launches tools, or executes a simulation.
pub fn compile_bytes(bytes: &[u8]) -> CompilationReport {
    let source_digest = sha256(bytes);
    if bytes.len() > MAX_SOURCE_BYTES {
        return invalid_report(source_digest, format!("source exceeds the {} byte limit", MAX_SOURCE_BYTES));
    }
    let source: SourceScenario = match toml_edit::de::from_slice(bytes) {
        Ok(source) => source,
        Err(error) => return invalid_report(source_digest, format!("strict TOML parse or typed field validation failed: {error}")),
    };
    if let Err(error) = validate_source(&source) { return invalid_report(source_digest, error); }

    let ir = build_ir(source);
    let envelope = SemanticEnvelope {
        compiler_version: COMPILER_VERSION,
        schema_version: SCHEMA_VERSION,
        registry_revision: REGISTRY_REVISION,
        parser_profile: TOML_PARSER_PROFILE,
        scenario: &ir,
    };
    let semantic_bytes = match serde_json::to_vec(&envelope) {
        Ok(bytes) => bytes,
        Err(error) => return invalid_report(source_digest, format!("normalized typed IR serialization failed: {error}")),
    };
    let semantic_digest = sha256(&semantic_bytes);

    let profile_supported = profile_is_supported(&ir.flow);
    let mut obligations = vec![CompileObligation {
        id: "model-profile".to_owned(),
        resolution: if profile_supported { Resolution::SupportedExact } else { Resolution::Unsupported },
        detail: if profile_supported {
            "flow formulation, turbulence model, boundaries, convective scheme and time integration match the frozen NACA0012 profile".to_owned()
        } else {
            "requested model feature combination is not present in the frozen local capability registry".to_owned()
        },
    }];
    obligations.push(CompileObligation {
        id: "input-artifact-bytes".to_owned(),
        resolution: Resolution::Blocked,
        detail: "upstream config/mesh object IDs are recorded, but exact retrieved bytes and independent SHA-256 content digests are not supplied to this offline compiler".to_owned(),
    });
    obligations.push(CompileObligation {
        id: "solver-executable-identity".to_owned(),
        resolution: Resolution::Blocked,
        detail: "no frozen local solver executable/build digest or registered execution adapter identity is supplied".to_owned(),
    });
    obligations.push(CompileObligation {
        id: "reference-data-identity".to_owned(),
        resolution: Resolution::Blocked,
        detail: "reference dataset bytes and content digests must be pinned before scoring or qualification".to_owned(),
    });

    let unsupported = obligations.iter().any(|o| o.resolution == Resolution::Unsupported);
    let blocked = obligations.iter().any(|o| o.resolution == Resolution::Blocked);
    let disposition = if unsupported { Disposition::Unsupported } else if blocked { Disposition::Blocked } else { Disposition::PlanReady };
    let diagnostics = obligations.iter()
        .filter(|o| matches!(o.resolution, Resolution::Blocked | Resolution::Unsupported))
        .map(|o| format!("{}: {}", o.id, o.detail))
        .collect();

    CompilationReport {
        compiler_id: COMPILER_ID.to_owned(),
        compiler_version: COMPILER_VERSION.to_owned(),
        registry_revision: REGISTRY_REVISION.to_owned(),
        source_sha256: source_digest,
        semantic_ir_sha256: Some(semantic_digest),
        disposition,
        obligations,
        diagnostics,
        scenario_ir: Some(ir),
        claim_ceiling: claim_ceiling(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &[u8] = include_bytes!("../../../benchmarks/aero/naca0012-inc-rans/case.toml");

    fn replace_once(source: &str, before: &str, after: &str) -> String {
        assert_eq!(source.matches(before).count(), 1);
        source.replacen(before, after, 1)
    }

    #[test]
    fn frozen_naca_case_compiles_to_typed_but_blocked_plan() {
        let report = compile_bytes(FIXTURE);
        assert_eq!(report.disposition, Disposition::Blocked);
        assert!(report.scenario_ir.is_some());
        assert!(report.semantic_ir_sha256.is_some());
        assert_eq!(report.obligations.len(), 4);
        assert!(report.obligations.iter().any(|o| o.id == "model-profile" && o.resolution == Resolution::SupportedExact));
        assert_eq!(report.obligations.iter().filter(|o| o.resolution == Resolution::Blocked).count(), 3);
        assert!(report.claim_ceiling.contains("no solver execution"));
    }

    #[test]
    fn duplicate_keys_and_unknown_fields_are_invalid() {
        let duplicate = b"case_id = 'first'\ncase_id = 'second'\n";
        assert_eq!(compile_bytes(duplicate).disposition, Disposition::Invalid);
        let unknown = String::from_utf8(FIXTURE.to_vec()).unwrap() + "\nunknown_authority_field = true\n";
        let report = compile_bytes(unknown.as_bytes());
        assert_eq!(report.disposition, Disposition::Invalid);
        assert!(report.semantic_ir_sha256.is_none());
    }

    #[test]
    fn source_and_semantic_identities_have_distinct_invariants() {
        let original = std::str::from_utf8(FIXTURE).unwrap();
        let whitespace_variant = format!("# harmless source comment\n{original}");
        let baseline = compile_bytes(FIXTURE);
        let reformatted = compile_bytes(whitespace_variant.as_bytes());
        assert_ne!(baseline.source_sha256, reformatted.source_sha256);
        assert_eq!(baseline.semantic_ir_sha256, reformatted.semantic_ir_sha256);
        let semantic_variant = replace_once(original, "chord_m = 1.0", "chord_m = 1.25");
        let changed = compile_bytes(semantic_variant.as_bytes());
        assert_ne!(baseline.semantic_ir_sha256, changed.semantic_ir_sha256);
    }

    #[test]
    fn invalid_quantities_and_non_planar_velocity_fail_closed() {
        let source = std::str::from_utf8(FIXTURE).unwrap();
        let negative_density = replace_once(source, "density_kg_m3 = 2.13163", "density_kg_m3 = -2.13163");
        assert_eq!(compile_bytes(negative_density.as_bytes()).disposition, Disposition::Invalid);
        let out_of_plane = replace_once(source, "[52.1572, 0.0, 0.0]", "[52.1572, 0.0, 0.1]");
        assert_eq!(compile_bytes(out_of_plane.as_bytes()).disposition, Disposition::Invalid);
        let non_finite = replace_once(source, "reynolds_number_per_chord = 6000000.0", "reynolds_number_per_chord = inf");
        assert_eq!(compile_bytes(non_finite.as_bytes()).disposition, Disposition::Invalid);
    }

    #[test]
    fn unsupported_model_profile_is_never_blocked_as_if_supported() {
        let source = std::str::from_utf8(FIXTURE).unwrap();
        let changed = replace_once(source, "turbulence_model = \"Spalart-Allmaras\"", "turbulence_model = \"unregistered-model\"");
        let report = compile_bytes(changed.as_bytes());
        assert_eq!(report.disposition, Disposition::Unsupported);
        assert!(report.obligations.iter().any(|o| o.id == "model-profile" && o.resolution == Resolution::Unsupported));
    }

    #[test]
    fn angle_velocity_and_reynolds_cross_constraints_are_checked() {
        let source = std::str::from_utf8(FIXTURE).unwrap();
        let wrong_angle = replace_once(source, "angle_of_attack_deg = 10.0", "angle_of_attack_deg = 11.0");
        assert_eq!(compile_bytes(wrong_angle.as_bytes()).disposition, Disposition::Invalid);
        let wrong_reynolds = replace_once(source, "reynolds_number_per_chord = 6000000.0", "reynolds_number_per_chord = 5900000.0");
        assert_eq!(compile_bytes(wrong_reynolds.as_bytes()).disposition, Disposition::Invalid);
    }

    #[test]
    fn toml_1_1_only_escape_is_not_accepted_by_the_toml_1_0_frontend() {
        let toml_11_only = br#"value = "\e""#;
        assert_eq!(compile_bytes(toml_11_only).disposition, Disposition::Invalid);
    }

    #[test]
    fn resource_limit_is_checked_before_parsing() {
        let oversized = vec![b' '; MAX_SOURCE_BYTES + 1];
        assert_eq!(compile_bytes(&oversized).disposition, Disposition::Invalid);
    }
}
