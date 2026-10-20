// SPDX-License-Identifier: AGPL-3.0-or-later
//! Auditable mesh-refinement studies over exact request/receipt/output comparisons.
//!
//! This module audits a declared coarse-to-fine mesh series. It does not execute
//! a solver, prove asymptotic convergence, validate a model against physical reality,
//! or qualify a design for engineering use.

use crate::{
    reference_compare::compare_reference_observables, sha256_hex, validate_solver_receipt_bytes,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub type MeshStudyResult<T> = Result<T, String>;

const STUDY_SCHEMA: &str = "physical-mesh-refinement-study-v0.1";
const REFERENCE_SCHEMA: &str = "physical-reference-observables-v0.1";
const REPORT_SCHEMA: &str = "physical-numerical-comparison-report-v0.1";
const MINIMUM_LEVELS: usize = 3;
const MAXIMUM_LEVELS: usize = 32;

/// Audit a frozen mesh-refinement manifest using exact request, reference,
/// per-level solver-receipt bytes, normalized output artifacts and comparison
/// report bytes. Every report is recomputed by the canonical numerical comparator
/// and must be structurally identical to the supplied report.
///
/// The request is common to the series; per-level mesh/solver configuration is
/// bound by each receipt. If a workflow changes the physical request per level,
/// it must use a separately designed scenario protocol rather than silently mixing
/// subjects in one mesh-refinement study.
pub fn audit_mesh_refinement_study(
    study_bytes: &[u8],
    request_bytes: &[u8],
    reference_bytes: &[u8],
    artifact_root: &Path,
    solver_receipt_bytes: &[Vec<u8>],
    comparison_report_bytes: &[Vec<u8>],
) -> MeshStudyResult<Value> {
    let study_sha256 = sha256_hex(study_bytes);
    let request_sha256 = sha256_hex(request_bytes);
    let reference_sha256 = sha256_hex(reference_bytes);

    let study_value: Value = serde_json::from_slice(study_bytes)
        .map_err(|error| format!("invalid mesh study JSON: {error}"))?;
    let study = parse_study(&study_value)?;
    if study.request_sha256 != request_sha256 {
        return Err("mesh study request_sha256 does not match exact request bytes".to_owned());
    }
    if study.reference_sha256 != reference_sha256 {
        return Err("mesh study reference_sha256 does not match exact reference bytes".to_owned());
    }
    if solver_receipt_bytes.len() != study.levels.len() {
        return Err(format!(
            "solver receipt count {} does not match manifest level count {}",
            solver_receipt_bytes.len(),
            study.levels.len()
        ));
    }
    if comparison_report_bytes.len() != study.levels.len() {
        return Err(format!(
            "comparison report count {} does not match manifest level count {}",
            comparison_report_bytes.len(),
            study.levels.len()
        ));
    }

    let request_value: Value = serde_json::from_slice(request_bytes)
        .map_err(|error| format!("invalid solver request JSON: {error}"))?;
    if get_str(&request_value, "request_id", "request")? != study.request_id
        || get_str(&request_value, "scene_sha256", "request")? != study.scene_sha256
        || get_u64(&request_value, "scene_revision", "request")? != study.scene_revision
    {
        return Err("mesh study request/scene identity does not match exact request bytes".to_owned());
    }

    let reference_value: Value = serde_json::from_slice(reference_bytes)
        .map_err(|error| format!("invalid reference JSON: {error}"))?;
    let reference = parse_reference(&reference_value, &study, &reference_sha256)?;

    let mut levels = Vec::<Value>::new();
    let mut primary_errors = Vec::<f64>::new();
    let mut level_statuses = Vec::<String>::new();
    let mut diagnostic_list = Vec::<String>::new();
    let mut identities = Vec::<Option<SolverIdentity>>::new();
    let mut receipt_hashes = BTreeSet::<String>::new();
    let mut has_unresolved = false;
    let mut strictly_decreasing = true;

    for (index, ((level, receipt_bytes), report_bytes)) in study
        .levels
        .iter()
        .zip(solver_receipt_bytes.iter())
        .zip(comparison_report_bytes.iter())
        .enumerate()
    {
        let observed_receipt_sha256 = sha256_hex(receipt_bytes);
        if observed_receipt_sha256 != level.solver_receipt_sha256 {
            return Err(format!(
                "level '{}' solver-receipt SHA-256 mismatch",
                level.level_id
            ));
        }
        let observed_report_sha256 = sha256_hex(report_bytes);
        if observed_report_sha256 != level.comparison_report_sha256 {
            return Err(format!(
                "level '{}' comparison-report SHA-256 mismatch",
                level.level_id
            ));
        }
        if !receipt_hashes.insert(observed_receipt_sha256.clone()) {
            has_unresolved = true;
            diagnostic_list.push(format!(
                "level '{}' reuses a solver receipt already bound to another mesh level",
                level.level_id
            ));
        }

        let contract_report = validate_solver_receipt_bytes(request_bytes, receipt_bytes)?;
        if contract_report["receipt_contract_validation"] != "PASS" {
            has_unresolved = true;
            diagnostic_list.push(format!(
                "level '{}' receipt/request contract did not PASS",
                level.level_id
            ));
        }

        let receipt_value: Value = serde_json::from_slice(receipt_bytes)
            .map_err(|error| format!("level '{}' receipt JSON invalid: {error}", level.level_id))?;
        let report_value: Value = serde_json::from_slice(report_bytes)
            .map_err(|error| format!("level '{}' comparison report JSON invalid: {error}", level.level_id))?;
        let observation_output_id = get_str(
            &report_value,
            "observation_output_id",
            "comparison report",
        )?;

        let recomputed_report = compare_reference_observables(
            request_bytes,
            receipt_bytes,
            artifact_root,
            observation_output_id,
            reference_bytes,
        )?;
        if recomputed_report != report_value {
            return Err(format!(
                "level '{}' comparison report does not reproduce from the exact request, receipt, artifact bytes and reference",
                level.level_id
            ));
        }

        let comparison_status = get_str(&recomputed_report, "comparison_status", "comparison report")?;
        let artifact_status = get_str(
            &recomputed_report,
            "artifact_verification_status",
            "comparison report",
        )?;
        let producer_status = get_str(&recomputed_report, "producer_status", "comparison report")?;
        if artifact_status != "PASS" || producer_status != "PASS" {
            has_unresolved = true;
            diagnostic_list.push(format!(
                "level '{}' lacks a producer PASS or fully verified output bytes",
                level.level_id
            ));
        }
        if comparison_status == "UNRESOLVED" {
            has_unresolved = true;
            diagnostic_list.push(format!(
                "level '{}' numerical comparison is UNRESOLVED",
                level.level_id
            ));
        }
        level_statuses.push(comparison_status.to_owned());

        let identity = solver_identity(&receipt_value);
        if producer_status == "PASS" && artifact_status == "PASS" {
            match identity {
                Ok(identity) => identities.push(Some(identity)),
                Err(reason) => {
                    identities.push(None);
                    has_unresolved = true;
                    diagnostic_list.push(format!("level '{}': {reason}", level.level_id));
                }
            }
        } else {
            identities.push(None);
        }

        let primary = find_primary_observable(
            &recomputed_report,
            &study.primary_observable_id,
            &reference,
        )?;
        if let Some(item) = primary.as_ref() {
            primary_errors.push(item.absolute_error);
        } else {
            primary_errors.push(f64::NAN);
        }

        if index > 0 {
            let previous_error = primary_errors[index - 1];
            let current_error = primary_errors[index];
            let decreased = previous_error.is_finite()
                && current_error.is_finite()
                && current_error < previous_error;
            if !decreased {
                strictly_decreasing = false;
            }
        }

        let error_trend = if index == 0 || !primary_errors[index].is_finite() {
            "not_applicable"
        } else {
            let previous_error = primary_errors[index - 1];
            let current_error = primary_errors[index];
            if !previous_error.is_finite() {
                "unresolved"
            } else if current_error < previous_error {
                "decreased"
            } else if current_error == previous_error {
                "unchanged"
            } else {
                "increased"
            }
        };

        levels.push(json!({
            "level_id": level.level_id,
            "mesh_size_h_m": level.mesh_size_h_m,
            "degrees_of_freedom": level.degrees_of_freedom,
            "solver_receipt_sha256": observed_receipt_sha256,
            "comparison_report_sha256": observed_report_sha256,
            "comparison_status": comparison_status,
            "artifact_verification_status": artifact_status,
            "producer_status": producer_status,
            "receipt_adapter_id": identity.as_ref().map(|item| item.adapter_id.as_str()),
            "receipt_adapter_version": identity.as_ref().map(|item| item.adapter_version.as_str()),
            "receipt_adapter_executable_sha256": identity.as_ref().map(|item| item.executable_sha256.as_str()),
            "receipt_adapter_configuration_sha256": identity.as_ref().map(|item| item.configuration_sha256.as_str()),
            "solver_name": identity.as_ref().map(|item| item.solver_name.as_str()),
            "solver_version": identity.as_ref().map(|item| item.solver_version.as_str()),
            "solver_build_ref": identity.as_ref().map(|item| item.solver_build_ref.as_str()),
            "primary_observable_id": study.primary_observable_id,
            "primary_observed_value": primary.as_ref().map(|item| item.observed_value),
            "primary_reference_value": primary.as_ref().map(|item| item.reference_value),
            "primary_absolute_error": primary.as_ref().map(|item| item.absolute_error),
            "primary_allowed_absolute_error": primary.as_ref().map(|item| item.allowed_absolute_error),
            "primary_within_tolerance": primary.as_ref().map(|item| item.within_tolerance),
            "error_trend_from_previous_level": error_trend
        }));
    }

    let identity_reference = identities.iter().flatten().next().cloned();
    let solver_identity_consistent = identity_reference.as_ref().is_some_and(|expected| {
        identities.iter().all(|maybe_identity| {
            maybe_identity
                .as_ref()
                .is_some_and(|identity| identity.same_solver_build(expected))
        })
    });
    if !solver_identity_consistent {
        has_unresolved = true;
        diagnostic_list.push(
            "solver/adapter build identity is missing or changes across mesh levels; refinement results are not directly comparable"
                .to_owned(),
        );
    }

    let mut interval_orders = Vec::<Value>::new();
    for index in 0..study.levels.len().saturating_sub(1) {
        let coarse = &study.levels[index];
        let fine = &study.levels[index + 1];
        let coarse_error = primary_errors[index];
        let fine_error = primary_errors[index + 1];
        let h_ratio = coarse.mesh_size_h_m / fine.mesh_size_h_m;
        let order = if coarse_error > 0.0
            && fine_error > 0.0
            && coarse_error.is_finite()
            && fine_error.is_finite()
            && h_ratio > 1.0
        {
            let estimate = (coarse_error / fine_error).ln() / h_ratio.ln();
            estimate.is_finite().then_some(estimate)
        } else {
            None
        };
        let estimate_status = if !coarse_error.is_finite() || !fine_error.is_finite() {
            "unresolved"
        } else if coarse_error == 0.0 || fine_error == 0.0 {
            "undefined_zero_error"
        } else if order.is_some() {
            "estimated"
        } else {
            "overflow"
        };
        interval_orders.push(json!({
            "coarse_level_id": coarse.level_id,
            "fine_level_id": fine.level_id,
            "coarse_mesh_size_h_m": coarse.mesh_size_h_m,
            "fine_mesh_size_h_m": fine.mesh_size_h_m,
            "coarse_absolute_error": if coarse_error.is_finite() { json!(coarse_error) } else { Value::Null },
            "fine_absolute_error": if fine_error.is_finite() { json!(fine_error) } else { Value::Null },
            "observed_order_estimate": order,
            "order_estimate_status": estimate_status,
            "error_decreased": coarse_error.is_finite() && fine_error.is_finite() && fine_error < coarse_error
        }));
    }

    let finest_index = study.levels.len() - 1;
    let finest_comparison_status = &level_statuses[finest_index];
    let finest_error = primary_errors[finest_index];
    let finest_within_tolerance = finest_error.is_finite()
        && finest_error
            <= reference.primary_absolute_tolerance
                + reference.primary_relative_tolerance * reference.primary_expected_value.abs();

    let convergence_gate_failed = study.require_strictly_decreasing_error && !strictly_decreasing;
    let status = if has_unresolved {
        "UNRESOLVED"
    } else if finest_comparison_status != "PASS"
        || !finest_within_tolerance
        || convergence_gate_failed
    {
        "FAIL"
    } else {
        "PASS"
    };

    if convergence_gate_failed {
        diagnostic_list.push(
            "strictly decreasing primary-observable error was required but not observed at every refinement step"
                .to_owned(),
        );
    }
    if finest_comparison_status != "PASS" || !finest_within_tolerance {
        diagnostic_list.push(
            "finest mesh did not pass the complete numerical reference comparison and primary-observable tolerance gate"
                .to_owned(),
        );
    }

    Ok(json!({
        "schema_version": "physical-mesh-refinement-study-report-v0.1",
        "study_status": status,
        "study_id": study.study_id,
        "study_manifest_sha256": study_sha256,
        "reference_id": reference.reference_id,
        "reference_sha256": reference_sha256,
        "comparison_protocol_id": study.comparison_protocol_id,
        "request_id": study.request_id,
        "request_sha256": study.request_sha256,
        "scene_sha256": study.scene_sha256,
        "scene_revision": study.scene_revision,
        "case_id": study.case_id,
        "mesh_family_id": study.mesh_family_id,
        "mesh_generator_identity": study.mesh_generator_identity,
        "refinement_method": study.refinement_method,
        "solver_identity_consistent": solver_identity_consistent,
        "solver_name": identity_reference.as_ref().map(|item| item.solver_name.as_str()),
        "solver_version": identity_reference.as_ref().map(|item| item.solver_version.as_str()),
        "solver_build_ref": identity_reference.as_ref().map(|item| item.solver_build_ref.as_str()),
        "adapter_id": identity_reference.as_ref().map(|item| item.adapter_id.as_str()),
        "adapter_version": identity_reference.as_ref().map(|item| item.adapter_version.as_str()),
        "adapter_executable_sha256": identity_reference.as_ref().map(|item| item.executable_sha256.as_str()),
        "primary_observable_id": study.primary_observable_id,
        "primary_unit": reference.primary_unit,
        "primary_reference_value": reference.primary_expected_value,
        "primary_absolute_tolerance": reference.primary_absolute_tolerance,
        "primary_relative_tolerance": reference.primary_relative_tolerance,
        "level_count": study.levels.len(),
        "minimum_levels_required": study.minimum_levels,
        "strictly_decreasing_error_required": study.require_strictly_decreasing_error,
        "strictly_decreasing_primary_error": strictly_decreasing,
        "finest_level_id": study.levels[finest_index].level_id,
        "finest_comparison_status": finest_comparison_status,
        "finest_primary_absolute_error": if finest_error.is_finite() { json!(finest_error) } else { Value::Null },
        "finest_primary_within_tolerance": finest_within_tolerance,
        "levels": levels,
        "observed_order_estimates": interval_orders,
        "diagnostics": diagnostic_list,
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "claim_ceiling": "mesh-refinement trend and finest-level reference comparison for the named outputs and declared mesh family only; no execution authentication, physical validation, universal convergence claim or engineering-use qualification"
    }))
}

#[derive(Clone, Debug)]
struct StudyLevel {
    level_id: String,
    mesh_size_h_m: f64,
    degrees_of_freedom: u64,
    solver_receipt_sha256: String,
    comparison_report_sha256: String,
}

#[derive(Clone, Debug)]
struct StudyManifest {
    study_id: String,
    case_id: String,
    request_id: String,
    request_sha256: String,
    scene_sha256: String,
    scene_revision: u64,
    reference_id: String,
    reference_sha256: String,
    comparison_protocol_id: String,
    primary_observable_id: String,
    mesh_family_id: String,
    mesh_generator_identity: String,
    refinement_method: String,
    minimum_levels: usize,
    require_strictly_decreasing_error: bool,
    levels: Vec<StudyLevel>,
}

#[derive(Clone, Debug)]
struct ReferenceForStudy {
    reference_id: String,
    case_id: String,
    request_id: String,
    request_sha256: String,
    scene_sha256: String,
    scene_revision: u64,
    comparison_protocol_id: String,
    primary_expected_value: f64,
    primary_unit: String,
    primary_absolute_tolerance: f64,
    primary_relative_tolerance: f64,
    observables: BTreeMap<String, ReferenceObservable>,
}

#[derive(Clone, Debug)]
struct ReferenceObservable {
    expected_value: f64,
    unit: String,
    absolute_tolerance: f64,
    relative_tolerance: f64,
}

#[derive(Clone, Debug)]
struct PrimaryObservable {
    observed_value: f64,
    reference_value: f64,
    absolute_error: f64,
    allowed_absolute_error: f64,
    within_tolerance: bool,
}

#[derive(Clone, Debug)]
struct SolverIdentity {
    receipt_sha256: String,
    adapter_id: String,
    adapter_version: String,
    executable_sha256: String,
    configuration_sha256: String,
    solver_name: String,
    solver_version: String,
    solver_build_ref: String,
}

impl SolverIdentity {
    fn same_solver_build(&self, other: &Self) -> bool {
        self.adapter_id == other.adapter_id
            && self.adapter_version == other.adapter_version
            && self.executable_sha256 == other.executable_sha256
            && self.solver_name == other.solver_name
            && self.solver_version == other.solver_version
            && self.solver_build_ref == other.solver_build_ref
    }
}

fn parse_study(value: &Value) -> MeshStudyResult<StudyManifest> {
    exact_keys(
        value,
        &[
            "schema_version",
            "study_id",
            "case_id",
            "request_id",
            "request_sha256",
            "scene_sha256",
            "scene_revision",
            "reference_id",
            "reference_sha256",
            "comparison_protocol_id",
            "primary_observable_id",
            "mesh_family_id",
            "mesh_generator_identity",
            "refinement_method",
            "minimum_levels",
            "require_strictly_decreasing_error",
            "levels",
        ],
        "mesh study",
    )?;
    expect_str(value, "schema_version", STUDY_SCHEMA, "mesh study")?;
    let study_id = token(get_str(value, "study_id", "mesh study")?, "study ID")?;
    let case_id = token(get_str(value, "case_id", "mesh study")?, "case ID")?;
    let request_id = token(get_str(value, "request_id", "mesh study")?, "request ID")?;
    let request_sha256 = digest(value, "request_sha256", "mesh study")?;
    let scene_sha256 = digest(value, "scene_sha256", "mesh study")?;
    let scene_revision = get_u64(value, "scene_revision", "mesh study")?;
    let reference_id = token(get_str(value, "reference_id", "mesh study")?, "reference ID")?;
    let reference_sha256 = digest(value, "reference_sha256", "mesh study")?;
    let comparison_protocol_id = token(
        get_str(value, "comparison_protocol_id", "mesh study")?,
        "comparison protocol ID",
    )?;
    let primary_observable_id = token(
        get_str(value, "primary_observable_id", "mesh study")?,
        "primary observable ID",
    )?;
    let mesh_family_id = token(
        get_str(value, "mesh_family_id", "mesh study")?,
        "mesh family ID",
    )?;
    let mesh_generator_identity = non_empty(
        get_str(value, "mesh_generator_identity", "mesh study")?,
        "mesh study.mesh_generator_identity",
        512,
    )?;
    let refinement_method = non_empty(
        get_str(value, "refinement_method", "mesh study")?,
        "mesh study.refinement_method",
        128,
    )?;
    let minimum_levels = get_u64(value, "minimum_levels", "mesh study")? as usize;
    if !(MINIMUM_LEVELS..=MAXIMUM_LEVELS).contains(&minimum_levels) {
        return Err(format!(
            "mesh study.minimum_levels must be between {MINIMUM_LEVELS} and {MAXIMUM_LEVELS}"
        ));
    }
    let require_strictly_decreasing_error = value
        .get("require_strictly_decreasing_error")
        .and_then(Value::as_bool)
        .ok_or_else(|| "mesh study.require_strictly_decreasing_error must be boolean".to_owned())?;
    let raw_levels = get_array(value, "levels", "mesh study")?;
    if raw_levels.len() < minimum_levels || raw_levels.len() < MINIMUM_LEVELS {
        return Err(format!(
            "mesh study requires at least {minimum_levels} levels (and never fewer than {MINIMUM_LEVELS})"
        ));
    }
    if raw_levels.len() > MAXIMUM_LEVELS {
        return Err(format!("mesh study may not exceed {MAXIMUM_LEVELS} levels"));
    }

    let mut levels: Vec<StudyLevel> = Vec::with_capacity(raw_levels.len());
    let mut ids = BTreeSet::new();
    for (index, raw) in raw_levels.iter().enumerate() {
        let at = format!("mesh study.levels[{index}]");
        exact_keys(
            raw,
            &[
                "level_id",
                "mesh_size_h_m",
                "degrees_of_freedom",
                "solver_receipt_sha256",
                "comparison_report_sha256",
            ],
            &at,
        )?;
        let level_id = token(get_str(raw, "level_id", &at)?, "mesh level ID")?;
        if !ids.insert(level_id.clone()) {
            return Err(format!("{at}.level_id '{level_id}' is duplicated"));
        }
        let mesh_size_h_m = get_finite_number(raw, "mesh_size_h_m", &at)?;
        if mesh_size_h_m <= 0.0 {
            return Err(format!("{at}.mesh_size_h_m must be positive"));
        }
        let degrees_of_freedom = get_u64(raw, "degrees_of_freedom", &at)?;
        if degrees_of_freedom == 0 {
            return Err(format!("{at}.degrees_of_freedom must be positive"));
        }
        let solver_receipt_sha256 = digest(raw, "solver_receipt_sha256", &at)?;
        let comparison_report_sha256 = digest(raw, "comparison_report_sha256", &at)?;
        if let Some(previous) = levels.last() {
            if mesh_size_h_m >= previous.mesh_size_h_m {
                return Err(format!(
                    "{at}.mesh_size_h_m must strictly decrease in coarse-to-fine order"
                ));
            }
            if degrees_of_freedom <= previous.degrees_of_freedom {
                return Err(format!(
                    "{at}.degrees_of_freedom must strictly increase in coarse-to-fine order"
                ));
            }
        }
        levels.push(StudyLevel {
            level_id,
            mesh_size_h_m,
            degrees_of_freedom,
            solver_receipt_sha256,
            comparison_report_sha256,
        });
    }

    Ok(StudyManifest {
        study_id,
        case_id,
        request_id,
        request_sha256,
        scene_sha256,
        scene_revision,
        reference_id,
        reference_sha256,
        comparison_protocol_id,
        primary_observable_id,
        mesh_family_id,
        mesh_generator_identity,
        refinement_method,
        minimum_levels,
        require_strictly_decreasing_error,
        levels,
    })
}

fn parse_reference(
    value: &Value,
    study: &StudyManifest,
    reference_sha256: &str,
) -> MeshStudyResult<ReferenceForStudy> {
    exact_keys(
        value,
        &[
            "schema_version",
            "reference_id",
            "case_id",
            "request_id",
            "scene_sha256",
            "scene_revision",
            "request_sha256",
            "comparison_protocol_id",
            "oracle",
            "observables",
        ],
        "reference manifest",
    )?;
    expect_str(value, "schema_version", REFERENCE_SCHEMA, "reference manifest")?;
    let reference_id = token(get_str(value, "reference_id", "reference manifest")?, "reference ID")?;
    let case_id = token(get_str(value, "case_id", "reference manifest")?, "case ID")?;
    let request_id = token(get_str(value, "request_id", "reference manifest")?, "request ID")?;
    let request_sha256 = digest(value, "request_sha256", "reference manifest")?;
    let scene_sha256 = digest(value, "scene_sha256", "reference manifest")?;
    let scene_revision = get_u64(value, "scene_revision", "reference manifest")?;
    let comparison_protocol_id = token(
        get_str(value, "comparison_protocol_id", "reference manifest")?,
        "comparison protocol ID",
    )?;
    if study.reference_sha256 != reference_sha256 {
        return Err("mesh study reference digest does not match exact reference bytes".to_owned());
    }
    if reference_id != study.reference_id
        || case_id != study.case_id
        || request_id != study.request_id
        || request_sha256 != study.request_sha256
        || scene_sha256 != study.scene_sha256
        || scene_revision != study.scene_revision
        || comparison_protocol_id != study.comparison_protocol_id
    {
        return Err("reference manifest identity does not match mesh study identity".to_owned());
    }
    let oracle = value
        .get("oracle")
        .ok_or_else(|| "reference manifest.oracle is missing".to_owned())?;
    exact_keys(oracle, &["method", "identity", "source_ref"], "reference.oracle")?;
    let oracle_method = get_str(oracle, "method", "reference.oracle")?;
    if !["analytical_exact", "independent_numerical"].contains(&oracle_method) {
        return Err("reference oracle method is unsupported".to_owned());
    }
    let obs = get_array(value, "observables", "reference manifest")?;
    if obs.is_empty() {
        return Err("reference manifest must include observables".to_owned());
    }
    let mut observables = BTreeMap::new();
    let mut primary = None;
    for (index, item) in obs.iter().enumerate() {
        let at = format!("reference manifest.observables[{index}]");
        exact_keys(
            item,
            &[
                "observable_id",
                "expected_value",
                "unit",
                "absolute_tolerance",
                "relative_tolerance",
            ],
            &at,
        )?;
        let id = token(get_str(item, "observable_id", &at)?, "reference observable ID")?;
        let expected_value = get_finite_number(item, "expected_value", &at)?;
        let unit = non_empty(get_str(item, "unit", &at)?, &format!("{at}.unit"), 64)?;
        let absolute_tolerance = get_finite_number(item, "absolute_tolerance", &at)?;
        let relative_tolerance = get_finite_number(item, "relative_tolerance", &at)?;
        if absolute_tolerance < 0.0 || relative_tolerance < 0.0 {
            return Err(format!("{at} tolerances must be non-negative"));
        }
        let allowed = absolute_tolerance + relative_tolerance * expected_value.abs();
        if !allowed.is_finite() {
            return Err(format!("{at} allowed error overflows"));
        }
        let record = ReferenceObservable {
            expected_value,
            unit: unit.clone(),
            absolute_tolerance,
            relative_tolerance,
        };
        if observables.insert(id.clone(), record).is_some() {
            return Err(format!("{at}.observable_id '{id}' is duplicated"));
        }
        if id == study.primary_observable_id {
            primary = Some((expected_value, unit, absolute_tolerance, relative_tolerance));
        }
    }
    let (primary_expected_value, primary_unit, primary_absolute_tolerance, primary_relative_tolerance) =
        primary.ok_or_else(|| {
            format!(
                "reference manifest lacks primary observable '{}'",
                study.primary_observable_id
            )
        })?;

    Ok(ReferenceForStudy {
        reference_id,
        case_id,
        request_id,
        request_sha256,
        scene_sha256,
        scene_revision,
        comparison_protocol_id,
        primary_expected_value,
        primary_unit,
        primary_absolute_tolerance,
        primary_relative_tolerance,
        observables,
    })
}

fn find_primary_observable(
    report: &Value,
    primary_id: &str,
    reference: &ReferenceForStudy,
) -> MeshStudyResult<Option<PrimaryObservable>> {
    if get_str(report, "comparison_status", "comparison report")? == "UNRESOLVED" {
        return Ok(None);
    }
    let results = get_array(report, "observable_results", "comparison report")?;
    let primary = results
        .iter()
        .find(|item| item.get("observable_id").and_then(Value::as_str) == Some(primary_id))
        .ok_or_else(|| format!("comparison report lacks primary observable '{primary_id}'"))?;
    let unit = get_str(primary, "unit", "primary observable")?;
    let reference_value = get_finite_number(primary, "reference_value", "primary observable")?;
    let observed_value = get_finite_number(primary, "observed_value", "primary observable")?;
    let absolute_error = get_finite_number(primary, "absolute_error", "primary observable")?;
    let allowed_absolute_error =
        get_finite_number(primary, "allowed_absolute_error", "primary observable")?;
    let within_tolerance = primary
        .get("within_tolerance")
        .and_then(Value::as_bool)
        .ok_or_else(|| "primary observable within_tolerance must be boolean".to_owned())?;
    let expected = reference
        .observables
        .get(primary_id)
        .ok_or_else(|| "primary observable missing from reference registry".to_owned())?;
    if unit != expected.unit
        || reference_value != expected.expected_value
        || allowed_absolute_error
            != expected.absolute_tolerance
                + expected.relative_tolerance * expected.expected_value.abs()
        || within_tolerance != (absolute_error <= allowed_absolute_error)
    {
        return Err("primary comparison result is inconsistent with the exact reference".to_owned());
    }
    Ok(Some(PrimaryObservable {
        observed_value,
        reference_value,
        absolute_error,
        allowed_absolute_error,
        within_tolerance,
    }))
}

fn solver_identity(receipt: &Value) -> MeshStudyResult<SolverIdentity> {
    let adapter = receipt
        .get("adapter")
        .ok_or_else(|| "solver receipt has no adapter".to_owned())?;
    Ok(SolverIdentity {
        receipt_sha256: sha256_hex(
            &serde_json::to_vec(receipt)
                .map_err(|error| format!("receipt serialization failed: {error}"))?,
        ),
        adapter_id: token(get_str(adapter, "adapter_id", "receipt.adapter")?, "adapter ID")?,
        adapter_version: non_empty(
            get_str(adapter, "version", "receipt.adapter")?,
            "receipt.adapter.version",
            128,
        )?,
        executable_sha256: digest(adapter, "executable_sha256", "receipt.adapter")?,
        configuration_sha256: digest(adapter, "configuration_sha256", "receipt.adapter")?,
        solver_name: non_empty(
            get_str(adapter, "solver_name", "receipt.adapter")?,
            "receipt.adapter.solver_name",
            256,
        )?,
        solver_version: non_empty(
            get_str(adapter, "solver_version", "receipt.adapter")?,
            "receipt.adapter.solver_version",
            128,
        )?,
        solver_build_ref: non_empty(
            get_str(adapter, "solver_build_ref", "receipt.adapter")?,
            "receipt.adapter.solver_build_ref",
            2048,
        )?,
    })
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> MeshStudyResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn get_str<'a>(value: &'a Value, key: &str, at: &str) -> MeshStudyResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn expect_str(value: &Value, key: &str, expected: &str, at: &str) -> MeshStudyResult<()> {
    if get_str(value, key, at)? != expected {
        return Err(format!("{at}.{key} must equal '{expected}'"));
    }
    Ok(())
}

fn get_array<'a>(value: &'a Value, key: &str, at: &str) -> MeshStudyResult<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))
}

