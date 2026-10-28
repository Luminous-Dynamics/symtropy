// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Narrow executable checker for the v0.1 synthetic disruption-rehearsal contract.
//!
//! This module evaluates declared abstract input states only. It does not run
//! maritime physics, simulate a vessel, assess safety, or grant authority.

use serde::Serialize;
use std::env;
use std::fs;

const MAX_SCENARIO_BYTES: usize = 1_000_000;
const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "fixture_id",
    "fixture_status",
    "source_kind",
    "claim_ceiling",
    "description",
    "assessment",
    "model_scope",
    "candidate_plans",
    "hard_constraints",
    "evidence_requirements",
    "scenario_cases",
    "expected_global_properties",
    "execution",
];
const AUTHORITY_KEYS: &[&str] = &[
    "engineering_admission",
    "navigation_authorized",
    "physical_truth_claim_authorized",
    "berth_availability_inference_authorized",
    "reservation_or_custody_event_authorized",
    "physical_execution_authorized",
];
const INPUT_KEYS: &[&str] = &[
    "water_level_envelope_state",
    "energy_reserve_state",
    "terminal_window_state",
    "forecast_evidence_state",
    "waiting_resource_state",
    "alternate_capacity_state",
];
/// Verbose display names declared in `model_scope.abstract_inputs_only` — these are the
/// human-readable labels for the same inputs tracked by `INPUT_KEYS` in scenario cases.
const MODEL_SCOPE_INPUT_NAMES: &[&str] = &[
    "declared_water_level_envelope_state",
    "energy_reserve_constraint_state",
    "terminal_window_state",
    "forecast_evidence_state",
    "waiting_resource_availability_state",
    "alternate_capacity_confirmation_state",
];
const PLAN_IDS: &[&str] = &[
    "continue_original_slot",
    "hold_and_reassess",
    "prepare_alternate_terminal",
];
const CASE_IDS: &[&str] = &[
    "baseline_declared_inputs",
    "adverse_water_level_envelope",
    "shore_power_unavailable",
    "forecast_invalidated_before_decision",
    "common_cause_terminal_and_power_disruption",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
struct DispositionReport {
    case_id: String,
    plan_id: String,
    status: String,
    blockers: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct EvidenceRequirementSpec {
    requirement_id: &'static str,
    blocker_code: &'static str,
    trigger_blocker: &'static str,
    required_evidence_kind: &'static str,
    minimum_fields: &'static [&'static str],
    resolution_scope: &'static str,
}

const EVIDENCE_REQUIREMENTS: &[EvidenceRequirementSpec] = &[
    EvidenceRequirementSpec {
        requirement_id: "terminal_window_confirmation",
        blocker_code: "terminal_window_constraint",
        trigger_blocker: "terminal_window_open",
        required_evidence_kind: "authoritative_terminal_window_state",
        minimum_fields: &[
            "terminal_identity",
            "window_state",
            "window_start_utc",
            "window_end_utc",
            "issuer_identity",
            "source_revision",
            "captured_at_utc",
            "artifact_sha256",
        ],
        resolution_scope: "Can resolve only the declared terminal-window evidence gap; it does not create a booking or authorize vessel movement.",
    },
    EvidenceRequirementSpec {
        requirement_id: "water_level_envelope_evidence",
        blocker_code: "water_level_envelope_constraint",
        trigger_blocker: "water_level_envelope_supported_and_satisfied",
        required_evidence_kind: "supported_water_level_envelope",
        minimum_fields: &[
            "spatial_coverage",
            "time_coverage",
            "vertical_datum",
            "water_level_envelope",
            "uncertainty_statement",
            "source_receipt_sha256",
            "supported_model_capability_id",
        ],
        resolution_scope: "Can establish whether an input envelope is available within a declared model path; it does not calculate or certify under-keel clearance.",
    },
    EvidenceRequirementSpec {
        requirement_id: "energy_reserve_assessment",
        blocker_code: "energy_reserve_constraint",
        trigger_blocker: "energy_reserve_minimum_satisfied",
        required_evidence_kind: "qualified_energy_reserve_assessment",
        minimum_fields: &[
            "declared_load_profile",
            "usable_energy",
            "reserve_policy_id",
            "temperature_and_age_derating",
            "shore_power_state",
            "model_capability_id",
            "validation_reference",
        ],
        resolution_scope: "Can support a bounded energy feasibility check only if the model path is admitted; no assumption may replace absent loads or derating.",
    },
    EvidenceRequirementSpec {
        requirement_id: "forecast_temporal_binding",
        blocker_code: "forecast_temporal_applicability",
        trigger_blocker: "forecast_temporal_applicability",
        required_evidence_kind: "byte_bound_forecast_timing_evidence",
        minimum_fields: &[
            "receipt_sha256",
            "artifact_sha256",
            "product_id",
            "dataset_id",
            "dataset_revision",
            "model_initialization_time_utc",
            "issued_at_utc",
            "valid_from_utc",
            "valid_until_utc",
            "assessment_time_utc",
            "target_time_utc",
            "policy_id",
        ],
        resolution_scope: "Can resolve temporal applicability for the pinned revision only; it does not validate forecast skill or authorize navigation.",
    },
    EvidenceRequirementSpec {
        requirement_id: "waiting_resource_availability",
        blocker_code: "waiting_resource_availability",
        trigger_blocker: "designated_waiting_resource_declared_available",
        required_evidence_kind: "waiting_resource_and_dependency_state",
        minimum_fields: &[
            "resource_identity",
            "available_capacity",
            "valid_from_utc",
            "valid_until_utc",
            "shared_dependency_ids",
            "independence_evidence",
            "issuer_identity",
            "source_revision",
        ],
        resolution_scope: "Can resolve the declared waiting-resource state only; claimed redundancy is not presumed independent without evidence.",
    },
    EvidenceRequirementSpec {
        requirement_id: "alternate_terminal_capacity",
        blocker_code: "alternate_terminal_capacity",
        trigger_blocker: "alternate_capacity_confirmed",
        required_evidence_kind: "authoritative_alternate_terminal_capacity_response",
        minimum_fields: &[
            "terminal_identity",
            "capacity_or_slot_state",
            "requested_window_utc",
            "response_window_utc",
            "issuer_identity",
            "source_revision",
            "captured_at_utc",
            "artifact_sha256",
        ],
        resolution_scope: "Can resolve capacity evidence for reassessment; it does not create a reservation, guarantee acceptance, or authorize execution.",
    },
    EvidenceRequirementSpec {
        requirement_id: "holding_energy_model_support",
        blocker_code: "holding_energy_model_unsupported",
        trigger_blocker: "declared energy behaviour while holding is outside the model scope",
        required_evidence_kind: "supported_holding_energy_model",
        minimum_fields: &[
            "holding_duration_envelope",
            "auxiliary_load_profile",
            "usable_energy",
            "reserve_policy_id",
            "temperature_and_age_derating",
            "model_capability_id",
            "validation_reference",
        ],
        resolution_scope: "Requires an explicitly supported energy model before comparing hold duration; until then the result remains unsupported.",
    },
    EvidenceRequirementSpec {
        requirement_id: "alternate_terminal_model_support",
        blocker_code: "alternate_terminal_model_unsupported",
        trigger_blocker: "alternate-terminal feasibility model not admitted",
        required_evidence_kind: "admitted_alternate_terminal_feasibility_model",
        minimum_fields: &[
            "terminal_identity",
            "declared_constraint_set",
            "input_semantics_and_units",
            "model_capability_id",
            "validation_reference",
            "model_revision",
            "source_data_receipts",
        ],
        resolution_scope: "Requires a separately admitted, validated model path; a capacity confirmation alone cannot establish alternative feasibility.",
    },
];

fn evidence_requirement_for_blocker(blocker: &str) -> Option<&'static EvidenceRequirementSpec> {
    EVIDENCE_REQUIREMENTS
        .iter()
        .find(|requirement| requirement.trigger_blocker == blocker)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct EvidenceGapReport {
    case_id: String,
    plan_id: String,
    blocker_code: String,
    blocker_reason: String,
    requirement_id: String,
    required_evidence_kind: String,
    minimum_fields: Vec<String>,
    resolution_scope: String,
}

fn evidence_gap(case_id: &str, plan_id: &str, blocker: &str) -> Result<EvidenceGapReport, String> {
    let requirement = evidence_requirement_for_blocker(blocker)
        .ok_or_else(|| format!("no evidence requirement registered for blocker {blocker:?}"))?;
    Ok(EvidenceGapReport {
        case_id: case_id.to_owned(),
        plan_id: plan_id.to_owned(),
        blocker_code: requirement.blocker_code.to_owned(),
        blocker_reason: blocker.to_owned(),
        requirement_id: requirement.requirement_id.to_owned(),
        required_evidence_kind: requirement.required_evidence_kind.to_owned(),
        minimum_fields: requirement
            .minimum_fields
            .iter()
            .map(|field| (*field).to_owned())
            .collect(),
        resolution_scope: requirement.resolution_scope.to_owned(),
    })
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct RehearsalReport {
    status: &'static str,
    schema_id: String,
    fixture_id: String,
    scenario_sha256: String,
    scenario_case_count: usize,
    candidate_plan_count: usize,
    dispositions: Vec<DispositionReport>,
    evidence_gaps: Vec<EvidenceGapReport>,
    robustly_feasible_candidate_plans: Vec<String>,
    engineering_admission: &'static str,
    navigation_authorized: bool,
    physical_truth_claim_authorized: bool,
    berth_availability_inference_authorized: bool,
    reservation_or_custody_event_authorized: bool,
    physical_execution_authorized: bool,
    source_commit_binding: String,
    contract_evaluation_performed: bool,
    physical_simulation_performed: bool,
}

#[derive(Debug)]
struct AbstractInputs {
    water: String,
    energy: String,
    terminal: String,
    forecast: String,
    waiting: String,
    alternate_capacity: String,
}

fn text_field(value: &super::JsonValue, key: &str, path: &str) -> Result<String, String> {
    Ok(
        super::require_nonempty_text(super::field(value, key, path)?, &format!("{path}.{key}"))?
            .to_owned(),
    )
}

fn expect_text_array(
    value: &super::JsonValue,
    path: &str,
    expected: &[&str],
) -> Result<(), String> {
    let actual = super::array(value, path)?;
    if actual.len() != expected.len() {
        return Err(format!(
            "{path}: expected {} entries, found {}",
            expected.len(),
            actual.len()
        ));
    }
    for (index, (value, expected_text)) in actual.iter().zip(expected).enumerate() {
        super::expect_text(value, &format!("{path}[{index}]"), expected_text)?;
    }
    Ok(())
}

fn canonical_utc_seconds(value: &str, path: &str) -> Result<i64, String> {
    if value.len() != 20 || !super::valid_utc_timestamp(value) {
        return Err(format!("{path}: expected canonical YYYY-MM-DDTHH:MM:SSZ"));
    }
    let parse = |start: usize, end: usize| -> Result<i64, String> {
        value
            .get(start..end)
            .ok_or_else(|| format!("{path}: truncated timestamp"))?
            .parse::<i64>()
            .map_err(|_| format!("{path}: invalid timestamp digits"))
    };
    let year = parse(0, 4)?;
    let month = parse(5, 7)?;
    let day = parse(8, 10)?;
    let hour = parse(11, 13)?;
    let minute = parse(14, 16)?;
    let second = parse(17, 19)?;
    if year == 0 {
        return Err(format!("{path}: year zero is invalid"));
    }
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = era * 146_097 + day_of_era - 719_468;
    Ok(days_since_epoch * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn validate_static_contract(root: &super::JsonValue) -> Result<(), String> {
    let assessment = super::field(root, "assessment", "rehearsal")?;
    super::exact_keys(
        assessment,
        &[
            "assessment_time_utc",
            "target_time_utc",
            "time_policy_id",
            "forecast_input",
        ],
        "rehearsal.assessment",
    )?;
    for key in ["assessment_time_utc", "target_time_utc"] {
        let value = text_field(assessment, key, "rehearsal.assessment")?;
        if !super::valid_utc_timestamp(&value) || value.len() != 20 {
            return Err(format!(
                "rehearsal.assessment.{key}: expected canonical whole-second UTC timestamp"
            ));
        }
    }
    if text_field(assessment, "time_policy_id", "rehearsal.assessment")? != "synthetic-policy-v0.1"
    {
        return Err("rehearsal.assessment.time_policy_id is not the pinned v0.1 policy".to_owned());
    }
    let forecast = super::field(assessment, "forecast_input", "rehearsal.assessment")?;
    super::exact_keys(
        forecast,
        &[
            "fixture_reference",
            "source_artifact_sha256",
            "receipt_sha256",
            "binding_state",
            "issued_at_utc",
            "model_initialization_time_utc",
            "valid_from_utc",
            "valid_until_utc",
            "max_issue_age_seconds",
            "max_forecast_lead_seconds",
            "expected_temporal_arithmetic_for_test_design_only",
            "execution_status",
            "interpretation",
        ],
        "rehearsal.assessment.forecast_input",
    )?;
    for key in ["source_artifact_sha256", "receipt_sha256"] {
        if !matches!(
            super::field(forecast, key, "forecast_input")?,
            super::JsonValue::Null
        ) {
            return Err(format!(
                "forecast_input.{key} must remain null until exact real bytes are bound"
            ));
        }
    }
    super::expect_text(
        super::field(forecast, "binding_state", "forecast_input")?,
        "forecast_input.binding_state",
        "not_bound_in_this_design_fixture",
    )?;
    super::expect_text(
        super::field(forecast, "execution_status", "forecast_input")?,
        "forecast_input.execution_status",
        "not_executed",
    )?;
    for key in [
        "issued_at_utc",
        "model_initialization_time_utc",
        "valid_from_utc",
        "valid_until_utc",
    ] {
        let value = text_field(forecast, key, "forecast_input")?;
        if !super::valid_utc_timestamp(&value) || value.len() != 20 {
            return Err(format!(
                "forecast_input.{key}: expected canonical whole-second UTC timestamp"
            ));
        }
    }
    for key in ["max_issue_age_seconds", "max_forecast_lead_seconds"] {
        if super::integer(
            super::field(forecast, key, "forecast_input")?,
            &format!("forecast_input.{key}"),
        )? == 0
        {
            return Err(format!("forecast_input.{key} must be positive"));
        }
    }
    let arithmetic = super::field(
        forecast,
        "expected_temporal_arithmetic_for_test_design_only",
        "forecast_input",
    )?;
    super::exact_keys(
        arithmetic,
        &["issue_age_seconds", "forecast_lead_seconds"],
        "forecast_input.expected_temporal_arithmetic_for_test_design_only",
    )?;
    for key in ["issue_age_seconds", "forecast_lead_seconds"] {
        if super::integer(
            super::field(arithmetic, key, "forecast arithmetic")?,
            &format!("forecast arithmetic.{key}"),
        )? == 0
        {
            return Err(format!("forecast arithmetic.{key} must be positive"));
        }
    }
    let assessment_seconds = canonical_utc_seconds(
        &text_field(assessment, "assessment_time_utc", "rehearsal.assessment")?,
        "rehearsal.assessment.assessment_time_utc",
    )?;
    let target_seconds = canonical_utc_seconds(
        &text_field(assessment, "target_time_utc", "rehearsal.assessment")?,
        "rehearsal.assessment.target_time_utc",
    )?;
    let issued_seconds = canonical_utc_seconds(
        &text_field(forecast, "issued_at_utc", "forecast_input")?,
        "forecast_input.issued_at_utc",
    )?;
    let initialized_seconds = canonical_utc_seconds(
        &text_field(forecast, "model_initialization_time_utc", "forecast_input")?,
        "forecast_input.model_initialization_time_utc",
    )?;
    let valid_from_seconds = canonical_utc_seconds(
        &text_field(forecast, "valid_from_utc", "forecast_input")?,
        "forecast_input.valid_from_utc",
    )?;
    let valid_until_seconds = canonical_utc_seconds(
        &text_field(forecast, "valid_until_utc", "forecast_input")?,
        "forecast_input.valid_until_utc",
    )?;
    if initialized_seconds > issued_seconds
        || issued_seconds > assessment_seconds
        || assessment_seconds > target_seconds
    {
        return Err(
            "forecast chronology invalid: require initialization <= issue <= assessment <= target"
                .to_owned(),
        );
    }
    if valid_from_seconds > target_seconds || target_seconds > valid_until_seconds {
        return Err("forecast target time is outside the declared valid interval".to_owned());
    }
    let observed_issue_age = (assessment_seconds - issued_seconds) as u64;
    let observed_lead = (target_seconds - initialized_seconds) as u64;
    let declared_issue_age = super::integer(
        super::field(arithmetic, "issue_age_seconds", "forecast arithmetic")?,
        "forecast arithmetic.issue_age_seconds",
    )?;
    let declared_lead = super::integer(
        super::field(arithmetic, "forecast_lead_seconds", "forecast arithmetic")?,
        "forecast arithmetic.forecast_lead_seconds",
    )?;
    if observed_issue_age != declared_issue_age || observed_lead != declared_lead {
        return Err(format!(
            "forecast temporal arithmetic mismatch: issue_age observed={observed_issue_age} \
             declared={declared_issue_age}; lead observed={observed_lead} \
             declared={declared_lead}"
        ));
    }
    let max_issue_age = super::integer(
        super::field(forecast, "max_issue_age_seconds", "forecast_input")?,
        "forecast_input.max_issue_age_seconds",
    )?;
    let max_lead = super::integer(
        super::field(forecast, "max_forecast_lead_seconds", "forecast_input")?,
        "forecast_input.max_forecast_lead_seconds",
    )?;
    if observed_issue_age > max_issue_age || observed_lead > max_lead {
        return Err(
            "forecast temporal arithmetic exceeds the declared synthetic policy bounds".to_owned(),
        );
    }

    let scope = super::field(root, "model_scope", "rehearsal")?;
    super::exact_keys(
        scope,
        &[
            "execution_profile",
            "physics_claims",
            "abstract_inputs_only",
            "unsupported_if_requested",
            "missing_or_unsupported_is_never_zero",
        ],
        "rehearsal.model_scope",
    )?;
    super::expect_text(
        super::field(scope, "execution_profile", "model_scope")?,
        "model_scope.execution_profile",
        "EngineeringDeterministic",
    )?;
    if !super::array(
        super::field(scope, "physics_claims", "model_scope")?,
        "model_scope.physics_claims",
    )?
    .is_empty()
    {
        return Err(
            "model_scope.physics_claims must stay empty in this abstract evaluator".to_owned(),
        );
    }
    expect_text_array(
        super::field(scope, "abstract_inputs_only", "model_scope")?,
        "model_scope.abstract_inputs_only",
        MODEL_SCOPE_INPUT_NAMES,
    )?;
    expect_text_array(
        super::field(scope, "unsupported_if_requested", "model_scope")?,
        "model_scope.unsupported_if_requested",
        &[
            "hydrodynamics",
            "under_keel_clearance",
            "vessel maneuvering",
            "mooring loads",
            "shore-power electrical dynamics",
            "terminal capacity derived from real operations",
        ],
    )?;
    if !super::bool_value(
        super::field(scope, "missing_or_unsupported_is_never_zero", "model_scope")?,
        "model_scope.missing_or_unsupported_is_never_zero",
    )? {
        return Err("missing_or_unsupported_is_never_zero must remain true".to_owned());
    }

    let evidence_requirements = super::array(
        super::field(root, "evidence_requirements", "rehearsal")?,
        "rehearsal.evidence_requirements",
    )?;
    if evidence_requirements.len() != EVIDENCE_REQUIREMENTS.len() {
        return Err(format!(
            "expected {} evidence requirements, found {}",
            EVIDENCE_REQUIREMENTS.len(),
            evidence_requirements.len()
        ));
    }
    for (index, (value, expected)) in evidence_requirements
        .iter()
        .zip(EVIDENCE_REQUIREMENTS)
        .enumerate()
    {
        let path = format!("rehearsal.evidence_requirements[{index}]");
        super::exact_keys(
            value,
            &[
                "requirement_id",
                "blocker_code",
                "trigger_blocker",
                "required_evidence_kind",
                "minimum_fields",
                "resolution_scope",
            ],
            &path,
        )?;
        super::expect_text(
            super::field(value, "requirement_id", &path)?,
            &format!("{path}.requirement_id"),
            expected.requirement_id,
        )?;
        super::expect_text(
            super::field(value, "blocker_code", &path)?,
            &format!("{path}.blocker_code"),
            expected.blocker_code,
        )?;
        super::expect_text(
            super::field(value, "trigger_blocker", &path)?,
            &format!("{path}.trigger_blocker"),
            expected.trigger_blocker,
        )?;
        super::expect_text(
            super::field(value, "required_evidence_kind", &path)?,
            &format!("{path}.required_evidence_kind"),
            expected.required_evidence_kind,
        )?;
        expect_text_array(
            super::field(value, "minimum_fields", &path)?,
            &format!("{path}.minimum_fields"),
            expected.minimum_fields,
        )?;
        super::expect_text(
            super::field(value, "resolution_scope", &path)?,
            &format!("{path}.resolution_scope"),
            expected.resolution_scope,
        )?;
    }

    let constraints = super::array(
        super::field(root, "hard_constraints", "rehearsal")?,
        "rehearsal.hard_constraints",
    )?;
    const IDS: [&str; 6] = [
        "water_level_envelope_supported_and_satisfied",
        "energy_reserve_minimum_satisfied",
        "terminal_window_open",
        "forecast_temporal_applicability",
        "designated_waiting_resource_declared_available",
        "alternate_capacity_confirmed",
    ];
    if constraints.len() != IDS.len() {
        return Err(format!(
            "expected {} v0.1 constraints, found {}",
            IDS.len(),
            constraints.len()
        ));
    }
    for (index, (constraint, id)) in constraints.iter().zip(IDS).enumerate() {
        let path = format!("rehearsal.hard_constraints[{index}]");
        let actual_id = text_field(constraint, "constraint_id", &path)?;
        if actual_id != id {
            return Err(format!(
                "{path}.constraint_id: unexpected or reordered ID {actual_id:?}"
            ));
        }
        let status_key = match id {
            "water_level_envelope_supported_and_satisfied"
            | "terminal_window_open"
            | "designated_waiting_resource_declared_available" => "missing_state",
            "energy_reserve_minimum_satisfied" => "unsupported_state",
            "forecast_temporal_applicability" => "invalid_or_missing_state",
            "alternate_capacity_confirmed" => "default_state",
            _ => unreachable!(),
        };
        let keys: &[&str] = match status_key {
            "missing_state" => &["constraint_id", "kind", "unit", "source", "missing_state"],
            "unsupported_state" => &[
                "constraint_id",
                "kind",
                "unit",
                "source",
                "unsupported_state",
            ],
            "invalid_or_missing_state" => &[
                "constraint_id",
                "kind",
                "unit",
                "source",
                "invalid_or_missing_state",
            ],
            "default_state" => &["constraint_id", "kind", "unit", "source", "default_state"],
            _ => return Err(format!("{path}: unsupported constraint status field")),
        };
        super::exact_keys(constraint, keys, &path)?;
        let expected_kind = if id == "forecast_temporal_applicability" {
            "evidence_gate"
        } else {
            "hard"
        };
        super::expect_text(
            super::field(constraint, "kind", &path)?,
            &format!("{path}.kind"),
            expected_kind,
        )?;
        let expected_status = if id == "energy_reserve_minimum_satisfied" {
            "UNSUPPORTED"
        } else {
            "INDETERMINATE"
        };
        super::expect_text(
            super::field(constraint, status_key, &path)?,
            &format!("{path}.{status_key}"),
            expected_status,
        )?;
        let unit = super::field(constraint, "unit", &path)?;
        if id == "forecast_temporal_applicability" {
            super::expect_text(unit, &format!("{path}.unit"), "seconds")?;
        } else if !matches!(unit, super::JsonValue::Null) {
            return Err(format!(
                "{path}.unit must remain null for abstract state constraints"
            ));
        }
        text_field(constraint, "source", &path)?;
    }

    expect_text_array(
        super::field(root, "expected_global_properties", "rehearsal")?,
        "rehearsal.expected_global_properties",
        &[
            "No hard constraint can be compensated by a lower delay or cost metric.",
            "A missing, stale, conflicting or invalidated input cannot improve a dependent result.",
            "Adding a required adverse case cannot enlarge the robustly-feasible candidate set.",
            "Duplicating correlated evidence cannot be counted as independent corroboration.",
            "An alternate-terminal preparation never creates a reservation or capacity-confirmation event.",
            "Every result remains NotAdmitted for navigation, physical truth, berth availability and physical execution.",
            "No numerical value in this fixture represents a real physical quantity or real-world estimate.",
        ],
    )?;
    Ok(())
}

fn validate_abstract_inputs(
    value: &super::JsonValue,
    path: &str,
) -> Result<AbstractInputs, String> {
    super::exact_keys(value, INPUT_KEYS, path)?;
    let input = AbstractInputs {
        water: text_field(value, "water_level_envelope_state", path)?,
        energy: text_field(value, "energy_reserve_state", path)?,
        terminal: text_field(value, "terminal_window_state", path)?,
        forecast: text_field(value, "forecast_evidence_state", path)?,
        waiting: text_field(value, "waiting_resource_state", path)?,
        alternate_capacity: text_field(value, "alternate_capacity_state", path)?,
    };

    if !["within_declared_limit", "outside_declared_limit", "unknown"]
        .contains(&input.water.as_str())
        || !["adequate", "inadequate_for_declared_plan"].contains(&input.energy.as_str())
        || !["open", "closed"].contains(&input.terminal.as_str())
        || !["temporally_applicable_not_admitted", "invalidated"].contains(&input.forecast.as_str())
        || !["declared_available", "unknown_due_to_shared_dependency"]
            .contains(&input.waiting.as_str())
        || !["unconfirmed", "confirmed"].contains(&input.alternate_capacity.as_str())
    {
        return Err(format!(
            "{path}: unknown abstract input state; refusing to guess"
        ));
    }
    Ok(input)
}

fn disposition(
    plan_id: &str,
    input: &AbstractInputs,
) -> Result<(&'static str, Vec<&'static str>), String> {
    match plan_id {
        "continue_original_slot" => {
            // A known hard-constraint violation takes precedence over missing
            // evidence because this candidate is already infeasible in the case.
            if input.terminal == "closed" {
                Ok(("INFEASIBLE", vec!["terminal_window_open"]))
            } else if input.water == "outside_declared_limit" {
                Ok((
                    "INFEASIBLE",
                    vec!["water_level_envelope_supported_and_satisfied"],
                ))
            } else if input.energy == "inadequate_for_declared_plan" {
                Ok(("INFEASIBLE", vec!["energy_reserve_minimum_satisfied"]))
            } else if input.forecast != "temporally_applicable_not_admitted"
                || input.water == "unknown"
            {
                let mut blockers = Vec::new();
                if input.forecast != "temporally_applicable_not_admitted" {
                    blockers.push("forecast_temporal_applicability");
                }
                if input.water == "unknown" {
                    blockers.push("water_level_envelope_supported_and_satisfied");
                }
                Ok(("INDETERMINATE", blockers))
            } else {
                Ok(("FEASIBLE_WITHIN_DECLARED_MODEL", Vec::new()))
            }
        }
        "hold_and_reassess" => {
            if input.waiting == "unknown_due_to_shared_dependency" {
                Ok((
                    "INDETERMINATE",
                    vec!["designated_waiting_resource_declared_available"],
                ))
            } else if input.energy == "inadequate_for_declared_plan" {
                Ok((
                    "UNSUPPORTED",
                    vec!["declared energy behaviour while holding is outside the model scope"],
                ))
            } else {
                Ok(("FEASIBLE_WITHIN_DECLARED_MODEL", Vec::new()))
            }
        }
        "prepare_alternate_terminal" => {
            if input.alternate_capacity != "confirmed" {
                Ok(("INDETERMINATE", vec!["alternate_capacity_confirmed"]))
            } else {
                // v0.1 has no admitted alternate-terminal physics. Capacity
                // confirmation alone must not silently qualify the alternative.
                Ok((
                    "UNSUPPORTED",
                    vec!["alternate-terminal feasibility model not admitted"],
                ))
            }
        }
        _ => Err(format!("unknown plan ID {plan_id:?}; refusing to evaluate")),
    }
}

fn evaluate_bytes(bytes: &[u8], source_commit: &str) -> Result<RehearsalReport, String> {
    if bytes.len() > MAX_SCENARIO_BYTES {
        return Err(format!(
            "scenario fixture exceeds {MAX_SCENARIO_BYTES}-byte bound"
        ));
    }
    let root = super::parse_strict(bytes)?;
    super::exact_keys(&root, ROOT_KEYS, "rehearsal")?;
    super::expect_text(
        super::field(&root, "schema_id", "rehearsal")?,
        "rehearsal.schema_id",
        "luminous.symtropy.maritime-disruption-rehearsal",
    )?;
    super::expect_text(
        super::field(&root, "schema_version", "rehearsal")?,
        "rehearsal.schema_version",
        "0.1.0",
    )?;
    super::expect_text(
        super::field(&root, "fixture_status", "rehearsal")?,
        "rehearsal.fixture_status",
        "synthetic_contract_evaluable_physics_not_executed",
    )?;
    super::expect_text(
        super::field(&root, "source_kind", "rehearsal")?,
        "rehearsal.source_kind",
        "synthetic_fixture",
    )?;
    let fixture_id = text_field(&root, "fixture_id", "rehearsal")?;
    validate_static_contract(&root)?;

    let ceiling = super::field(&root, "claim_ceiling", "rehearsal")?;
    super::exact_keys(ceiling, AUTHORITY_KEYS, "rehearsal.claim_ceiling")?;
    super::expect_text(
        super::field(ceiling, "engineering_admission", "claim_ceiling")?,
        "claim_ceiling.engineering_admission",
        "NotAdmitted",
    )?;
    for key in [
        "navigation_authorized",
        "physical_truth_claim_authorized",
        "berth_availability_inference_authorized",
        "reservation_or_custody_event_authorized",
        "physical_execution_authorized",
    ] {
        if super::bool_value(
            super::field(ceiling, key, "claim_ceiling")?,
            &format!("claim_ceiling.{key}"),
        )? {
            return Err(format!("claim_ceiling.{key} must remain false"));
        }
    }

    let execution = super::field(&root, "execution", "rehearsal")?;
    super::exact_keys(
        execution,
        &[
            "executed",
            "rust_build_test_status",
            "independent_oracle_status",
            "canonical_replay_status",
            "real_source_capture_status",
            "expected_status",
        ],
        "rehearsal.execution",
    )?;
    if super::bool_value(
        super::field(execution, "executed", "rehearsal.execution")?,
        "rehearsal.execution.executed",
    )? {
        return Err("design fixture must not claim that the physical scenario executed".to_owned());
    }
    for (key, expected) in [
        ("rust_build_test_status", "not_yet_run_on_exact_branch_head"),
        (
            "independent_oracle_status",
            "implemented_in_source_not_yet_run",
        ),
        ("canonical_replay_status", "test_implemented_not_yet_run"),
        ("real_source_capture_status", "still_open"),
        (
            "expected_status",
            "contract_evaluable_no_physical_simulation",
        ),
    ] {
        super::expect_text(
            super::field(execution, key, "rehearsal.execution")?,
            &format!("rehearsal.execution.{key}"),
            expected,
        )?;
    }

    let plans = super::array(
        super::field(&root, "candidate_plans", "rehearsal")?,
        "rehearsal.candidate_plans",
    )?;
    if plans.len() != PLAN_IDS.len() {
        return Err(format!(
            "expected {} fixed v0.1 candidate plans, found {}",
            PLAN_IDS.len(),
            plans.len()
        ));
    }
    for (index, plan) in plans.iter().enumerate() {
        let path = format!("rehearsal.candidate_plans[{index}]");
        let expected_plan_keys: &[&str] = if index == 2 {
            &[
                "plan_id",
                "kind",
                "preconditions",
                "side_effects",
                "not_a_booking",
                "preparation_does_not_reserve_capacity",
            ]
        } else {
            &[
                "plan_id",
                "kind",
                "preconditions",
                "side_effects",
                "not_a_booking",
            ]
        };
        super::exact_keys(plan, expected_plan_keys, &path)?;
        let plan_id = text_field(plan, "plan_id", &path)?;
        if plan_id != PLAN_IDS[index] {
            return Err(format!("{path}.plan_id: unexpected plan ID {plan_id:?}"));
        }
        super::expect_text(
            super::field(plan, "kind", &path)?,
            &format!("{path}.kind"),
            "simulated_intent",
        )?;
        if !super::bool_value(
            super::field(plan, "not_a_booking", &path)?,
            &format!("{path}.not_a_booking"),
        )? {
            return Err(format!("{path}.not_a_booking must be true"));
        }
        let expected_preconditions: &[&str] = match plan_id.as_str() {
            "continue_original_slot" => &[
                "forecast_temporally_applicable",
                "water_level_envelope_supported_and_satisfied",
                "energy_reserve_minimum_satisfied",
                "terminal_window_open",
            ],
            "hold_and_reassess" => &[
                "designated_waiting_resource_declared_available",
                "reassessment_time_and_policy_declared",
            ],
            "prepare_alternate_terminal" => &[
                "alternate_terminal_identity_resolved",
                "alternate_capacity_confirmed",
                "alternate_constraints_supported",
            ],
            _ => return Err(format!("{path}.plan_id: unsupported plan")),
        };
        expect_text_array(
            super::field(plan, "preconditions", &path)?,
            &format!("{path}.preconditions"),
            expected_preconditions,
        )?;
        if index == 2
            && !super::bool_value(
                super::field(plan, "preparation_does_not_reserve_capacity", &path)?,
                &format!("{path}.preparation_does_not_reserve_capacity"),
            )?
        {
            return Err("preparing an alternate terminal must not reserve capacity".to_owned());
        }
        if !super::array(
            super::field(plan, "side_effects", &path)?,
            &format!("{path}.side_effects"),
        )?
        .is_empty()
        {
            return Err(format!("{path}.side_effects must remain empty"));
        }
    }

    let cases = super::array(
        super::field(&root, "scenario_cases", "rehearsal")?,
        "rehearsal.scenario_cases",
    )?;
    if cases.len() != CASE_IDS.len() {
        return Err(format!(
            "expected {} frozen v0.1 scenario cases, found {}",
            CASE_IDS.len(),
            cases.len()
        ));
    }

    let mut dispositions = Vec::with_capacity(cases.len() * plans.len());
    let mut evidence_gaps = Vec::new();
    let mut per_plan_all_required_feasible = vec![true; plans.len()];
    for (case_index, case) in cases.iter().enumerate() {
        let case_path = format!("rehearsal.scenario_cases[{case_index}]");
        let case_id = text_field(case, "case_id", &case_path)?;
        let expected_case_keys: &[&str] = match case_id.as_str() {
            "baseline_declared_inputs" | "shore_power_unavailable" => &[
                "case_id",
                "required",
                "disturbances",
                "abstract_inputs",
                "expected_plan_dispositions",
                "expected_blockers",
            ],
            "adverse_water_level_envelope" => &[
                "case_id",
                "required",
                "disturbances",
                "abstract_inputs",
                "expected_plan_dispositions",
                "expected_blockers",
                "interpretation",
            ],
            "forecast_invalidated_before_decision"
            | "common_cause_terminal_and_power_disruption" => &[
                "case_id",
                "required",
                "disturbances",
                "abstract_inputs",
                "expected_plan_dispositions",
                "expected_blockers",
                "metamorphic_property",
            ],
            _ => return Err(format!("{case_path}.case_id: unknown case {case_id:?}")),
        };
        super::exact_keys(case, expected_case_keys, &case_path)?;
        if case_id != CASE_IDS[case_index] {
            return Err(format!(
                "{case_path}.case_id: unexpected case ID {case_id:?}"
            ));
        }
        if !super::bool_value(
            super::field(case, "required", &case_path)?,
            &format!("{case_path}.required"),
        )? {
            return Err(format!(
                "{case_path}.required must remain true in the v0.1 required set"
            ));
        }
        let input = validate_abstract_inputs(
            super::field(case, "abstract_inputs", &case_path)?,
            &format!("{case_path}.abstract_inputs"),
        )?;
        let disturbances = super::array(
            super::field(case, "disturbances", &case_path)?,
            &format!("{case_path}.disturbances"),
        )?;
        for (disturbance_index, disturbance) in disturbances.iter().enumerate() {
            super::require_nonempty_text(
                disturbance,
                &format!("{case_path}.disturbances[{disturbance_index}]"),
            )?;
        }
        if case_id == "adverse_water_level_envelope" {
            super::expect_text(
                super::field(case, "interpretation", &case_path)?,
                &format!("{case_path}.interpretation"),
                "This state is an abstract hostile control; no real water level or vessel clearance is represented.",
            )?;
        }
        if matches!(
            case_id.as_str(),
            "forecast_invalidated_before_decision" | "common_cause_terminal_and_power_disruption"
        ) {
            super::require_nonempty_text(
                super::field(case, "metamorphic_property", &case_path)?,
                &format!("{case_path}.metamorphic_property"),
            )?;
        }
        let expected = super::object(
            super::field(case, "expected_plan_dispositions", &case_path)?,
            &format!("{case_path}.expected_plan_dispositions"),
        )?;
        if expected.len() != PLAN_IDS.len()
            || expected.keys().any(|key| !PLAN_IDS.contains(&key.as_str()))
        {
            return Err(format!(
                "{case_path}.expected_plan_dispositions must contain exactly the three known plan IDs"
            ));
        }
        let expected_blockers = super::object(
            super::field(case, "expected_blockers", &case_path)?,
            &format!("{case_path}.expected_blockers"),
        )?;
        if expected_blockers
            .keys()
            .any(|key| !PLAN_IDS.contains(&key.as_str()))
        {
            return Err(format!(
                "{case_path}.expected_blockers contains an unknown plan ID"
            ));
        }

        for (plan_index, plan_id) in PLAN_IDS.iter().enumerate() {
            let (status, blockers) = disposition(plan_id, &input)?;
            let expected_status = expected
                .get(*plan_id)
                .ok_or_else(|| format!("{case_path}: missing expected status for {plan_id}"))?;
            super::expect_text(
                expected_status,
                &format!("{case_path}.expected_plan_dispositions.{plan_id}"),
                status,
            )?;
            let expected_blocker_values = expected_blockers.get(*plan_id);
            if blockers.is_empty() {
                if expected_blocker_values.is_some() {
                    return Err(format!(
                        "{case_path}.expected_blockers.{plan_id} must be absent when no blockers exist"
                    ));
                }
            } else {
                let declared = expected_blocker_values
                    .ok_or_else(|| format!("{case_path}.expected_blockers is missing {plan_id}"))?;
                let declared = super::array(
                    declared,
                    &format!("{case_path}.expected_blockers.{plan_id}"),
                )?;
                if declared.len() != blockers.len() {
                    return Err(format!(
                        "{case_path}.expected_blockers.{plan_id} count mismatch"
                    ));
                }
                for (blocker_index, (value, expected_blocker)) in
                    declared.iter().zip(&blockers).enumerate()
                {
                    super::expect_text(
                        value,
                        &format!("{case_path}.expected_blockers.{plan_id}[{blocker_index}]"),
                        expected_blocker,
                    )?;
                }
            }
            if status != "FEASIBLE_WITHIN_DECLARED_MODEL" {
                per_plan_all_required_feasible[plan_index] = false;
            }
            if status == "INDETERMINATE" || status == "UNSUPPORTED" {
                for blocker in &blockers {
                    evidence_gaps.push(evidence_gap(&case_id, plan_id, blocker)?);
                }
            }
            dispositions.push(DispositionReport {
                case_id: case_id.clone(),
                plan_id: (*plan_id).to_owned(),
                status: status.to_owned(),
                blockers: blockers.into_iter().map(str::to_owned).collect(),
            });
        }
        if expected.keys().any(|key| !PLAN_IDS.contains(&key.as_str())) {
            return Err(format!(
                "{case_path}.expected_plan_dispositions contains an unknown plan ID"
            ));
        }
    }

    let robustly_feasible_candidate_plans = PLAN_IDS
        .iter()
        .zip(per_plan_all_required_feasible)
        .filter(|(_, is_feasible)| *is_feasible)
        .map(|(plan_id, _)| (*plan_id).to_owned())
        .collect();

    Ok(RehearsalReport {
        status: "SYNTHETIC_REHEARSAL_CONTRACT_MATCH_NOT_ADMITTED",
        schema_id: "luminous.symtropy.maritime-disruption-rehearsal".to_owned(),
        fixture_id,
        scenario_sha256: super::sha256_bytes(bytes),
        scenario_case_count: cases.len(),
        candidate_plan_count: plans.len(),
        dispositions,
        evidence_gaps,
        robustly_feasible_candidate_plans,
        engineering_admission: "NotAdmitted",
        navigation_authorized: false,
        physical_truth_claim_authorized: false,
        berth_availability_inference_authorized: false,
        reservation_or_custody_event_authorized: false,
        physical_execution_authorized: false,
        source_commit_binding: source_commit.to_owned(),
        contract_evaluation_performed: true,
        physical_simulation_performed: false,
    })
}

pub(super) fn run_cli(scenario_path: &str) -> Result<(), String> {
    let metadata = fs::metadata(scenario_path)
        .map_err(|error| format!("cannot stat scenario fixture {scenario_path}: {error}"))?;
    if metadata.len() > MAX_SCENARIO_BYTES as u64 {
        return Err(format!(
            "scenario fixture exceeds {MAX_SCENARIO_BYTES}-byte bound"
        ));
    }
    let bytes = fs::read(scenario_path)
        .map_err(|error| format!("cannot read scenario fixture {scenario_path}: {error}"))?;
    let source_commit =
        env::var("MARITIME_SOURCE_COMMIT").unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let report = evaluate_bytes(&bytes, &source_commit)?;
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize rehearsal report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &[u8] =
        include_bytes!("../../../../docs/tech/MARITIME_DISRUPTION_REHEARSAL_V0_1.json");

    // Separate calendar oracle using day/month iteration instead of the
    // production Gregorian-era conversion. This fixture's dates are post-1970.
    fn oracle_epoch_seconds(timestamp: &str) -> i64 {
        assert_eq!(timestamp.len(), 20);
        let year: i64 = timestamp[0..4].parse().unwrap();
        let month: usize = timestamp[5..7].parse().unwrap();
        let day: i64 = timestamp[8..10].parse().unwrap();
        let hour: i64 = timestamp[11..13].parse().unwrap();
        let minute: i64 = timestamp[14..16].parse().unwrap();
        let second: i64 = timestamp[17..19].parse().unwrap();
        assert!(year >= 1970);
        let is_leap = |y: i64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let mut days = 0_i64;
        for y in 1970..year {
            days += if is_leap(y) { 366 } else { 365 };
        }
        let month_lengths = [
            31_i64,
            if is_leap(year) { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        for length in month_lengths.iter().take(month.saturating_sub(1)) {
            days += *length;
        }
        days += day - 1;
        days * 86_400 + hour * 3_600 + minute * 60 + second
    }

    // Intentionally separate test oracle: direct serde_json parsing and a distinct
    // classifier. It does not use the production JsonValue parser or disposition().
    fn oracle_status(plan: &str, case: &Value) -> &'static str {
        let input = &case["abstract_inputs"];
        let water = input["water_level_envelope_state"].as_str().unwrap();
        let energy = input["energy_reserve_state"].as_str().unwrap();
        let terminal = input["terminal_window_state"].as_str().unwrap();
        let forecast = input["forecast_evidence_state"].as_str().unwrap();
        let waiting = input["waiting_resource_state"].as_str().unwrap();
        let alternate = input["alternate_capacity_state"].as_str().unwrap();

        match plan {
            "continue_original_slot" if terminal == "closed" => "INFEASIBLE",
            "continue_original_slot" if water == "outside_declared_limit" => "INFEASIBLE",
            "continue_original_slot" if energy == "inadequate_for_declared_plan" => "INFEASIBLE",
            "continue_original_slot"
                if water == "unknown" || forecast != "temporally_applicable_not_admitted" =>
            {
                "INDETERMINATE"
            }
            "continue_original_slot" => "FEASIBLE_WITHIN_DECLARED_MODEL",
            "hold_and_reassess" if waiting != "declared_available" => "INDETERMINATE",
            "hold_and_reassess" if energy != "adequate" => "UNSUPPORTED",
            "hold_and_reassess" => "FEASIBLE_WITHIN_DECLARED_MODEL",
            "prepare_alternate_terminal" if alternate != "confirmed" => "INDETERMINATE",
            "prepare_alternate_terminal" => "UNSUPPORTED",
            _ => "UNSUPPORTED",
        }
    }

    fn oracle_requirement(blocker: &str) -> (&'static str, &'static str) {
        match blocker {
            "terminal_window_open" => {
                ("terminal_window_constraint", "terminal_window_confirmation")
            }
            "water_level_envelope_supported_and_satisfied" => (
                "water_level_envelope_constraint",
                "water_level_envelope_evidence",
            ),
            "energy_reserve_minimum_satisfied" => {
                ("energy_reserve_constraint", "energy_reserve_assessment")
            }
            "forecast_temporal_applicability" => (
                "forecast_temporal_applicability",
                "forecast_temporal_binding",
            ),
            "designated_waiting_resource_declared_available" => (
                "waiting_resource_availability",
                "waiting_resource_availability",
            ),
            "alternate_capacity_confirmed" => {
                ("alternate_terminal_capacity", "alternate_terminal_capacity")
            }
            "declared energy behaviour while holding is outside the model scope" => (
                "holding_energy_model_unsupported",
                "holding_energy_model_support",
            ),
            "alternate-terminal feasibility model not admitted" => (
                "alternate_terminal_model_unsupported",
                "alternate_terminal_model_support",
            ),
            _ => ("unknown", "unknown"),
        }
    }

    fn oracle_blockers(plan: &str, case: &Value) -> Vec<&'static str> {
        let input = &case["abstract_inputs"];
        let water = input["water_level_envelope_state"].as_str().unwrap();
        let energy = input["energy_reserve_state"].as_str().unwrap();
        let terminal = input["terminal_window_state"].as_str().unwrap();
        let forecast = input["forecast_evidence_state"].as_str().unwrap();
        let waiting = input["waiting_resource_state"].as_str().unwrap();
        let alternate = input["alternate_capacity_state"].as_str().unwrap();
        match plan {
            "continue_original_slot" if terminal == "closed" => vec!["terminal_window_open"],
            "continue_original_slot" if water == "outside_declared_limit" => {
                vec!["water_level_envelope_supported_and_satisfied"]
            }
            "continue_original_slot" if energy == "inadequate_for_declared_plan" => {
                vec!["energy_reserve_minimum_satisfied"]
            }
            "continue_original_slot" => {
                let mut blockers = Vec::new();
                if forecast != "temporally_applicable_not_admitted" {
                    blockers.push("forecast_temporal_applicability");
                }
                if water == "unknown" {
                    blockers.push("water_level_envelope_supported_and_satisfied");
                }
                blockers
            }
            "hold_and_reassess" if waiting != "declared_available" => {
                vec!["designated_waiting_resource_declared_available"]
            }
            "hold_and_reassess" if energy != "adequate" => {
                vec!["declared energy behaviour while holding is outside the model scope"]
            }
            "hold_and_reassess" => Vec::new(),
            "prepare_alternate_terminal" if alternate != "confirmed" => {
                vec!["alternate_capacity_confirmed"]
            }
            "prepare_alternate_terminal" => {
                vec!["alternate-terminal feasibility model not admitted"]
            }
            _ => vec!["unknown plan"],
        }
    }

    #[test]
    fn frozen_fixture_matches_independent_oracle() {
        let fixture: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON parses");
        let report = evaluate_bytes(FIXTURE, "test-source-commit").expect("fixture validates");
        let cases = fixture["scenario_cases"]
            .as_array()
            .expect("cases are array");
        assert_eq!(report.scenario_case_count, 5);
        assert_eq!(report.candidate_plan_count, 3);
        assert_eq!(report.dispositions.len(), 15);

        for case in cases {
            let case_id = case["case_id"].as_str().unwrap();
            for plan in PLAN_IDS {
                let expected = oracle_status(plan, case);
                let declared = case["expected_plan_dispositions"][plan]
                    .as_str()
                    .expect("declared expectation");
                assert_eq!(declared, expected, "frozen oracle: {case_id} / {plan}");
                let actual = report
                    .dispositions
                    .iter()
                    .find(|d| d.case_id == case_id && d.plan_id == *plan)
                    .expect("production disposition exists");
                assert_eq!(
                    actual.status, expected,
                    "runtime oracle: {case_id} / {plan}"
                );

                let oracle_blockers = oracle_blockers(plan, case);
                assert_eq!(
                    actual.blockers,
                    oracle_blockers
                        .iter()
                        .map(|value| (*value).to_owned())
                        .collect::<Vec<_>>(),
                    "blocker oracle: {case_id} / {plan}"
                );
                let declared_blockers = case["expected_blockers"].get(plan);
                if oracle_blockers.is_empty() {
                    assert!(
                        declared_blockers.is_none(),
                        "unexpected declared blockers: {case_id} / {plan}"
                    );
                } else {
                    let declared = declared_blockers
                        .and_then(Value::as_array)
                        .expect("declared blocker array");
                    assert_eq!(
                        declared
                            .iter()
                            .map(|v| v.as_str().unwrap())
                            .collect::<Vec<_>>(),
                        oracle_blockers,
                        "frozen blocker expectations: {case_id} / {plan}"
                    );
                }
            }
        }
        assert!(report.robustly_feasible_candidate_plans.is_empty());
        assert_eq!(report.engineering_admission, "NotAdmitted");
        assert!(!report.navigation_authorized);
        assert!(!report.physical_truth_claim_authorized);
        assert!(!report.berth_availability_inference_authorized);
        assert!(!report.reservation_or_custody_event_authorized);
        assert!(!report.physical_execution_authorized);
        assert!(!report.physical_simulation_performed);
        let mut expected_gaps = Vec::new();
        for result in &report.dispositions {
            if result.status != "INDETERMINATE" && result.status != "UNSUPPORTED" {
                continue;
            }
            let case = cases
                .iter()
                .find(|case| case["case_id"].as_str() == Some(result.case_id.as_str()))
                .unwrap();
            for blocker in oracle_blockers(&result.plan_id, case) {
                expected_gaps.push((
                    result.case_id.as_str(),
                    result.plan_id.as_str(),
                    blocker,
                    oracle_requirement(blocker),
                ));
            }
        }
        let actual_gaps: Vec<_> = report
            .evidence_gaps
            .iter()
            .map(|gap| {
                (
                    gap.case_id.as_str(),
                    gap.plan_id.as_str(),
                    gap.blocker_code.as_str(),
                    gap.blocker_reason.as_str(),
                    gap.requirement_id.as_str(),
                )
            })
            .collect();
        let expected_gaps = expected_gaps
            .into_iter()
            .map(
                |(case_id, plan_id, blocker_reason, (blocker_code, requirement_id))| {
                    (
                        case_id,
                        plan_id,
                        blocker_code,
                        blocker_reason,
                        requirement_id,
                    )
                },
            )
            .collect::<Vec<_>>();
        assert_eq!(actual_gaps, expected_gaps);
        for gap in &report.evidence_gaps {
            assert_ne!(gap.blocker_code, "unknown");
            assert_ne!(gap.requirement_id, "unknown");
            assert!(!gap.minimum_fields.is_empty());
            assert!(!gap.resolution_scope.is_empty());
        }
    }

    #[test]
    fn forecast_arithmetic_matches_independent_calendar_oracle() {
        let fixture: Value = serde_json::from_slice(FIXTURE).expect("fixture parses");
        let assessment = fixture["assessment"]["assessment_time_utc"]
            .as_str()
            .unwrap();
        let target = fixture["assessment"]["target_time_utc"].as_str().unwrap();
        let forecast = &fixture["assessment"]["forecast_input"];
        let issued = forecast["issued_at_utc"].as_str().unwrap();
        let initialized = forecast["model_initialization_time_utc"].as_str().unwrap();
        let valid_from = forecast["valid_from_utc"].as_str().unwrap();
        let valid_until = forecast["valid_until_utc"].as_str().unwrap();

        let issue_age = oracle_epoch_seconds(assessment) - oracle_epoch_seconds(issued);
        let forecast_lead = oracle_epoch_seconds(target) - oracle_epoch_seconds(initialized);
        assert_eq!(issue_age, 7_200);
        assert_eq!(forecast_lead, 32_400);
        assert!(oracle_epoch_seconds(initialized) <= oracle_epoch_seconds(issued));
        assert!(oracle_epoch_seconds(issued) <= oracle_epoch_seconds(assessment));
        assert!(oracle_epoch_seconds(assessment) <= oracle_epoch_seconds(target));
        assert!(oracle_epoch_seconds(valid_from) <= oracle_epoch_seconds(target));
        assert!(oracle_epoch_seconds(target) <= oracle_epoch_seconds(valid_until));
        assert!(issue_age as u64 <= forecast["max_issue_age_seconds"].as_u64().unwrap());
        assert!(forecast_lead as u64 <= forecast["max_forecast_lead_seconds"].as_u64().unwrap());
        evaluate_bytes(FIXTURE, "time-oracle-test")
            .expect("production evaluator agrees with fixture");
    }

    #[test]
    fn invalidating_required_forecast_never_improves_original_plan() {
        let fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        let baseline = fixture["scenario_cases"][0]["abstract_inputs"].clone();
        let mut invalidated = baseline.clone();
        invalidated["forecast_evidence_state"] = Value::String("invalidated".to_owned());
        invalidated["water_level_envelope_state"] = Value::String("unknown".to_owned());

        let baseline_bytes = serde_json::to_vec(&baseline).unwrap();
        let invalidated_bytes = serde_json::to_vec(&invalidated).unwrap();
        let baseline_jv = super::super::parse_strict(&baseline_bytes).unwrap();
        let invalidated_jv = super::super::parse_strict(&invalidated_bytes).unwrap();

        let (baseline_status, _) = disposition(
            "continue_original_slot",
            &validate_abstract_inputs(&baseline_jv, "baseline").unwrap(),
        )
        .unwrap();
        let (invalidated_status, _) = disposition(
            "continue_original_slot",
            &validate_abstract_inputs(&invalidated_jv, "invalidated").unwrap(),
        )
        .unwrap();
        assert_eq!(baseline_status, "FEASIBLE_WITHIN_DECLARED_MODEL");
        assert_eq!(invalidated_status, "INDETERMINATE");
    }

    #[test]
    fn confirmed_alternate_capacity_does_not_imply_feasibility() {
        let inputs = AbstractInputs {
            water: "within_declared_limit".to_owned(),
            energy: "adequate".to_owned(),
            terminal: "open".to_owned(),
            forecast: "temporally_applicable_not_admitted".to_owned(),
            waiting: "declared_available".to_owned(),
            alternate_capacity: "confirmed".to_owned(),
        };
        let (status, blockers) = disposition("prepare_alternate_terminal", &inputs).unwrap();
        assert_eq!(status, "UNSUPPORTED");
        assert_eq!(
            blockers,
            vec!["alternate-terminal feasibility model not admitted"]
        );
    }

    #[test]
    fn known_constraint_violation_is_not_hidden_by_missing_forecast() {
        let inputs = AbstractInputs {
            water: "unknown".to_owned(),
            energy: "adequate".to_owned(),
            terminal: "closed".to_owned(),
            forecast: "invalidated".to_owned(),
            waiting: "declared_available".to_owned(),
            alternate_capacity: "unconfirmed".to_owned(),
        };
        let (status, blockers) = disposition("continue_original_slot", &inputs).unwrap();
        assert_eq!(status, "INFEASIBLE");
        assert_eq!(blockers, vec!["terminal_window_open"]);
    }

    #[test]
    fn identical_inputs_produce_identical_report_bytes() {
        let first = evaluate_bytes(FIXTURE, "fixed-test-source").unwrap();
        let second = evaluate_bytes(FIXTURE, "fixed-test-source").unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn authority_escalation_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["claim_ceiling"]["navigation_authorized"] = Value::Bool(true);
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn unknown_abstract_state_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["scenario_cases"][0]["abstract_inputs"]["water_level_envelope_state"] =
            Value::String("probably_ok".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn changed_evidence_requirement_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["evidence_requirements"][0]["resolution_scope"] =
            Value::String("creates a confirmed booking".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn changed_declared_blocker_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["scenario_cases"][1]["expected_blockers"]["continue_original_slot"][0] =
            Value::String("invented_reason".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn changed_constraint_semantics_are_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["hard_constraints"][0]["missing_state"] = Value::String("PASS".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn incorrect_forecast_arithmetic_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["assessment"]["forecast_input"]["expected_temporal_arithmetic_for_test_design_only"]
            ["issue_age_seconds"] = Value::Number(serde_json::Number::from(7201));
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn future_issued_forecast_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["assessment"]["forecast_input"]["issued_at_utc"] =
            Value::String("2026-10-15T13:00:00Z".to_owned());
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }

    #[test]
    fn unknown_root_field_is_rejected() {
        let mut fixture: Value = serde_json::from_slice(FIXTURE).unwrap();
        fixture["surprise_authority"] = Value::Bool(true);
        let changed = serde_json::to_vec(&fixture).unwrap();
        assert!(evaluate_bytes(&changed, "test").is_err());
    }
}
