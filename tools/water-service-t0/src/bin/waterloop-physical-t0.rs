// SPDX-License-Identifier: AGPL-3.0-or-later
//! Frozen synthetic evaluator for the first WaterLoop physical test-cell contract.
//!
//! This binary evaluates arithmetic and evidence-admission semantics only. It is
//! not a hydraulic solver, water-quality authority, or physical qualification.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CASE_COUNT: usize = 34;
const SCHEMA_VERSION: &str = "waterloop-physical-t0-v0.1";
const CLAIM_CEILING: &str = "synthetic_water_service_arithmetic_and_contract_semantics_only";

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {key}"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key}: expected finite number"))
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{key}: expected non-empty string"))
}

fn boolean(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("{key}: expected boolean"))
}

fn nonnegative(value: f64, name: &str) -> Result<f64, String> {
    if value < 0.0 {
        Err(format!("{name}: must be non-negative"))
    } else {
        Ok(value)
    }
}

fn positive(value: f64, name: &str) -> Result<f64, String> {
    if value <= 0.0 {
        Err(format!("{name}: must be positive"))
    } else {
        Ok(value)
    }
}

fn nearly_equal(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1.0e-9 * scale
}

fn case_id(case: &Value) -> Result<String, String> {
    text(case, "case_id")
}

fn compute_service_continuity(input: &Value) -> Result<Value, String> {
    let initial = nonnegative(number(input, "initial_storage_l")?, "initial_storage_l")?;
    let maximum = positive(number(input, "maximum_storage_l")?, "maximum_storage_l")?;
    let reserve = nonnegative(
        number(input, "protected_discretionary_reserve_l")?,
        "protected_discretionary_reserve_l",
    )?;
    if initial > maximum || reserve > maximum {
        return Err("initial storage or protected reserve exceeds maximum storage".to_owned());
    }

    let intended_profile = text(input, "intended_use_profile_id")?;
    let stored_profile = text(input, "stored_quality_profile_id")?;
    let stored_quality_admitted = boolean(input, "stored_use_quality_admitted")?;
    let steps = field(input, "steps")?
        .as_array()
        .ok_or_else(|| "steps: expected array".to_owned())?;
    if steps.is_empty() {
        return Err("steps: must not be empty".to_owned());
    }

    let mut total_duration = 0.0;
    for (index, step) in steps.iter().enumerate() {
        let duration = positive(number(step, "duration_hours")?, "duration_hours")?;
        total_duration += duration;
        for key in [
            "source_inflow_l",
            "essential_demand_l",
            "discretionary_demand_l",
            "measured_service_capacity_l",
        ] {
            nonnegative(number(step, key)?, key)?;
        }
        boolean(step, "source_inflow_use_quality_admitted")?;
        boolean(step, "service_measurement_admitted")?;
        boolean(step, "service_floor_met")?;
        if index > 0 && !total_duration.is_finite() {
            return Err("total scenario duration overflowed".to_owned());
        }
    }

    let mut abstention_reason = if !stored_quality_admitted {
        Some("STORED_WATER_QUALITY_NOT_ADMITTED")
    } else if intended_profile != stored_profile {
        Some("STORED_WATER_USE_PROFILE_MISMATCH")
    } else {
        None
    };
    let mut abstention_step = None;
    for (index, step) in steps.iter().enumerate() {
        if !boolean(step, "service_measurement_admitted")? {
            abstention_reason = Some("SERVICE_MEASUREMENT_NOT_ADMITTED");
            abstention_step = Some(index + 1);
            break;
        }
    }
    if let Some(reason) = abstention_reason {
        return Ok(json!({
            "service_disposition": "INDETERMINATE",
            "reason": reason,
            "credit_eligible": false,
            "abstention_step": abstention_step,
            "intended_use_profile_id": intended_profile,
            "total_duration_hours": total_duration,
            "total_essential_demand_l": null,
            "essential_delivered_l": null,
            "essential_unmet_l": null,
            "essential_volume_coverage": null,
            "critical_interval_availability": null,
            "critical_intervals_met": null,
            "critical_intervals_total": null,
            "first_essential_shortfall_step": null,
            "hours_to_first_essential_shortfall": null,
            "total_discretionary_demand_l": null,
            "discretionary_delivered_l": null,
            "discretionary_unmet_l": null,
            "admitted_source_inflow_l": null,
            "rejected_source_inflow_l": null,
            "overflow_l": null,
            "end_storage_l": null
        }));
    }

    let mut storage = initial;
    let mut essential_demand = 0.0;
    let mut essential_delivered = 0.0;
    let mut discretionary_demand = 0.0;
    let mut discretionary_delivered = 0.0;
    let mut admitted_inflow = 0.0;
    let mut rejected_inflow = 0.0;
    let mut overflow_total = 0.0;
    let mut critical_intervals = 0_u64;
    let mut critical_intervals_met = 0_u64;
    let mut first_shortfall = None;
    let mut elapsed_hours = 0.0;

    for (index, step) in steps.iter().enumerate() {
        let duration = number(step, "duration_hours")?;
        let inflow = number(step, "source_inflow_l")?;
        let inflow_admitted = boolean(step, "source_inflow_use_quality_admitted")?;
        let critical = number(step, "essential_demand_l")?;
        let discretionary = number(step, "discretionary_demand_l")?;
        let measured_capacity = number(step, "measured_service_capacity_l")?;
        let service_floor_met = boolean(step, "service_floor_met")?;

        essential_demand += critical;
        discretionary_demand += discretionary;

        if inflow_admitted {
            storage += inflow;
            admitted_inflow += inflow;
        } else {
            rejected_inflow += inflow;
        }

        let overflow = (storage - maximum).max(0.0);
        storage -= overflow;
        overflow_total += overflow;

        // This capacity is a fixture-boundary service measurement, not a pump
        // curve or hydraulic model. Below-floor delivery earns no declared
        // service credit in this benchmark profile.
        let mut service_capacity = if service_floor_met {
            measured_capacity
        } else {
            0.0
        };

        // Critical demand is always allocated before discretionary demand.
        let delivered_critical = critical.min(service_capacity).min(storage);
        storage -= delivered_critical;
        service_capacity -= delivered_critical;
        essential_delivered += delivered_critical;

        // Discretionary demand cannot consume the protected reserve. Essential
        // demand may consume it when necessary; the shortfall remains visible.
        let discretionary_available = (storage - reserve).max(0.0);
        let delivered_discretionary = discretionary
            .min(service_capacity)
            .min(discretionary_available);
        storage -= delivered_discretionary;
        discretionary_delivered += delivered_discretionary;

        if critical > 0.0 {
            critical_intervals += 1;
            if delivered_critical + 1.0e-9 >= critical {
                critical_intervals_met += 1;
            } else if first_shortfall.is_none() {
                first_shortfall = Some(index + 1);
            }
        }
        elapsed_hours += duration;
    }

    let coverage = if essential_demand > 0.0 {
        Some(essential_delivered / essential_demand)
    } else {
        None
    };
    let interval_availability = if critical_intervals > 0 {
        Some(critical_intervals_met as f64 / critical_intervals as f64)
    } else {
        None
    };
    let hours_to_shortfall = first_shortfall.map(|step| {
        steps
            .iter()
            .take(step - 1)
            .map(|row| number(row, "duration_hours").unwrap_or(0.0))
            .sum::<f64>()
    });

    Ok(json!({
        "service_disposition": "EVIDENCE_ADMITTED",
        "reason": "ALL_REQUIRED_MEASUREMENT_AND_USE_EVIDENCE_ADMITTED",
        "credit_eligible": true,
        "intended_use_profile_id": intended_profile,
        "total_duration_hours": elapsed_hours,
        "total_essential_demand_l": essential_demand,
        "essential_delivered_l": essential_delivered,
        "essential_unmet_l": essential_demand - essential_delivered,
        "essential_volume_coverage": coverage,
        "critical_interval_availability": interval_availability,
        "critical_intervals_met": critical_intervals_met,
        "critical_intervals_total": critical_intervals,
        "first_essential_shortfall_step": first_shortfall,
        "hours_to_first_essential_shortfall": hours_to_shortfall,
        "total_discretionary_demand_l": discretionary_demand,
        "discretionary_delivered_l": discretionary_delivered,
        "discretionary_unmet_l": discretionary_demand - discretionary_delivered,
        "admitted_source_inflow_l": admitted_inflow,
        "rejected_source_inflow_l": rejected_inflow,
        "overflow_l": overflow_total,
        "end_storage_l": storage
    }))
}

