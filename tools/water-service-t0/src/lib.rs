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

pub fn evaluate_membrane_ro_m0(inputs: &Value, oracle: &Value) -> Result<Value, String> {
    const EXPECTED_CASES: usize = 5;
    const CEILING: &str =
        "synthetic_transport_arithmetic_only_no_empirical_RO_performance_or_fouling_claim";

    if string(inputs, &["schema_version"])? != "membrane-ro-m0-cases-v0.1"
        || string(inputs, &["evidence_class"])? != "synthetic_transport_arithmetic"
        || string(inputs, &["evidence_ceiling"])? != CEILING
        || string(oracle, &["schema_version"])? != "membrane-ro-m0-oracle-v0.1"
        || string(oracle, &["evidence_class"])? != "synthetic_transport_arithmetic"
        || string(oracle, &["evidence_ceiling"])? != CEILING
    {
        return Err("membrane RO M0 input/oracle identity or evidence ceiling mismatch".to_owned());
    }

    let cases = at(inputs, &["cases"])?
        .as_array()
        .ok_or_else(|| "membrane RO M0 cases must be an array".to_owned())?;
    let oracle_cases = at(oracle, &["cases"])?
        .as_array()
        .ok_or_else(|| "membrane RO M0 oracle cases must be an array".to_owned())?;
    if cases.len() != EXPECTED_CASES || oracle_cases.len() != EXPECTED_CASES {
        return Err(format!(
            "membrane RO M0 expected {EXPECTED_CASES} input and oracle cases; got {} and {}",
            cases.len(),
            oracle_cases.len()
        ));
    }

    let mut expected_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = string(row, &["case_id"])?;
        if expected_by_id.insert(id.clone(), row).is_some() {
            return Err(format!("membrane RO M0 duplicate oracle case ID: {id}"));
        }
    }

    let mut seen = BTreeSet::new();
    let mut results = Vec::with_capacity(EXPECTED_CASES);
    let mut mismatches = Vec::new();

    for case in cases {
        let id = case_id(case)?;
        if !seen.insert(id.clone()) {
            return Err(format!("membrane RO M0 duplicate input case ID: {id}"));
        }
        let input = at(case, &["inputs"])?;
        let flux = number(input, &["flux_l_m2_h"])?;
        let delta_pressure = number(input, &["delta_pressure_bar"])?;
        let osmotic_wall = number(input, &["osmotic_pressure_wall_bar"])?;
        let osmotic_permeate = number(input, &["osmotic_pressure_permeate_bar"])?;
        let rejection = number(input, &["rejection_fraction"])?;
        let mass_transfer = number(input, &["mass_transfer_coefficient_l_m2_h"])?;

        if flux <= 0.0
            || delta_pressure <= 0.0
            || osmotic_wall < 0.0
            || osmotic_permeate < 0.0
            || !(0.0 < rejection && rejection < 1.0)
            || mass_transfer <= 0.0
        {
            return Err(format!("{id}: invalid transport input domain"));
        }

        let net_driving_pressure = delta_pressure - (osmotic_wall - osmotic_permeate);
        if !net_driving_pressure.is_finite() || net_driving_pressure <= 0.0 {
            return Err(format!("{id}: net driving pressure must be positive"));
        }

        let cp_exponent = (flux / mass_transfer).exp();
        if !cp_exponent.is_finite() {
            return Err(format!(
                "{id}: concentration-polarization exponential overflow"
            ));
        }
        let cp_modulus = (1.0 - rejection) + rejection * cp_exponent;
        let water_permeance_reference = flux / net_driving_pressure;
        let water_permeance_pressure_only = flux / delta_pressure;
        let solute_permeance_reference = flux * (1.0 - rejection) / (rejection * cp_exponent);
        let solute_permeance_no_cp = flux * (1.0 - rejection) / rejection;
        if !cp_modulus.is_finite()
            || !water_permeance_reference.is_finite()
            || !solute_permeance_reference.is_finite()
            || solute_permeance_reference <= 0.0
        {
            return Err(format!("{id}: non-finite or invalid transport result"));
        }

        let actual = json!({
            "net_driving_pressure_bar": net_driving_pressure,
            "concentration_polarization_modulus": cp_modulus,
            "water_permeance_reference_l_m2_h_bar": water_permeance_reference,
            "water_permeance_pressure_only_l_m2_h_bar": water_permeance_pressure_only,
            "water_permeance_pressure_only_percent_error":
                100.0 * (water_permeance_pressure_only - water_permeance_reference)
                    / water_permeance_reference,
            "solute_permeance_reference_l_m2_h": solute_permeance_reference,
            "solute_permeance_no_concentration_polarization_l_m2_h": solute_permeance_no_cp,
            "solute_permeance_no_concentration_polarization_percent_error":
                100.0 * (solute_permeance_no_cp - solute_permeance_reference)
                    / solute_permeance_reference
        });

        let oracle_row = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("membrane RO M0 oracle missing case {id}"))?;
        let expected = at(oracle_row, &["expected"])?;
        let matches = compare_json(&actual, expected);
        if !matches {
            mismatches.push(json!({
                "case_id": id,
                "expected": expected,
                "actual": actual
            }));
        }
        results.push(json!({
            "case_id": id,
            "family": "RO_TRANSPORT",
            "matched": matches,
            "computed": actual
        }));
    }

    if seen.len() != expected_by_id.len() {
        return Err(format!(
            "membrane RO M0 input/oracle identity mismatch: input={}, oracle={}",
            seen.len(),
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
        "schema_version": "membrane-ro-m0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_transport_arithmetic",
        "evidence_ceiling": CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "This is synthetic formula arithmetic, not a measured membrane result.",
            "The cases do not qualify a fouling mechanism or a cleaning intervention.",
            "No feed chemistry, membrane integrity, quality compliance or field performance is established.",
            "The simplified pressure-only and no-CP calculations are diagnostic comparisons, not operational recommendations."
        ]
    }))
}

