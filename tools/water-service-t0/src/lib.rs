// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const INPUT_CASE_COUNT: usize = 23;
pub const ORACLE_SCHEMA_VERSION: &str = "water-service-t0-oracle-v0.1";
pub const CLAIM_CEILING: &str =
    "arithmetic_and_contract_semantics_only_no_physical_or_health_claim";

fn at<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Value, String> {
    let mut current = value;
    for key in path {
        current = if let Some(array) = current.as_array() {
            let index = key
                .parse::<usize>()
                .map_err(|_| format!("expected numeric array index at {}", path.join(".")))?;
            array.get(index)
        } else {
            current.get(*key)
        }
        .ok_or_else(|| format!("missing JSON field: {}", path.join(".")))?;
    }
    Ok(current)
}

fn number(value: &Value, path: &[&str]) -> Result<f64, String> {
    at(value, path)?
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("expected finite number at {}", path.join(".")))
}

fn string(value: &Value, path: &[&str]) -> Result<String, String> {
    at(value, path)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("expected string at {}", path.join(".")))
}

fn boolean(value: &Value, path: &[&str]) -> Result<bool, String> {
    at(value, path)?
        .as_bool()
        .ok_or_else(|| format!("expected boolean at {}", path.join(".")))
}

fn close(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1.0e-9 * scale
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => close(a, b),
            _ => false,
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b.iter())
                    .all(|(left, right)| compare_json(left, right))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| compare_json(value, other)))
        }
        _ => actual == expected,
    }
}

fn case_id(case: &Value) -> Result<String, String> {
    string(case, &["case_id"])
}