fn compute_water_economics(input: &Value) -> Result<Value, String> {
    let currency = text(input, "currency")?;
    if currency != "ZAR" {
        return Err("T0 water economics requires currency ZAR".to_owned());
    }
    let price_date = text(input, "price_date")?;
    let region = text(input, "region")?;
    let cost_profile = text(input, "cost_use_profile_id")?;
    let service_profile = text(input, "service_use_profile_id")?;
    let period_days = positive(number(input, "cost_period_days")?, "cost_period_days")?;
    let cost_admitted = boolean(input, "cost_ledger_admitted")?;
    let tariff_admitted = boolean(input, "tariff_evidence_admitted")?;
    let income_admitted = boolean(input, "income_evidence_admitted")?;
    let service_admitted = boolean(input, "service_evidence_admitted")?;
    let service_floors_met = boolean(input, "service_floors_met")?;
    let schedule = text(input, "tariff_schedule_revision")?;
    let bill_method = text(input, "tariff_bill_method_id")?;
    let tariff_period_days = if tariff_admitted {
        Some(positive(
            number(input, "tariff_billing_period_days")?,
            "tariff_billing_period_days",
        )?)
    } else {
        None
    };
    let tariff_period_matches =
        tariff_period_days.is_some_and(|days| nearly_equal(days, period_days));

    let mut gross_system_cost = None;
    let mut energy_cost = None;
    if cost_admitted {
        let capex = nonnegative(
            number(input, "monthly_capex_equivalent_zar")?,
            "monthly_capex_equivalent_zar",
        )?;
        let maintenance = nonnegative(
            number(input, "maintenance_repair_zar")?,
            "maintenance_repair_zar",
        )?;
        let energy_kwh = nonnegative(number(input, "electricity_kwh")?, "electricity_kwh")?;
        let energy_rate = nonnegative(
            number(input, "electricity_tariff_zar_per_kwh")?,
            "electricity_tariff_zar_per_kwh",
        )?;
        let consumables = nonnegative(
            number(input, "consumables_replacement_zar")?,
            "consumables_replacement_zar",
        )?;
        let external_water = nonnegative(
            number(input, "external_water_delivery_zar")?,
            "external_water_delivery_zar",
        )?;
        let residual = nonnegative(
            number(input, "residual_disposal_zar")?,
            "residual_disposal_zar",
        )?;
        let computed_energy = energy_kwh * energy_rate;
        energy_cost = Some(computed_energy);
        gross_system_cost =
            Some(capex + maintenance + computed_energy + consumables + external_water + residual);
    }

    let mut baseline_bill = None;
    let mut actual_bill = None;
    let mut baseline_fixed = None;
    let mut actual_fixed = None;
    let mut bill_delta = None;
    if tariff_admitted && tariff_period_matches {
        let baseline = nonnegative(
            number(input, "baseline_utility_bill_zar")?,
            "baseline_utility_bill_zar",
        )?;
        let actual = nonnegative(
            number(input, "actual_utility_bill_zar")?,
            "actual_utility_bill_zar",
        )?;
        let fixed_before = nonnegative(
            number(input, "baseline_fixed_charge_zar")?,
            "baseline_fixed_charge_zar",
        )?;
        let fixed_after = nonnegative(
            number(input, "actual_fixed_charge_zar")?,
            "actual_fixed_charge_zar",
        )?;
        if fixed_before > baseline || fixed_after > actual {
            return Err("fixed utility charge exceeds its admitted total bill".to_owned());
        }
        baseline_bill = Some(baseline);
        actual_bill = Some(actual);
        baseline_fixed = Some(fixed_before);
        actual_fixed = Some(fixed_after);
        bill_delta = Some(actual - baseline);
    }

    let service_use_matches = cost_profile == service_profile;
    let service_credit = service_admitted && service_use_matches;
    let useful_volume = if service_credit {
        Some(nonnegative(
            number(input, "verified_useful_delivered_m3")?,
            "verified_useful_delivered_m3",
        )?)
    } else {
        None
    };
    let critical_person_days = if service_credit {
        Some(nonnegative(
            number(input, "critical_person_days_delivered")?,
            "critical_person_days_delivered",
        )?)
    } else {
        None
    };
    let income = if income_admitted {
        Some(positive(
            number(input, "monthly_disposable_income_zar")?,
            "monthly_disposable_income_zar",
        )?)
    } else {
        None
    };

    let incremental_cost = match (gross_system_cost, bill_delta) {
        (Some(system), Some(delta)) => Some(system + delta),
        _ => None,
    };
    let monthly_spend = match (gross_system_cost, actual_bill) {
        (Some(system), Some(actual)) => Some(system + actual),
        _ => None,
    };
    let gross_cost_per_useful = match (gross_system_cost, useful_volume) {
        (Some(cost), Some(volume)) if volume > 0.0 => Some(cost / volume),
        _ => None,
    };
    let incremental_cost_per_useful = match (incremental_cost, useful_volume) {
        (Some(cost), Some(volume)) if volume > 0.0 => Some(cost / volume),
        _ => None,
    };
    let cost_per_critical_person_day = match (gross_system_cost, critical_person_days) {
        (Some(cost), Some(days)) if days > 0.0 => Some(cost / days),
        _ => None,
    };
    let affordability_burden = match (monthly_spend, income) {
        (Some(spend), Some(income)) => Some(spend / income),
        _ => None,
    };

    let service_credit_disposition = if !service_admitted {
        "INDETERMINATE"
    } else if !service_use_matches {
        "NOT_CREDITABLE"
    } else {
        "ADMITTED"
    };
    let service_equivalence = if !service_admitted {
        "INDETERMINATE"
    } else if !service_use_matches {
        "NOT_COMPARABLE"
    } else if !service_floors_met {
        "FAIL"
    } else {
        "PASS"
    };
    let economic_comparison =
        if !service_admitted || !cost_admitted || !tariff_admitted || !tariff_period_matches {
            "INDETERMINATE"
        } else if !service_use_matches {
            "NOT_COMPARABLE"
        } else if !service_floors_met {
            "SERVICE_FLOORS_NOT_MET"
        } else {
            "ADMITTED"
        };
    let affordability_disposition = if !income_admitted
        || !cost_admitted
        || !tariff_admitted
        || !tariff_period_matches
        || !service_admitted
    {
        "INDETERMINATE"
    } else if !service_use_matches {
        "NOT_COMPARABLE"
    } else if !service_floors_met {
        "NOT_SERVICE_EQUIVALENT"
    } else {
        "COMPUTED_NO_UNIVERSAL_THRESHOLD_APPLIED"
    };

    Ok(json!({
        "cost_ledger_disposition": if cost_admitted { "ADMITTED" } else { "INDETERMINATE" },
        "cost_reason": if cost_admitted { "ALL_DECLARED_COST_COMPONENTS_ADMITTED" } else { "COST_LEDGER_NOT_ADMITTED" },
        "tariff_disposition": if tariff_admitted && tariff_period_matches { "ADMITTED" } else { "INDETERMINATE" },
        "tariff_reason": if !tariff_admitted { "TARIFF_EVIDENCE_NOT_ADMITTED_NO_SAVINGS_CREDIT" } else if !tariff_period_matches { "TARIFF_BILLING_PERIOD_MISMATCH" } else { "VERSIONED_BASELINE_AND_ACTUAL_BILLS_ADMITTED" },
        "tariff_billing_period_days": tariff_period_days,
        "tariff_schedule_revision": schedule,
        "tariff_bill_method_id": bill_method,
        "service_credit_disposition": service_credit_disposition,
        "service_equivalence_disposition": service_equivalence,
        "service_reason": if !service_admitted { "SERVICE_EVIDENCE_NOT_ADMITTED" } else if !service_use_matches { "COST_AND_SERVICE_USE_PROFILE_MISMATCH" } else if !service_floors_met { "DECLARED_SERVICE_FLOORS_NOT_MET" } else { "MATCHED_USE_PROFILE_AND_SERVICE_FLOORS_MET" },
        "economic_comparison_disposition": economic_comparison,
        "affordability_disposition": affordability_disposition,
        "economic_preference_eligible": economic_comparison == "ADMITTED",
        "currency": currency,
        "price_date": price_date,
        "region": region,
        "cost_period_days": period_days,
        "cost_use_profile_id": cost_profile,
        "service_use_profile_id": service_profile,
        "verified_useful_delivered_m3": useful_volume,
        "critical_person_days_delivered": critical_person_days,
        "gross_system_cost_zar": gross_system_cost,
        "electricity_cost_zar": energy_cost,
        "baseline_utility_bill_zar": baseline_bill,
        "actual_utility_bill_zar": actual_bill,
        "baseline_fixed_charge_zar": baseline_fixed,
        "actual_fixed_charge_zar": actual_fixed,
        "utility_bill_delta_zar": bill_delta,
        "incremental_household_cost_zar": incremental_cost,
        "monthly_water_spend_zar": monthly_spend,
        "gross_system_cost_per_verified_useful_m3_zar": gross_cost_per_useful,
        "incremental_household_cost_per_verified_useful_m3_zar": incremental_cost_per_useful,
        "direct_system_cost_per_critical_person_day_zar": cost_per_critical_person_day,
        "monthly_water_spend_burden": affordability_burden
    }))
}