pub fn evaluate_membrane_ro_m0b(inputs: &Value, oracle: &Value) -> Result<Value, String> {
    const EXPECTED_CASES: usize = 3;
    const CEILING: &str = "synthetic_regression_arithmetic_only_no_empirical_RO_performance_or_paper_reproduction_claim";

    if string(inputs, &["schema_version"])? != "membrane-ro-m0b-regression-cases-v0.1"
        || string(inputs, &["evidence_class"])? != "synthetic_multi_pressure_regression"
        || string(inputs, &["evidence_ceiling"])? != CEILING
        || string(oracle, &["schema_version"])? != "membrane-ro-m0b-regression-oracle-v0.1"
        || string(oracle, &["evidence_class"])? != "synthetic_multi_pressure_regression"
        || string(oracle, &["evidence_ceiling"])? != CEILING
    {
        return Err(
            "membrane RO M0B input/oracle identity or evidence ceiling mismatch".to_owned(),
        );
    }

    let cases = at(inputs, &["cases"])?
        .as_array()
        .ok_or_else(|| "membrane RO M0B cases must be an array".to_owned())?;
    let oracle_cases = at(oracle, &["cases"])?
        .as_array()
        .ok_or_else(|| "membrane RO M0B oracle cases must be an array".to_owned())?;
    if cases.len() != EXPECTED_CASES || oracle_cases.len() != EXPECTED_CASES {
        return Err(format!(
            "membrane RO M0B expected {EXPECTED_CASES} input and oracle cases; got {} and {}",
            cases.len(),
            oracle_cases.len()
        ));
    }

    let mut expected_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = string(row, &["case_id"])?;
        if expected_by_id.insert(id.clone(), row).is_some() {
            return Err(format!("membrane RO M0B duplicate oracle case ID: {id}"));
        }
    }

    let mut seen = BTreeSet::new();
    let mut results = Vec::with_capacity(EXPECTED_CASES);
    let mut mismatches = Vec::new();

    for case in cases {
        let id = case_id(case)?;
        if !seen.insert(id.clone()) {
            return Err(format!("membrane RO M0B duplicate input case ID: {id}"));
        }
        let replicates = at(case, &["replicates"])?
            .as_array()
            .ok_or_else(|| format!("{id}: replicates must be an array"))?;
        if replicates.len() < 2 {
            return Err(format!("{id}: at least two replicates are required"));
        }

        let mut replicate_ids = BTreeSet::new();
        let mut replicate_results = Vec::with_capacity(replicates.len());
        let mut slopes = Vec::with_capacity(replicates.len());

        for replicate in replicates {
            let replicate_id = string(replicate, &["replicate_id"])?;
            if !replicate_ids.insert(replicate_id.clone()) {
                return Err(format!("{id}: duplicate replicate ID {replicate_id}"));
            }
            let points = at(replicate, &["points"])?
                .as_array()
                .ok_or_else(|| format!("{id}/{replicate_id}: points must be an array"))?;
            if points.len() < 3 {
                return Err(format!(
                    "{id}/{replicate_id}: at least three pressure points are required"
                ));
            }

            let mut rows = Vec::with_capacity(points.len());
            for point in points {
                let delta_pressure = number(point, &["delta_pressure_bar"])?;
                let osmotic_wall = number(point, &["osmotic_pressure_wall_bar"])?;
                let osmotic_permeate = number(point, &["osmotic_pressure_permeate_bar"])?;
                let flux = number(point, &["flux_l_m2_h"])?;
                let ndp = delta_pressure - (osmotic_wall - osmotic_permeate);
                if !ndp.is_finite() || ndp <= 0.0 || !flux.is_finite() || flux <= 0.0 {
                    return Err(format!(
                        "{id}/{replicate_id}: nonpositive or non-finite NDP/flux"
                    ));
                }
                rows.push((ndp, flux));
            }

            let denominator = rows.iter().map(|(x, _)| x * x).sum::<f64>();
            let numerator = rows.iter().map(|(x, y)| x * y).sum::<f64>();
            if !denominator.is_finite() || denominator <= 0.0 {
                return Err(format!(
                    "{id}/{replicate_id}: invalid regression denominator"
                ));
            }
            let slope = numerator / denominator;
            let residual_rmse = (rows
                .iter()
                .map(|(x, y)| (y - slope * x).powi(2))
                .sum::<f64>()
                / rows.len() as f64)
                .sqrt();
            let mean_pointwise_ratio =
                rows.iter().map(|(x, y)| y / x).sum::<f64>() / rows.len() as f64;
            if !slope.is_finite()
                || slope <= 0.0
                || !residual_rmse.is_finite()
                || !mean_pointwise_ratio.is_finite()
            {
                return Err(format!("{id}/{replicate_id}: invalid regression output"));
            }
            slopes.push(slope);
            replicate_results.push(json!({
                "replicate_id": replicate_id,
                "point_count": rows.len(),
                "slope_l_m2_h_bar": slope,
                "residual_rmse_l_m2_h": residual_rmse,
                "mean_pointwise_ratio_l_m2_h_bar": mean_pointwise_ratio
            }));
        }

        let mean_slope = slopes.iter().sum::<f64>() / slopes.len() as f64;
        let sample_variance = slopes
            .iter()
            .map(|slope| (slope - mean_slope).powi(2))
            .sum::<f64>()
            / (slopes.len() - 1) as f64;
        let sample_sd = sample_variance.sqrt();
        let total_points = replicate_results
            .iter()
            .map(|row| row["point_count"].as_u64().unwrap_or(0))
            .sum::<u64>();

        let actual = json!({
            "replicate_count": replicate_results.len(),
            "total_point_count": total_points,
            "replicates": replicate_results,
            "mean_slope_l_m2_h_bar": mean_slope,
            "sample_sd_of_replicate_slopes_l_m2_h_bar": sample_sd
        });

        let oracle_row = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("membrane RO M0B oracle missing case {id}"))?;
        let expected = at(oracle_row, &["expected"])?;
        let matches = compare_json(&actual, expected);
        if !matches {
            mismatches.push(json!({
                "case_id": id,
                "expected": expected,
                "actual": actual
            }));
        }
        results.push(json!({
            "case_id": id,
            "family": "RO_M0B_THROUGH_ORIGIN_REGRESSION",
            "matched": matches,
            "computed": actual
        }));
    }

    if seen.len() != expected_by_id.len() {
        return Err(format!(
            "membrane RO M0B input/oracle identity mismatch: input={}, oracle={}",
            seen.len(),
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
        "schema_version": "membrane-ro-m0b-regression-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_multi_pressure_regression",
        "evidence_ceiling": CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "This is synthetic through-origin regression arithmetic, not a measured membrane result.",
            "It does not reproduce the Armstrong OSF workbook or its fitted reference_A values.",
            "No fouling mechanism, cleaning intervention, quality outcome or plant performance is established.",
            "Do not extrapolate beyond the represented pressure and feed conditions."
        ]
    }))
}

