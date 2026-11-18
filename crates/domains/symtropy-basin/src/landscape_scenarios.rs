// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Cross-repository landscape scenario contract.
//!
//! This module pins a scenario to exact design bytes and explicit boundary
//! conditions. It intentionally refuses to treat Sol-Atlas v0's
//! LOCAL_CARTESIAN_DRAWING_UNITS as physical space. Schema qualification and
//! synthetic benchmarks are allowed; analytical/scientific claims require a
//! future measured/geospatial document contract and validated model profile.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;

pub const SCENARIO_MANIFEST_SCHEMA: &str = "luminous.landscape-scenario-manifest.v0";
pub const DESIGN_SNAPSHOT_SCHEMA: &str = "luminous.landscape-design.v0";
pub const DESIGN_SNAPSHOT_SCHEMA_URI: &str =
    "https://luminousdynamics.io/schemas/landscape-design-v0.schema.json";
pub const EVALUATION_PLAN_SCHEMA: &str = "luminous.landscape-evaluation-plan.v0";
pub const EVALUATION_PLAN_SCHEMA_URI: &str =
    "https://luminousdynamics.io/schemas/landscape-evaluation-plan-v0.schema.json";
pub const LOCAL_DRAWING_FRAME: &str = "LOCAL_CARTESIAN_DRAWING_UNITS";
pub const SYNTHETIC_GEOMETRY_STATUS: &str = "synthetic-benchmark";
pub const SCHEMATIC_GEOMETRY_STATUS: &str = "schematic-design-intent";
pub const MAX_SCENARIO_FEATURES: usize = 100_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LandscapeRunPurpose {
    #[serde(rename = "schema-qualification")]
    SchemaQualification,
    #[serde(rename = "synthetic-benchmark")]
    SyntheticBenchmark,
    #[serde(rename = "analytical-reference")]
    AnalyticalReference,
    #[serde(rename = "scientific-simulation")]
    ScientificSimulation,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DesignSnapshotRef {
    /// SHA-256 of the exact snapshot byte sequence consumed by the scenario.
    pub sha256: String,
    pub schema: String,
    pub schema_uri: String,
    pub scope: String,
    pub geometry_status: String,
    pub coordinate_frame_kind: String,
    pub georeferenced: bool,
    pub feature_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LandscapeScenarioManifest {
    pub schema: String,
    pub scenario_id: String,
    pub design_snapshot: DesignSnapshotRef,
    /// Optional reference to the preregistered evaluation plan for this run.
    /// If present, the exact plan bytes are required for validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_plan: Option<EvaluationPlanRef>,
    pub model_id: String,
    pub model_version: String,
    pub purpose: LandscapeRunPurpose,
    pub seed: u64,
    /// SHA-256 of a canonical, versioned boundary-condition document.
    /// Use a canonical empty-boundary document when the purpose needs none.
    pub boundary_conditions_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvaluationPlanRef {
    /// SHA-256 of the exact evaluation-plan byte sequence.
    pub sha256: String,
    pub schema: String,
    pub schema_uri: String,
    pub evaluation_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractViolation {
    pub code: &'static str,
    pub path: &'static str,
    pub detail: String,
}

impl ContractViolation {
    fn new(code: &'static str, path: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            path,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}: {}", self.code, self.path, self.detail)
    }
}

impl std::error::Error for ContractViolation {}

/// Construct a reference from the exact bytes about to be simulated.
///
/// This only extracts metadata used by the scenario gate. It is not a
/// replacement for the authoritative source-document validator; Sol-Atlas
/// remains responsible for validating its full document schema before handoff.
pub fn design_snapshot_ref(bytes: &[u8]) -> Result<DesignSnapshotRef, ContractViolation> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ContractViolation::new(
            "snapshot-json-invalid",
            "$",
            format!("snapshot bytes are not valid JSON: {error}"),
        )
    })?;

    let string = |path: &'static str, key: &'static str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                ContractViolation::new("snapshot-field-invalid", path, "expected a string")
            })
    };

    let schema = string("$.schema", "schema")?;
    let schema_uri = string("$.schema_uri", "schema_uri")?;
    let scope = string("$.scope", "scope")?;
    let geometry_status = string("$.geometry_status", "geometry_status")?;

    let coordinate_frame = value.get("coordinate_frame").ok_or_else(|| {
        ContractViolation::new(
            "snapshot-coordinate-frame-missing",
            "$.coordinate_frame",
            "a declared coordinate frame is required",
        )
    })?;
    let coordinate_frame_kind = coordinate_frame
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ContractViolation::new(
                "snapshot-coordinate-frame-invalid",
                "$.coordinate_frame.kind",
                "expected a string",
            )
        })?
        .to_owned();
    let georeferenced = coordinate_frame
        .get("georeferenced")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            ContractViolation::new(
                "snapshot-coordinate-frame-invalid",
                "$.coordinate_frame.georeferenced",
                "expected a boolean",
            )
        })?;
    let feature_count = value
        .get("features")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| {
            ContractViolation::new(
                "snapshot-features-invalid",
                "$.features",
                "expected an array",
            )
        })?;

    Ok(DesignSnapshotRef {
        sha256: sha256_hex(bytes),
        schema,
        schema_uri,
        scope,
        geometry_status,
        coordinate_frame_kind,
        georeferenced,
        feature_count,
    })
}