fn compute(case: &Value) -> Result<Value, String> {
    let id = case_id(case)?;
    let input = field(case, "inputs")?;

    match id.as_str() {
        "WL-T0-001-balanced-volume-closure" => {
            let initial = nonnegative(number(input, "initial_storage_l")?, "initial_storage_l")?;
            let source = nonnegative(number(input, "source_in_l")?, "source_in_l")?;
            let delivered = nonnegative(number(input, "delivered_l")?, "delivered_l")?;
            let overflow = nonnegative(number(input, "overflow_l")?, "overflow_l")?;
            let drain = nonnegative(number(input, "drain_l")?, "drain_l")?;
            let final_storage = nonnegative(number(input, "final_storage_l")?, "final_storage_l")?;
            let uncertainty = field(input, "u95_components_l")?
                .as_array()
                .ok_or_else(|| "u95_components_l: expected array".to_owned())?;
            if uncertainty.is_empty() {
                return Err("u95_components_l: must not be empty".to_owned());
            }
            let mut uncertainty_bound = 0.0;
            for (index, value) in uncertainty.iter().enumerate() {
                let component = value
                    .as_f64()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or_else(|| format!("u95_components_l[{index}]: invalid bound"))?;
                uncertainty_bound += component;
            }
            let input_total = initial + source;
            let output_total = delivered + overflow + drain + final_storage;
            let residual = input_total - output_total;
            Ok(json!({
                "input_total_l": input_total,
                "output_total_l": output_total,
                "signed_residual_l": residual,
                "conservative_u95_bound_l": uncertainty_bound,
                "closure_disposition": if residual.abs() <= uncertainty_bound { "CLOSED_WITHIN_DECLARED_BOUND" } else { "NOT_CLOSED" },
                "closure_pass": residual.abs() <= uncertainty_bound
            }))
        }
        "WL-T0-002-unbalanced-volume-closure" => {
            let initial = nonnegative(number(input, "initial_storage_l")?, "initial_storage_l")?;
            let source = nonnegative(number(input, "source_in_l")?, "source_in_l")?;
            let delivered = nonnegative(number(input, "delivered_l")?, "delivered_l")?;
            let overflow = nonnegative(number(input, "overflow_l")?, "overflow_l")?;
            let drain = nonnegative(number(input, "drain_l")?, "drain_l")?;
            let final_storage = nonnegative(number(input, "final_storage_l")?, "final_storage_l")?;
            let bound = nonnegative(
                number(input, "conservative_u95_bound_l")?,
                "conservative_u95_bound_l",
            )?;
            let residual = initial + source - delivered - overflow - drain - final_storage;
            Ok(json!({
                "signed_residual_l": residual,
                "conservative_u95_bound_l": bound,
                "closure_disposition": if residual.abs() <= bound { "CLOSED_WITHIN_DECLARED_BOUND" } else { "NOT_CLOSED" },
                "closure_pass": residual.abs() <= bound
            }))
        }
        "WL-T0-003-deliverable-storage-sufficient"
        | "WL-T0-004-deliverable-storage-insufficient" => {
            let gross = nonnegative(
                number(input, "measured_tank_volume_l")?,
                "measured_tank_volume_l",
            )?;
            let heel = nonnegative(
                number(input, "minimum_operating_heel_l")?,
                "minimum_operating_heel_l",
            )?;
            let deliverable = nonnegative(
                number(input, "measured_deliverable_l")?,
                "measured_deliverable_l",
            )?;
            let required = nonnegative(
                number(input, "required_service_volume_l")?,
                "required_service_volume_l",
            )?;
            if heel > gross {
                return Err("minimum operating heel exceeds measured tank volume".to_owned());
            }
            let inventory_ceiling = gross - heel;
            if deliverable > inventory_ceiling + 1.0e-9 {
                return Err(
                    "measured deliverable exceeds gross inventory minus operating heel".to_owned(),
                );
            }
            let sufficient = deliverable >= required;
            Ok(json!({
                "inventory_above_heel_ceiling_l": inventory_ceiling,
                "measured_deliverable_l": deliverable,
                "required_service_volume_l": required,
                "delivery_margin_l": deliverable - required,
                "service_disposition": if sufficient { "SUFFICIENT" } else { "INSUFFICIENT" },
                "service_credit_eligible": sufficient
            }))
        }
        "WL-T0-005-qualified-source-substitution"
        | "WL-T0-006-unqualified-source-no-credit"
        | "WL-T0-013-source-quality-without-evidence-no-credit" => {
            let recovered = nonnegative(
                number(input, "recovered_outlet_delivery_l")?,
                "recovered_outlet_delivery_l",
            )?;
            let demand = nonnegative(
                number(input, "matched_end_use_demand_l")?,
                "matched_end_use_demand_l",
            )?;
            let source_confirmed = boolean(input, "source_identity_confirmed")?;
            let meter_calibrated = boolean(input, "meter_calibration_current")?;
            let same_window = boolean(input, "same_comparison_window")?;
            let quality_evidence_ref = boolean(input, "quality_evidence_reference_present")?;
            let quality_profile_revision = boolean(input, "quality_profile_revision_present")?;
            let recovered_use = text(input, "recovered_use_class")?;
            let baseline_use = text(input, "baseline_use_class")?;
            let quality = text(input, "quality_disposition")?;
            let use_matches = recovered_use == baseline_use;
            let eligible = source_confirmed
                && meter_calibrated
                && same_window
                && quality_evidence_ref
                && quality_profile_revision
                && use_matches
                && quality == "QUALIFIED_FOR_DECLARED_USE";
            let credit = if eligible { recovered.min(demand) } else { 0.0 };
            let reason = if !source_confirmed {
                "SOURCE_IDENTITY_UNCONFIRMED"
            } else if !meter_calibrated {
                "METER_CALIBRATION_NOT_CURRENT"
            } else if !same_window {
                "COMPARISON_WINDOW_MISMATCH"
            } else if !quality_evidence_ref {
                "QUALITY_EVIDENCE_REFERENCE_MISSING"
            } else if !quality_profile_revision {
                "QUALITY_PROFILE_REVISION_MISSING"
            } else if !use_matches {
                "END_USE_MISMATCH"
            } else if quality != "QUALIFIED_FOR_DECLARED_USE" {
                "QUALITY_NOT_QUALIFIED_FOR_DECLARED_USE"
            } else {
                "ALL_CREDIT_GATES_MET"
            };
            Ok(json!({
                "source_substitution_disposition": if eligible { "CREDITABLE_FOR_DECLARED_USE" } else { "NOT_CREDITABLE" },
                "reason": reason,
                "credited_mains_displacement_l": credit,
                "unmet_matched_demand_l": demand - credit,
                "recovered_delivery_not_credited_l": recovered - credit
            }))
        }
        "WL-T0-007-hydraulic-power-known-answer" => {
            let density = positive(number(input, "water_density_kg_m3")?, "water_density_kg_m3")?;
            let gravity = positive(number(input, "gravity_m_s2")?, "gravity_m_s2")?;
            let flow = nonnegative(number(input, "flow_m3_s")?, "flow_m3_s")?;
            let head = nonnegative(
                number(input, "total_dynamic_head_m")?,
                "total_dynamic_head_m",
            )?;
            let hydraulic_power = density * gravity * flow * head;
            Ok(json!({
                "hydraulic_power_w": hydraulic_power
            }))
        }
        "WL-T0-008-wire-to-water-energy" => {
            let hydraulic_energy = nonnegative(
                number(input, "hydraulic_output_energy_kwh")?,
                "hydraulic_output_energy_kwh",
            )?;
            let electrical_energy = positive(
                number(input, "electrical_input_energy_kwh")?,
                "electrical_input_energy_kwh",
            )?;
            let delivered_volume = positive(
                number(input, "measured_delivered_volume_m3")?,
                "measured_delivered_volume_m3",
            )?;
            let windows_aligned = boolean(input, "hydraulic_and_electrical_windows_aligned")?;
            let delivery_boundary_matched = boolean(input, "delivered_volume_boundary_matched")?;
            let auxiliary_loads_included = boolean(input, "auxiliary_loads_included")?;
            if !windows_aligned || !delivery_boundary_matched || !auxiliary_loads_included {
                let reason = if !windows_aligned {
                    "ENERGY_WINDOWS_NOT_ALIGNED"
                } else if !delivery_boundary_matched {
                    "DELIVERY_BOUNDARY_MISMATCH"
                } else {
                    "AUXILIARY_LOADS_OMITTED"
                };
                return Ok(json!({
                    "measurement_disposition": "INDETERMINATE",
                    "reason": reason,
                    "wire_to_water_efficiency": null,
                    "electrical_specific_energy_kwh_m3": null
                }));
            }
            let efficiency = hydraulic_energy / electrical_energy;
            if efficiency > 1.0 + 1.0e-9 {
                return Err(
                    "wire-to-water efficiency exceeds unity; reject evidence/model boundary"
                        .to_owned(),
                );
            }
            Ok(json!({
                "measurement_disposition": "ADMITTED",
                "reason": "BOUNDARIES_AND_WINDOWS_MATCH",
                "wire_to_water_efficiency": efficiency,
                "electrical_specific_energy_kwh_m3": electrical_energy / delivered_volume
            }))
        }
        "WL-T0-009-service-floor-met"
        | "WL-T0-010-service-floor-not-met"
        | "WL-T0-014-service-floor-missing-pressure-evidence" => {
            let pressure = nonnegative(
                number(input, "measured_outlet_pressure_pa")?,
                "measured_outlet_pressure_pa",
            )?;
            let minimum_pressure = nonnegative(
                number(input, "minimum_service_pressure_pa")?,
                "minimum_service_pressure_pa",
            )?;
            let flow = nonnegative(
                number(input, "measured_outlet_flow_m3_s")?,
                "measured_outlet_flow_m3_s",
            )?;
            let minimum_flow = nonnegative(
                number(input, "minimum_service_flow_m3_s")?,
                "minimum_service_flow_m3_s",
            )?;
            let pressure_admitted = boolean(input, "pressure_measurement_admitted")?;
            let flow_admitted = boolean(input, "flow_measurement_admitted")?;
            let pressure_met = pressure >= minimum_pressure;
            let flow_met = flow >= minimum_flow;
            let evidence_complete = pressure_admitted && flow_admitted;
            let feasible = evidence_complete && pressure_met && flow_met;
            let reason = if !evidence_complete {
                "CRITICAL_MEASUREMENT_NOT_ADMITTED"
            } else if !pressure_met {
                "MINIMUM_OUTLET_PRESSURE_NOT_MET"
            } else if !flow_met {
                "MINIMUM_OUTLET_FLOW_NOT_MET"
            } else {
                "ALL_SERVICE_FLOORS_MET"
            };
            Ok(json!({
                "service_disposition": if !evidence_complete { "INDETERMINATE" } else if feasible { "FEASIBLE" } else { "INFEASIBLE" },
                "reason": reason,
                "pressure_floor_met": if pressure_admitted { json!(pressure_met) } else { Value::Null },
                "flow_floor_met": if flow_admitted { json!(flow_met) } else { Value::Null },
                "service_credit_eligible": feasible
            }))
        }
        "WL-T0-011-leak-event-metrics" => {
            let tp = number(input, "true_positive_events")?;
            let fp = number(input, "false_positive_events")?;
            let fn_count = number(input, "false_negative_events")?;
            let sensor_days = positive(number(input, "valid_sensor_days")?, "valid_sensor_days")?;
            for (name, value) in [
                ("true_positive_events", tp),
                ("false_positive_events", fp),
                ("false_negative_events", fn_count),
            ] {
                if value < 0.0 || value.fract() != 0.0 {
                    return Err(format!("{name}: expected non-negative integer count"));
                }
            }
            let precision_denominator = tp + fp;
            let recall_denominator = tp + fn_count;
            if precision_denominator == 0.0 || recall_denominator == 0.0 {
                return Err(
                    "precision or recall denominator is zero; result is undefined".to_owned(),
                );
            }
            Ok(json!({
                "event_precision": tp / precision_denominator,
                "event_recall": tp / recall_denominator,
                "false_alarms_per_100_sensor_days": fp / sensor_days * 100.0
            }))
        }
        "WL-T0-012-missed-leak-volume" => {
            let rate = nonnegative(
                number(input, "reference_leak_rate_l_min")?,
                "reference_leak_rate_l_min",
            )?;
            let latency = nonnegative(
                number(input, "detection_latency_min")?,
                "detection_latency_min",
            )?;
            let rate_bound =
                nonnegative(number(input, "leak_rate_u95_l_min")?, "leak_rate_u95_l_min")?;
            let latency_bound = nonnegative(number(input, "latency_u95_min")?, "latency_u95_min")?;
            let point = rate * latency;
            let upper = (rate + rate_bound) * (latency + latency_bound);
            Ok(json!({
                "point_missed_volume_l": point,
                "conservative_upper_missed_volume_l": upper
            }))
        }
        "WL-T0-015-energy-boundary-mismatch-abstains" => {
            let hydraulic_energy = nonnegative(
                number(input, "hydraulic_output_energy_kwh")?,
                "hydraulic_output_energy_kwh",
            )?;
            let electrical_energy = positive(
                number(input, "electrical_input_energy_kwh")?,
                "electrical_input_energy_kwh",
            )?;
            let delivered_volume = positive(
                number(input, "measured_delivered_volume_m3")?,
                "measured_delivered_volume_m3",
            )?;
            let windows_aligned = boolean(input, "hydraulic_and_electrical_windows_aligned")?;
            let delivery_boundary_matched = boolean(input, "delivered_volume_boundary_matched")?;
            let auxiliary_loads_included = boolean(input, "auxiliary_loads_included")?;
            if !windows_aligned || !delivery_boundary_matched || !auxiliary_loads_included {
                let reason = if !windows_aligned {
                    "ENERGY_WINDOWS_NOT_ALIGNED"
                } else if !delivery_boundary_matched {
                    "DELIVERY_BOUNDARY_MISMATCH"
                } else {
                    "AUXILIARY_LOADS_OMITTED"
                };
                return Ok(json!({
                    "measurement_disposition": "INDETERMINATE",
                    "reason": reason,
                    "wire_to_water_efficiency": null,
                    "electrical_specific_energy_kwh_m3": null
                }));
            }
            let efficiency = hydraulic_energy / electrical_energy;
            if efficiency > 1.0 + 1.0e-9 {
                return Err(
                    "wire-to-water efficiency exceeds unity; reject evidence/model boundary"
                        .to_owned(),
                );
            }
            Ok(json!({
                "measurement_disposition": "ADMITTED",
                "reason": "BOUNDARIES_AND_WINDOWS_MATCH",
                "wire_to_water_efficiency": efficiency,
                "electrical_specific_energy_kwh_m3": electrical_energy / delivered_volume
            }))
        }

        "WL-T0-016-continuity-normal-service"
        | "WL-T0-017-sa-reg4-72h-alternative-drinking-supply"
        | "WL-T0-018-critical-reserve-insufficient"
        | "WL-T0-019-pump-outage-gravity-fallback"
        | "WL-T0-020-compound-demand-surge-critical-priority"
        | "WL-T0-021-source-restoration-after-outage"
        | "WL-T0-022-outlet-service-floor-failure"
        | "WL-T0-023-unadmitted-service-measurement-abstains"
        | "WL-T0-024-stored-water-use-quality-unqualified"
        | "WL-T0-025-unqualified-source-inflow-not-credited" => compute_service_continuity(input),
        "WL-T0-026-economics-admitted-cost-and-service"
        | "WL-T0-027-economics-rising-block-bill-delta"
        | "WL-T0-028-economics-unadmitted-tariff-no-savings"
        | "WL-T0-029-economics-zero-useful-volume-undefined"
        | "WL-T0-030-economics-unadmitted-cost-ledger"
        | "WL-T0-031-economics-income-unavailable"
        | "WL-T0-032-economics-use-profile-mismatch"
        | "WL-T0-033-economics-low-cost-service-floor-failure"
        | "WL-T0-034-economics-tariff-billing-period-mismatch" => compute_water_economics(input),
        _ => Err(format!("unsupported WaterLoop T0 case ID: {id}")),
    }
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => nearly_equal(a, b),
            _ => false,
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| compare_json(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| compare_json(value, other)))
        }
        _ => actual == expected,
    }
}