fn compute_pump(case: &Value) -> Result<Value, String> {
    let id = case_id(case)?;
    let i = at(case, &["inputs"])?;
    match id.as_str() {
        "PS-T0-001-single-pump-known-answer" => {
            let rho = number(i, &["water_density_kg_m3"])?;
            let g = number(i, &["gravity_m_s2"])?;
            let q = number(i, &["flow_m3_s"])?;
            let head = number(i, &["total_dynamic_head_m"])?;
            let electrical = number(i, &["electrical_input_power_w"])?;
            if rho <= 0.0 || g <= 0.0 || q < 0.0 || head < 0.0 || electrical <= 0.0 {
                return Err(format!("{id}: invalid physical input"));
            }
            let hydraulic = rho * g * q * head;
            let efficiency = hydraulic / electrical;
            if !(0.0..=1.0).contains(&efficiency) || efficiency == 0.0 {
                return Err(format!("{id}: efficiency outside (0, 1]"));
            }
            let specific_energy = rho * g * head / (3.6e6 * efficiency);
            Ok(json!({
                "hydraulic_power_w": hydraulic,
                "wire_to_water_efficiency": efficiency,
                "electrical_specific_energy_kwh_m3": specific_energy
            }))
        }
        "PS-T0-002-system-curve-intersection" => {
            let h0 = number(i, &["pump_curve", "shutoff_head_m"])?;
            let kp = number(i, &["pump_curve", "quadratic_coefficient_m_per_m6"])?;
            let hs = number(i, &["system_curve", "static_head_m"])?;
            let ks = number(i, &["system_curve", "quadratic_coefficient_m_per_m6"])?;
            if h0 <= hs || kp < 0.0 || ks < 0.0 || kp + ks <= 0.0 {
                return Err(format!("{id}: no physical positive-flow intersection"));
            }
            let q = ((h0 - hs) / (kp + ks)).sqrt();
            let system_head = hs + ks * q * q;
            let pump_head = h0 - kp * q * q;
            Ok(json!({
                "flow_m3_s": q,
                "head_m": system_head,
                "residual_m": (pump_head - system_head).abs()
            }))
        }
        "PS-T0-003-two-identical-pumps-in-parallel" => {
            let h0 = number(i, &["single_pump_curve", "shutoff_head_m"])?;
            let kp = number(i, &["single_pump_curve", "quadratic_coefficient_m_per_m6"])?;
            let hs = number(i, &["system_curve", "static_head_m"])?;
            let ks = number(i, &["system_curve", "quadratic_coefficient_m_per_m6"])?;
            let count = number(i, &["pump_count"])?;
            if count < 1.0 || count.fract() != 0.0 || h0 <= hs || kp < 0.0 || ks < 0.0 {
                return Err(format!("{id}: invalid parallel-pump profile"));
            }
            let count_squared = count * count;
            let q_total = ((h0 - hs) / (kp / count_squared + ks)).sqrt();
            let q_each = q_total / count;
            let head = hs + ks * q_total * q_total;
            Ok(json!({
                "total_flow_m3_s": q_total,
                "flow_per_pump_m3_s": q_each,
                "common_head_m": head,
                "flow_split_residual_m3_s": (q_total - q_each * count).abs()
            }))
        }
        "PS-T0-004-minimum-service-pressure-not-met" => {
            let measured_head = number(i, &["measured_head_m"])?;
            let minimum_head = number(i, &["minimum_service_head_m"])?;
            let measured_flow = number(i, &["measured_flow_m3_s"])?;
            let minimum_flow = number(i, &["minimum_service_flow_m3_s"])?;
            let feasible = measured_head >= minimum_head && measured_flow >= minimum_flow;
            Ok(json!({
                "service_disposition": if feasible { "FEASIBLE" } else { "INFEASIBLE" },
                "reason": if measured_head < minimum_head {
                    "MINIMUM_SERVICE_HEAD_NOT_MET"
                } else if measured_flow < minimum_flow {
                    "MINIMUM_SERVICE_FLOW_NOT_MET"
                } else {
                    "SERVICE_FLOORS_MET"
                },
                "energy_credit_eligible": feasible
            }))
        }
        "PS-T0-005-commanded-on-but-no-measured-flow" => {
            let command = string(i, &["pump_command"])?;
            let flow = number(i, &["measured_flow_m3_s"])?;
            let disposition = if command == "ON" && flow == 0.0 {
                "NO_DELIVERY"
            } else if flow > 0.0 {
                "FLOW_OBSERVED"
            } else {
                "INDETERMINATE"
            };
            Ok(json!({
                "delivery_disposition": disposition,
                "command_is_not_flow_evidence": command == "ON" && flow == 0.0
            }))
        }
        "PS-T0-006-critical-pressure-observation-missing" => {
            let pressure_missing = at(i, &["measured_delivery_pressure"])?.is_null();
            let feasible = !pressure_missing;
            Ok(json!({
                "service_disposition": if pressure_missing {
                    "INDETERMINATE"
                } else {
                    "ASSESSABLE"
                },
                "schedule_feasible": feasible
            }))
        }
        "PS-T0-007-storage-reserve-during-pump-outage" => {
            let start = number(i, &["usable_storage_start_m3"])?;
            let demand = number(i, &["demand_m3_s"])?;
            let duration = number(i, &["outage_duration_s"])?;
            let reserve = number(i, &["minimum_reserve_m3"])?;
            let demand_volume = demand * duration;
            let end = start - demand_volume;
            let service = start >= demand_volume && end >= reserve;
            Ok(json!({
                "demand_volume_m3": demand_volume,
                "storage_end_m3": end,
                "service_disposition": if service {
                    "SERVICE_MAINTAINED_WITHIN_DECLARED_RESERVE"
                } else {
                    "SERVICE_RESERVE_VIOLATED"
                }
            }))
        }
        "PS-T0-008-out-of-domain-pump-curve" => {
            let lower = number(i, &["curve_flow_domain_m3_s", "0"])?;
            let upper = number(i, &["curve_flow_domain_m3_s", "1"])?;
            let requested = number(i, &["requested_flow_m3_s"])?;
            let extrapolation = boolean(i, &["extrapolation_allowed"])?;
            let unsupported = requested < lower || requested > upper;
            Ok(json!({
                "calculation_disposition": if unsupported && !extrapolation {
                    "UNSUPPORTED"
                } else {
                    "IN_DOMAIN"
                },
                "schedule_feasible": !unsupported || extrapolation
            }))
        }
        "PS-T0-009-vfd-versus-fixed-speed-throttling" => {
            let q = number(i, &["flow_m3_s"])?;
            let rho = number(i, &["water_density_kg_m3"])?;
            let g = number(i, &["gravity_m_s2"])?;
            let static_head = number(i, &["static_head_m"])?;
            let system_k = number(i, &["system_loss_coefficient_m_per_m6"])?;
            let h0 = number(i, &["fixed_speed_pump_curve", "shutoff_head_m"])?;
            let pump_k = number(
                i,
                &["fixed_speed_pump_curve", "quadratic_coefficient_m_per_m6"],
            )?;
            let vfd_h0 = number(i, &["vfd_curve", "reference_speed_shutoff_head_m"])?;
            let vfd_k = number(i, &["vfd_curve", "quadratic_coefficient_m_per_m6"])?;
            let eta = number(i, &["assumed_total_wire_to_water_efficiency_both_modes"])?;
            if q <= 0.0 || rho <= 0.0 || g <= 0.0 || eta <= 0.0 || eta > 1.0 {
                return Err(format!("{id}: invalid VFD comparison input"));
            }
            let system_head = static_head + system_k * q * q;
            let fixed_head = h0 - pump_k * q * q;
            let throttle_loss = fixed_head - system_head;
            let speed_ratio = ((system_head + vfd_k * q * q) / vfd_h0).sqrt();
            let vfd_head = vfd_h0 * speed_ratio * speed_ratio - vfd_k * q * q;
            let e_fixed = rho * g * fixed_head / (3.6e6 * eta);
            let e_vfd = rho * g * vfd_head / (3.6e6 * eta);
            let reduction = if e_fixed > 0.0 {
                (e_fixed - e_vfd) / e_fixed
            } else {
                return Err(format!("{id}: non-positive fixed-speed energy"));
            };
            Ok(json!({
                "system_head_m": system_head,
                "fixed_speed_pump_head_m": fixed_head,
                "fixed_speed_throttle_loss_m": throttle_loss,
                "vfd_speed_ratio": speed_ratio,
                "vfd_pump_head_m": vfd_head,
                "fixed_speed_specific_energy_kwh_m3": e_fixed,
                "vfd_specific_energy_kwh_m3": e_vfd,
                "idealized_energy_reduction_fraction": reduction
            }))
        }
        _ => Err(format!("unrecognized pump case: {id}")),
    }
}