impl LandscapeScenarioManifest {
    /// Validate the manifest against the exact bytes the solver will consume.
    ///
    /// This gate validates identity, source/schema agreement, digests, scale,
    /// input bounds and claim eligibility. It does not establish that a solver
    /// is scientifically valid or that its outputs match the physical world.
    pub fn validate_against_inputs(
        &self,
        snapshot_bytes: &[u8],
        boundary_conditions_bytes: &[u8],
    ) -> Result<(), Vec<ContractViolation>> {
        self.validate_with_evaluation_plan_bytes(snapshot_bytes, boundary_conditions_bytes, None)
    }

    /// Validate a scenario that pins an explicit evaluation plan.
    ///
    /// The evaluation plan must already have passed its authoritative
    /// Sol-Atlas validator. Symtropy verifies byte identity and that the
    /// plan points to this exact snapshot/scope; it does not duplicate the
    /// full evaluation-plan validator or infer causal validity.
    pub fn validate_against_evaluation_plan(
        &self,
        snapshot_bytes: &[u8],
        boundary_conditions_bytes: &[u8],
        evaluation_plan_bytes: &[u8],
    ) -> Result<(), Vec<ContractViolation>> {
        self.validate_with_evaluation_plan_bytes(
            snapshot_bytes,
            boundary_conditions_bytes,
            Some(evaluation_plan_bytes),
        )
    }