fn collect_unique_ids(rows: &[Value], name: &str) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for row in rows {
        let id = case_id(row)?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate {name} case ID: {id}"));
        }
    }
    Ok(ids)
}

fn string_id_set(value: &Value, name: &str) -> Result<BTreeSet<String>, String> {
    let rows = value
        .as_array()
        .ok_or_else(|| format!("{name}: expected array"))?;
    let mut ids = BTreeSet::new();
    for row in rows {
        let id = row
            .as_str()
            .ok_or_else(|| format!("{name}: expected string IDs"))?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("duplicate ID in {name}: {id}"));
        }
    }
    Ok(ids)
}

fn evaluate(inputs: &Value, manifest: &Value, oracle: &Value) -> Result<Value, String> {
    if text(inputs, "schema_version")? != SCHEMA_VERSION
        || text(manifest, "schema_version")? != SCHEMA_VERSION
        || text(oracle, "schema_version")? != SCHEMA_VERSION
    {
        return Err("schema version mismatch".to_owned());
    }
    for (name, value) in [
        ("inputs", inputs),
        ("manifest", manifest),
        ("oracle", oracle),
    ] {
        if text(value, "evidence_class")? != "synthetic_only"
            || text(value, "claim_ceiling")? != CLAIM_CEILING
        {
            return Err(format!("{name}: evidence class or claim ceiling mismatch"));
        }
    }

    let cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "inputs.cases: expected array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle.cases: expected array".to_owned())?;
    let input_ids = collect_unique_ids(cases, "input")?;
    let oracle_ids = collect_unique_ids(oracle_cases, "oracle")?;
    let manifest_ids = string_id_set(field(manifest, "required_case_ids")?, "required_case_ids")?;
    let case_count = field(manifest, "case_count")?
        .as_u64()
        .ok_or_else(|| "manifest.case_count: expected unsigned integer".to_owned())?;

    if case_count as usize != CASE_COUNT
        || cases.len() != CASE_COUNT
        || oracle_cases.len() != CASE_COUNT
        || input_ids != oracle_ids
        || input_ids != manifest_ids
    {
        return Err(format!(
            "case identity/count mismatch: inputs={}, oracle={}, manifest={}, declared={case_count}",
            input_ids.len(),
            oracle_ids.len(),
            manifest_ids.len()
        ));
    }

    let mut oracle_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = case_id(row)?;
        oracle_by_id.insert(id, row);
    }

    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
        let id = case_id(case)?;
        let family = text(case, "family")?;
        let actual = compute(case)?;
        let oracle_row = oracle_by_id
            .get(&id)
            .ok_or_else(|| format!("oracle is missing {id}"))?;
        let expected_family = text(oracle_row, "family")?;
        let expected = field(oracle_row, "expected")?;
        let matched = family == expected_family && compare_json(&actual, expected);
        if !matched {
            mismatches.push(json!({
                "case_id": id,
                "input_family": family,
                "oracle_family": expected_family,
                "expected": expected,
                "computed": actual
            }));
        }
        results.push(json!({
            "case_id": id,
            "family": family,
            "matched": matched,
            "computed": actual
        }));
    }

    results.sort_by(|a, b| {
        a.get("case_id")
            .and_then(Value::as_str)
            .cmp(&b.get("case_id").and_then(Value::as_str))
    });
    let matched = results.len().saturating_sub(mismatches.len());
    Ok(json!({
        "schema_version": "waterloop-physical-t0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() { "MatchedFrozenOracle" } else { "OracleMismatch" },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_only",
        "claim_ceiling": CLAIM_CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "Synthetic arithmetic and evidence-contract semantics only; no physical equipment or instrument is qualified.",
            "No potable-water, pathogen-removal, SANS 241, treatment-efficacy, plumbing-code or health claim is established.",
            "No field leak detector, water saving, outage outcome or pump performance is established.",
            "No result authorizes pump/valve actuation, isolation, excavation, procurement or occupied deployment."
        ]
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err(
            "usage: waterloop-physical-t0 [INPUTS] [MANIFEST] [ORACLE] [RECEIPT]".to_owned(),
        );
    }
    let input_path = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or("research/water-service-engineering/waterloop-physical-t0-v0.1.json"),
    );
    let manifest_path = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("research/water-service-engineering/waterloop-physical-manifest-v0.1.json"),
    );
    let oracle_path = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("research/water-service-engineering/waterloop-physical-oracle-v0.1.json"),
    );
    let receipt_path = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/water-service-t0/waterloop-physical-receipt.json"),
    );

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let evaluation = evaluate(&inputs, &manifest, &oracle)?;
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"]
        .as_array()
        .is_some_and(Vec::is_empty);
    let success = total == CASE_COUNT as u64 && matched == CASE_COUNT as u64 && no_mismatches;

    let raw_revision = env::var("WATERLOOP_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes =
        fs::read(&lock_path).map_err(|error| format!("read {}: {error}", lock_path.display()))?;

    let receipt = json!({
        "schema_version": "waterloop-physical-t0-run-v0.1",
        "verification_state": if !success {
            "OracleMismatch"
        } else if revision.is_some() {
            "PassedExactRevision"
        } else {
            "PassedLocalUnpinned"
        },
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "manifest_sha256": sha256(&manifest_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "result_count": total,
        "matched_case_count": matched,
        "mismatches": evaluation["mismatches"].clone(),
        "results": evaluation["results"].clone(),
        "evidence_class": "synthetic_only",
        "claim_ceiling": CLAIM_CEILING,
        "nonclaims": evaluation["nonclaims"].clone()
    });

    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("serialize receipt: {error}"))?
        + "\n";
    fs::write(&receipt_path, serialized.as_bytes())
        .map_err(|error| format!("write {}: {error}", receipt_path.display()))?;
    println!("{serialized}");
    Ok(if success { 0 } else { 2 })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATERLOOP_PHYSICAL_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let raw = match name {
            "inputs" => include_str!(
                "../../../../research/water-service-engineering/waterloop-physical-t0-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-service-engineering/waterloop-physical-manifest-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-service-engineering/waterloop-physical-oracle-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("frozen fixture JSON parses")
    }

    #[test]
    fn all_thirty_four_cases_match_the_separate_oracle() {
        let result = evaluate(&fixture("inputs"), &fixture("manifest"), &fixture("oracle"))
            .expect("evaluation completes");
        assert_eq!(result["result_count"], CASE_COUNT);
        assert_eq!(result["matched_case_count"], CASE_COUNT);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().is_some_and(Vec::is_empty));
    }

    #[test]
    fn duplicate_input_id_is_rejected() {
        let mut inputs = fixture("inputs");
        let first = inputs["cases"][0].clone();
        inputs["cases"]
            .as_array_mut()
            .expect("cases array")
            .push(first);
        assert!(evaluate(&inputs, &fixture("manifest"), &fixture("oracle")).is_err());
    }

    #[test]
    fn incomplete_manifest_case_set_is_rejected() {
        let mut manifest = fixture("manifest");
        manifest["required_case_ids"]
            .as_array_mut()
            .expect("ID array")
            .pop();
        assert!(evaluate(&fixture("inputs"), &manifest, &fixture("oracle")).is_err());
    }

    #[test]
    fn volume_residual_outside_uncertainty_is_not_closed() {
        let case = fixture("inputs")["cases"][1].clone();
        let actual = compute(&case).expect("valid arithmetic inputs");
        assert!(!actual["closure_pass"].as_bool().expect("closure boolean"));
        assert_eq!(actual["closure_disposition"], "NOT_CLOSED");
    }

    #[test]
    fn unknown_quality_never_earns_source_substitution_credit() {
        let case = fixture("inputs")["cases"][5].clone();
        let actual = compute(&case).expect("valid evidence state");
        assert_eq!(actual["credited_mains_displacement_l"], 0.0);
        assert_eq!(actual["source_substitution_disposition"], "NOT_CREDITABLE");
    }

    #[test]
    fn deliverable_cannot_exceed_inventory_above_heel() {
        let mut case = fixture("inputs")["cases"][2].clone();
        case["inputs"]["measured_deliverable_l"] = json!(999.0);
        assert!(compute(&case).is_err());
    }

    #[test]
    fn efficiency_above_unity_is_rejected() {
        let mut case = fixture("inputs")["cases"][7].clone();
        case["inputs"]["hydraulic_output_energy_kwh"] = json!(2.0);
        case["inputs"]["electrical_input_energy_kwh"] = json!(1.0);
        assert!(compute(&case).is_err());
    }

    #[test]
    fn quality_label_without_evidence_reference_is_not_credited() {
        let case = fixture("inputs")["cases"][12].clone();
        let actual = compute(&case).expect("synthetic source-substitution case");
        assert_eq!(actual["source_substitution_disposition"], "NOT_CREDITABLE");
        assert_eq!(actual["credited_mains_displacement_l"], 0.0);
        assert_eq!(actual["reason"], "QUALITY_EVIDENCE_REFERENCE_MISSING");
    }

    #[test]
    fn missing_pressure_measurement_abstains_from_service_credit() {
        let case = fixture("inputs")["cases"][13].clone();
        let actual = compute(&case).expect("synthetic service-floor case");
        assert_eq!(actual["service_disposition"], "INDETERMINATE");
        assert!(
            !actual["service_credit_eligible"]
                .as_bool()
                .expect("service credit boolean")
        );
        assert_eq!(actual["pressure_floor_met"], Value::Null);
    }

    #[test]
    fn mismatched_energy_boundary_abstains_from_efficiency_claim() {
        let case = fixture("inputs")["cases"][14].clone();
        let actual = compute(&case).expect("synthetic energy-boundary case");
        assert_eq!(actual["measurement_disposition"], "INDETERMINATE");
        assert_eq!(actual["wire_to_water_efficiency"], Value::Null);
    }

    #[test]
    fn zero_sensor_days_cannot_create_a_finite_false_alarm_rate() {
        let mut case = fixture("inputs")["cases"][10].clone();
        case["inputs"]["valid_sensor_days"] = json!(0.0);
        assert!(compute(&case).is_err());
    }

    #[test]
    fn oracle_disagreement_is_reported_not_hidden() {
        let mut oracle = fixture("oracle");
        oracle["cases"][0]["expected"]["closure_pass"] = json!(false);
        let result = evaluate(&fixture("inputs"), &fixture("manifest"), &oracle)
            .expect("oracle disagreement is a recorded evaluation");
        assert_eq!(result["verification_state"], "OracleMismatch");
        assert_eq!(result["matched_case_count"], 32);
        assert_eq!(
            result["mismatches"].as_array().expect("mismatches").len(),
            1
        );
    }

    #[test]
    fn continuity_allocates_essential_demand_before_discretionary() {
        let case = fixture("inputs")["cases"][19].clone();
        let actual = compute(&case).expect("synthetic compound-demand case");
        assert_eq!(actual["essential_delivered_l"], 60.0);
        assert_eq!(actual["discretionary_delivered_l"], 50.0);
        assert_eq!(actual["end_storage_l"], 10.0);
        assert_eq!(actual["critical_interval_availability"], 1.0);
    }

    #[test]
    fn continuity_abstains_without_stored_water_use_quality() {
        let case = fixture("inputs")["cases"][23].clone();
        let actual = compute(&case).expect("synthetic quality-admission case");
        assert_eq!(actual["service_disposition"], "INDETERMINATE");
        assert_eq!(actual["reason"], "STORED_WATER_QUALITY_NOT_ADMITTED");
        assert_eq!(actual["essential_delivered_l"], Value::Null);
        assert!(!actual["credit_eligible"].as_bool().expect("credit boolean"));
    }

    #[test]
    fn continuity_excludes_unqualified_source_inflow() {
        let case = fixture("inputs")["cases"][24].clone();
        let actual = compute(&case).expect("synthetic source-inflow case");
        assert_eq!(actual["admitted_source_inflow_l"], 0.0);
        assert_eq!(actual["rejected_source_inflow_l"], 50.0);
        assert_eq!(actual["essential_delivered_l"], 20.0);
    }

    #[test]
    fn continuity_abstains_on_unadmitted_service_measurement() {
        let case = fixture("inputs")["cases"][22].clone();
        let actual = compute(&case).expect("synthetic metrology case");
        assert_eq!(actual["service_disposition"], "INDETERMINATE");
        assert_eq!(actual["reason"], "SERVICE_MEASUREMENT_NOT_ADMITTED");
        assert_eq!(actual["end_storage_l"], Value::Null);
    }

    #[test]
    fn continuity_does_not_credit_service_below_pressure_flow_floor() {
        let case = fixture("inputs")["cases"][21].clone();
        let actual = compute(&case).expect("synthetic service-floor case");
        assert_eq!(actual["essential_delivered_l"], 30.0);
        assert_eq!(actual["essential_unmet_l"], 30.0);
        assert_eq!(actual["first_essential_shortfall_step"], 2);
        assert_eq!(actual["hours_to_first_essential_shortfall"], 24.0);
    }

    #[test]
    fn economics_preserves_fixed_charges_and_reports_burden() {
        let case = fixture("inputs")["cases"][25].clone();
        let actual = compute(&case).expect("admitted cost/service case");
        assert_eq!(actual["gross_system_cost_zar"], 680.0);
        assert_eq!(actual["baseline_fixed_charge_zar"], 150.0);
        assert_eq!(actual["actual_fixed_charge_zar"], 150.0);
        assert_eq!(actual["incremental_household_cost_zar"], 480.0);
        assert!((actual["monthly_water_spend_burden"].as_f64().unwrap() - 0.06).abs() < 1.0e-12);
    }

    #[test]
    fn economics_does_not_invent_savings_without_tariff_evidence() {
        let case = fixture("inputs")["cases"][27].clone();
        let actual = compute(&case).expect("unadmitted tariff case");
        assert_eq!(actual["gross_system_cost_zar"], 680.0);
        assert_eq!(actual["utility_bill_delta_zar"], Value::Null);
        assert_eq!(actual["incremental_household_cost_zar"], Value::Null);
        assert_eq!(actual["economic_preference_eligible"], false);
    }

    #[test]
    fn economics_zero_useful_volume_has_undefined_unit_cost() {
        let case = fixture("inputs")["cases"][28].clone();
        let actual = compute(&case).expect("zero delivered volume");
        assert_eq!(
            actual["gross_system_cost_per_verified_useful_m3_zar"],
            Value::Null
        );
        assert_eq!(
            actual["incremental_household_cost_per_verified_useful_m3_zar"],
            Value::Null
        );
    }

    #[test]
    fn economics_does_not_turn_missing_income_into_zero_or_a_pass() {
        let case = fixture("inputs")["cases"][30].clone();
        let actual = compute(&case).expect("missing income case");
        assert_eq!(actual["monthly_water_spend_zar"], 1080.0);
        assert_eq!(actual["monthly_water_spend_burden"], Value::Null);
        assert_eq!(actual["affordability_disposition"], "INDETERMINATE");
        assert_eq!(actual["economic_comparison_disposition"], "ADMITTED");
        assert_eq!(actual["economic_preference_eligible"], true);
    }

    #[test]
    fn economics_billing_period_mismatch_abstains_from_bill_comparison() {
        let case = fixture("inputs")["cases"][33].clone();
        let actual = compute(&case).expect("tariff billing-period mismatch");
        assert_eq!(actual["gross_system_cost_zar"], 680.0);
        assert_eq!(actual["tariff_disposition"], "INDETERMINATE");
        assert_eq!(actual["tariff_reason"], "TARIFF_BILLING_PERIOD_MISMATCH");
        assert_eq!(actual["utility_bill_delta_zar"], Value::Null);
        assert_eq!(actual["economic_comparison_disposition"], "INDETERMINATE");
        assert_eq!(actual["affordability_disposition"], "INDETERMINATE");
        assert_eq!(actual["economic_preference_eligible"], false);
    }

    #[test]
    fn economics_rejects_use_profile_mismatch_for_volume_credit() {
        let case = fixture("inputs")["cases"][31].clone();
        let actual = compute(&case).expect("use mismatch case");
        assert_eq!(actual["service_credit_disposition"], "NOT_CREDITABLE");
        assert_eq!(actual["verified_useful_delivered_m3"], Value::Null);
        assert_eq!(
            actual["gross_system_cost_per_verified_useful_m3_zar"],
            Value::Null
        );
        assert_eq!(actual["economic_comparison_disposition"], "NOT_COMPARABLE");
    }

    #[test]
    fn economics_low_cost_does_not_override_failed_service_floor() {
        let case = fixture("inputs")["cases"][32].clone();
        let actual = compute(&case).expect("service-floor failure");
        assert_eq!(actual["cost_ledger_disposition"], "ADMITTED");
        assert_eq!(actual["service_equivalence_disposition"], "FAIL");
        assert_eq!(
            actual["economic_comparison_disposition"],
            "SERVICE_FLOORS_NOT_MET"
        );
        assert_eq!(actual["economic_preference_eligible"], false);
    }

    #[test]
    fn non_synthetic_claim_ceiling_is_rejected() {
        let mut inputs = fixture("inputs");
        inputs["claim_ceiling"] = json!("physical_water_quality_qualified");
        assert!(evaluate(&inputs, &fixture("manifest"), &fixture("oracle")).is_err());
    }
}
