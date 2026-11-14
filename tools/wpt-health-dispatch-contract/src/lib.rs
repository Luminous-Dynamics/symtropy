// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed contract for admitting asset-health forecasts into WPT dispatch.
//!
//! A forecast can influence supervisory allocation only when provenance,
//! calibration, censoring support, operating-domain support and probability
//! coherence are admitted. It never overrides electrical, thermal, grid,
//! network-stability, receiver-authorization or handover hard limits.
//!
//! This crate assesses *decision-support eligibility*. It does not authorize
//! energization, certify safety, train RUL models or establish field validity.

use serde_json::Value;
use std::collections::BTreeSet;

const EVIDENCE_FLAGS: &[&str] = &[
    "SYNTHETIC_ONLY",
    "SOURCE_INCOMPLETE",
    "CENSORING_SUPPORT_LIMITED",
    "CALIBRATION_DRIFT",
    "DOMAIN_SHIFT",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EligibilityState {
    DecisionSupportEligible,
    PolicyRiskExceeded,
    SyntheticOnlyNotEligible,
    NotEvaluable,
    OutOfDomain,
    HardLimitViolation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendedAction {
    MayInformAllocationOnly,
    DerateOrReserveMaintenance,
    ExcludeFromPlannedServiceHorizon,
    ConservativeFallbackNoUprating,
    DeenergizeAndEscalate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EligibilityReport {
    pub run_id: String,
    pub asset_id: String,
    pub segment_id: String,
    pub state: EligibilityState,
    /// True means the forecast may be used as a supervisory decision input,
    /// not that the asset is authorized or safe to energize.
    pub forecast_eligible_for_decision_support: bool,
    pub target_event_cif: f64,
    pub maximum_target_event_cif: f64,
    pub risk_policy_exceeded: bool,
    pub horizon_hours: f64,
    pub minimum_censoring_survival: f64,
    pub maximum_ipcw_weight: f64,
    pub recommended_action: RecommendedAction,
    pub reasons: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be numeric"))?;
    if !number.is_finite() {
        return Err(fail(&format!("{path}.{key}"), "must be finite"));
    }
    Ok(number)
}

fn bool_field(value: &Value, key: &str, path: &str) -> Result<bool, String> {
    field(value, key, path)?
        .as_bool()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be boolean"))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_digest_fields(value: &Value, fields: &[&str], path: &str) -> Result<(), String> {
    for key in fields {
        let digest = string_field(value, key, path)?;
        if !valid_sha256(digest) {
            return Err(fail(
                &format!("{path}.{key}"),
                "must be lowercase SHA-256 hex",
            ));
        }
    }
    Ok(())
}

fn string_flags(run: &Value) -> Result<BTreeSet<String>, String> {
    let values = field(run, "quality_flags", "$")?
        .as_array()
        .ok_or_else(|| fail("$.quality_flags", "must be an array"))?;
    let mut flags = BTreeSet::new();
    for value in values {
        let flag = value
            .as_str()
            .ok_or_else(|| fail("$.quality_flags", "all flags must be strings"))?;
        if !EVIDENCE_FLAGS.contains(&flag) {
            return Err(fail(
                "$.quality_flags",
                format!("unknown quality flag {flag}"),
            ));
        }
        if !flags.insert(flag.to_string()) {
            return Err(fail(
                "$.quality_flags",
                format!("duplicate quality flag {flag}"),
            ));
        }
    }
    Ok(flags)
}

struct EligibilityReportContext {
    run_id: String,
    asset_id: String,
    segment_id: String,
    target_event_cif: f64,
    maximum_target_event_cif: f64,
    horizon_hours: f64,
    minimum_censoring_survival: f64,
    maximum_ipcw_weight: f64,
}

fn report(
    context: &EligibilityReportContext,
    state: EligibilityState,
    recommended_action: RecommendedAction,
    reasons: Vec<String>,
) -> EligibilityReport {
    let eligible = matches!(
        state,
        EligibilityState::DecisionSupportEligible | EligibilityState::PolicyRiskExceeded
    );
    EligibilityReport {
        run_id: context.run_id.clone(),
        asset_id: context.asset_id.clone(),
        segment_id: context.segment_id.clone(),
        state,
        forecast_eligible_for_decision_support: eligible,
        target_event_cif: context.target_event_cif,
        maximum_target_event_cif: context.maximum_target_event_cif,
        risk_policy_exceeded: state == EligibilityState::PolicyRiskExceeded,
        horizon_hours: context.horizon_hours,
        minimum_censoring_survival: context.minimum_censoring_survival,
        maximum_ipcw_weight: context.maximum_ipcw_weight,
        recommended_action,
        reasons,
        claim_ceiling: "Decision-support eligibility only. No energization authorization, safety certification, field RUL validation or standards compliance is implied.",
    }
}

/// Validate an asset-risk receipt and classify how it may influence dispatch.
///
/// Order of precedence:
/// 1. A hard physical/authorization violation always blocks dispatch.
/// 2. Synthetic-only evidence is never eligible for operational decision support.
/// 3. Missing provenance, calibration, censoring support or incoherent forecasts
///    produce NOT_EVALUABLE.
/// 4. Unsupported materials, routes, temperatures or duty cycles produce
///    OUT_OF_DOMAIN.
/// 5. A supported forecast above the operator's risk budget remains eligible
///    evidence but returns a derate/maintenance action (or exclusion if the
///    policy is a hard risk cap).
pub fn evaluate_eligibility(run: &Value) -> Result<EligibilityReport, String> {
    exact_keys(
        run,
        &[
            "schema_version",
            "run_id",
            "evidence_kind",
            "asset_identity",
            "forecast",
            "censoring_support",
            "domain_support",
            "hard_limits",
            "dispatch_policy",
            "quality_flags",
        ],
        "$",
    )?;
    if string_field(run, "schema_version", "$")? != "wpt-health-dispatch-eligibility-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }

    let run_id = string_field(run, "run_id", "$")?.to_string();
    let evidence_kind = string_field(run, "evidence_kind", "$")?;
    if ![
        "MEASURED",
        "PUBLISHED_MEASURED",
        "SIMULATED",
        "SYNTHETIC_KNOWN_TRUTH",
    ]
    .contains(&evidence_kind)
    {
        return Err(fail("$.evidence_kind", "unknown evidence kind"));
    }

    let flags = string_flags(run)?;
    let identity = field(run, "asset_identity", "$")?;
    exact_keys(
        identity,
        &[
            "asset_id",
            "segment_id",
            "installation_id",
            "route_id",
            "material_lot_id",
            "asset_state_artifact_sha256",
        ],
        "$.asset_identity",
    )?;
    let asset_id = string_field(identity, "asset_id", "$.asset_identity")?.to_string();
    let segment_id = string_field(identity, "segment_id", "$.asset_identity")?.to_string();
    string_field(identity, "installation_id", "$.asset_identity")?;
    string_field(identity, "route_id", "$.asset_identity")?;
    string_field(identity, "material_lot_id", "$.asset_identity")?;

    let forecast = field(run, "forecast", "$")?;
    exact_keys(
        forecast,
        &[
            "estimand",
            "target_event_code",
            "competing_event_codes",
            "cause_specific_cumulative_incidence",
            "horizon_hours",
            "target_event_cif",
            "event_free_survival",
            "model_sha256",
            "outcome_data_sha256",
            "calibration_receipt_sha256",
            "calibration_population_id",
            "calibration_stratum_id",
            "calibration_method",
            "calibration_status",
            "calibration_horizon_max_hours",
            "hazard_cif_coherence_verified",
            "source_locator",
        ],
        "$.forecast",
    )?;
    if string_field(forecast, "estimand", "$.forecast")? != "TARGET_EVENT_CIF_AT_HORIZON" {
        return Err(fail(
            "$.forecast.estimand",
            "unsupported event-risk estimand",
        ));
    }
    string_field(forecast, "calibration_population_id", "$.forecast")?;
    string_field(forecast, "calibration_stratum_id", "$.forecast")?;
    let horizon = number_field(forecast, "horizon_hours", "$.forecast")?;
    let target_cif = number_field(forecast, "target_event_cif", "$.forecast")?;
    let event_free_survival = number_field(forecast, "event_free_survival", "$.forecast")?;
    let maximum_horizon = number_field(forecast, "calibration_horizon_max_hours", "$.forecast")?;
    if horizon <= 0.0 || maximum_horizon <= 0.0 {
        return Err(fail(
            "$.forecast",
            "forecast and calibration horizons must be positive",
        ));
    }
    if !(0.0..=1.0).contains(&target_cif)
        || !(0.0..=1.0).contains(&event_free_survival)
        || target_cif + event_free_survival > 1.0 + 1e-9
    {
        return Err(fail(
            "$.forecast",
            "target CIF and event-free survival must form a valid probability subdistribution",
        ));
    }
    let target_code = string_field(forecast, "target_event_code", "$.forecast")?;
    let allowed_target_codes = [
        "STRUCTURAL_CRACK_LIMIT",
        "RUTTING_LIMIT",
        "INSULATION_RESISTANCE_LIMIT",
        "COIL_OPEN_CIRCUIT",
        "THERMAL_LIMIT",
        "SERVICE_AVAILABILITY_LIMIT",
        "OTHER",
    ];
    let allowed_competing_codes = [
        "PLANNED_REPLACEMENT",
        "UNRELATED_ASSET_FAILURE",
        "ROAD_RECONSTRUCTION",
    ];
    if !allowed_target_codes.contains(&target_code) {
        return Err(fail(
            "$.forecast.target_event_code",
            "unknown target event code",
        ));
    }

    let competing_rows = field(forecast, "competing_event_codes", "$.forecast")?
        .as_array()
        .ok_or_else(|| fail("$.forecast.competing_event_codes", "must be an array"))?;
    let mut competing_codes = BTreeSet::new();
    for (index, row) in competing_rows.iter().enumerate() {
        let code = row
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                fail(
                    "$.forecast.competing_event_codes",
                    "all codes must be non-empty strings",
                )
            })?;
        if code == target_code {
            return Err(fail(
                "$.forecast.competing_event_codes",
                "target and competing event codes must be disjoint",
            ));
        }
        if !allowed_competing_codes.contains(&code) {
            return Err(fail(
                &format!("$.forecast.competing_event_codes[{index}]"),
                format!("unknown competing-event code {code}"),
            ));
        }
        if !competing_codes.insert(code.to_string()) {
            return Err(fail(
                "$.forecast.competing_event_codes",
                format!("duplicate competing-event code {code}"),
            ));
        }
    }

    let cause_rows = field(
        forecast,
        "cause_specific_cumulative_incidence",
        "$.forecast",
    )?
    .as_array()
    .ok_or_else(|| {
        fail(
            "$.forecast.cause_specific_cumulative_incidence",
            "must be an array",
        )
    })?;
    let mut expected_causes = competing_codes.clone();
    expected_causes.insert(target_code.to_string());
    let mut observed_causes = BTreeSet::new();
    let mut cause_probability_sum = event_free_survival;
    let mut selected_target_cif = None::<f64>;
    for (index, row) in cause_rows.iter().enumerate() {
        let row_path = format!("$.forecast.cause_specific_cumulative_incidence[{index}]");
        exact_keys(row, &["event_code", "probability"], &row_path)?;
        let code = string_field(row, "event_code", &row_path)?;
        if !expected_causes.contains(code) {
            return Err(fail(
                &row_path,
                format!("cause {code} is not declared as target or competing event"),
            ));
        }
        if !observed_causes.insert(code.to_string()) {
            return Err(fail(&row_path, format!("duplicate cause {code}")));
        }
        let probability = number_field(row, "probability", &row_path)?;
        if !(0.0..=1.0).contains(&probability) {
            return Err(fail(&row_path, "probability must be within [0, 1]"));
        }
        cause_probability_sum += probability;
        if !cause_probability_sum.is_finite() {
            return Err(fail(&row_path, "cause probability sum overflowed"));
        }
        if code == target_code {
            selected_target_cif = Some(probability);
        }
    }
    if observed_causes != expected_causes {
        return Err(fail(
            "$.forecast.cause_specific_cumulative_incidence",
            "must include the target event and every declared competing event exactly once",
        ));
    }
    if (cause_probability_sum - 1.0).abs() > 1e-9 {
        return Err(fail(
            "$.forecast.cause_specific_cumulative_incidence",
            format!(
                "event-free survival plus all cause-specific CIFs must sum to 1 (got {cause_probability_sum})"
            ),
        ));
    }
    if (selected_target_cif.unwrap_or(-1.0) - target_cif).abs() > 1e-9 {
        return Err(fail(
            "$.forecast.target_event_cif",
            "target_event_cif must equal the matching cause-specific CIF",
        ));
    }

    let censoring = field(run, "censoring_support", "$")?;
    exact_keys(
        censoring,
        &[
            "assumption",
            "estimator_id",
            "minimum_g_observed",
            "minimum_g_required",
            "maximum_ipcw_weight",
            "maximum_ipcw_weight_allowed",
            "support_status",
        ],
        "$.censoring_support",
    )?;
    let minimum_g = number_field(censoring, "minimum_g_observed", "$.censoring_support")?;
    let required_g = number_field(censoring, "minimum_g_required", "$.censoring_support")?;
    let maximum_weight = number_field(censoring, "maximum_ipcw_weight", "$.censoring_support")?;
    let allowed_weight = number_field(
        censoring,
        "maximum_ipcw_weight_allowed",
        "$.censoring_support",
    )?;
    let censoring_assumption = string_field(censoring, "assumption", "$.censoring_support")?;
    string_field(censoring, "estimator_id", "$.censoring_support")?;
    let censoring_status = string_field(censoring, "support_status", "$.censoring_support")?;
    if !["SUPPORTED", "POSITIVITY_FAILURE", "NOT_ESTIMATED"].contains(&censoring_status) {
        return Err(fail(
            "$.censoring_support.support_status",
            "unknown censoring support status",
        ));
    }
    if !(0.0..=1.0).contains(&minimum_g)
        || required_g <= 0.0
        || required_g > 1.0
        || maximum_weight < 0.0
        || allowed_weight <= 0.0
    {
        return Err(fail(
            "$.censoring_support",
            "invalid censoring support bounds",
        ));
    }

    let domain = field(run, "domain_support", "$")?;
    exact_keys(
        domain,
        &[
            "support_status",
            "material_family",
            "validated_material_family",
            "ambient_temperature_c",
            "minimum_validated_temperature_c",
            "maximum_validated_temperature_c",
            "duty_cycle_percent",
            "maximum_validated_duty_cycle_percent",
            "route_class",
            "validated_route_class",
        ],
        "$.domain_support",
    )?;
    let domain_status = string_field(domain, "support_status", "$.domain_support")?;
    if !["IN_DOMAIN", "OUT_OF_DOMAIN", "UNKNOWN"].contains(&domain_status) {
        return Err(fail(
            "$.domain_support.support_status",
            "unknown operating-domain support status",
        ));
    }
    let material = string_field(domain, "material_family", "$.domain_support")?;
    let validated_material = string_field(domain, "validated_material_family", "$.domain_support")?;
    let ambient = number_field(domain, "ambient_temperature_c", "$.domain_support")?;
    let min_temperature = number_field(
        domain,
        "minimum_validated_temperature_c",
        "$.domain_support",
    )?;
    let max_temperature = number_field(
        domain,
        "maximum_validated_temperature_c",
        "$.domain_support",
    )?;
    let duty_cycle = number_field(domain, "duty_cycle_percent", "$.domain_support")?;
    let maximum_duty = number_field(
        domain,
        "maximum_validated_duty_cycle_percent",
        "$.domain_support",
    )?;
    let route = string_field(domain, "route_class", "$.domain_support")?;
    let validated_route = string_field(domain, "validated_route_class", "$.domain_support")?;
    if min_temperature > max_temperature
        || !(0.0..=100.0).contains(&duty_cycle)
        || !(0.0..=100.0).contains(&maximum_duty)
    {
        return Err(fail("$.domain_support", "invalid validated domain bounds"));
    }

    let hard = field(run, "hard_limits", "$")?;
    exact_keys(
        hard,
        &[
            "receiver_authorized",
            "electrical_limits_pass",
            "thermal_limits_pass",
            "grid_pcc_envelope_pass",
            "network_stability_pass",
            "segment_handover_safe",
        ],
        "$.hard_limits",
    )?;
    let hard_limits_pass = [
        "receiver_authorized",
        "electrical_limits_pass",
        "thermal_limits_pass",
        "grid_pcc_envelope_pass",
        "network_stability_pass",
        "segment_handover_safe",
    ]
    .iter()
    .map(|key| bool_field(hard, key, "$.hard_limits"))
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .all(|passes| passes);

    let policy = field(run, "dispatch_policy", "$")?;
    exact_keys(
        policy,
        &[
            "policy_id",
            "maximum_target_event_cif",
            "planned_service_horizon_hours",
            "maintenance_lead_time_hours",
            "decision_mode",
            "policy_sha256",
        ],
        "$.dispatch_policy",
    )?;
    string_field(policy, "policy_id", "$.dispatch_policy")?;
    let maximum_risk = number_field(policy, "maximum_target_event_cif", "$.dispatch_policy")?;
    let service_horizon =
        number_field(policy, "planned_service_horizon_hours", "$.dispatch_policy")?;
    let maintenance_lead_time =
        number_field(policy, "maintenance_lead_time_hours", "$.dispatch_policy")?;
    let decision_mode = string_field(policy, "decision_mode", "$.dispatch_policy")?;
    if !(0.0..=1.0).contains(&maximum_risk)
        || service_horizon <= 0.0
        || maintenance_lead_time < 0.0
        || !["SOFT_HEALTH_CONSTRAINT_ONLY", "HARD_RISK_CAP"].contains(&decision_mode)
    {
        return Err(fail("$.dispatch_policy", "invalid risk policy"));
    }
    if (service_horizon - horizon).abs() > 1e-9 {
        return Err(fail(
            "$.dispatch_policy.planned_service_horizon_hours",
            "planned service horizon must equal the forecast horizon",
        ));
    }

    let report_context = EligibilityReportContext {
        run_id,
        asset_id,
        segment_id,
        target_event_cif: target_cif,
        maximum_target_event_cif: maximum_risk,
        horizon_hours: horizon,
        minimum_censoring_survival: minimum_g,
        maximum_ipcw_weight: maximum_weight,
    };
    let mut reasons = Vec::new();
    if !hard_limits_pass {
        reasons.push(
            "one or more authorization, electrical, thermal, grid/PCC, stability or handover hard limits failed"
                .into(),
        );
        return Ok(report(
            &report_context,
            EligibilityState::HardLimitViolation,
            RecommendedAction::DeenergizeAndEscalate,
            reasons,
        ));
    }

    if evidence_kind == "SYNTHETIC_KNOWN_TRUTH"
        || evidence_kind == "SIMULATED"
        || flags.contains("SYNTHETIC_ONLY")
    {
        reasons.push(
            "synthetic/simulated evidence cannot qualify an operational health forecast".into(),
        );
        return Ok(report(
            &report_context,
            EligibilityState::SyntheticOnlyNotEligible,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }
    if flags.contains("SOURCE_INCOMPLETE") || flags.contains("CALIBRATION_DRIFT") {
        reasons.push("source provenance is incomplete or calibration drift is declared".into());
        return Ok(report(
            &report_context,
            EligibilityState::NotEvaluable,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }

    let identity_digests = validate_digest_fields(
        identity,
        &["asset_state_artifact_sha256"],
        "$.asset_identity",
    );
    let forecast_digests = validate_digest_fields(
        forecast,
        &[
            "model_sha256",
            "outcome_data_sha256",
            "calibration_receipt_sha256",
        ],
        "$.forecast",
    );
    if identity_digests.is_err() || forecast_digests.is_err() {
        reasons.push("one or more asset/model/outcome/calibration digests are invalid".into());
        return Ok(report(
            &report_context,
            EligibilityState::NotEvaluable,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }

    let calibration_status = string_field(forecast, "calibration_status", "$.forecast")?;
    let calibration_method = string_field(forecast, "calibration_method", "$.forecast")?;
    let policy_digest = string_field(policy, "policy_sha256", "$.dispatch_policy")?;
    if !valid_sha256(policy_digest) {
        reasons.push("dispatch policy digest is invalid".into());
        return Ok(report(
            &report_context,
            EligibilityState::NotEvaluable,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }
    let source_locator = field(forecast, "source_locator", "$.forecast")?;
    if evidence_kind == "PUBLISHED_MEASURED"
        && source_locator
            .as_str()
            .is_none_or(|value| value.trim().is_empty())
    {
        reasons.push("published-measured forecasts require a non-empty source locator".into());
        return Ok(report(
            &report_context,
            EligibilityState::NotEvaluable,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }
    if calibration_status != "CALIBRATED"
        || ![
            "A_CALIBRATION",
            "D_CALIBRATION",
            "CIF_RELIABILITY_CURVE",
            "CLUSTER_BOOTSTRAP_COVERAGE",
            "OTHER_REVIEWED",
        ]
        .contains(&calibration_method)
        || horizon > maximum_horizon
        || !bool_field(forecast, "hazard_cif_coherence_verified", "$.forecast")?
    {
        reasons.push(
            "forecast calibration, horizon support or hazard/CIF coherence is insufficient".into(),
        );
        return Ok(report(
            &report_context,
            EligibilityState::NotEvaluable,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }

    if censoring_status != "SUPPORTED"
        || ![
            "INDEPENDENT_RIGHT_CENSORING",
            "CONDITIONAL_INDEPENDENT_CENSORING",
        ]
        .contains(&censoring_assumption)
        || minimum_g < required_g
        || maximum_weight > allowed_weight
        || flags.contains("CENSORING_SUPPORT_LIMITED")
    {
        reasons.push(
            "censoring support, positivity or weight bounds are not qualified for this horizon"
                .into(),
        );
        return Ok(report(
            &report_context,
            EligibilityState::NotEvaluable,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }

    if domain_status == "UNKNOWN" {
        reasons.push("operating-domain support is unknown".into());
        return Ok(report(
            &report_context,
            EligibilityState::NotEvaluable,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }
    let outside_domain = domain_status == "OUT_OF_DOMAIN"
        || material != validated_material
        || route != validated_route
        || ambient < min_temperature
        || ambient > max_temperature
        || duty_cycle > maximum_duty
        || flags.contains("DOMAIN_SHIFT");
    if outside_domain {
        reasons.push(
            "material, route, temperature or duty cycle is outside calibrated domain support"
                .into(),
        );
        return Ok(report(
            &report_context,
            EligibilityState::OutOfDomain,
            RecommendedAction::ConservativeFallbackNoUprating,
            reasons,
        ));
    }

    let risk_exceeded = target_cif > maximum_risk;
    if risk_exceeded {
        reasons.push(format!(
            "target-event cumulative incidence exceeds the configured dispatch risk budget; maintenance lead time is {maintenance_lead_time} hours"
        ));
        let action = if decision_mode == "HARD_RISK_CAP" {
            RecommendedAction::ExcludeFromPlannedServiceHorizon
        } else {
            RecommendedAction::DerateOrReserveMaintenance
        };
        return Ok(report(
            &report_context,
            EligibilityState::PolicyRiskExceeded,
            action,
            reasons,
        ));
    }

    reasons
        .push("forecast evidence gates pass; result may inform supervisory allocation only".into());
    Ok(report(
        &report_context,
        EligibilityState::DecisionSupportEligible,
        RecommendedAction::MayInformAllocationOnly,
        reasons,
    ))
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-health-dispatch-v0/health-dispatch-eligibility.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic health-dispatch fixture JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    fn promote_fixture_to_measured_for_policy_unit_test(run: &mut Value) {
        run["evidence_kind"] = Value::String("MEASURED".into());
        run["quality_flags"] = Value::Array(vec![]);
    }

    #[test]
    fn rejects_missing_declared_competing_cause_cif() {
        let mut run = fixture();
        run["forecast"]["cause_specific_cumulative_incidence"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("target event and every declared competing event")
        );
    }

    #[test]
    fn rejects_cif_vector_that_does_not_sum_with_survival_to_one() {
        let mut run = fixture();
        run["forecast"]["cause_specific_cumulative_incidence"][1]["probability"] = Value::from(0.5);
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("must sum to 1")
        );
    }

    #[test]
    fn target_event_cif_must_match_selected_cause_entry() {
        let mut run = fixture();
        run["forecast"]["target_event_cif"] = Value::from(0.04);
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("must equal the matching cause-specific CIF")
        );
    }

    #[test]
    fn target_and_competing_event_codes_must_be_disjoint() {
        let mut run = fixture();
        run["forecast"]["competing_event_codes"] =
            Value::Array(vec![Value::String("STRUCTURAL_CRACK_LIMIT".into())]);
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("must be disjoint")
        );
    }

    #[test]
    fn synthetic_fixture_is_never_operationally_eligible() {
        let report = evaluate_eligibility(&fixture()).unwrap();
        assert_eq!(report.state, EligibilityState::SyntheticOnlyNotEligible);
        assert!(!report.forecast_eligible_for_decision_support);
        assert_eq!(
            report.recommended_action,
            RecommendedAction::ConservativeFallbackNoUprating
        );
    }

    #[test]
    fn fully_supported_measured_style_receipt_is_eligible_for_decision_support() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::DecisionSupportEligible);
        assert!(report.forecast_eligible_for_decision_support);
        assert!(!report.risk_policy_exceeded);
        assert_eq!(
            report.recommended_action,
            RecommendedAction::MayInformAllocationOnly
        );
    }

    #[test]
    fn valid_forecast_above_soft_risk_budget_recommends_derating() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["forecast"]["target_event_cif"] = Value::from(0.08);
        run["forecast"]["event_free_survival"] = Value::from(0.86);
        run["forecast"]["cause_specific_cumulative_incidence"][0]["probability"] =
            Value::from(0.08);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::PolicyRiskExceeded);
        assert!(report.forecast_eligible_for_decision_support);
        assert_eq!(
            report.recommended_action,
            RecommendedAction::DerateOrReserveMaintenance
        );
    }

    #[test]
    fn hard_risk_cap_excludes_asset_from_planned_service_horizon() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["forecast"]["target_event_cif"] = Value::from(0.08);
        run["forecast"]["event_free_survival"] = Value::from(0.86);
        run["forecast"]["cause_specific_cumulative_incidence"][0]["probability"] =
            Value::from(0.08);
        run["dispatch_policy"]["decision_mode"] = Value::String("HARD_RISK_CAP".into());
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::PolicyRiskExceeded);
        assert_eq!(
            report.recommended_action,
            RecommendedAction::ExcludeFromPlannedServiceHorizon
        );
    }

    #[test]
    fn hard_limit_failure_overrides_eligible_risk_forecast() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["hard_limits"]["thermal_limits_pass"] = Value::Bool(false);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::HardLimitViolation);
        assert!(!report.forecast_eligible_for_decision_support);
        assert_eq!(
            report.recommended_action,
            RecommendedAction::DeenergizeAndEscalate
        );
    }

    #[test]
    fn insufficient_censoring_positivity_is_not_evaluable() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["censoring_support"]["minimum_g_observed"] = Value::from(0.05);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
        assert!(!report.forecast_eligible_for_decision_support);
    }

    #[test]
    fn excessive_ipcw_weight_is_not_evaluable() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["censoring_support"]["maximum_ipcw_weight"] = Value::from(20.0);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn domain_shift_blocks_forecast_integration() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["domain_support"]["ambient_temperature_c"] = Value::from(65.0);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::OutOfDomain);
        assert_eq!(
            report.recommended_action,
            RecommendedAction::ConservativeFallbackNoUprating
        );
    }

    #[test]
    fn material_lot_mismatch_is_out_of_domain() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["domain_support"]["material_family"] = Value::String("PG64-22".into());
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::OutOfDomain);
    }

    #[test]
    fn calibration_horizon_cannot_be_extrapolated() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["forecast"]["calibration_horizon_max_hours"] = Value::from(500.0);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn hazard_cif_incoherence_blocks_forecast() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["forecast"]["hazard_cif_coherence_verified"] = Value::Bool(false);
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn missing_source_or_calibration_digest_is_not_evaluable() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["forecast"]["calibration_receipt_sha256"] = Value::String("bad".into());
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn unknown_or_dependent_censoring_is_not_evaluable() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["censoring_support"]["assumption"] = Value::String("UNKNOWN_OR_DEPENDENT".into());
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn published_measured_forecast_requires_source_locator() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["evidence_kind"] = Value::String("PUBLISHED_MEASURED".into());
        run["forecast"]["source_locator"] = Value::Null;
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn unrecognized_domain_support_status_fails_closed() {
        let mut run = fixture();
        run["domain_support"]["support_status"] = Value::String("MAYBE_SUPPORTED".into());
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("unknown operating-domain support status")
        );
    }

    #[test]
    fn missing_calibration_population_id_is_rejected() {
        let mut run = fixture();
        run["forecast"]["calibration_population_id"] = Value::String(" ".into());
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("must be a non-empty string")
        );
    }

    #[test]
    fn unknown_domain_support_is_not_evaluable_not_out_of_domain() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["domain_support"]["support_status"] = Value::String("UNKNOWN".into());
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn policy_digest_is_required_for_health_risk_budget() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["dispatch_policy"]["policy_sha256"] = Value::String("bad".into());
        let report = evaluate_eligibility(&run).unwrap();
        assert_eq!(report.state, EligibilityState::NotEvaluable);
    }

    #[test]
    fn inconsistent_service_horizon_is_rejected() {
        let mut run = fixture();
        promote_fixture_to_measured_for_policy_unit_test(&mut run);
        run["dispatch_policy"]["planned_service_horizon_hours"] = Value::from(360.0);
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("must equal the forecast horizon")
        );
    }

    #[test]
    fn unknown_properties_fail_closed() {
        let mut run = fixture();
        run["forecast"]["future_risk_adjustment"] = Value::from(0.01);
        assert!(
            evaluate_eligibility(&run)
                .unwrap_err()
                .contains("unknown property")
        );
    }
}