fn compute_potable(case: &Value) -> Result<Value, String> {
    let id = case_id(case)?;
    let i = at(case, &["inputs"])?;
    match id.as_str() {
        "PW-T0-001-complete-water-and-marker-mass-boundary" => {
            let feed_water = number(i, &["feed_water_l"])?;
            let water_out = [
                "product_water_l",
                "reject_water_l",
                "backwash_water_l",
                "sludge_liquor_water_l",
                "evaporation_or_other_water_loss_l",
            ]
            .iter()
            .map(|key| number(i, &[*key]))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .sum::<f64>();
            let marker_in = feed_water * number(i, &["feed_marker_concentration_mg_l"])?;
            let marker_out = number(i, &["product_water_l"])?
                * number(i, &["product_marker_concentration_mg_l"])?
                + number(i, &["reject_water_l"])?
                    * number(i, &["reject_marker_concentration_mg_l"])?
                + number(i, &["backwash_water_l"])?
                    * number(i, &["backwash_marker_concentration_mg_l"])?
                + number(i, &["sludge_liquor_water_l"])?
                    * number(i, &["sludge_liquor_marker_concentration_mg_l"])?
                + number(i, &["captured_dry_residual_marker_mg"])?;
            if feed_water < 0.0 || water_out < 0.0 {
                return Err(format!("{id}: negative water quantity"));
            }
            Ok(json!({
                "water_in_l": feed_water,
                "water_out_l": water_out,
                "water_balance_residual_l": (feed_water - water_out).abs(),
                "marker_in_mg": marker_in,
                "marker_accounted_mg": marker_out,
                "marker_balance_residual_mg": (marker_in - marker_out).abs(),
                "evidence_disposition": "SYNTHETIC_ARITHMETIC_ONLY"
            }))
        }
        "PW-T0-002-missing-required-analyte" => {
            let required = at(i, &["profile_required_analytes"])?
                .as_array()
                .ok_or_else(|| format!("{id}: required analytes must be an array"))?;
            let observed = at(i, &["observed_analytes"])?
                .as_array()
                .ok_or_else(|| format!("{id}: observed analytes must be an array"))?;
            let complete = required.iter().all(|req| observed.contains(req));
            Ok(json!({
                "assessment_disposition": if complete { "SYNTHETIC_PASS" } else { "INDETERMINATE" },
                "release_allowed": false
            }))
        }
        "PW-T0-003-treatment-bypass" => {
            let bypass = boolean(i, &["declared_train_has_stage_bypass"])?
                && boolean(i, &["bypass_observed"])?
                && boolean(i, &["profile_requires_stage"])?;
            Ok(json!({
                "assessment_disposition": if bypass { "INDETERMINATE" } else { "ASSESSABLE" },
                "release_allowed": false
            }))
        }
        "PW-T0-004-breakthrough-at-end-of-life" => {
            let threshold = number(i, &["synthetic_upper_limit_mg_l"])?;
            let value = number(i, &["measured_marker_alpha_mg_l"])?;
            let uncertainty = number(i, &["absolute_uncertainty_mg_l"])?;
            if uncertainty < 0.0 {
                return Err(format!("{id}: negative uncertainty"));
            }
            let lower = value - uncertainty;
            let upper = value + uncertainty;
            let disposition = if lower > threshold {
                "SYNTHETIC_NONCOMPLIANT"
            } else if upper <= threshold {
                "SYNTHETIC_PASS"
            } else {
                "SYNTHETIC_INDETERMINATE"
            };
            Ok(json!({
                "assessment_disposition": disposition,
                "release_allowed": false
            }))
        }
        "PW-T0-005-source-water-out-of-envelope" => {
            let admitted = boolean(i, &["source_water_profile_admitted"])?;
            Ok(json!({
                "assessment_disposition": if admitted { "ASSESSABLE" } else { "UNSUPPORTED" },
                "release_allowed": false
            }))
        }
        "PW-T0-006-stale-method-or-calibration" => {
            let method_current = boolean(i, &["critical_method_current"])?;
            let calibration_current = boolean(i, &["critical_instrument_calibration_current"])?;
            let current = method_current && calibration_current;
            Ok(json!({
                "assessment_disposition": if current { "ASSESSABLE" } else { "INDETERMINATE" },
                "release_allowed": false
            }))
        }
        "PW-T0-007-storage-recontamination" => {
            let storage_outlet = string(i, &["storage_outlet_disposition"])?;
            let failed = storage_outlet == "SYNTHETIC_NONCOMPLIANT";
            Ok(json!({
                "point_of_use_service_disposition": if failed {
                    "SYNTHETIC_NONCOMPLIANT"
                } else {
                    storage_outlet.as_str()
                },
                "release_allowed": false
            }))
        }
        "PW-T0-008-reject-backwash-residual-omission" => {
            let measured = boolean(i, &["residual_stream_measured"])?;
            Ok(json!({
                "volume_closure_disposition": if measured { "ASSESSABLE" } else { "INCOMPLETE" },
                "recovery_claim_allowed": false
            }))
        }
        _ => Err(format!("unrecognized potable treatment case: {id}")),
    }
}

