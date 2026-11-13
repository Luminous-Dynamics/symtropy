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

pub fn evaluate_membrane_measurement_integrity(
    inputs: &Value,
    oracle: &Value,
) -> Result<Value, String> {
    const EXPECTED_CASES: usize = 8;
    const CEILING: &str = "synthetic_signal_diagnostics_only_no_real_sensor_or_fouling_claim";
    const CHANNELS: [&str; 3] = [
        "flux_l_m2_h",
        "tmp_bar",
        "permeability_l_m2_h_bar",
    ];

    if string(inputs, &["schema_version"])?
        != "membrane-measurement-integrity-cases-v0.1"
        || string(inputs, &["evidence_class"])? != "synthetic_measurement_integrity"
        || string(inputs, &["evidence_ceiling"])? != CEILING
        || string(oracle, &["schema_version"])?
            != "membrane-measurement-integrity-oracle-v0.1"
        || string(oracle, &["evidence_class"])? != "synthetic_measurement_integrity"
        || string(oracle, &["evidence_ceiling"])? != CEILING
    {
        return Err(
            "membrane measurement integrity input/oracle identity or evidence ceiling mismatch"
                .to_owned(),
        );
    }

    let sampling_interval = at(inputs, &["sampling_interval_seconds"])?
        .as_u64()
        .ok_or_else(|| "sampling interval must be an unsigned integer".to_owned())?;
    if sampling_interval != 300 {
        return Err("synthetic M1 corpus requires a declared 300-second cadence".to_owned());
    }
    let policy = at(inputs, &["policy"])?;
    let max_missing_fraction = number(policy, &["max_missing_fraction"])?;
    let min_stuck_run = at(policy, &["min_stuck_run"])?
        .as_u64()
        .ok_or_else(|| "min_stuck_run must be an unsigned integer".to_owned())?
        as usize;
    let flatline_epsilon_fraction = number(policy, &["flatline_epsilon_fraction"])?;
    let peer_variation_fraction = number(policy, &["peer_variation_fraction"])?;
    let consistency_limit = number(policy, &["consistency_relative_error_limit"])?;
    let process_tmp_rise_limit = number(policy, &["process_tmp_rise_fraction"])?;
    let process_flux_drop_limit = number(policy, &["process_flux_drop_fraction"])?;
    let min_consistency_points = at(policy, &["min_consistency_points"])?
        .as_u64()
        .ok_or_else(|| "min_consistency_points must be an unsigned integer".to_owned())?
        as usize;
    if !(0.0..=1.0).contains(&max_missing_fraction)
        || min_stuck_run < 2
        || flatline_epsilon_fraction < 0.0
        || peer_variation_fraction < 0.0
        || consistency_limit < 0.0
        || process_tmp_rise_limit < 0.0
        || process_flux_drop_limit < 0.0
        || min_consistency_points == 0
    {
        return Err("invalid membrane measurement-integrity policy domain".to_owned());
    }

    let cases = at(inputs, &["cases"])?
        .as_array()
        .ok_or_else(|| "measurement integrity cases must be an array".to_owned())?;
    let oracle_cases = at(oracle, &["cases"])?
        .as_array()
        .ok_or_else(|| "measurement integrity oracle cases must be an array".to_owned())?;
    if cases.len() != EXPECTED_CASES || oracle_cases.len() != EXPECTED_CASES {
        return Err(format!(
            "measurement integrity expected {EXPECTED_CASES} input and oracle cases; got {} and {}",
            cases.len(),
            oracle_cases.len()
        ));
    }

    let mut expected_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = string(row, &["case_id"])?;
        if expected_by_id.insert(id.clone(), row).is_some() {
            return Err(format!("measurement integrity duplicate oracle case ID: {id}"));
        }
    }

    fn read_series(case: &Value, name: &str) -> Result<Vec<Option<f64>>, String> {
        let values = at(case, &["channels", name])?
            .as_array()
            .ok_or_else(|| format!("{}: channel {name} must be an array", case_id(case).unwrap_or_default()))?;
        values
            .iter()
            .map(|value| {
                if value.is_null() {
                    Ok(None)
                } else {
                    value
                        .as_f64()
                        .filter(|number| number.is_finite())
                        .map(Some)
                        .ok_or_else(|| format!("channel {name} contains a non-finite/non-numeric value"))
                }
            })
            .collect()
    }

    fn relative_range(values: &[Option<f64>]) -> f64 {
        let present: Vec<f64> = values.iter().filter_map(|value| *value).collect();
        if present.is_empty() {
            return 0.0;
        }
        let mean = present.iter().sum::<f64>() / present.len() as f64;
        let min = present.iter().copied().fold(f64::INFINITY, f64::min);
        let max = present.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if mean.abs() <= 1.0e-12 {
            0.0
        } else {
            (max - min) / mean.abs()
        }
    }

    fn longest_flatline(values: &[Option<f64>], epsilon: f64) -> usize {
        let mut longest = 0usize;
        let mut run = 0usize;
        let mut previous: Option<f64> = None;
        for value in values {
            match value {
                None => {
                    run = 0;
                    previous = None;
                }
                Some(current) => {
                    if previous.is_some_and(|prior| (current - prior).abs() <= epsilon) {
                        run += 1;
                    } else {
                        run = 1;
                    }
                    longest = longest.max(run);
                    previous = Some(*current);
                }
            }
        }
        longest
    }

    fn median(values: &mut [f64]) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        values.sort_by(f64::total_cmp);
        let middle = values.len() / 2;
        if values.len() % 2 == 1 {
            Some(values[middle])
        } else {
            Some((values[middle - 1] + values[middle]) / 2.0)
        }
    }

    let mut seen = BTreeSet::new();
    let mut results = Vec::with_capacity(EXPECTED_CASES);
    let mut mismatches = Vec::new();

    for case in cases {
        let id = case_id(case)?;
        if !seen.insert(id.clone()) {
            return Err(format!("measurement integrity duplicate input case ID: {id}"));
        }

        let times = at(case, &["sample_times_seconds"])?
            .as_array()
            .ok_or_else(|| format!("{id}: sample_times_seconds must be an array"))?;
        if times.len() < 5 {
            return Err(format!("{id}: at least five timestamps are required"));
        }
        let mut last_time = None;
        for raw_time in times {
            let time = raw_time
                .as_i64()
                .ok_or_else(|| format!("{id}: sample timestamp must be an integer"))?;
            if let Some(previous) = last_time
                && time - previous != sampling_interval as i64
            {
                return Err(format!("{id}: sample timestamps must be strictly 300 seconds apart"));
            }
            last_time = Some(time);
        }

        let mut series: BTreeMap<&str, Vec<Option<f64>>> = BTreeMap::new();
        let mut missing_fraction = serde_json::Map::new();
        let mut flatline_runs = serde_json::Map::new();
        let mut range_fractions = serde_json::Map::new();
        let mut outside_envelope = Vec::new();
        for name in CHANNELS {
            let values = read_series(case, name)?;
            if values.len() != times.len() {
                return Err(format!("{id}: channel {name} length does not match timestamps"));
            }
            let present: Vec<f64> = values.iter().filter_map(|value| *value).collect();
            let missing = (values.len() - present.len()) as f64 / values.len() as f64;
            let mean = if present.is_empty() {
                0.0
            } else {
                present.iter().sum::<f64>() / present.len() as f64
            };
            let flatline = longest_flatline(
                &values,
                (mean.abs() * flatline_epsilon_fraction).max(1.0e-12),
            );
            let range = relative_range(&values);
            let lower = number(at(case, &["declared_envelopes", name])?, &["min"])?;
            let upper = number(at(case, &["declared_envelopes", name])?, &["max"])?;
            if lower > upper {
                return Err(format!("{id}: invalid declared envelope for {name}"));
            }
            if present.iter().any(|value| *value < lower || *value > upper) {
                outside_envelope.push(name.to_owned());
            }
            missing_fraction.insert(name.to_owned(), json!(missing));
            flatline_runs.insert(name.to_owned(), json!(flatline));
            range_fractions.insert(name.to_owned(), json!(range));
            series.insert(name, values);
        }

        let mut common_gap_max = 0usize;
        let mut common_gap_run = 0usize;
        for index in 0..times.len() {
            let all_missing = CHANNELS
                .iter()
                .all(|name| series.get(name).is_some_and(|values| values[index].is_none()));
            if all_missing {
                common_gap_run += 1;
                common_gap_max = common_gap_max.max(common_gap_run);
            } else {
                common_gap_run = 0;
            }
        }

        let missing_channels: Vec<String> = CHANNELS
            .iter()
            .filter(|name| {
                missing_fraction
                    .get(**name)
                    .and_then(Value::as_f64)
                    .is_some_and(|fraction| fraction > max_missing_fraction)
            })
            .map(|name| (*name).to_owned())
            .collect();

        let mut stuck_channels = Vec::new();
        for name in CHANNELS {
            let flatline = flatline_runs
                .get(name)
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            if flatline >= min_stuck_run {
                let peers_vary = CHANNELS
                    .iter()
                    .filter(|peer| **peer != name)
                    .any(|peer| {
                        range_fractions
                            .get(*peer)
                            .and_then(Value::as_f64)
                            .is_some_and(|range| range >= peer_variation_fraction)
                    });
                if peers_vary {
                    stuck_channels.push(name.to_owned());
                }
            }
        }

        let flux = series
            .get("flux_l_m2_h")
            .ok_or_else(|| format!("{id}: flux channel missing"))?;
        let tmp = series
            .get("tmp_bar")
            .ok_or_else(|| format!("{id}: TMP channel missing"))?;
        let permeability = series
            .get("permeability_l_m2_h_bar")
            .ok_or_else(|| format!("{id}: permeability channel missing"))?;
        let mut residuals = Vec::new();
        for index in 0..times.len() {
            if let (Some(flux_value), Some(tmp_value), Some(permeability_value)) =
                (flux[index], tmp[index], permeability[index])
                && tmp_value > 0.0
            {
                residuals.push(
                    (flux_value / tmp_value - permeability_value).abs()
                        / permeability_value.abs().max(1.0e-9),
                );
            }
        }
        let max_residual = residuals.iter().copied().reduce(f64::max);
        let median_residual = median(&mut residuals);
        let inconsistent = residuals.len() >= min_consistency_points
            && median_residual.is_some_and(|value| value > consistency_limit);

        fn relative_delta(values: &[Option<f64>]) -> f64 {
            let present: Vec<f64> = values.iter().filter_map(|value| *value).collect();
            if present.len() < 2 {
                0.0
            } else {
                (present[present.len() - 1] - present[0]) / present[0].abs().max(1.0e-9)
            }
        }
        let tmp_rise = relative_delta(tmp);
        let flux_drop = -relative_delta(flux);
        let permeability_drop = -relative_delta(permeability);
        let coherent_process_shift = tmp_rise >= process_tmp_rise_limit
            && flux_drop >= process_flux_drop_limit
            && permeability_drop >= process_flux_drop_limit;

        let mut flags = Vec::new();
        if !outside_envelope.is_empty() {
            flags.push("OUTSIDE_DECLARED_SYNTHETIC_ENVELOPE");
        }
        if common_gap_max >= 2 {
            flags.push("COMMON_HISTORIAN_GAP");
        }
        if !missing_channels.is_empty() {
            flags.push("CHANNEL_MISSINGNESS");
        }
        if !stuck_channels.is_empty() {
            flags.push("STUCK_AT_WITH_PEER_MOVEMENT");
        }
        if inconsistent {
            flags.push("CROSS_CHANNEL_CONSISTENCY_RESIDUAL");
        }
        if coherent_process_shift {
            flags.push("COHERENT_HYDRAULIC_PROCESS_SHIFT");
        }

        let state = if !outside_envelope.is_empty() {
            "OUT_OF_DOMAIN_ABSTAIN"
        } else if common_gap_max >= 2 {
            "HISTORIAN_OUTAGE"
        } else if !stuck_channels.is_empty() && !missing_channels.is_empty() {
            "INDETERMINATE"
        } else if !missing_channels.is_empty() {
            "CHANNEL_DATA_GAP"
        } else if !stuck_channels.is_empty() {
            "STUCK_SENSOR_SUSPECTED"
        } else if inconsistent {
            "CROSS_CHANNEL_INCONSISTENCY"
        } else if coherent_process_shift {
            "PROCESS_SHIFT_COHERENT"
        } else {
            "HEALTHY_STABLE"
        };

        let suspect_channels = if !stuck_channels.is_empty() {
            stuck_channels.clone()
        } else if inconsistent {
            vec!["permeability_l_m2_h_bar".to_owned()]
        } else {
            Vec::new()
        };

        let actual = json!({
            "state": state,
            "suspect_channels": suspect_channels,
            "flags": flags,
            "missing_fraction_by_channel": missing_fraction,
            "longest_flatline_run_by_channel": flatline_runs,
            "range_fraction_by_channel": range_fractions,
            "max_consistency_relative_error": max_residual,
            "median_consistency_relative_error": median_residual,
            "coherent_process_deltas": {
                "tmp_rise_fraction": tmp_rise,
                "flux_drop_fraction": flux_drop,
                "permeability_drop_fraction": permeability_drop
            },
            "common_historian_gap_max_samples": common_gap_max,
            "fouling_mechanism": "NOT_IDENTIFIED"
        });

        let oracle_row = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("measurement integrity oracle missing case {id}"))?;
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
            "family": "MEMBRANE_MEASUREMENT_INTEGRITY",
            "matched": matches,
            "computed": actual
        }));
    }

    if seen.len() != expected_by_id.len() {
        return Err(format!(
            "measurement integrity input/oracle identity mismatch: input={}, oracle={}",
            seen.len(),
            expected_by_id.len()
        ));
    }
    results.sort_by(|left, right| {
        left.get("case_id")
            .and_then(Value::as_str)
            .cmp(&right.get("case_id").and_then(Value::as_str))
    });
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version": "membrane-measurement-integrity-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_measurement_integrity",
        "evidence_ceiling": CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "This is synthetic telemetry integrity arithmetic, not a plant sensor diagnosis.",
            "No fouling mechanism or membrane integrity conclusion is made.",
            "No water-quality, potability or normalized RO performance is established.",
            "This evaluator has no controller, CIP, chemical dosing or actuator authority."
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
            "measurement_integrity_inputs" => include_str!(
                "../../../research/water-service-engineering/membrane-measurement-integrity-cases-v0.1.json"
            ),
            "measurement_integrity_oracle" => include_str!(
                "../../../research/water-service-engineering/membrane-measurement-integrity-oracle-v0.1.json"
            ),
            _ => panic!("unknown test fixture"),
        };
        serde_json::from_str(text).expect("fixture JSON must parse")
    }

    #[test]
    fn all_eight_measurement_integrity_cases_match_separate_oracle() {
        let result = evaluate_membrane_measurement_integrity(
            &load("measurement_integrity_inputs"),
            &load("measurement_integrity_oracle"),
        )
        .expect("M1 measurement integrity cases must evaluate");
        assert_eq!(result["result_count"], 8);
        assert_eq!(result["matched_case_count"], 8);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert_eq!(result["mismatches"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn measurement_integrity_never_labels_a_coherent_process_shift_as_fouling() {
        let result = evaluate_membrane_measurement_integrity(
            &load("measurement_integrity_inputs"),
            &load("measurement_integrity_oracle"),
        )
        .expect("M1 measurement integrity cases must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "MI-M1-002-coherent-process-shift")
            .unwrap();
        assert_eq!(row["computed"]["state"], "PROCESS_SHIFT_COHERENT");
        assert_eq!(row["computed"]["fouling_mechanism"], "NOT_IDENTIFIED");
    }

    #[test]
    fn measurement_integrity_abstains_on_ambiguous_stuck_and_dropout() {
        let result = evaluate_membrane_measurement_integrity(
            &load("measurement_integrity_inputs"),
            &load("measurement_integrity_oracle"),
        )
        .expect("M1 measurement integrity cases must evaluate");
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "MI-M1-008-ambiguous-stuck-plus-peer-dropout")
            .unwrap();
        assert_eq!(row["computed"]["state"], "INDETERMINATE");
        assert_eq!(row["computed"]["fouling_mechanism"], "NOT_IDENTIFIED");
    }

    #[test]
    fn measurement_integrity_rejects_nonmonotonic_timebase() {
        let mut inputs = load("measurement_integrity_inputs");
        inputs["cases"][0]["sample_times_seconds"][2] = json!(250);
        assert!(evaluate_membrane_measurement_integrity(
            &inputs,
            &load("measurement_integrity_oracle")
        ).is_err());
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