fn get_u64(value: &Value, key: &str, at: &str) -> MeshStudyResult<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key} must be a non-negative integer"))
}

fn get_finite_number(value: &Value, key: &str, at: &str) -> MeshStudyResult<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{at}.{key} must be a finite number"))
}

fn digest(value: &Value, key: &str, at: &str) -> MeshStudyResult<String> {
    let digest = get_str(value, key, at)?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!("{at}.{key} must be a lowercase SHA-256 digest"));
    }
    Ok(digest.to_owned())
}

fn non_empty(value: &str, at: &str, max_bytes: usize) -> MeshStudyResult<String> {
    if value.trim().is_empty() || value.len() > max_bytes {
        return Err(format!("{at} must be non-empty and at most {max_bytes} bytes"));
    }
    Ok(value.to_owned())
}

fn token(value: &str, label: &str) -> MeshStudyResult<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':'))
    {
        return Err(format!("invalid {label}"));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reference_bytes() -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema_version":"physical-reference-observables-v0.1",
            "reference_id":"mesh-reference-v1",
            "case_id":"cantilever-case-v1",
            "request_id":"mesh-request-v1",
            "scene_sha256":"b".repeat(64),
            "scene_revision":7,
            "request_sha256":"a".repeat(64),
            "comparison_protocol_id":"cantilever-tolerance-v1",
            "oracle":{"method":"analytical_exact","identity":"independent cantilever formula","source_ref":"test-oracle://cantilever"},
            "observables":[
                {"observable_id":"tip_deflection_m","expected_value":1.0,"unit":"m","absolute_tolerance":0.003,"relative_tolerance":0.001},
                {"observable_id":"support_reaction_n","expected_value":100.0,"unit":"N","absolute_tolerance":0.01,"relative_tolerance":0.001}
            ]
        })).unwrap()
    }

    fn report_bytes(
        h: f64,
        value: f64,
        reaction: f64,
        status: &str,
    ) -> Vec<u8> {
        let reference = reference_bytes();
        let report_items = [
            ("tip_deflection_m", value, 1.0, "m", 0.003, 0.001),
            ("support_reaction_n", reaction, 100.0, "N", 0.01, 0.001),
        ];
        let results: Vec<Value> = report_items
            .iter()
            .map(|(id, observed, expected, unit, abs_tol, rel_tol)| {
                let error = (*observed - *expected).abs();
                let allowed = *abs_tol + *rel_tol * expected.abs();
                json!({
                    "observable_id":id,
                    "unit":unit,
                    "reference_value":expected,
                    "observed_value":observed,
                    "absolute_error":error,
                    "relative_error":if *expected == 0.0 { Value::Null } else { json!(error / expected.abs()) },
                    "relative_error_defined":*expected != 0.0,
                    "relative_error_status":if *expected == 0.0 { "undefined_zero_reference" } else { "computed" },
                    "absolute_tolerance":abs_tol,
                    "relative_tolerance":rel_tol,
                    "allowed_absolute_error":allowed,
                    "within_tolerance":error <= allowed
                })
            })
            .collect();
        let any_failing = results.iter().any(|item| item["within_tolerance"] == false);
        let status = if any_failing { "FAIL" } else { status };
        serde_json::to_vec(&json!({
            "schema_version":REPORT_SCHEMA,
            "comparison_status":status,
            "comparison_method":"per_observable_absolute_plus_relative_tolerance",
            "comparison_protocol_id":"cantilever-tolerance-v1",
            "reference_id":"mesh-reference-v1",
            "reference_sha256":sha256_hex(&reference),
            "oracle_method":"analytical_exact",
            "oracle_identity":"independent cantilever formula",
            "oracle_source_ref":"test-oracle://cantilever",
            "request_sha256":"a".repeat(64),
            "case_id":"cantilever-case-v1",
            "request_id":"mesh-request-v1",
            "scene_sha256":"b".repeat(64),
            "scene_revision":7,
            "observation_output_id":format!("mesh-{h}"),
            "observation_artifact_sha256":"c".repeat(64),
            "artifact_verification_status":"PASS",
            "producer_status":"PASS",
            "numerical_observable_comparison":status,
            "physical_result_qualified":false,
            "real_world_validation_status":"UNRESOLVED",
            "diagnostics":[],
            "observable_results":results,
            "claim_ceiling":"synthetic comparison report only"
        })).unwrap()
    }

    fn study_bytes(reports: &[Vec<u8>], errors: &[f64], require_decreasing: bool) -> Vec<u8> {
        let reference = reference_bytes();
        let hs = [0.4, 0.2, 0.1];
        let dofs = [100_u64, 400_u64, 1600_u64];
        let levels: Vec<Value> = hs
            .iter()
            .zip(dofs.iter())
            .zip(reports.iter())
            .enumerate()
            .map(|(index, ((h, dof), report))| {
                json!({
                    "level_id":format!("mesh-level-{}", index + 1),
                    "mesh_size_h_m":h,
                    "degrees_of_freedom":dof,
                    "comparison_report_sha256":sha256_hex(report)
                })
            })
            .collect();
        let _ = errors;
        serde_json::to_vec(&json!({
            "schema_version":STUDY_SCHEMA,
            "study_id":"cantilever-mesh-study-v1",
            "case_id":"cantilever-case-v1",
            "request_id":"mesh-request-v1",
            "request_sha256":"a".repeat(64),
            "scene_sha256":"b".repeat(64),
            "scene_revision":7,
            "reference_id":"mesh-reference-v1",
            "reference_sha256":sha256_hex(&reference),
            "comparison_protocol_id":"cantilever-tolerance-v1",
            "primary_observable_id":"tip_deflection_m",
            "mesh_family_id":"linear-solid-hex-v1",
            "mesh_generator_identity":"synthetic mesh generator v1",
            "refinement_method":"uniform",
            "minimum_levels":3,
            "require_strictly_decreasing_error":require_decreasing,
            "levels":levels
        })).unwrap()
    }

    #[test]
    fn three_level_study_passes_when_finest_level_and_refinement_gate_pass() {
        let reports = vec![
            report_bytes(0.4, 1.03, 100.02, "FAIL"),
            report_bytes(0.2, 1.01, 100.001, "FAIL"),
            report_bytes(0.1, 1.001, 100.0001, "PASS"),
        ];
        let reference = reference_bytes();
        let study = study_bytes(&reports, &[0.03, 0.01, 0.001], true);
        let report = audit_mesh_refinement_study(&study, &reference, &reports).unwrap();
        assert_eq!(report["study_status"], "PASS");
        assert_eq!(report["level_count"], 3);
        assert_eq!(report["finest_level_id"], "mesh-level-3");
        assert_eq!(report["finest_comparison_status"], "PASS");
        assert_eq!(report["strictly_decreasing_primary_error"], true);
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
        assert_eq!(report["observed_order_estimates"].as_array().unwrap().len(), 2);
        assert_eq!(report["observed_order_estimates"][0]["order_estimate_status"], "estimated");
    }

    #[test]
    fn exact_report_digest_is_required_for_every_mesh_level() {
        let reports = vec![
            report_bytes(0.4, 1.03, 100.02, "FAIL"),
            report_bytes(0.2, 1.01, 100.001, "FAIL"),
            report_bytes(0.1, 1.001, 100.0001, "PASS"),
        ];
        let reference = reference_bytes();
        let study_value: Value = serde_json::from_slice(&study_bytes(&reports, &[0.03, 0.01, 0.001], true)).unwrap();
        let mut modified = study_value.clone();
        modified["levels"][1]["comparison_report_sha256"] = json!("f".repeat(64));
        let study = serde_json::to_vec(&modified).unwrap();
        assert!(audit_mesh_refinement_study(&study, &reference, &reports)
            .unwrap_err()
            .contains("comparison report SHA-256 mismatch"));
    }

    #[test]
    fn non_monotonic_error_fails_when_strict_decrease_is_preregistered() {
        let reports = vec![
            report_bytes(0.4, 1.01, 100.02, "FAIL"),
            report_bytes(0.2, 1.02, 100.001, "FAIL"),
            report_bytes(0.1, 1.001, 100.0001, "PASS"),
        ];
        let reference = reference_bytes();
        let study = study_bytes(&reports, &[0.01, 0.02, 0.001], true);
        let report = audit_mesh_refinement_study(&study, &reference, &reports).unwrap();
        assert_eq!(report["study_status"], "FAIL");
        assert_eq!(report["strictly_decreasing_primary_error"], false);
        assert_eq!(
            report["diagnostics"].as_array().unwrap().len(),
            1
        );
    }

    #[test]
    fn non_monotonic_trend_is_reported_not_failed_when_not_required() {
        let reports = vec![
            report_bytes(0.4, 1.01, 100.02, "FAIL"),
            report_bytes(0.2, 1.02, 100.001, "FAIL"),
            report_bytes(0.1, 1.001, 100.0001, "PASS"),
        ];
        let reference = reference_bytes();
        let study = study_bytes(&reports, &[0.01, 0.02, 0.001], false);
        let report = audit_mesh_refinement_study(&study, &reference, &reports).unwrap();
        assert_eq!(report["study_status"], "PASS");
        assert_eq!(report["strictly_decreasing_primary_error"], false);
        assert_eq!(
            report["levels"][1]["error_trend_from_previous_level"],
            "increased"
        );
    }

    #[test]
    fn unresolved_mesh_comparison_keeps_study_unresolved() {
        let reports = vec![
            report_bytes(0.4, 1.03, 100.02, "FAIL"),
            report_bytes(0.2, 1.01, 100.001, "FAIL"),
            {
                let mut value: Value = serde_json::from_slice(&report_bytes(0.1, 1.001, 100.0001, "PASS")).unwrap();
                value["comparison_status"] = json!("UNRESOLVED");
                value["numerical_observable_comparison"] = json!("UNRESOLVED");
                value["observable_results"] = json!([]);
                serde_json::to_vec(&value).unwrap()
            },
        ];
        let reference = reference_bytes();
        let study = study_bytes(&reports, &[0.03, 0.01, 0.001], true);
        let report = audit_mesh_refinement_study(&study, &reference, &reports).unwrap();
        assert_eq!(report["study_status"], "UNRESOLVED");
        assert_eq!(report["finest_comparison_status"], "UNRESOLVED");
    }

    #[test]
    fn coarse_to_fine_order_and_degree_of_freedom_growth_are_required() {
        let reports = vec![
            report_bytes(0.4, 1.03, 100.02, "FAIL"),
            report_bytes(0.2, 1.01, 100.001, "FAIL"),
            report_bytes(0.1, 1.001, 100.0001, "PASS"),
        ];
        let reference = reference_bytes();
        let study_value: Value = serde_json::from_slice(&study_bytes(&reports, &[0.03, 0.01, 0.001], true)).unwrap();
        let mut h_reversed = study_value.clone();
        h_reversed["levels"][1]["mesh_size_h_m"] = json!(0.5);
        let study = serde_json::to_vec(&h_reversed).unwrap();
        assert!(audit_mesh_refinement_study(&study, &reference, &reports).is_err());

        let mut dof_flat = study_value;
        dof_flat["levels"][1]["degrees_of_freedom"] = json!(100);
        let study = serde_json::to_vec(&dof_flat).unwrap();
        assert!(audit_mesh_refinement_study(&study, &reference, &reports).is_err());
    }
}