fn compute_diagnostics(case: &Value) -> Result<Value, String> {
    let id = case_id(case)?;
    let i = at(case, &["inputs"])?;
    match id.as_str() {
        "WDN-T0-001-legitimate-demand-shift" => {
            let demand_changed = boolean(i, &["declared_legitimate_demand_change"])?;
            let leak_evidence = boolean(i, &["independent_leak_evidence"])?;
            Ok(json!({
                "diagnostic_disposition": if demand_changed && !leak_evidence {
                    "ALTERNATIVE_CAUSE_CANDIDATE"
                } else {
                    "REQUIRES_REVIEW"
                },
                "confirmed_leak": false,
                "physical_action_allowed": false
            }))
        }
        "WDN-T0-002-pump-valve-transient-confounder" => {
            let transient = boolean(i, &["pressure_transient"])?;
            let event_recorded = boolean(i, &["pump_or_valve_event_recorded"])?;
            let steady_only = boolean(i, &["steady_state_leak_model_only"])?;
            let indeterminate = transient && event_recorded && steady_only;
            Ok(json!({
                "diagnostic_disposition": if indeterminate {
                    "INDETERMINATE_PENDING_TRANSIENT_REVIEW"
                } else {
                    "REQUIRES_REVIEW"
                },
                "confirmed_leak": false
            }))
        }
        "WDN-T0-003-insufficient-localization-observability" => {
            let ambiguous = boolean(i, &["network_segment_ambiguity"])?;
            let sensors = number(i, &["independent_sensor_count"])?;
            let indeterminate = ambiguous || sensors < 2.0;
            Ok(json!({
                "localization_disposition": if indeterminate {
                    "INDETERMINATE"
                } else {
                    "ASSESSABLE"
                },
                "high_confidence_location_allowed": !indeterminate
            }))
        }
        "WDN-T0-004-common-mode-sensor-drift" => {
            let shared = boolean(i, &["pressure_sensor_shared_calibration_dependency"])?;
            let independent = boolean(i, &["independent_reference_available"])?;
            let common_mode = boolean(i, &["flow_sensor_drift"])? && shared && !independent;
            Ok(json!({
                "diagnostic_disposition": if common_mode { "INDETERMINATE" } else { "ASSESSABLE" },
                "sensor_consensus_is_not_independent": common_mode
            }))
        }
        "WDN-T0-005-missing-telemetry-no-leak-absence" => {
            let unobserved = number(i, &["unobserved_pipe_fraction"])?;
            let pressure_missing = boolean(i, &["critical_pressure_sensor_missing"])?;
            let partially_unobserved = unobserved > 0.0 || pressure_missing;
            Ok(json!({
                "network_disposition": if partially_unobserved {
                    "PARTIALLY_UNOBSERVED"
                } else {
                    "OBSERVABLE_WITHIN_PROFILE"
                },
                "leak_absence_claim_allowed": !partially_unobserved
            }))
        }
        "WDN-T0-006-repair-claim-without-outcome-observation" => {
            let completion = boolean(i, &["maintenance_completion_reported"])?;
            let outcome = boolean(i, &["post_repair_flow_pressure_evidence"])?;
            let reinspection = boolean(i, &["independent_reinspection"])?;
            let verified = completion && outcome && reinspection;
            Ok(json!({
                "repair_disposition": if verified { "VERIFIED" } else { "NOT_VERIFIED" },
                "saved_water_claim_allowed": verified
            }))
        }
        _ => Err(format!("unrecognized network diagnosis case: {id}")),
    }
}

fn collect_cases<'a>(input: &'a Value, family: &str) -> Result<&'a Vec<Value>, String> {
    at(input, &[family])?
        .as_array()
        .ok_or_else(|| format!("input family {family} must be an array"))
}

fn string_ids(value: &Value, path: &[&str]) -> Result<Vec<String>, String> {
    at(value, path)?
        .as_array()
        .ok_or_else(|| format!("{} must be an array", path.join(".")))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{} contains a non-string ID", path.join(".")))
        })
        .collect()
}

fn validate_ids(actual: &[String], expected: &[String], label: &str) -> Result<(), String> {
    let actual_set: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    let expected_set: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    if actual.len() != actual_set.len() {
        return Err(format!("{label}: duplicate case IDs"));
    }
    if actual_set != expected_set {
        return Err(format!("{label}: manifest/input case IDs differ"));
    }
    Ok(())
}

pub fn evaluate(
    inputs: &Value,
    manifest: &Value,
    network_manifest: &Value,
    oracle: &Value,
) -> Result<Value, String> {
    let manifest_ceiling = string(manifest, &["evidence_ceiling"])?;
    let network_ceiling = string(network_manifest, &["evidence_ceiling"])?;
    if string(inputs, &["schema_version"])? != "0.1.0"
        || string(manifest, &["schema_version"])? != "0.1.0"
        || string(network_manifest, &["schema_version"])? != "0.1.0"
        || string(inputs, &["evidence_class"])? != "synthetic_only"
        || string(inputs, &["evidence_ceiling"])? != CLAIM_CEILING
        || !manifest_ceiling.contains("no_physical_qualification")
        || !manifest_ceiling.contains("no_health_or_regulatory_claim")
        || !network_ceiling.contains("no_detector_or_field_network_qualified")
        || !network_ceiling.contains("no_verified_savings_or_repair_claim")
        || string(oracle, &["schema_version"])? != ORACLE_SCHEMA_VERSION
        || string(oracle, &["evidence_class"])? != "synthetic_only"
        || string(oracle, &["evidence_ceiling"])? != CLAIM_CEILING
        || !boolean(oracle, &["independently_reviewed_expected_outputs"])?
    {
        return Err("input/manifest/oracle identity or evidence ceiling mismatch".to_owned());
    }

    let pump = collect_cases(inputs, "pump_cases")?;
    let potable = collect_cases(inputs, "potable_treatment_cases")?;
    let diagnostics = collect_cases(inputs, "network_diagnostics_cases")?;

    let pump_ids = pump.iter().map(case_id).collect::<Result<Vec<_>, _>>()?;
    let potable_ids = potable.iter().map(case_id).collect::<Result<Vec<_>, _>>()?;
    let diagnostic_ids = diagnostics
        .iter()
        .map(case_id)
        .collect::<Result<Vec<_>, _>>()?;

    validate_ids(
        &pump_ids,
        &string_ids(manifest, &["pump_system", "proposed_case_ids"])?,
        "pump",
    )?;
    validate_ids(
        &potable_ids,
        &string_ids(manifest, &["potable_treatment", "proposed_case_ids"])?,
        "potable treatment",
    )?;
    validate_ids(
        &diagnostic_ids,
        &string_ids(
            manifest,
            &["pressurized_leak_diagnostics", "proposed_case_ids"],
        )?,
        "pressurized diagnostics",
    )?;
    validate_ids(
        &diagnostic_ids,
        &string_ids(network_manifest, &["proposed_t0_cases"])?,
        "network diagnostics manifest",
    )?;

    if pump.len() != 9 || potable.len() != 8 || diagnostics.len() != 6 {
        return Err(format!(
            "unexpected family counts: pump={}, potable={}, diagnostics={}",
            pump.len(),
            potable.len(),
            diagnostics.len()
        ));
    }

    let oracle_cases = at(oracle, &["cases"])?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    if oracle_cases.len() != INPUT_CASE_COUNT {
        return Err(format!(
            "expected {INPUT_CASE_COUNT} oracle rows, got {}",
            oracle_cases.len()
        ));
    }

    let mut expected_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = string(row, &["case_id"])?;
        if expected_by_id.insert(id.clone(), row).is_some() {
            return Err(format!("duplicate oracle case ID: {id}"));
        }
    }

    let mut results = Vec::with_capacity(INPUT_CASE_COUNT);
    let mut mismatches = Vec::new();

    for (family, cases, compute) in [
        (
            "pump_cases",
            pump,
            compute_pump as fn(&Value) -> Result<Value, String>,
        ),
        (
            "potable_treatment_cases",
            potable,
            compute_potable as fn(&Value) -> Result<Value, String>,
        ),
        (
            "network_diagnostics_cases",
            diagnostics,
            compute_diagnostics as fn(&Value) -> Result<Value, String>,
        ),
    ] {
        for case in cases {
            let id = case_id(case)?;
            let actual = compute(case)?;
            let oracle_row = expected_by_id
                .get(&id)
                .ok_or_else(|| format!("oracle missing case {id}"))?;
            let oracle_family = string(oracle_row, &["family"])?;
            let expected = at(oracle_row, &["expected"])?;
            let family_matches = oracle_family == family;
            let matches = family_matches && compare_json(&actual, expected);
            let result = json!({
                "case_id": id,
                "family": family,
                "matched": matches,
                "computed": actual
            });
            if !matches {
                mismatches.push(json!({
                    "case_id": id,
                    "family": family,
                    "expected_family": oracle_family,
                    "expected": expected,
                    "actual": actual
                }));
            }
            results.push(result);
        }
    }

    if expected_by_id.len() != results.len() {
        return Err(format!(
            "input/oracle identity mismatch: input={}, oracle={}",
            results.len(),
            expected_by_id.len()
        ));
    }
    results.sort_by(|a, b| {
        a.get("case_id")
            .and_then(Value::as_str)
            .cmp(&b.get("case_id").and_then(Value::as_str))
    });

    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version": "water-service-t0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_only",
        "claim_ceiling": CLAIM_CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "Synthetic arithmetic and contract semantics only; no pump, treatment train, sensor, network, laboratory or physical process is qualified.",
            "No water safety, potability, treatment efficacy, pressure-service, leakage or saved-water field claim is established.",
            "A benchmark result does not certify a product, authorize physical actuation, or replace operator and regulator decisions.",
            "The variable-speed comparison assumes equal total efficiency and is an analytic illustration, not measured savings."
        ]
    }))
}