    fn validate_with_evaluation_plan_bytes(
        &self,
        snapshot_bytes: &[u8],
        boundary_conditions_bytes: &[u8],
        evaluation_plan_bytes: Option<&[u8]>,
    ) -> Result<(), Vec<ContractViolation>> {
        let mut violations = self
            .validate_snapshot_metadata(snapshot_bytes)
            .err()
            .unwrap_or_default();

        let actual_boundary_digest = sha256_hex(boundary_conditions_bytes);
        if actual_boundary_digest != self.boundary_conditions_sha256 {
            violations.push(ContractViolation::new(
                "boundary-digest-mismatch",
                "$.boundary_conditions_sha256",
                "the declared digest does not match the exact boundary-condition bytes",
            ));
        }

        match (&self.evaluation_plan, evaluation_plan_bytes) {
            (None, None) => {}
            (Some(_), None) => violations.push(ContractViolation::new(
                "evaluation-plan-bytes-required",
                "$.evaluation_plan",
                "manifest pins an evaluation plan but its exact bytes were not supplied",
            )),
            (None, Some(_)) => violations.push(ContractViolation::new(
                "evaluation-plan-reference-required",
                "$.evaluation_plan",
                "evaluation-plan bytes cannot be supplied unless the manifest pins their reference",
            )),
            (Some(reference), Some(bytes)) => {
                self.validate_evaluation_plan_bytes(reference, bytes, &mut violations)
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    fn validate_evaluation_plan_bytes(
        &self,
        reference: &EvaluationPlanRef,
        bytes: &[u8],
        violations: &mut Vec<ContractViolation>,
    ) {
        if sha256_hex(bytes) != reference.sha256 {
            violations.push(ContractViolation::new(
                "evaluation-plan-digest-mismatch",
                "$.evaluation_plan.sha256",
                "the declared evaluation-plan digest does not match the exact plan bytes",
            ));
        }

        let plan: Value = match serde_json::from_slice(bytes) {
            Ok(plan) => plan,
            Err(error) => {
                violations.push(ContractViolation::new(
                    "evaluation-plan-json-invalid",
                    "$.evaluation_plan",
                    format!("evaluation-plan bytes are not valid JSON: {error}"),
                ));
                return;
            }
        };

        if plan.get("schema").and_then(Value::as_str) != Some(EVALUATION_PLAN_SCHEMA) {
            violations.push(ContractViolation::new(
                "evaluation-plan-schema-unsupported",
                "$.evaluation_plan.schema",
                format!("expected {EVALUATION_PLAN_SCHEMA}"),
            ));
        }
        if plan.get("schema_uri").and_then(Value::as_str) != Some(EVALUATION_PLAN_SCHEMA_URI) {
            violations.push(ContractViolation::new(
                "evaluation-plan-schema-uri-unsupported",
                "$.evaluation_plan.schema_uri",
                format!("expected {EVALUATION_PLAN_SCHEMA_URI}"),
            ));
        }
        if plan.get("evaluation_id").and_then(Value::as_str)
            != Some(reference.evaluation_id.as_str())
        {
            violations.push(ContractViolation::new(
                "evaluation-plan-id-mismatch",
                "$.evaluation_plan.evaluation_id",
                "declared evaluation ID differs from the exact evaluation-plan bytes",
            ));
        }
        if plan.get("project_snapshot_sha256").and_then(Value::as_str)
            != Some(self.design_snapshot.sha256.as_str())
        {
            violations.push(ContractViolation::new(
                "evaluation-plan-snapshot-mismatch",
                "$.evaluation_plan.project_snapshot_sha256",
                "evaluation plan must pin the same exact design snapshot as the scenario",
            ));
        }
        if plan.get("project_scope").and_then(Value::as_str)
            != Some(self.design_snapshot.scope.as_str())
        {
            violations.push(ContractViolation::new(
                "evaluation-plan-scope-mismatch",
                "$.evaluation_plan.project_scope",
                "evaluation-plan scope must match the scenario's design snapshot scope",
            ));
        }
    }

    fn validate_snapshot_metadata(
        &self,
        snapshot_bytes: &[u8],
    ) -> Result<(), Vec<ContractViolation>> {
        let mut violations = Vec::new();

        if self.schema != SCENARIO_MANIFEST_SCHEMA {
            violations.push(ContractViolation::new(
                "scenario-schema-unsupported",
                "$.schema",
                format!("expected {SCENARIO_MANIFEST_SCHEMA}"),
            ));
        }
        if self.scenario_id.trim().is_empty() {
            violations.push(ContractViolation::new(
                "scenario-id-empty",
                "$.scenario_id",
                "scenario ID must not be blank",
            ));
        }
        if self.model_id.trim().is_empty() {
            violations.push(ContractViolation::new(
                "model-id-empty",
                "$.model_id",
                "model ID must not be blank",
            ));
        }
        if self.model_version.trim().is_empty() {
            violations.push(ContractViolation::new(
                "model-version-empty",
                "$.model_version",
                "model version must not be blank",
            ));
        }

        validate_sha256(
            &self.design_snapshot.sha256,
            "$.design_snapshot.sha256",
            &mut violations,
        );
        validate_sha256(
            &self.boundary_conditions_sha256,
            "$.boundary_conditions_sha256",
            &mut violations,
        );
        if let Some(reference) = &self.evaluation_plan {
            validate_sha256(
                &reference.sha256,
                "$.evaluation_plan.sha256",
                &mut violations,
            );
            if reference.schema != EVALUATION_PLAN_SCHEMA {
                violations.push(ContractViolation::new(
                    "evaluation-plan-schema-unsupported",
                    "$.evaluation_plan.schema",
                    format!("expected {EVALUATION_PLAN_SCHEMA}"),
                ));
            }
            if reference.schema_uri != EVALUATION_PLAN_SCHEMA_URI {
                violations.push(ContractViolation::new(
                    "evaluation-plan-schema-uri-unsupported",
                    "$.evaluation_plan.schema_uri",
                    format!("expected {EVALUATION_PLAN_SCHEMA_URI}"),
                ));
            }
            if !valid_evaluation_id(&reference.evaluation_id) {
                violations.push(ContractViolation::new(
                    "evaluation-plan-id-invalid",
                    "$.evaluation_plan.evaluation_id",
                    "expected eval- followed by 3..=61 lowercase letters, digits, or hyphens",
                ));
            }
        }

        match design_snapshot_ref(snapshot_bytes) {
            Err(error) => violations.push(error),
            Ok(actual) => {
                if actual.sha256 != self.design_snapshot.sha256 {
                    violations.push(ContractViolation::new(
                        "snapshot-digest-mismatch",
                        "$.design_snapshot.sha256",
                        "the declared digest does not match the exact input bytes",
                    ));
                }
                if actual != self.design_snapshot {
                    violations.push(ContractViolation::new(
                        "snapshot-reference-mismatch",
                        "$.design_snapshot",
                        "declared schema, scope, frame, status, or feature count differs from input",
                    ));
                }
                self.validate_claim_eligibility(&actual, &mut violations);
            }
        }

        if self.design_snapshot.feature_count > MAX_SCENARIO_FEATURES {
            violations.push(ContractViolation::new(
                "scenario-feature-limit-exceeded",
                "$.design_snapshot.feature_count",
                format!("v0 limit is {MAX_SCENARIO_FEATURES} features"),
            ));
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    fn validate_claim_eligibility(
        &self,
        snapshot: &DesignSnapshotRef,
        violations: &mut Vec<ContractViolation>,
    ) {
        if snapshot.schema != DESIGN_SNAPSHOT_SCHEMA
            || snapshot.schema_uri != DESIGN_SNAPSHOT_SCHEMA_URI
        {
            violations.push(ContractViolation::new(
                "snapshot-schema-unsupported",
                "$.design_snapshot.schema_uri",
                "Symtropy does not recognize this design snapshot schema",
            ));
        }

        if !matches!(
            snapshot.scope.as_str(),
            "site-parcel" | "district" | "watershed" | "bioregion"
        ) {
            violations.push(ContractViolation::new(
                "snapshot-scope-unsupported",
                "$.design_snapshot.scope",
                "scope must be site-parcel, district, watershed, or bioregion",
            ));
        }

        // The accepted v0 design schema only represents arbitrary drawing
        // units. A caller cannot change the frame string and thereby promote
        // this schema to measured coordinates.
        if snapshot.coordinate_frame_kind != LOCAL_DRAWING_FRAME || snapshot.georeferenced {
            violations.push(ContractViolation::new(
                "snapshot-frame-unsupported",
                "$.design_snapshot.coordinate_frame_kind",
                "v0 scenario handoff only accepts LOCAL_CARTESIAN_DRAWING_UNITS with georeferenced=false",
            ));
        }

        match self.purpose {
            LandscapeRunPurpose::SchemaQualification => {}
            LandscapeRunPurpose::SyntheticBenchmark => {
                if snapshot.geometry_status != SYNTHETIC_GEOMETRY_STATUS {
                    violations.push(ContractViolation::new(
                        "synthetic-evidence-required",
                        "$.design_snapshot.geometry_status",
                        "synthetic benchmark purpose requires synthetic-benchmark evidence status",
                    ));
                }
            }
            LandscapeRunPurpose::AnalyticalReference
            | LandscapeRunPurpose::ScientificSimulation => {
                violations.push(ContractViolation::new(
                    "physical-claim-blocked-by-schematic-frame",
                    "$.design_snapshot.coordinate_frame_kind",
                    "v0 symbol-anchor geometry is not measured spatial geometry; analytical/scientific landscape claims are blocked",
                ));
            }
        }

        if self.purpose == LandscapeRunPurpose::AnalyticalReference
            && snapshot.geometry_status == SYNTHETIC_GEOMETRY_STATUS
        {
            violations.push(ContractViolation::new(
                "synthetic-input-not-analytical-evidence",
                "$.purpose",
                "a synthetic scene cannot qualify as an independent analytical reference",
            ));
        }
    }
}

fn valid_evaluation_id(value: &str) -> bool {
    let Some(suffix) = value.strip_prefix("eval-") else {
        return false;
    };
    (3..=61).contains(&suffix.len())
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && suffix
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn validate_sha256(value: &str, path: &'static str, violations: &mut Vec<ContractViolation>) {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        violations.push(ContractViolation::new(
            "sha256-format-invalid",
            path,
            "expected 64 lowercase hexadecimal characters",
        ));
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snapshot(geometry_status: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema": DESIGN_SNAPSHOT_SCHEMA,
            "schema_uri": DESIGN_SNAPSHOT_SCHEMA_URI,
            "project_name": "Landscape fixture",
            "scope": "site-parcel",
            "coordinate_frame": {
                "kind": LOCAL_DRAWING_FRAME,
                "origin": "canvas top-left",
                "x_axis": "right",
                "y_axis": "down",
                "georeferenced": false
            },
            "geometry_status": geometry_status,
            "features": [{
                "id": "feature-0001",
                "kind": "tree",
                "label": "Canopy tree",
                "x_du": 10,
                "y_du": 20,
                "status": if geometry_status == SYNTHETIC_GEOMETRY_STATUS {
                    "synthetic-benchmark"
                } else {
                    "design-intent"
                },
                "source": if geometry_status == SYNTHETIC_GEOMETRY_STATUS {
                    "synthetic-benchmark"
                } else {
                    "user-authored-schematic"
                },
                "geometry_representation": "schematic-symbol-anchor-v0"
            }]
        }))
        .expect("snapshot fixture serializes")
    }

    fn manifest(bytes: &[u8], purpose: LandscapeRunPurpose) -> LandscapeScenarioManifest {
        LandscapeScenarioManifest {
            schema: SCENARIO_MANIFEST_SCHEMA.to_string(),
            scenario_id: "landscape-fixture-001".to_string(),
            design_snapshot: design_snapshot_ref(bytes).unwrap(),
            evaluation_plan: None,
            model_id: "contract-test".to_string(),
            model_version: "v0".to_string(),
            purpose,
            seed: 1,
            boundary_conditions_sha256: sha256_hex(b"boundary-conditions-v0-empty"),
        }
    }

    fn evaluation_plan_for_snapshot(snapshot_bytes: &[u8]) -> (EvaluationPlanRef, Vec<u8>) {
        let snapshot_ref = design_snapshot_ref(snapshot_bytes).unwrap();
        let plan = json!({
            "schema": EVALUATION_PLAN_SCHEMA,
            "schema_uri": EVALUATION_PLAN_SCHEMA_URI,
            "evaluation_id": "eval-landscape-site-v0",
            "project_snapshot_sha256": snapshot_ref.sha256,
            "created_at_unix_s": 1792454400_u64,
            "project_scope": snapshot_ref.scope,
            "plan_status": "draft",
            "study_design": "descriptive-monitoring",
            "analysis_intent": "descriptive",
            "time_windows": {
                "baseline": {"start_unix_s": 10_u64, "end_unix_s": 20_u64},
                "follow_up": {"start_unix_s": 20_u64, "end_unix_s": 30_u64}
            },
            "arms": [{
                "id": "arm-reference",
                "role": "reference",
                "site_ref_sha256": sha256_hex(b"opaque-reference-site"),
                "allocation_method": "observational"
            }],
            "outcomes": [{
                "metric_id": "heat.shade_fraction_at_declared_time",
                "unit": "percent_of_area",
                "method_id": "shade-method-v0",
                "method_uri": "https://example.org/methods/shade-v0",
                "sampling_design": "weekly",
                "minimum_observations_per_arm_window": 1_u64,
                "uncertainty_method": "Interval from repeat observations.",
                "desired_direction": "non-directional",
                "target_band": null
            }],
            "causal_analysis_plan": null,
            "privacy": {
                "location_sensitivity": "restricted",
                "publication_policy": "private",
                "consent_status": "pending",
                "consent_record_ref": null,
                "aggregation_policy": ""
            },
            "interpretation_limit": "validation-does-not-prove-impact-or-causality"
        });
        let bytes = serde_json::to_vec(&plan).unwrap();
        let reference = EvaluationPlanRef {
            sha256: sha256_hex(&bytes),
            schema: EVALUATION_PLAN_SCHEMA.to_string(),
            schema_uri: EVALUATION_PLAN_SCHEMA_URI.to_string(),
            evaluation_id: "eval-landscape-site-v0".to_string(),
        };
        (reference, bytes)
    }

    #[test]
    fn scenario_manifest_schema_matches_rust_contract() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../schemas/landscape-scenario-manifest-v0.schema.json"
        ))
        .expect("scenario manifest schema must be valid JSON");

        assert_eq!(
            schema["$id"],
            "https://luminousdynamics.io/schemas/landscape-scenario-manifest-v0.schema.json"
        );
        assert_eq!(
            schema["properties"]["schema"]["const"],
            SCENARIO_MANIFEST_SCHEMA
        );
        assert_eq!(
            schema["properties"]["design_snapshot"]["properties"]["schema"]["const"],
            DESIGN_SNAPSHOT_SCHEMA
        );
        assert_eq!(
            schema["properties"]["design_snapshot"]["properties"]["schema_uri"]["const"],
            DESIGN_SNAPSHOT_SCHEMA_URI
        );
        assert_eq!(
            schema["properties"]["design_snapshot"]["properties"]["coordinate_frame_kind"]["const"],
            LOCAL_DRAWING_FRAME
        );
        assert_eq!(
            schema["$defs"]["evaluationPlanRef"]["properties"]["schema"]["const"],
            EVALUATION_PLAN_SCHEMA
        );
        assert_eq!(
            schema["$defs"]["evaluationPlanRef"]["properties"]["schema_uri"]["const"],
            EVALUATION_PLAN_SCHEMA_URI
        );
        assert!(
            schema["properties"]["evaluation_plan"]["oneOf"]
                .as_array()
                .unwrap()
                .len()
                == 2
        );
        for purpose in ["schema-qualification", "synthetic-benchmark"] {
            assert!(
                schema["properties"]["purpose"]["enum"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value.as_str() == Some(purpose))
            );
        }
    }

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn evaluation_plan_reference_binds_exact_plan_and_snapshot() {
        let snapshot_bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let (reference, plan_bytes) = evaluation_plan_for_snapshot(&snapshot_bytes);
        let mut manifest = manifest(&snapshot_bytes, LandscapeRunPurpose::SchemaQualification);
        manifest.evaluation_plan = Some(reference);

        assert_eq!(
            manifest.validate_against_evaluation_plan(
                &snapshot_bytes,
                b"boundary-conditions-v0-empty",
                &plan_bytes
            ),
            Ok(())
        );
    }

    #[test]
    fn evaluation_plan_reference_requires_exact_plan_bytes() {
        let snapshot_bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let (reference, _plan_bytes) = evaluation_plan_for_snapshot(&snapshot_bytes);
        let mut manifest = manifest(&snapshot_bytes, LandscapeRunPurpose::SchemaQualification);
        manifest.evaluation_plan = Some(reference);

        let errors = manifest
            .validate_against_inputs(&snapshot_bytes, b"boundary-conditions-v0-empty")
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "evaluation-plan-bytes-required")
        );
    }

    #[test]
    fn evaluation_plan_digest_and_id_must_match_exact_bytes() {
        let snapshot_bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let (reference, plan_bytes) = evaluation_plan_for_snapshot(&snapshot_bytes);
        let mut changed_plan: Value = serde_json::from_slice(&plan_bytes).unwrap();
        changed_plan["evaluation_id"] = json!("eval-mutated-plan-v0");
        let changed_bytes = serde_json::to_vec(&changed_plan).unwrap();
        let mut manifest = manifest(&snapshot_bytes, LandscapeRunPurpose::SchemaQualification);
        manifest.evaluation_plan = Some(reference);

        let errors = manifest
            .validate_against_evaluation_plan(
                &snapshot_bytes,
                b"boundary-conditions-v0-empty",
                &changed_bytes,
            )
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "evaluation-plan-digest-mismatch")
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == "evaluation-plan-id-mismatch")
        );
    }

    #[test]
    fn evaluation_plan_must_pin_same_snapshot_and_scope() {
        let snapshot_bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let (mut reference, plan_bytes) = evaluation_plan_for_snapshot(&snapshot_bytes);
        let mut changed_plan: Value = serde_json::from_slice(&plan_bytes).unwrap();
        changed_plan["project_snapshot_sha256"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        changed_plan["project_scope"] = json!("watershed");
        let changed_bytes = serde_json::to_vec(&changed_plan).unwrap();
        reference.sha256 = sha256_hex(&changed_bytes);
        let mut manifest = manifest(&snapshot_bytes, LandscapeRunPurpose::SchemaQualification);
        manifest.evaluation_plan = Some(reference);

        let errors = manifest
            .validate_against_evaluation_plan(
                &snapshot_bytes,
                b"boundary-conditions-v0-empty",
                &changed_bytes,
            )
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "evaluation-plan-snapshot-mismatch")
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == "evaluation-plan-scope-mismatch")
        );
    }

    #[test]
    fn evaluation_plan_bytes_without_a_pinned_reference_are_rejected() {
        let snapshot_bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let (_, plan_bytes) = evaluation_plan_for_snapshot(&snapshot_bytes);
        let manifest = manifest(&snapshot_bytes, LandscapeRunPurpose::SchemaQualification);
        let errors = manifest
            .validate_against_evaluation_plan(
                &snapshot_bytes,
                b"boundary-conditions-v0-empty",
                &plan_bytes,
            )
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "evaluation-plan-reference-required")
        );
    }

    #[test]
    fn schema_qualification_accepts_a_pinned_local_schematic() {
        let bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let manifest = manifest(&bytes, LandscapeRunPurpose::SchemaQualification);
        assert_eq!(
            manifest.validate_against_inputs(&bytes, b"boundary-conditions-v0-empty"),
            Ok(())
        );
    }

    #[test]
    fn synthetic_benchmark_requires_synthetic_source_status_and_exact_digest() {
        let bytes = snapshot(SYNTHETIC_GEOMETRY_STATUS);
        let manifest = manifest(&bytes, LandscapeRunPurpose::SyntheticBenchmark);
        assert_eq!(
            manifest.validate_against_inputs(&bytes, b"boundary-conditions-v0-empty"),
            Ok(())
        );

        let changed = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let errors = manifest
            .validate_against_inputs(&changed, b"boundary-conditions-v0-empty")
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "snapshot-digest-mismatch")
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == "synthetic-evidence-required")
        );
    }

    #[test]
    fn scientific_claim_is_blocked_for_v0_local_drawing_units() {
        let bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let manifest = manifest(&bytes, LandscapeRunPurpose::ScientificSimulation);
        let errors = manifest
            .validate_against_inputs(&bytes, b"boundary-conditions-v0-empty")
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| { error.code == "physical-claim-blocked-by-schematic-frame" })
        );
    }

    #[test]
    fn analytical_reference_rejects_synthetic_input() {
        let bytes = snapshot(SYNTHETIC_GEOMETRY_STATUS);
        let manifest = manifest(&bytes, LandscapeRunPurpose::AnalyticalReference);
        let errors = manifest
            .validate_against_inputs(&bytes, b"boundary-conditions-v0-empty")
            .unwrap_err();
        assert!(errors.iter().any(|error| {
            error.code == "physical-claim-blocked-by-schematic-frame"
                || error.code == "synthetic-input-not-analytical-evidence"
        }));
    }

    #[test]
    fn wrong_metadata_and_bad_boundary_digest_fail_closed() {
        let bytes = snapshot(SCHEMATIC_GEOMETRY_STATUS);
        let mut manifest = manifest(&bytes, LandscapeRunPurpose::SchemaQualification);
        manifest.design_snapshot.scope = "watershed".to_string();
        manifest.boundary_conditions_sha256 = "not-a-digest".to_string();

        let errors = manifest
            .validate_against_inputs(&bytes, b"boundary-conditions-v0-empty")
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "snapshot-reference-mismatch")
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == "sha256-format-invalid")
        );
        assert!(
            errors
                .iter()
                .any(|error| error.code == "boundary-digest-mismatch")
        );
    }

    #[test]
    fn manifest_serialization_is_deterministic() {
        let bytes = snapshot(SYNTHETIC_GEOMETRY_STATUS);
        let manifest = manifest(&bytes, LandscapeRunPurpose::SyntheticBenchmark);
        let first = serde_json::to_vec(&manifest).unwrap();
        let second = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(first, second);
        let parsed: LandscapeScenarioManifest = serde_json::from_slice(&first).unwrap();
        assert_eq!(parsed, manifest);
    }
}