pub fn evaluate_membrane_observability_f0(
    inputs: &Value,
    oracle: &Value,
) -> Result<Value, String> {
    const EXPECTED_CASES: usize = 13;
    const CEILING: &str =
        "synthetic_diagnostic_baseline_only_no_real_sensor_or_membrane_diagnosis";

    if string(inputs, &["schema_version"])? != "membrane-observability-fault-cases-v0.1"
        || string(inputs, &["evidence_class"])? != "synthetic_observability_challenge"
        || string(inputs, &["evidence_ceiling"])? != CEILING
        || string(oracle, &["schema_version"])? != "membrane-observability-fault-oracle-v0.1"
        || string(oracle, &["evidence_class"])? != "synthetic_observability_challenge"
        || string(oracle, &["evidence_ceiling"])? != CEILING
    {
        return Err("membrane observability F0 input/oracle identity mismatch".to_owned());
    }

    let thresholds = at(inputs, &["baseline_thresholds"])?;
    let pressure_threshold = number(thresholds, &["pressure_pair_residual_fraction"])?;
    let flow_threshold = number(thresholds, &["flow_pair_residual_fraction"])?;
    let clock_threshold = number(thresholds, &["absolute_clock_skew_seconds"])?;
    let missing_threshold = number(thresholds, &["critical_channel_missing_fraction"])?;
    let temperature_threshold = number(thresholds, &["absolute_temperature_shift_c"])?;
    let conductivity_threshold = number(thresholds, &["feed_conductivity_ratio_deviation"])?;
    let pump_threshold = number(thresholds, &["pump_speed_ratio_deviation"])?;
    let resistance_threshold = number(thresholds, &["normalized_resistance_ratio"])?;
    let uncertainty_threshold =
        number(thresholds, &["maximum_resistance_relative_uncertainty"])?;

    if [
        pressure_threshold,
        flow_threshold,
        clock_threshold,
        missing_threshold,
        temperature_threshold,
        conductivity_threshold,
        pump_threshold,
        resistance_threshold,
        uncertainty_threshold,
    ]
    .iter()
    .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("membrane observability F0 thresholds must be finite and nonnegative".to_owned());
    }

    let scenarios = at(inputs, &["scenarios"])?
        .as_array()
        .ok_or_else(|| "membrane observability F0 scenarios must be an array".to_owned())?;
    let oracle_cases = at(oracle, &["cases"])?
        .as_array()
        .ok_or_else(|| "membrane observability F0 oracle cases must be an array".to_owned())?;
    if scenarios.len() != EXPECTED_CASES || oracle_cases.len() != EXPECTED_CASES {
        return Err(format!(
            "membrane observability F0 expected {EXPECTED_CASES} scenarios and oracle rows; got {} and {}",
            scenarios.len(),
            oracle_cases.len()
        ));
    }

    let mut expected_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = string(row, &["scenario_id"])?;
        if expected_by_id.insert(id.clone(), row).is_some() {
            return Err(format!("membrane observability F0 duplicate oracle ID: {id}"));
        }
    }

    let mut seen = BTreeSet::new();
    let mut results = Vec::with_capacity(EXPECTED_CASES);
    let mut mismatches = Vec::new();
    let mut false_fouling_sensor_only = 0_u64;
    let mut false_fouling_regime_only = 0_u64;
    let mut false_cip_recommendations = 0_u64;

    for scenario in scenarios {
        let id = string(scenario, &["scenario_id"])?;
        if !seen.insert(id.clone()) {
            return Err(format!("membrane observability F0 duplicate scenario ID: {id}"));
        }
        let input = at(scenario, &["inputs"])?;
        let pressure_residual = number(input, &["pressure_sensor_pair_delta_fraction"])?;
        let flow_residual = number(input, &["flow_sensor_pair_delta_fraction"])?;
        let clock_skew = number(input, &["clock_skew_seconds"])?;
        let missing_fraction = number(input, &["critical_channel_missing_fraction"])?;
        let temperature_delta = number(input, &["temperature_delta_c"])?;
        let conductivity_ratio = number(input, &["feed_conductivity_ratio"])?;
        let pump_ratio = number(input, &["pump_speed_ratio"])?;
        if [
            pressure_residual,
            flow_residual,
            clock_skew,
            missing_fraction,
            temperature_delta,
            conductivity_ratio,
            pump_ratio,
        ]
        .iter()
        .any(|value| !value.is_finite())
            || pressure_residual < 0.0
            || flow_residual < 0.0
            || missing_fraction < 0.0
            || missing_fraction > 1.0
            || conductivity_ratio <= 0.0
            || pump_ratio <= 0.0
        {
            return Err(format!("{id}: invalid membrane observability F0 input domain"));
        }

        let resistance_value = at(input, &["normalized_resistance_ratio"])?
            .as_f64()
            .filter(|value| value.is_finite() && *value > 0.0);
        let uncertainty_value = at(input, &["normalized_resistance_relative_uncertainty"])?
            .as_f64()
            .filter(|value| value.is_finite() && *value >= 0.0);
        let resistance_observable = resistance_value.is_some()
            && uncertainty_value.is_some()
            && uncertainty_value.unwrap_or(f64::INFINITY) <= uncertainty_threshold;

        let sensor_fault = pressure_residual > pressure_threshold
            || flow_residual > flow_threshold
            || clock_skew.abs() > clock_threshold
            || missing_fraction > missing_threshold;
        let regime_shift = temperature_delta.abs() > temperature_threshold
            || (conductivity_ratio - 1.0).abs() > conductivity_threshold
            || (pump_ratio - 1.0).abs() > pump_threshold;
        let fouling_suspected = resistance_observable
            && resistance_value.unwrap_or(0.0) > resistance_threshold;

        let disposition = if sensor_fault && (fouling_suspected || regime_shift) {
            "MULTIPLE_CAUSES"
        } else if sensor_fault {
            "SENSOR_FAULT_SUSPECTED"
        } else if fouling_suspected && regime_shift {
            "MULTIPLE_CAUSES"
        } else if fouling_suspected {
            "FOULING_SUSPECTED"
        } else if regime_shift {
            "OPERATING_REGIME_SHIFT"
        } else if !resistance_observable {
            "INDETERMINATE"
        } else {
            "NO_FAULT"
        };

        let oracle_row = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("membrane observability F0 oracle missing scenario {id}"))?;
        let expected = string(oracle_row, &["expected_disposition"])?;
        let expected_cip = at(oracle_row, &["expected_cip_recommendation"])?
            .as_bool()
            .ok_or_else(|| format!("{id}: expected_cip_recommendation must be boolean"))?;
        let matched = disposition == expected && !expected_cip;
        if !matched {
            mismatches.push(json!({
                "scenario_id": id,
                "expected_disposition": expected,
                "actual_disposition": disposition,
                "expected_cip_recommendation": expected_cip
            }));
        }
        if expected == "SENSOR_FAULT_SUSPECTED" && disposition == "FOULING_SUSPECTED" {
            false_fouling_sensor_only += 1;
        }
        if expected == "OPERATING_REGIME_SHIFT" && disposition == "FOULING_SUSPECTED" {
            false_fouling_regime_only += 1;
        }
        if expected_cip {
            false_cip_recommendations += 1;
        }
        results.push(json!({
            "scenario_id": id,
            "expected_disposition": expected,
            "actual_disposition": disposition,
            "matched": matched,
            "cip_recommendation_authorized": false,
            "measurement_quarantine_required": sensor_fault,
            "regime_shift_detected": regime_shift,
            "fouling_proxy_observable": resistance_observable
        }));
    }

    if seen.len() != expected_by_id.len() {
        return Err(format!(
            "membrane observability F0 input/oracle identity mismatch: input={}, oracle={}",
            seen.len(),
            expected_by_id.len()
        ));
    }
    results.sort_by(|a, b| {
        a.get("scenario_id")
            .and_then(Value::as_str)
            .cmp(&b.get("scenario_id").and_then(Value::as_str))
    });
    let matched = results.iter().filter(|row| row["matched"] == true).count();

    Ok(json!({
        "schema_version": "membrane-observability-fault-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_observability_challenge",
        "evidence_ceiling": CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "false_fouling_alarms_sensor_only": false_fouling_sensor_only,
        "false_fouling_alarms_regime_shift_only": false_fouling_regime_only,
        "false_cip_recommendations": false_cip_recommendations,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "This is a transparent synthetic residual-rule baseline, not a trained diagnostic model.",
            "Normalized resistance is an assumed observer input and is not estimated from real plant telemetry here.",
            "No fouling chemistry, membrane integrity, water quality or cleaning need is established.",
            "No diagnostic output authorizes a CIP or plant actuator command."
        ]
    }))
}