pub fn evaluate_lead_lag_rssct_f0(inputs: &Value, oracle: &Value) -> Result<Value, String> {
    const SCALING_CASES: usize = 6;
    const LEAD_LAG_CASES: usize = 5;
    const CEILING: &str =
        "synthetic_scaling_arithmetic_and_sample_state_only_no_adsorbent_or_water_quality_claim";

    if string(inputs, &["schema_version"])? != "lead-lag-rssct-f0-cases-v0.1"
        || string(inputs, &["evidence_class"])? != "synthetic_adsorption_scaling_and_lead_lag_state"
        || string(inputs, &["evidence_ceiling"])? != CEILING
        || string(oracle, &["schema_version"])? != "lead-lag-rssct-f0-oracle-v0.1"
        || string(oracle, &["evidence_class"])? != "synthetic_adsorption_scaling_and_lead_lag_state"
        || string(oracle, &["evidence_ceiling"])? != CEILING
    {
        return Err("lead-lag RSSCT input/oracle identity or evidence ceiling mismatch".to_owned());
    }

    let scaling_cases = at(inputs, &["scaling_cases"])?
        .as_array()
        .ok_or_else(|| "RSSCT scaling cases must be an array".to_owned())?;
    let lead_lag_cases = at(inputs, &["lead_lag_cases"])?
        .as_array()
        .ok_or_else(|| "lead-lag cases must be an array".to_owned())?;
    let oracle_scaling = at(oracle, &["scaling_cases"])?
        .as_array()
        .ok_or_else(|| "RSSCT scaling oracle must be an array".to_owned())?;
    let oracle_lead_lag = at(oracle, &["lead_lag_cases"])?
        .as_array()
        .ok_or_else(|| "lead-lag oracle must be an array".to_owned())?;
    if scaling_cases.len() != SCALING_CASES
        || oracle_scaling.len() != SCALING_CASES
        || lead_lag_cases.len() != LEAD_LAG_CASES
        || oracle_lead_lag.len() != LEAD_LAG_CASES
    {
        return Err(format!(
            "lead-lag RSSCT expected {SCALING_CASES} scaling and {LEAD_LAG_CASES} lead-lag cases"
        ));
    }

    let mut scaling_oracle_by_id = BTreeMap::new();
    for row in oracle_scaling {
        let id = string(row, &["case_id"])?;
        if scaling_oracle_by_id.insert(id.clone(), row).is_some() {
            return Err(format!("duplicate RSSCT scaling oracle ID: {id}"));
        }
    }
    let mut lead_oracle_by_id = BTreeMap::new();
    for row in oracle_lead_lag {
        let id = string(row, &["case_id"])?;
        if lead_oracle_by_id.insert(id.clone(), row).is_some() {
            return Err(format!("duplicate lead-lag oracle ID: {id}"));
        }
    }

    let mut seen_scaling = BTreeSet::new();
    let mut scaling_results = Vec::with_capacity(SCALING_CASES);
    let mut mismatches = Vec::new();
    for case in scaling_cases {
        let id = case_id(case)?;
        if !seen_scaling.insert(id.clone()) {
            return Err(format!("duplicate RSSCT scaling input ID: {id}"));
        }
        let sf = number(case, &["particle_size_scale_factor"])?;
        let exponent = number(case, &["diffusivity_exponent"])?;
        let full_duration = number(case, &["full_scale_duration_days"])?;
        if sf <= 1.0 || full_duration <= 0.0 || !(exponent == 0.0 || exponent == 1.0) {
            return Err(format!("{id}: invalid RSSCT scaling domain"));
        }
        let ratio = sf.powf(exponent - 2.0);
        let small_duration = full_duration * ratio;
        if !ratio.is_finite() || !small_duration.is_finite() || ratio <= 0.0 {
            return Err(format!("{id}: invalid RSSCT scale result"));
        }
        let actual = json!({
            "ebct_and_time_ratio": ratio,
            "small_scale_duration_days": small_duration,
            "scaling_assumption": if exponent == 0.0 {
                "CONSTANT_DIFFUSIVITY"
            } else {
                "PROPORTIONAL_DIFFUSIVITY"
            }
        });
        let oracle_row = scaling_oracle_by_id
            .get(&id)
            .ok_or_else(|| format!("RSSCT scaling oracle missing case {id}"))?;
        let expected = at(oracle_row, &["expected"])?;
        let matched = compare_json(&actual, expected);
        if !matched {
            mismatches.push(json!({"case_id":id,"expected":expected,"actual":actual}));
        }
        scaling_results.push(json!({
            "case_id":id,
            "family":"RSSCT_SCALING_ARITHMETIC",
            "matched":matched,
            "computed":actual
        }));
    }
    if seen_scaling.len() != scaling_oracle_by_id.len() {
        return Err("RSSCT scaling input/oracle ID sets differ".to_owned());
    }

    let mut seen_lead = BTreeSet::new();
    let mut lead_results = Vec::with_capacity(LEAD_LAG_CASES);
    for case in lead_lag_cases {
        let id = case_id(case)?;
        if !seen_lead.insert(id.clone()) {
            return Err(format!("duplicate lead-lag input ID: {id}"));
        }
        let input = at(case, &["inputs"])?;
        let threshold = number(input, &["breakthrough_threshold_c_over_c0"])?;
        if threshold <= 0.0 {
            return Err(format!("{id}: breakthrough threshold must be positive"));
        }

        let interstage_state = string(input, &["lead_interstage_sample_state"])?;
        let interstage_binding = boolean(input, &["lead_interstage_claim_binding_valid"])?;
        let interstage_value = match at(input, &["lead_interstage_c_over_c0"])? {
            Value::Null => None,
            value => Some(
                value.as_f64()
                    .filter(|number| number.is_finite() && *number >= 0.0)
                    .ok_or_else(|| format!("{id}: invalid interstage C/C0"))?,
            ),
        };
        let interstage = match interstage_state.as_str() {
            "no_sample" => {
                if interstage_value.is_some() {
                    return Err(format!("{id}: no interstage sample must have null C/C0"));
                }
                "NO_CURRENT_CLAIM_SAMPLE"
            }
            "rejected_qc" => {
                if interstage_value.is_some() {
                    return Err(format!("{id}: rejected interstage sample must have null C/C0"));
                }
                "REJECTED_LAB_SAMPLE_NOT_CLAIM_EVIDENCE"
            }
            "accepted_below_lod" | "accepted_below_loq" => {
                if interstage_value.is_some() {
                    return Err(format!("{id}: censored interstage result must be null"));
                }
                "CENSORED_RESULT_NO_NUMERIC_COMPARISON"
            }
            "accepted_quantified" => {
                if !interstage_binding {
                    "SAMPLE_CLAIM_BINDING_INVALID"
                } else if let Some(value) = interstage_value {
                    if value >= threshold {
                        "BREAKTHROUGH_OBSERVED_AT_SAMPLE"
                    } else {
                        "BELOW_REGISTERED_THRESHOLD_AT_SAMPLE"
                    }
                } else {
                    "SAMPLE_VALUE_MISSING"
                }
            }
            _ => return Err(format!("{id}: unknown interstage sample state {interstage_state}")),
        };

        let final_state = string(input, &["final_outlet_sample_state"])?;
        let final_binding = boolean(input, &["final_outlet_claim_binding_valid"])?;
        let final_value = match at(input, &["final_outlet_c_over_c0"])? {
            Value::Null => None,
            value => Some(
                value.as_f64()
                    .filter(|number| number.is_finite() && *number >= 0.0)
                    .ok_or_else(|| format!("{id}: invalid final outlet C/C0"))?,
            ),
        };
        let final_outlet = match final_state.as_str() {
            "no_sample" => {
                if final_value.is_some() {
                    return Err(format!("{id}: no final sample must have null C/C0"));
                }
                "NO_CURRENT_CLAIM_SAMPLE"
            }
            "rejected_qc" => {
                if final_value.is_some() {
                    return Err(format!("{id}: rejected final sample must have null C/C0"));
                }
                "REJECTED_LAB_SAMPLE_NOT_CLAIM_EVIDENCE"
            }
            "accepted_below_lod" | "accepted_below_loq" => {
                if final_value.is_some() {
                    return Err(format!("{id}: censored final result must be null"));
                }
                "CENSORED_RESULT_NO_NUMERIC_COMPARISON"
            }
            "accepted_quantified" => {
                if !final_binding {
                    "SAMPLE_CLAIM_BINDING_INVALID"
                } else if let Some(value) = final_value {
                    if value >= threshold {
                        "BREAKTHROUGH_OBSERVED_AT_SAMPLE"
                    } else {
                        "BELOW_REGISTERED_THRESHOLD_AT_SAMPLE"
                    }
                } else {
                    "SAMPLE_VALUE_MISSING"
                }
            }
            _ => return Err(format!("{id}: unknown final sample state {final_state}")),
        };

        let guard_verified = boolean(input, &["guard_bed_identity_verified"])?;
        let history_continuous = boolean(input, &["service_history_continuous"])?;
        let interstage_breakthrough = interstage == "BREAKTHROUGH_OBSERVED_AT_SAMPLE";
        let final_breakthrough = final_outlet == "BREAKTHROUGH_OBSERVED_AT_SAMPLE";
        let guard = if !guard_verified || !history_continuous {
            "GUARD_BED_IDENTITY_OR_HISTORY_UNTRUSTED"
        } else if !interstage_breakthrough {
            "GUARD_CAPACITY_NOT_INFERABLE_FROM_CURRENT_INTERSTAGE_EVIDENCE"
        } else if final_outlet == "NO_CURRENT_CLAIM_SAMPLE"
            || final_outlet == "REJECTED_LAB_SAMPLE_NOT_CLAIM_EVIDENCE"
            || final_outlet == "CENSORED_RESULT_NO_NUMERIC_COMPARISON"
            || final_outlet == "SAMPLE_CLAIM_BINDING_INVALID"
        {
            "GUARD_FINAL_OUTLET_EVIDENCE_INSUFFICIENT"
        } else if final_breakthrough {
            "FINAL_OUTLET_BREAKTHROUGH_OBSERVED_AT_SAMPLE"
        } else {
            "FINAL_OUTLET_BELOW_REGISTERED_THRESHOLD_AT_SAMPLE"
        };
        let advisory = if guard_verified && history_continuous && interstage_breakthrough {
            "REVIEW_LEAD_BED_ROTATION_WITH_APPROVED_PROCEDURE"
        } else {
            "ABSTAIN_OR_HOLD"
        };
        let actual = json!({
            "interstage_disposition":interstage,
            "final_outlet_disposition":final_outlet,
            "guard_bed_evidence_disposition":guard,
            "maintenance_advisory":advisory,
            "automatic_bed_switch_authorized":false,
            "final_water_release_allowed":false,
            "water_safety_claim_allowed":false
        });
        let oracle_row = lead_oracle_by_id
            .get(&id)
            .ok_or_else(|| format!("lead-lag oracle missing case {id}"))?;
        let expected = at(oracle_row, &["expected"])?;
        let matched = compare_json(&actual, expected);
        if !matched {
            mismatches.push(json!({"case_id":id,"expected":expected,"actual":actual}));
        }
        lead_results.push(json!({
            "case_id":id,
            "family":"LEAD_LAG_SAMPLE_STATE",
            "matched":matched,
            "computed":actual
        }));
    }
    if seen_lead.len() != lead_oracle_by_id.len() {
        return Err("lead-lag input/oracle ID sets differ".to_owned());
    }

    scaling_results.sort_by(|a,b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    lead_results.sort_by(|a,b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let result_count = scaling_results.len() + lead_results.len();
    let matched_count = result_count - mismatches.len();
    Ok(json!({
        "schema_version":"lead-lag-rssct-f0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty() {"MatchedFrozenOracle"} else {"OracleMismatch"},
        "qualification_status":"NotRun",
        "evidence_class":"synthetic_adsorption_scaling_and_lead_lag_state",
        "evidence_ceiling":CEILING,
        "result_count":result_count,
        "matched_case_count":matched_count,
        "mismatches":mismatches,
        "scaling_results":scaling_results,
        "lead_lag_results":lead_results,
        "nonclaims":[
            "RSSCT arithmetic does not qualify CD or PD similarity for any media or source-water matrix.",
            "An interstage sample is not a final outlet sample.",
            "No sample or censored result is not evidence of no breakthrough.",
            "Lead-bed rotation is advisory only and requires an approved procedure.",
            "No final water release, potable-water safety, contaminant reduction, media life or pilot performance is qualified."
        ]
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Value {
        let text = match name {
            "inputs" => include_str!(
                "../../../research/water-service-engineering/t0-known-answer-vectors-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../research/water-service-engineering/benchmark-manifest-v0.1.json"
            ),
            "network" => include_str!(
                "../../../research/water-service-engineering/network-diagnostics-manifest-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../research/water-service-engineering/t0-known-answer-oracle-v0.1.json"
            ),
            "lead_lag_rssct_inputs" => include_str!(
                "../../../research/water-service-engineering/lead-lag-rssct-f0-cases-v0.1.json"
            ),
            "lead_lag_rssct_oracle" => include_str!(
                "../../../research/water-service-engineering/lead-lag-rssct-f0-oracle-v0.1.json"
            ),
            _ => panic!("unknown test fixture"),
        };
        serde_json::from_str(text).expect("fixture JSON must parse")
    }

    #[test]
    fn all_six_rssct_and_five_lead_lag_cases_match_oracle() {
        let result = evaluate_lead_lag_rssct_f0(
            &load("lead_lag_rssct_inputs"),
            &load("lead_lag_rssct_oracle"),
        )
        .expect("lead-lag RSSCT F0 must evaluate");
        assert_eq!(result["result_count"], 11);
        assert_eq!(result["matched_case_count"], 11);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert_eq!(result["mismatches"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn rssct_cd_and_pd_are_not_collapsed_into_one_prediction() {
        let result = evaluate_lead_lag_rssct_f0(
            &load("lead_lag_rssct_inputs"),
            &load("lead_lag_rssct_oracle"),
        )
        .expect("lead-lag RSSCT F0 must evaluate");
        let rows = result["scaling_results"].as_array().unwrap();
        let cd = rows.iter().find(|r| r["case_id"] == "RSSCT-F0-004-sf-5-CD").unwrap();
        let pd = rows.iter().find(|r| r["case_id"] == "RSSCT-F0-005-sf-5-PD").unwrap();
        assert!(
            cd["computed"]["small_scale_duration_days"]
                .as_f64()
                .unwrap()
                < pd["computed"]["small_scale_duration_days"]
                    .as_f64()
                    .unwrap()
        );
    }

    #[test]
    fn interstage_breakthrough_does_not_substitute_for_missing_final_sample() {
        let result = evaluate_lead_lag_rssct_f0(
            &load("lead_lag_rssct_inputs"),
            &load("lead_lag_rssct_oracle"),
        )
        .expect("lead-lag RSSCT F0 must evaluate");
        let row = result["lead_lag_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["case_id"] == "LEADLAG-F0-002-interstage-breakthrough-final-missing")
            .unwrap();
        assert_eq!(
            row["computed"]["guard_bed_evidence_disposition"],
            "GUARD_FINAL_OUTLET_EVIDENCE_INSUFFICIENT"
        );
    }

    #[test]
    fn lead_lag_never_authorizes_switch_or_final_release() {
        let result = evaluate_lead_lag_rssct_f0(
            &load("lead_lag_rssct_inputs"),
            &load("lead_lag_rssct_oracle"),
        )
        .expect("lead-lag RSSCT F0 must evaluate");
        for row in result["lead_lag_results"].as_array().unwrap() {
            assert_eq!(row["computed"]["automatic_bed_switch_authorized"], false);
            assert_eq!(row["computed"]["final_water_release_allowed"], false);
            assert_eq!(row["computed"]["water_safety_claim_allowed"], false);
        }
    }

    #[test]
    fn lost_guard_identity_forces_abstention() {
        let result = evaluate_lead_lag_rssct_f0(
            &load("lead_lag_rssct_inputs"),
            &load("lead_lag_rssct_oracle"),
        )
        .expect("lead-lag RSSCT F0 must evaluate");
        let row = result["lead_lag_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["case_id"] == "LEADLAG-F0-005-interstage-breakthrough-guard-identity-lost")
            .unwrap();
        assert_eq!(row["computed"]["maintenance_advisory"], "ABSTAIN_OR_HOLD");
    }

    #[test]
    fn lead_lag_oracle_mismatch_is_reported_not_hidden() {
        let mut oracle = load("lead_lag_rssct_oracle");
        oracle["lead_lag_cases"][0]["expected"]["maintenance_advisory"] = json!("SWITCH_NOW");
        let result = evaluate_lead_lag_rssct_f0(&load("lead_lag_rssct_inputs"), &oracle)
            .expect("oracle mismatch should produce a receipt");
        assert_eq!(result["verification_state"], "OracleMismatch");
        assert_eq!(result["matched_case_count"], 10);
        assert_eq!(result["mismatches"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn all_23_inputs_match_separate_frozen_oracle() {
        let result = evaluate(
            &load("inputs"),
            &load("manifest"),
            &load("network"),
            &load("oracle"),
        )
        .expect("evaluation must complete");
        assert_eq!(result["result_count"], 23);
        assert_eq!(result["matched_case_count"], 23);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert_eq!(result["mismatches"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn duplicate_input_id_fails_before_evaluation() {
        let mut inputs = load("inputs");
        let first = inputs["pump_cases"][0].clone();
        inputs["pump_cases"].as_array_mut().unwrap().push(first);
        let result = evaluate(
            &inputs,
            &load("manifest"),
            &load("network"),
            &load("oracle"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn missing_manifest_case_fails_closed() {
        let mut manifest = load("manifest");
        manifest["pump_system"]["proposed_case_ids"]
            .as_array_mut()
            .unwrap()
            .pop();
        let result = evaluate(
            &load("inputs"),
            &manifest,
            &load("network"),
            &load("oracle"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn numeric_comparison_uses_relative_tolerance() {
        assert!(close(2452.5, 2452.500001));
        assert!(!close(2452.5, 2453.0));
    }

    #[test]
    fn uncertainty_overlap_is_not_silently_passed() {
        let mut case = load("inputs")["potable_treatment_cases"][3].clone();
        case["inputs"]["measured_marker_alpha_mg_l"] = json!(0.98);
        case["inputs"]["absolute_uncertainty_mg_l"] = json!(0.05);
        let output = compute_potable(&case).expect("synthetic calculation");
        assert_eq!(output["assessment_disposition"], "SYNTHETIC_INDETERMINATE");
        assert!(!output["release_allowed"].as_bool().unwrap());
    }

    #[test]
    fn common_mode_sensor_errors_do_not_create_independent_consensus() {
        let case = load("inputs")["network_diagnostics_cases"][3].clone();
        let output = compute_diagnostics(&case).expect("diagnostic calculation");
        assert_eq!(output["diagnostic_disposition"], "INDETERMINATE");
        assert!(
            output["sensor_consensus_is_not_independent"]
                .as_bool()
                .unwrap()
        );
    }

    #[test]
    fn oracle_mismatch_is_reported_not_hidden() {
        let mut oracle = load("oracle");
        oracle["cases"][0]["expected"]["hydraulic_power_w"] = json!(1.0);
        let result = evaluate(
            &load("inputs"),
            &load("manifest"),
            &load("network"),
            &oracle,
        )
        .expect("mismatch should produce an evaluation receipt");
        assert_eq!(result["verification_state"], "OracleMismatch");
        assert_eq!(result["matched_case_count"], 22);
        assert_eq!(result["mismatches"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn incompatible_network_evidence_ceiling_is_rejected() {
        let mut network = load("network");
        network["evidence_ceiling"] = json!("field_detector_qualified");
        let result = evaluate(
            &load("inputs"),
            &load("manifest"),
            &network,
            &load("oracle"),
        );
        assert!(result.is_err());
    }
}