pub fn evaluate_membrane_observability_f1(
    inputs: &Value,
    oracle: &Value,
) -> Result<Value, String> {
    const EXPECTED_TRACES: usize = 9;
    const STEPS_PER_TRACE: usize = 8;
    const CEILING: &str =
        "synthetic_trace_evaluation_only_no_plant_detection_latency_or_alarm_rate_claim";

    if string(inputs, &["schema_version"])? != "membrane-observability-f1-traces-v0.1"
        || string(inputs, &["evidence_class"])? != "synthetic_time_indexed_fault_injection"
        || string(inputs, &["evidence_ceiling"])? != CEILING
        || string(oracle, &["schema_version"])? != "membrane-observability-f1-trace-oracle-v0.1"
        || string(oracle, &["evidence_class"])? != "synthetic_time_indexed_fault_injection"
        || string(oracle, &["evidence_ceiling"])? != CEILING
    {
        return Err("membrane observability F1 input/oracle identity mismatch".to_owned());
    }

    let thresholds = at(inputs, &["baseline_thresholds"])?;
    let pressure_threshold = number(thresholds, &["pressure_pair_residual_fraction"])?;
    let flow_threshold = number(thresholds, &["flow_pair_residual_fraction"])?;
    let clock_threshold = number(thresholds, &["absolute_clock_skew_seconds"])?;
    let missing_threshold = number(thresholds, &["critical_channel_missing_fraction"])?;
    let temperature_threshold = number(thresholds, &["absolute_temperature_shift_c"])?;
    let conductivity_threshold = number(thresholds, &["feed_conductivity_ratio_deviation"])?;
    let pump_threshold = number(thresholds, &["pump_speed_ratio_deviation"])?;
    let resistance_threshold = number(thresholds, &["normalized_resistance_ratio"])?;
    let uncertainty_threshold =
        number(thresholds, &["maximum_resistance_relative_uncertainty"])?;

    let traces = at(inputs, &["traces"])?
        .as_array()
        .ok_or_else(|| "membrane observability F1 traces must be an array".to_owned())?;
    let oracle_traces = at(oracle, &["traces"])?
        .as_array()
        .ok_or_else(|| "membrane observability F1 oracle traces must be an array".to_owned())?;
    if traces.len() != EXPECTED_TRACES || oracle_traces.len() != EXPECTED_TRACES {
        return Err(format!(
            "membrane observability F1 expected {EXPECTED_TRACES} traces; got {} and {}",
            traces.len(),
            oracle_traces.len()
        ));
    }

    let mut oracle_by_id = BTreeMap::new();
    for trace in oracle_traces {
        let id = string(trace, &["trace_id"])?;
        if oracle_by_id.insert(id.clone(), trace).is_some() {
            return Err(format!("membrane observability F1 duplicate oracle trace ID: {id}"));
        }
    }

    let mut seen = BTreeSet::new();
    let mut results = Vec::with_capacity(EXPECTED_TRACES);
    let mut mismatches = Vec::new();
    let mut total_windows = 0_u64;
    let mut matched_windows = 0_u64;
    let mut false_fouling_sensor_windows = 0_u64;
    let mut false_fouling_regime_windows = 0_u64;
    let mut false_fouling_stable_windows = 0_u64;
    let mut false_cip_recommendations = 0_u64;
    let mut detection_delays = Vec::new();

    for trace in traces {
        let id = string(trace, &["trace_id"])?;
        if !seen.insert(id.clone()) {
            return Err(format!("membrane observability F1 duplicate trace ID: {id}"));
        }
        let event_class = string(trace, &["event_class"])?;
        let steps = at(trace, &["steps"])?
            .as_array()
            .ok_or_else(|| format!("{id}: steps must be an array"))?;
        if steps.len() != STEPS_PER_TRACE {
            return Err(format!(
                "{id}: expected {STEPS_PER_TRACE} steps; got {}",
                steps.len()
            ));
        }
        let oracle_trace = oracle_by_id
            .get(&id)
            .ok_or_else(|| format!("membrane observability F1 oracle missing trace {id}"))?;
        let expected_steps = at(oracle_trace, &["expected_dispositions"])?
            .as_array()
            .ok_or_else(|| format!("{id}: oracle expected_dispositions must be an array"))?;
        let expected_cip = at(oracle_trace, &["expected_cip_recommendations"])?
            .as_array()
            .ok_or_else(|| format!("{id}: oracle expected_cip_recommendations must be an array"))?;
        if expected_steps.len() != STEPS_PER_TRACE || expected_cip.len() != STEPS_PER_TRACE {
            return Err(format!("{id}: oracle step count mismatch"));
        }

        let event_onset = at(trace, &["event_onset_step_index"])?
            .as_u64();
        let is_fouling_event = event_class == "FOULING" || event_class == "FOULING_AND_SENSOR";
        if is_fouling_event != event_onset.is_some() {
            return Err(format!("{id}: fouling event and onset index must agree"));
        }

        let mut actual_dispositions = Vec::with_capacity(STEPS_PER_TRACE);
        let mut detected_step = None;
        let mut trace_false_fouling = 0_u64;
        let mut trace_cip_recommendations = 0_u64;

        for (index, step) in steps.iter().enumerate() {
            let time_index = at(step, &["time_index"])?
                .as_u64()
                .ok_or_else(|| format!("{id}: time_index must be an unsigned integer"))?;
            if time_index != index as u64 {
                return Err(format!("{id}: time indices must be contiguous from zero"));
            }
            let input = at(step, &["inputs"])?;
            let pressure_residual = number(input, &["pressure_sensor_pair_delta_fraction"])?;
            let flow_residual = number(input, &["flow_sensor_pair_delta_fraction"])?;
            let clock_skew = number(input, &["clock_skew_seconds"])?;
            let missing_fraction = number(input, &["critical_channel_missing_fraction"])?;
            let temperature_delta = number(input, &["temperature_delta_c"])?;
            let conductivity_ratio = number(input, &["feed_conductivity_ratio"])?;
            let pump_ratio = number(input, &["pump_speed_ratio"])?;
            let resistance_value = at(input, &["normalized_resistance_ratio"])?
                .as_f64()
                .filter(|value| value.is_finite() && *value > 0.0);
            let uncertainty_value = at(input, &["normalized_resistance_relative_uncertainty"])?
                .as_f64()
                .filter(|value| value.is_finite() && *value >= 0.0);

            let sensor_fault = pressure_residual > pressure_threshold
                || flow_residual > flow_threshold
                || clock_skew.abs() > clock_threshold
                || missing_fraction > missing_threshold;
            let regime_shift = temperature_delta.abs() > temperature_threshold
                || (conductivity_ratio - 1.0).abs() > conductivity_threshold
                || (pump_ratio - 1.0).abs() > pump_threshold;
            let resistance_observable = resistance_value.is_some()
                && uncertainty_value.is_some()
                && uncertainty_value.unwrap_or(f64::INFINITY) <= uncertainty_threshold;
            let fouling_suspected = resistance_observable
                && resistance_value.unwrap_or(0.0) > resistance_threshold;

            let disposition = if sensor_fault && (fouling_suspected || regime_shift) {
                "MULTIPLE_CAUSES"
            } else if sensor_fault {
                "SENSOR_FAULT_SUSPECTED"
            } else if fouling_suspected && regime_shift {
                "MULTIPLE_CAUSES"
            } else if fouling_suspected {
                "FOULING_SUSPECTED"
            } else if regime_shift {
                "OPERATING_REGIME_SHIFT"
            } else if !resistance_observable {
                "INDETERMINATE"
            } else {
                "NO_FAULT"
            };

            let expected_row = &expected_steps[index];
            let expected_time = at(expected_row, &["time_index"])?
                .as_u64()
                .ok_or_else(|| format!("{id}: oracle time_index must be an unsigned integer"))?;
            if expected_time != time_index {
                return Err(format!("{id}: oracle time index mismatch at step {index}"));
            }
            let expected_disposition = string(expected_row, &["expected_disposition"])?;
            let cip_expected = expected_cip[index]
                .as_bool()
                .ok_or_else(|| format!("{id}: CIP oracle values must be booleans"))?;
            if cip_expected {
                false_cip_recommendations += 1;
            }
            if disposition == "FOULING_SUSPECTED" || disposition == "MULTIPLE_CAUSES" {
                if event_class == "SENSOR_ONLY" {
                    false_fouling_sensor_windows += 1;
                    trace_false_fouling += 1;
                } else if event_class == "REGIME_ONLY" {
                    false_fouling_regime_windows += 1;
                    trace_false_fouling += 1;
                } else if event_class == "NONE" {
                    false_fouling_stable_windows += 1;
                    trace_false_fouling += 1;
                }
            }
            if is_fouling_event
                && time_index >= event_onset.unwrap_or(u64::MAX)
                && detected_step.is_none()
                && (disposition == "FOULING_SUSPECTED" || disposition == "MULTIPLE_CAUSES")
            {
                detected_step = Some(time_index);
            }
            if disposition == expected_disposition && !cip_expected {
                matched_windows += 1;
            } else {
                mismatches.push(json!({
                    "trace_id": id,
                    "time_index": time_index,
                    "expected_disposition": expected_disposition,
                    "actual_disposition": disposition,
                    "expected_cip_recommendation": cip_expected
                }));
            }
            actual_dispositions.push(disposition);
            total_windows += 1;
            trace_cip_recommendations += u64::from(cip_expected);
        }

        let expected_detected = at(oracle_trace, &["expected_detected_step_index"])?
            .as_u64();
        if detected_step != expected_detected {
            mismatches.push(json!({
                "trace_id": id,
                "expected_detected_step_index": expected_detected,
                "actual_detected_step_index": detected_step
            }));
        }
        let delay = match (event_onset, detected_step) {
            (Some(onset), Some(detected)) => Some(detected.saturating_sub(onset)),
            _ => None,
        };
        if let Some(value) = delay {
            detection_delays.push(value);
        }
        results.push(json!({
            "trace_id": id,
            "event_class": event_class,
            "event_onset_step_index": event_onset,
            "actual_dispositions": actual_dispositions,
            "detected_step_index": detected_step,
            "detection_delay_steps": delay,
            "false_fouling_windows": trace_false_fouling,
            "cip_recommendations": trace_cip_recommendations
        }));
    }

    if seen.len() != oracle_by_id.len() || total_windows != (EXPECTED_TRACES * STEPS_PER_TRACE) as u64 {
        return Err("membrane observability F1 input/oracle coverage mismatch".to_owned());
    }
    results.sort_by(|a, b| {
        a.get("trace_id")
            .and_then(Value::as_str)
            .cmp(&b.get("trace_id").and_then(Value::as_str))
    });
    let mean_delay = if detection_delays.is_empty() {
        None
    } else {
        Some(
            detection_delays.iter().sum::<u64>() as f64
                / detection_delays.len() as f64,
        )
    };

    Ok(json!({
        "schema_version": "membrane-observability-f1-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_time_indexed_fault_injection",
        "evidence_ceiling": CEILING,
        "trace_count": results.len(),
        "total_window_count": total_windows,
        "matched_window_count": matched_windows,
        "false_fouling_windows_sensor_only": false_fouling_sensor_windows,
        "false_fouling_windows_regime_only": false_fouling_regime_windows,
        "false_fouling_windows_stable_control": false_fouling_stable_windows,
        "false_cip_recommendations": false_cip_recommendations,
        "detected_fouling_event_count": detection_delays.len(),
        "mean_detected_event_delay_steps": mean_delay,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "Synthetic time-indexed fault-injection traces are not plant-level detection-delay evidence.",
            "The baseline uses a supplied normalized-resistance observer feature and does not estimate it from field data.",
            "No real event lead time, false-alarm rate, mechanism classification, cleaning benefit or membrane lifetime is established.",
            "No diagnostic output authorizes CIP or plant actuator commands."
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
            "membrane_m0_inputs" => include_str!(
                "../../../research/water-service-engineering/membrane-ro-m0-cases-v0.1.json"
            ),
            "membrane_m0_oracle" => include_str!(
                "../../../research/water-service-engineering/membrane-ro-m0-oracle-v0.1.json"
            ),
            "membrane_m0b_inputs" => include_str!(
                "../../../research/water-service-engineering/membrane-ro-m0b-regression-cases-v0.1.json"
            ),
            "membrane_m0b_oracle" => include_str!(
                "../../../research/water-service-engineering/membrane-ro-m0b-regression-oracle-v0.1.json"
            ),
            "membrane_observability_inputs" => include_str!(
                "../../../research/water-service-engineering/membrane-observability-fault-cases-v0.1.json"
            ),
            "membrane_observability_oracle" => include_str!(
                "../../../research/water-service-engineering/membrane-observability-fault-oracle-v0.1.json"
            ),
            "membrane_observability_f1_inputs" => include_str!(
                "../../../research/water-service-engineering/membrane-observability-f1-traces-v0.1.json"
            ),
            "membrane_observability_f1_oracle" => include_str!(
                "../../../research/water-service-engineering/membrane-observability-f1-trace-oracle-v0.1.json"
            ),
            _ => panic!("unknown test fixture"),
        };
        serde_json::from_str(text).expect("fixture JSON must parse")
    }

    #[test]
    fn all_seventy_two_membrane_observability_f1_windows_match_separate_oracle() {
        let result = evaluate_membrane_observability_f1(
            &load("membrane_observability_f1_inputs"),
            &load("membrane_observability_f1_oracle"),
        )
        .expect("membrane observability F1 traces must evaluate");
        assert_eq!(result["trace_count"], 9);
        assert_eq!(result["total_window_count"], 72);
        assert_eq!(result["matched_window_count"], 72);
        assert_eq!(result["false_fouling_windows_sensor_only"], 0);
        assert_eq!(result["false_fouling_windows_regime_only"], 0);
        assert_eq!(result["false_fouling_windows_stable_control"], 0);
        assert_eq!(result["false_cip_recommendations"], 0);
        assert_eq!(result["detected_fouling_event_count"], 2);
        assert_eq!(result["mean_detected_event_delay_steps"], 0.0);
    }

    #[test]
    fn membrane_observability_f1_sensor_fault_clear_recovers_to_no_fault() {
        let result = evaluate_membrane_observability_f1(
            &load("membrane_observability_f1_inputs"),
            &load("membrane_observability_f1_oracle"),
        )
        .expect("membrane observability F1 traces must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["trace_id"] == "MTR-009-pressure-drift-clears")
            .unwrap();
        assert_eq!(row["actual_dispositions"][5], "SENSOR_FAULT_SUSPECTED");
        assert_eq!(row["actual_dispositions"][6], "NO_FAULT");
        assert_eq!(row["actual_dispositions"][7], "NO_FAULT");
    }

    #[test]
    fn all_thirteen_membrane_observability_f0_cases_match_separate_oracle() {
        let result = evaluate_membrane_observability_f0(
            &load("membrane_observability_inputs"),
            &load("membrane_observability_oracle"),
        )
        .expect("membrane observability F0 must evaluate");
        assert_eq!(result["result_count"], 13);
        assert_eq!(result["matched_case_count"], 13);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert_eq!(result["false_fouling_alarms_sensor_only"], 0);
        assert_eq!(result["false_fouling_alarms_regime_shift_only"], 0);
        assert_eq!(result["false_cip_recommendations"], 0);
    }

    #[test]
    fn membrane_observability_f0_abstains_when_resistance_is_unobservable() {
        let result = evaluate_membrane_observability_f0(
            &load("membrane_observability_inputs"),
            &load("membrane_observability_oracle"),
        )
        .expect("membrane observability F0 must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["scenario_id"] == "MOBS-012-resistance-unobservable")
            .unwrap();
        assert_eq!(row["actual_disposition"], "INDETERMINATE");
    }

    #[test]
    fn membrane_observability_f0_preserves_multiple_cause_disposition() {
        let result = evaluate_membrane_observability_f0(
            &load("membrane_observability_inputs"),
            &load("membrane_observability_oracle"),
        )
        .expect("membrane observability F0 must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["scenario_id"] == "MOBS-010-fouling-plus-pressure-sensor-drift")
            .unwrap();
        assert_eq!(row["actual_disposition"], "MULTIPLE_CAUSES");
    }

    #[test]
    fn all_three_membrane_ro_m0b_groups_match_separate_oracle() {
        let result =
            evaluate_membrane_ro_m0b(&load("membrane_m0b_inputs"), &load("membrane_m0b_oracle"))
                .expect("M0B regression must evaluate");
        assert_eq!(result["result_count"], 3);
        assert_eq!(result["matched_case_count"], 3);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert_eq!(result["mismatches"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn membrane_ro_m0b_uses_grouped_replicates_not_a_flat_point_pool() {
        let result =
            evaluate_membrane_ro_m0b(&load("membrane_m0b_inputs"), &load("membrane_m0b_oracle"))
                .expect("M0B regression must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "MRO-M0B-003-heteroscedastic-noise-three-replicates")
            .unwrap();
        assert_eq!(row["computed"]["replicate_count"], 3);
        assert!(
            row["computed"]["sample_sd_of_replicate_slopes_l_m2_h_bar"]
                .as_f64()
                .unwrap()
                > 0.0
        );
    }

    #[test]
    fn membrane_ro_m0b_rejects_nonpositive_net_driving_pressure() {
        let mut inputs = load("membrane_m0b_inputs");
        inputs["cases"][0]["replicates"][0]["points"][0]["osmotic_pressure_wall_bar"] = json!(30.0);
        assert!(evaluate_membrane_ro_m0b(&inputs, &load("membrane_m0b_oracle")).is_err());
    }

    #[test]
    fn all_five_membrane_ro_m0_cases_match_separate_oracle() {
        let result =
            evaluate_membrane_ro_m0(&load("membrane_m0_inputs"), &load("membrane_m0_oracle"))
                .expect("M0 transport calculations must evaluate");
        assert_eq!(result["result_count"], 5);
        assert_eq!(result["matched_case_count"], 5);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert_eq!(result["mismatches"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn membrane_ro_m0_separates_hydraulic_pressure_from_net_driving_pressure() {
        let result =
            evaluate_membrane_ro_m0(&load("membrane_m0_inputs"), &load("membrane_m0_oracle"))
                .expect("M0 transport calculations must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "MRO-M0-001-high-rejection-moderate-CP")
            .unwrap();
        assert!(
            row["computed"]["net_driving_pressure_bar"]
                .as_f64()
                .unwrap()
                < 20.0
        );
        assert!(
            row["computed"]["water_permeance_reference_l_m2_h_bar"]
                .as_f64()
                .unwrap()
                > row["computed"]["water_permeance_pressure_only_l_m2_h_bar"]
                    .as_f64()
                    .unwrap()
        );
    }

    #[test]
    fn membrane_ro_m0_no_cp_assumption_changes_solute_permeance() {
        let result =
            evaluate_membrane_ro_m0(&load("membrane_m0_inputs"), &load("membrane_m0_oracle"))
                .expect("M0 transport calculations must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "MRO-M0-003-lower-rejection-higher-CP")
            .unwrap();
        assert!(
            row["computed"]["solute_permeance_no_concentration_polarization_l_m2_h"]
                .as_f64()
                .unwrap()
                > row["computed"]["solute_permeance_reference_l_m2_h"]
                    .as_f64()
                    .unwrap()
        );
    }

    #[test]
    fn membrane_ro_m0_rejects_nonpositive_net_driving_pressure() {
        let mut inputs = load("membrane_m0_inputs");
        inputs["cases"][0]["inputs"]["osmotic_pressure_wall_bar"] = json!(30.0);
        let result = evaluate_membrane_ro_m0(&inputs, &load("membrane_m0_oracle"));
        assert!(result.is_err());
    }

    #[test]
    fn membrane_ro_m0_oracle_mismatch_is_reported_not_hidden() {
        let mut oracle = load("membrane_m0_oracle");
        oracle["cases"][0]["expected"]["water_permeance_reference_l_m2_h_bar"] = json!(999.0);
        let result = evaluate_membrane_ro_m0(&load("membrane_m0_inputs"), &oracle)
            .expect("mismatch should produce an evaluation result");
        assert_eq!(result["verification_state"], "OracleMismatch");
        assert_eq!(result["matched_case_count"], 4);
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
