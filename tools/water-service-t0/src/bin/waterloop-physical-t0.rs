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

const CASE_COUNT: usize = 42;
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

fn indeterminate_sizing(reason: &str, profile: Option<String>) -> Value {
    json!({
        "design_disposition": "INDETERMINATE",
        "reason": reason,
        "source_or_use_profile_id": profile,
        "qualification_status": "NotRun",
        "required_treatment_rate_l_per_h": null,
        "maximum_treatment_rate_l_per_h": null,
        "treatment_capacity_margin_l_per_h": null,
        "idealized_equalization_working_volume_l": null,
        "idealized_starting_inventory_l": null,
        "period_mass_balance_residual_l": null,
        "essential_daily_demand_l": null,
        "autonomy_hours": null,
        "reserve_fraction": null,
        "usable_storage_volume_l": null,
        "usable_fraction": null,
        "gross_storage_volume_l": null,
        "flow_l_per_min": null,
        "total_dynamic_head_m": null,
        "wire_to_water_efficiency": null,
        "hydraulic_power_w": null,
        "hydraulic_specific_energy_kwh_m3": null
    })
}

fn compute_design_sizing(case_id: &str, input: &Value) -> Result<Value, String> {
    if case_id == "WL-T0-042-sizing-synthetic-claim-promotion-reject" {
        let evidence = text(input, "evidence_class")?;
        let requested = text(input, "requested_claim_class")?;
        if evidence == "synthetic_only"
            && (requested == "process_qualification" || requested == "field_design_release")
        {
            return Ok(json!({
                "design_disposition": "REJECT_CLAIM_PROMOTION",
                "reason": "SYNTHETIC_SIZING_CANNOT_SUPPORT_PROCESS_OR_FIELD_QUALIFICATION",
                "evidence_class": evidence,
                "requested_claim_class": requested,
                "qualification_status": "NotRun"
            }));
        }
        return Ok(json!({
            "design_disposition": "WITHIN_EVIDENCE_CEILING",
            "reason": "CLAIM_WITHIN_DECLARED_SYNTHETIC_EVIDENCE_CLASS",
            "evidence_class": evidence,
            "requested_claim_class": requested,
            "qualification_status": "NotRun"
        }));
    }

    let profile = text(input, "profile_id")?;
    if !boolean(input, "design_inputs_admitted")? {
        return Ok(indeterminate_sizing(
            "DESIGN_INPUTS_NOT_ADMITTED",
            Some(profile),
        ));
    }

    match case_id {
        "WL-T0-035-sizing-raw-equalization-periodic"
        | "WL-T0-036-sizing-treatment-capacity-insufficient" => {
            let period_hours = positive(number(input, "period_hours")?, "period_hours")?;
            let maximum_rate = positive(
                number(input, "maximum_treatment_rate_l_per_h")?,
                "maximum_treatment_rate_l_per_h",
            )?;
            let steps = field(input, "steps")?
                .as_array()
                .ok_or_else(|| "steps: expected array".to_owned())?;
            if steps.is_empty() {
                return Err("steps: must not be empty".to_owned());
            }
            let mut total_duration = 0.0;
            let mut total_inflow = 0.0;
            let mut available_hours = 0.0;
            for (index, step) in steps.iter().enumerate() {
                let duration = positive(number(step, "duration_hours")?, "duration_hours")?;
                let inflow = nonnegative(number(step, "source_inflow_l")?, "source_inflow_l")?;
                let available = boolean(step, "treatment_available")?;
                total_duration += duration;
                total_inflow += inflow;
                if available {
                    available_hours += duration;
                }
                if !total_duration.is_finite() || !total_inflow.is_finite() {
                    return Err(format!("steps[{index}]: cumulative design input overflow"));
                }
            }
            if !nearly_equal(total_duration, period_hours) {
                return Ok(json!({
                    "design_disposition": "REJECT",
                    "reason": "STEP_DURATIONS_DO_NOT_MATCH_DECLARED_PERIOD",
                    "profile_id": profile,
                    "declared_period_hours": period_hours,
                    "sum_step_duration_hours": total_duration,
                    "required_treatment_rate_l_per_h": null,
                    "idealized_equalization_working_volume_l": null,
                    "qualification_status": "NotRun"
                }));
            }
            if available_hours <= 0.0 {
                return Ok(json!({
                    "design_disposition": "REJECT",
                    "reason": "ZERO_TREATMENT_AVAILABILITY",
                    "profile_id": profile,
                    "period_hours": period_hours,
                    "total_daily_inflow_l": total_inflow,
                    "available_process_hours": available_hours,
                    "required_treatment_rate_l_per_h": null,
                    "idealized_equalization_working_volume_l": null,
                    "qualification_status": "NotRun"
                }));
            }
            if total_inflow <= 0.0 {
                return Ok(json!({
                    "design_disposition": "REJECT",
                    "reason": "NO_POSITIVE_SOURCE_VOLUME_IN_DESIGN_PERIOD",
                    "profile_id": profile,
                    "period_hours": period_hours,
                    "total_daily_inflow_l": total_inflow,
                    "available_process_hours": available_hours,
                    "required_treatment_rate_l_per_h": null,
                    "idealized_equalization_working_volume_l": null,
                    "qualification_status": "NotRun"
                }));
            }
            let required_rate = total_inflow / available_hours;
            let capacity_margin = maximum_rate - required_rate;
            if required_rate > maximum_rate + 1.0e-9 {
                return Ok(json!({
                    "design_disposition": "INFEASIBLE",
                    "reason": "MAX_TREATMENT_RATE_BELOW_REQUIRED_PERIODIC_THROUGHPUT",
                    "profile_id": profile,
                    "period_hours": period_hours,
                    "total_daily_inflow_l": total_inflow,
                    "available_process_hours": available_hours,
                    "required_treatment_rate_l_per_h": required_rate,
                    "maximum_treatment_rate_l_per_h": maximum_rate,
                    "treatment_capacity_margin_l_per_h": capacity_margin,
                    "idealized_equalization_working_volume_l": null,
                    "idealized_starting_inventory_l": null,
                    "period_mass_balance_residual_l": null,
                    "qualification_status": "NotRun"
                }));
            }
            let mut cumulative = 0.0;
            let mut minimum_cumulative = 0.0_f64;
            let mut maximum_cumulative = 0.0_f64;
            let mut total_process_outflow = 0.0;
            for step in steps {
                let duration = number(step, "duration_hours")?;
                let inflow = number(step, "source_inflow_l")?;
                let available = boolean(step, "treatment_available")?;
                let process_outflow = if available {
                    required_rate * duration
                } else {
                    0.0
                };
                cumulative += inflow - process_outflow;
                total_process_outflow += process_outflow;
                minimum_cumulative = minimum_cumulative.min(cumulative);
                maximum_cumulative = maximum_cumulative.max(cumulative);
            }
            let residual = total_inflow - total_process_outflow;
            if !nearly_equal(residual, 0.0) {
                return Err("periodic sizing schedule failed internal mass balance".to_owned());
            }
            let working_volume = maximum_cumulative - minimum_cumulative;
            let starting_inventory = (-minimum_cumulative).max(0.0);
            Ok(json!({
                "design_disposition": "ADMITTED_PRELIMINARY_SIZING",
                "reason": "PERIODIC_FEED_AND_PROCESS_SCHEDULE_BALANCED",
                "profile_id": profile,
                "period_hours": period_hours,
                "total_daily_inflow_l": total_inflow,
                "available_process_hours": available_hours,
                "required_treatment_rate_l_per_h": required_rate,
                "maximum_treatment_rate_l_per_h": maximum_rate,
                "treatment_capacity_margin_l_per_h": capacity_margin,
                "idealized_equalization_working_volume_l": working_volume,
                "idealized_starting_inventory_l": starting_inventory,
                "period_process_outflow_l": total_process_outflow,
                "period_mass_balance_residual_l": residual,
                "sizing_assumptions": [
                    "Discrete source inflow is treated as arriving within each declared interval before that interval's process draw.",
                    "Process operates at the required average rate during declared available intervals.",
                    "The schedule repeats periodically and total process outflow equals total source inflow.",
                    "No freeboard, dead volume, fouling margin, safety factor, treatment loss, sludge, reject, purge or backwash allowance is included.",
                    "The result is an idealized minimum working inventory, not a tank selection or fabrication volume."
                ],
                "qualification_status": "NotRun"
            }))
        }
        "WL-T0-037-sizing-treated-service-autonomy" => {
            let demand = positive(
                number(input, "essential_daily_demand_l")?,
                "essential_daily_demand_l",
            )?;
            let autonomy = positive(number(input, "autonomy_hours")?, "autonomy_hours")?;
            let reserve = nonnegative(number(input, "reserve_fraction")?, "reserve_fraction")?;
            let usable_fraction = positive(number(input, "usable_fraction")?, "usable_fraction")?;
            if usable_fraction > 1.0 {
                return Ok(json!({
                    "design_disposition": "REJECT",
                    "reason": "USABLE_FRACTION_EXCEEDS_ONE",
                    "profile_id": profile,
                    "usable_fraction": usable_fraction,
                    "usable_storage_volume_l": null,
                    "gross_storage_volume_l": null,
                    "qualification_status": "NotRun"
                }));
            }
            let base_volume = demand * autonomy / 24.0;
            let usable_volume = base_volume * (1.0 + reserve);
            let gross_volume = usable_volume / usable_fraction;
            Ok(json!({
                "design_disposition": "ADMITTED_PRELIMINARY_SIZING",
                "reason": "DECLARED_ESSENTIAL_DEMAND_AUTONOMY_AND_RESERVE",
                "profile_id": profile,
                "essential_daily_demand_l": demand,
                "autonomy_hours": autonomy,
                "reserve_fraction": reserve,
                "usable_fraction": usable_fraction,
                "base_autonomy_volume_l": base_volume,
                "usable_storage_volume_l": usable_volume,
                "gross_storage_volume_l": gross_volume,
                "sizing_assumptions": [
                    "Demand is constant at the declared daily average over the autonomy horizon.",
                    "Reserve fraction is applied to the base autonomy volume.",
                    "Usable fraction is an owner-supplied geometric/operational input.",
                    "No water-quality or cross-use credit is created by storage sizing.",
                    "Freeboard, dead zones, tank geometry, structural loading and code constraints are excluded."
                ],
                "qualification_status": "NotRun"
            }))
        }
        "WL-T0-038-sizing-pump-hydraulic-energy" | "WL-T0-039-sizing-pump-efficiency-invalid" => {
            let flow_l_per_min = positive(number(input, "flow_l_per_min")?, "flow_l_per_min")?;
            let head = positive(
                number(input, "total_dynamic_head_m")?,
                "total_dynamic_head_m",
            )?;
            let efficiency = positive(
                number(input, "wire_to_water_efficiency")?,
                "wire_to_water_efficiency",
            )?;
            let density = positive(number(input, "water_density_kg_m3")?, "water_density_kg_m3")?;
            let gravity = positive(number(input, "gravity_m_s2")?, "gravity_m_s2")?;
            if efficiency > 1.0 {
                return Ok(json!({
                    "design_disposition": "REJECT",
                    "reason": "WIRE_TO_WATER_EFFICIENCY_EXCEEDS_ONE",
                    "profile_id": profile,
                    "flow_l_per_min": flow_l_per_min,
                    "total_dynamic_head_m": head,
                    "wire_to_water_efficiency": efficiency,
                    "hydraulic_power_w": null,
                    "hydraulic_specific_energy_kwh_m3": null,
                    "qualification_status": "NotRun"
                }));
            }
            let flow_m3_s = flow_l_per_min / 60_000.0;
            let power = density * gravity * flow_m3_s * head / efficiency;
            let specific_energy = density * gravity * head / (efficiency * 3_600_000.0);
            Ok(json!({
                "design_disposition": "ADMITTED_PRELIMINARY_SIZING",
                "reason": "DECLARED_FLOW_HEAD_AND_WIRE_TO_WATER_EFFICIENCY",
                "profile_id": profile,
                "flow_l_per_min": flow_l_per_min,
                "flow_m3_s": flow_m3_s,
                "total_dynamic_head_m": head,
                "wire_to_water_efficiency": efficiency,
                "water_density_kg_m3": density,
                "gravity_m_s2": gravity,
                "hydraulic_power_w": power,
                "hydraulic_specific_energy_kwh_m3": specific_energy,
                "sizing_assumptions": [
                    "Steady incompressible water flow at the declared density.",
                    "Total dynamic head is supplied by the design owner and must include relevant static and dynamic losses.",
                    "Wire-to-water efficiency is an explicitly supplied assumption, not a pump curve guarantee.",
                    "No electrical auxiliary/standby loads or control losses are included.",
                    "This calculation does not select or qualify a pump."
                ],
                "qualification_status": "NotRun"
            }))
        }
        "WL-T0-040-sizing-zero-process-availability" => {
            let steps = field(input, "steps")?
                .as_array()
                .ok_or_else(|| "steps: expected array".to_owned())?;
            let available = steps.iter().try_fold(0.0, |total, step| {
                let duration = positive(number(step, "duration_hours")?, "duration_hours")?;
                if boolean(step, "treatment_available")? {
                    Ok::<f64, String>(total + duration)
                } else {
                    Ok(total)
                }
            })?;
            if available == 0.0 {
                Ok(json!({
                    "design_disposition": "REJECT",
                    "reason": "ZERO_TREATMENT_AVAILABILITY",
                    "available_process_hours": available,
                    "required_treatment_rate_l_per_h": null,
                    "qualification_status": "NotRun"
                }))
            } else {
                Ok(json!({
                    "design_disposition": "ADMITTED_PRELIMINARY_SIZING",
                    "reason": "NONZERO_TREATMENT_AVAILABILITY",
                    "available_process_hours": available,
                    "qualification_status": "NotRun"
                }))
            }
        }
        "WL-T0-041-sizing-unadmitted-design-inputs" => Ok(indeterminate_sizing(
            "DESIGN_INPUTS_NOT_ADMITTED",
            Some(profile),
        )),
        _ => Err(format!(
            "unsupported WaterLoop design sizing case: {case_id}"
        )),
    }
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

        "WL-T0-035-sizing-raw-equalization-periodic"
        | "WL-T0-036-sizing-treatment-capacity-insufficient"
        | "WL-T0-037-sizing-treated-service-autonomy"
        | "WL-T0-038-sizing-pump-hydraulic-energy"
        | "WL-T0-039-sizing-pump-efficiency-invalid"
        | "WL-T0-040-sizing-zero-process-availability"
        | "WL-T0-041-sizing-unadmitted-design-inputs" => compute_design_sizing(&id, input),
        "WL-T0-042-sizing-synthetic-claim-promotion-reject" => compute_design_sizing(&id, input),
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

    const DESIGN_ARTIFACTS: [(&str, &str); 11] = [
        (
            "module_interface",
            "research/water-service-engineering/WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json",
        ),
        (
            "pid_tag_schedule",
            "research/water-service-engineering/WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.json",
        ),
        (
            "preliminary_bom",
            "research/water-service-engineering/WATERLOOP_PILOT_SKID_PRELIMINARY_BOM_V0_1.json",
        ),
        (
            "fmea_starter",
            "research/water-service-engineering/WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json",
        ),
        (
            "challenge_profiles",
            "research/water-service-engineering/WATERLOOP_GREYWATER_CHALLENGE_PROFILES_V0_1.json",
        ),
        (
            "architecture_document",
            "research/water-service-engineering/WATERLOOP_MODULAR_TREATMENT_ARCHITECTURE_V0_1.md",
        ),
        (
            "module_interface_schema",
            "research/water-service-engineering/WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.schema.json",
        ),
        (
            "pid_schema",
            "research/water-service-engineering/WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.schema.json",
        ),
        (
            "bom_schema",
            "research/water-service-engineering/WATERLOOP_PILOT_SKID_PRELIMINARY_BOM_V0_1.schema.json",
        ),
        (
            "fmea_schema",
            "research/water-service-engineering/WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.schema.json",
        ),
        (
            "challenge_profile_schema",
            "research/water-service-engineering/WATERLOOP_GREYWATER_CHALLENGE_PROFILES_V0_1.schema.json",
        ),
    ];
    let mut design_artifact_digests = serde_json::Map::new();
    for (name, relative_path) in DESIGN_ARTIFACTS {
        let bytes = fs::read(relative_path)
            .map_err(|error| format!("read design artifact {relative_path}: {error}"))?;
        design_artifact_digests.insert(name.to_owned(), json!(sha256(&bytes)));
    }

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
        "design_artifact_sha256": design_artifact_digests,
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

    fn modular_fixture(name: &str) -> Value {
        let raw = match name {
            "modules" => include_str!(
                "../../../../research/water-service-engineering/WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json"
            ),
            "pid" => include_str!(
                "../../../../research/water-service-engineering/WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.json"
            ),
            "bom" => include_str!(
                "../../../../research/water-service-engineering/WATERLOOP_PILOT_SKID_PRELIMINARY_BOM_V0_1.json"
            ),
            "fmea" => include_str!(
                "../../../../research/water-service-engineering/WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json"
            ),
            "profiles" => include_str!(
                "../../../../research/water-service-engineering/WATERLOOP_GREYWATER_CHALLENGE_PROFILES_V0_1.json"
            ),
            _ => panic!("unknown modular design fixture"),
        };
        serde_json::from_str(raw).expect("modular design fixture JSON parses")
    }

    fn design_string_array(value: &Value, path: &str, issues: &mut Vec<String>) -> Vec<String> {
        let Some(rows) = value.as_array() else {
            issues.push(format!("{path}: expected array"));
            return Vec::new();
        };
        rows.iter()
            .enumerate()
            .filter_map(|(index, row)| {
                row.as_str().map(str::to_owned).or_else(|| {
                    issues.push(format!("{path}[{index}]: expected string"));
                    None
                })
            })
            .collect()
    }

    fn audit_modular_design(
        modules_doc: &Value,
        pid_doc: &Value,
        bom_doc: &Value,
        fmea_doc: &Value,
        profiles_doc: &Value,
    ) -> Vec<String> {
        let mut issues = Vec::new();
        let mut module_ids = BTreeSet::new();
        let mut bom_tags = BTreeSet::new();
        let mut fmea_module_coverage = BTreeSet::new();
        let mut bom_module_coverage = BTreeSet::new();

        let Some(modules) = modules_doc.get("modules").and_then(Value::as_array) else {
            issues.push(
                "WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json:modules: expected array".to_owned(),
            );
            return issues;
        };
        if modules.is_empty() {
            issues.push(
                "WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json:modules: must not be empty"
                    .to_owned(),
            );
        }
        for (index, module) in modules.iter().enumerate() {
            let prefix = format!("WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json:modules[{index}]");
            let id = module
                .get("module_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            if id.is_empty() {
                issues.push(format!("{prefix}.module_id: missing"));
            } else if !module_ids.insert(id.to_owned()) {
                issues.push(format!("{prefix}.module_id: duplicate {id}"));
            }
            let failure = module
                .get("failure_state")
                .and_then(Value::as_str)
                .unwrap_or("");
            if failure.trim().is_empty() {
                issues.push(format!("{prefix}.failure_state: missing"));
            }
            let required_ports = design_string_array(
                &module["required_ports"],
                &format!("{prefix}.required_ports"),
                &mut issues,
            );
            if required_ports.is_empty() {
                issues.push(format!("{prefix}.required_ports: must not be empty"));
            }
            if required_ports.iter().any(|port| !port.starts_with("P-")) {
                issues.push(format!(
                    "{prefix}.required_ports: every port must use the P- namespace"
                ));
            }
            let requirements = design_string_array(
                &module["requirements"],
                &format!("{prefix}.requirements"),
                &mut issues,
            );
            if requirements.is_empty() {
                issues.push(format!("{prefix}.requirements: must not be empty"));
            }
            if module["module_class"] == "treatment_candidate"
                && !["HOLD", "ISOLATE", "BYPASS"]
                    .iter()
                    .any(|word| failure.contains(word))
            {
                issues.push(format!(
                    "{prefix}.failure_state: treatment candidate must fail to hold/isolate/bypass"
                ));
            }
        }
        let universal_safe_state = modules_doc
            .get("universal_safe_state")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !universal_safe_state.contains("NO_AUTOMATIC_RELEASE")
            && !universal_safe_state.contains("NO AUTOMATIC RELEASE")
        {
            issues.push("WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json:universal_safe_state: missing no-automatic-release invariant".to_owned());
        }
        let prohibited = design_string_array(
            &modules_doc["prohibited_connections"],
            "WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json:prohibited_connections",
            &mut issues,
        );
        if !prohibited
            .iter()
            .any(|value| value.contains("potable-to-reclaimed"))
        {
            issues.push("WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json:prohibited_connections: potable/reclaimed cross-tie prohibition missing".to_owned());
        }

        let Some(pid_tags) = pid_doc.get("tags").and_then(Value::as_array) else {
            issues.push(
                "WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.json:tags: expected array"
                    .to_owned(),
            );
            return issues;
        };
        let mut pid_tag_ids = BTreeSet::new();
        for (index, tag) in pid_tags.iter().enumerate() {
            let prefix =
                format!("WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.json:tags[{index}]");
            let id = tag.get("tag").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() {
                issues.push(format!("{prefix}.tag: missing"));
            } else if !pid_tag_ids.insert(id.to_owned()) {
                issues.push(format!("{prefix}.tag: duplicate {id}"));
            }
            for required in ["function", "boundary"] {
                if tag
                    .get(required)
                    .and_then(Value::as_str)
                    .is_none_or(str::is_empty)
                {
                    issues.push(format!("{prefix}.{required}: missing"));
                }
            }
            if design_string_array(
                &tag["interlocks"],
                &format!("{prefix}.interlocks"),
                &mut issues,
            )
            .is_empty()
            {
                issues.push(format!("{prefix}.interlocks: must not be empty"));
            }
            let refs = design_string_array(
                &tag["module_ids"],
                &format!("{prefix}.module_ids"),
                &mut issues,
            );
            if refs.is_empty() {
                issues.push(format!(
                    "{prefix}.module_ids: must reference at least one module"
                ));
            }
            for module_id in refs {
                if !module_ids.contains(&module_id) {
                    issues.push(format!(
                        "{prefix}.module_ids: orphan module reference {module_id}"
                    ));
                }
            }
        }
        let pid_invariants = design_string_array(
            &pid_doc["safety_invariants"],
            "WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.json:safety_invariants",
            &mut issues,
        );
        if pid_invariants.len() < 3
            || !pid_invariants
                .iter()
                .any(|item| item.contains("pressurised cross-tie"))
            || !pid_invariants
                .iter()
                .any(|item| item.contains("No release"))
        {
            issues.push("WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.json:safety_invariants: cross-connection and no-release invariants required".to_owned());
        }

        let Some(items) = bom_doc.get("items").and_then(Value::as_array) else {
            issues.push(
                "WATERLOOP_PILOT_SKID_PRELIMINARY_BOM_V0_1.json:items: expected array".to_owned(),
            );
            return issues;
        };
        for (index, item) in items.iter().enumerate() {
            let prefix = format!("WATERLOOP_PILOT_SKID_PRELIMINARY_BOM_V0_1.json:items[{index}]");
            let tag = item.get("tag").and_then(Value::as_str).unwrap_or("");
            if tag.is_empty() {
                issues.push(format!("{prefix}.tag: missing"));
            } else if !bom_tags.insert(tag.to_owned()) {
                issues.push(format!("{prefix}.tag: duplicate {tag}"));
            }
            if !item
                .get("description")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty())
            {
                issues.push(format!("{prefix}.description: missing"));
            }
            if !item.get("quantity").is_some_and(Value::is_null) {
                issues.push(format!(
                    "{prefix}.quantity: must remain unset before design freeze"
                ));
            }
            let procurement = item
                .get("procurement_status")
                .and_then(Value::as_str)
                .unwrap_or("");
            if procurement != "not_specified" && procurement != "candidate_only" {
                issues.push(format!(
                    "{prefix}.procurement_status: premature procurement state {procurement}"
                ));
            }
            let refs = design_string_array(
                &item["module_ids"],
                &format!("{prefix}.module_ids"),
                &mut issues,
            );
            if refs.is_empty() {
                issues.push(format!(
                    "{prefix}.module_ids: must reference at least one module"
                ));
            }
            for module_id in refs {
                if !module_ids.contains(&module_id) {
                    issues.push(format!(
                        "{prefix}.module_ids: orphan module reference {module_id}"
                    ));
                } else {
                    bom_module_coverage.insert(module_id);
                }
            }
            for forbidden in ["sku", "approved_material_grade", "final_pressure_rating"] {
                if item.get(forbidden).is_some() {
                    issues.push(format!(
                        "{prefix}.{forbidden}: must not be frozen in conceptual BOM"
                    ));
                }
            }
        }
        for tag in &pid_tag_ids {
            if !bom_tags.contains(tag) {
                issues.push(format!("WATERLOOP_PILOT_SKID_P_AND_ID_TAG_SCHEDULE_V0_1.json:tags: tag {tag} has no BOM category"));
            }
        }
        for module_id in &module_ids {
            if !bom_module_coverage.contains(module_id) {
                issues.push(format!("WATERLOOP_PILOT_SKID_PRELIMINARY_BOM_V0_1.json:items: module {module_id} has no BOM category"));
            }
        }

        let Some(hazards) = fmea_doc.get("hazards").and_then(Value::as_array) else {
            issues.push(
                "WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json:hazards: expected array"
                    .to_owned(),
            );
            return issues;
        };
        let mut hazard_ids = BTreeSet::new();
        for (index, hazard) in hazards.iter().enumerate() {
            let prefix =
                format!("WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json:hazards[{index}]");
            let id = hazard
                .get("hazard_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            if id.is_empty() {
                issues.push(format!("{prefix}.hazard_id: missing"));
            } else if !hazard_ids.insert(id.to_owned()) {
                issues.push(format!("{prefix}.hazard_id: duplicate {id}"));
            }
            for required in [
                "failure",
                "effect",
                "detection",
                "safe_state",
                "verification",
            ] {
                if !hazard
                    .get(required)
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    issues.push(format!("{prefix}.{required}: missing"));
                }
            }
            let safe_state = hazard
                .get("safe_state")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            if ![
                "hold",
                "held",
                "divert",
                "isolate",
                "stop",
                "lockout",
                "indeterminate",
                "disconnect",
                "do not commission",
            ]
            .iter()
            .any(|word| safe_state.contains(word))
            {
                issues.push(format!("{prefix}.safe_state: no fail-safe action detected"));
            }
            let refs = design_string_array(
                &hazard["module_ids"],
                &format!("{prefix}.module_ids"),
                &mut issues,
            );
            if refs.is_empty() {
                issues.push(format!(
                    "{prefix}.module_ids: must reference at least one module"
                ));
            }
            for module_id in refs {
                if !module_ids.contains(&module_id) {
                    issues.push(format!(
                        "{prefix}.module_ids: orphan module reference {module_id}"
                    ));
                } else {
                    fmea_module_coverage.insert(module_id);
                }
            }
        }
        for module_id in &module_ids {
            if !fmea_module_coverage.contains(module_id) {
                issues.push(format!("WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json:hazards: module {module_id} has no hazard coverage"));
            }
        }
        let invariants = design_string_array(
            &fmea_doc["invariants"],
            "WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json:invariants",
            &mut issues,
        );
        if !invariants
            .iter()
            .any(|value| value.contains("No automated release"))
        {
            issues.push("WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json:invariants: no-automated-release invariant missing".to_owned());
        }
        if fmea_doc
            .get("not_completed")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            issues.push("WATERLOOP_MODULAR_TREATMENT_FMEA_STARTER_V0_1.json:not_completed: unresolved formal safety work must remain visible".to_owned());
        }

        let Some(profiles) = profiles_doc.get("profiles").and_then(Value::as_array) else {
            issues.push(
                "WATERLOOP_GREYWATER_CHALLENGE_PROFILES_V0_1.json:profiles: expected array"
                    .to_owned(),
            );
            return issues;
        };
        let profile_ids = profiles
            .iter()
            .filter_map(|profile| {
                profile
                    .get("profile_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect::<BTreeSet<_>>();
        if let Some(scenarios) = profiles_doc.get("scenarios").and_then(Value::as_array) {
            for (index, scenario) in scenarios.iter().enumerate() {
                let prefix =
                    format!("WATERLOOP_GREYWATER_CHALLENGE_PROFILES_V0_1.json:scenarios[{index}]");
                let profile_id = scenario
                    .get("profile_id")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if !profile_ids.contains(profile_id) {
                    issues.push(format!("{prefix}.profile_id: unknown profile {profile_id}"));
                }
                let disposition = scenario
                    .get("expected_disposition")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if profile_id == "GW-SYN-04-kitchen-source-excluded"
                    && disposition != "REJECT_SOURCE"
                {
                    issues.push(format!(
                        "{prefix}.expected_disposition: excluded kitchen source cannot be accepted"
                    ));
                }
                if profile_id == "GW-SYN-05-source-unknown"
                    && disposition != "INDETERMINATE_HOLD_OR_DIVERT"
                {
                    issues.push(format!(
                        "{prefix}.expected_disposition: unknown source must remain indeterminate"
                    ));
                }
                if profile_id == "GW-SYN-06-storage-age-violation"
                    && disposition != "HOLD_NO_REUSE_RELEASE"
                {
                    issues.push(format!("{prefix}.expected_disposition: over-age untreated source must not be released"));
                }
                if disposition.contains("QUALITY_QUALIFIED")
                    || disposition == "APPROVED_FOR_HUMAN_USE"
                {
                    issues.push(format!("{prefix}.expected_disposition: synthetic challenge cannot qualify water quality"));
                }
            }
        } else {
            issues.push(
                "WATERLOOP_GREYWATER_CHALLENGE_PROFILES_V0_1.json:scenarios: expected array"
                    .to_owned(),
            );
        }
        issues
    }

    #[test]
    fn all_forty_two_cases_match_the_separate_oracle() {
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
        assert_eq!(result["matched_case_count"], 41);
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
    fn modular_skid_design_artifacts_pass_cross_file_audit() {
        let issues = audit_modular_design(
            &modular_fixture("modules"),
            &modular_fixture("pid"),
            &modular_fixture("bom"),
            &modular_fixture("fmea"),
            &modular_fixture("profiles"),
        );
        assert!(
            issues.is_empty(),
            "design artefact violations:\\n{}",
            issues.join("\\n")
        );
    }

    #[test]
    fn modular_design_audit_rejects_orphan_module_references() {
        let mut pid = modular_fixture("pid");
        pid["tags"][0]["module_ids"][0] = json!("MISSING-MODULE");
        let issues = audit_modular_design(
            &modular_fixture("modules"),
            &pid,
            &modular_fixture("bom"),
            &modular_fixture("fmea"),
            &modular_fixture("profiles"),
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("orphan module reference MISSING-MODULE"))
        );
    }

    #[test]
    fn modular_design_audit_rejects_missing_p_and_id_interlocks() {
        let mut pid = modular_fixture("pid");
        pid["tags"][0]["interlocks"] = json!([]);
        let issues = audit_modular_design(
            &modular_fixture("modules"),
            &pid,
            &modular_fixture("bom"),
            &modular_fixture("fmea"),
            &modular_fixture("profiles"),
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("tags[0].interlocks"))
        );
    }

    #[test]
    fn modular_design_audit_rejects_bom_quantities_before_design_freeze() {
        let mut bom = modular_fixture("bom");
        bom["items"][0]["quantity"] = json!(1);
        let issues = audit_modular_design(
            &modular_fixture("modules"),
            &modular_fixture("pid"),
            &bom,
            &modular_fixture("fmea"),
            &modular_fixture("profiles"),
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("items[0].quantity"))
        );
    }

    #[test]
    fn modular_design_audit_rejects_fmea_without_verification() {
        let mut fmea = modular_fixture("fmea");
        fmea["hazards"][0]["verification"] = json!("");
        let issues = audit_modular_design(
            &modular_fixture("modules"),
            &modular_fixture("pid"),
            &modular_fixture("bom"),
            &fmea,
            &modular_fixture("profiles"),
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("hazards[0].verification"))
        );
    }

    #[test]
    fn modular_design_audit_rejects_excluded_source_as_bench_eligible() {
        let mut profiles = modular_fixture("profiles");
        profiles["scenarios"][3]["expected_disposition"] =
            json!("ELIGIBLE_FOR_BENCH_SCREENING_ONLY");
        let issues = audit_modular_design(
            &modular_fixture("modules"),
            &modular_fixture("pid"),
            &modular_fixture("bom"),
            &modular_fixture("fmea"),
            &profiles,
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("excluded kitchen source cannot be accepted"))
        );
    }

    #[test]
    fn modular_design_audit_rejects_missing_potable_cross_connection_prohibition() {
        let mut modules = modular_fixture("modules");
        modules["prohibited_connections"] = json!(["no overflow to stormwater"]);
        let issues = audit_modular_design(
            &modules,
            &modular_fixture("pid"),
            &modular_fixture("bom"),
            &modular_fixture("fmea"),
            &modular_fixture("profiles"),
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("potable/reclaimed cross-tie prohibition missing"))
        );
    }

    #[test]
    fn sizing_equalisation_reports_inventory_and_working_volume_separately() {
        let out = compute(&fixture("inputs")["cases"][34]).expect("equalisation sizing");
        assert_eq!(out["required_treatment_rate_l_per_h"], 15.0);
        assert_eq!(out["idealized_equalization_working_volume_l"], 30.0);
        assert_eq!(out["idealized_starting_inventory_l"], 30.0);
        assert_eq!(out["period_mass_balance_residual_l"], 0.0);
    }

    #[test]
    fn sizing_rejects_process_rate_below_required_periodic_throughput() {
        let out = compute(&fixture("inputs")["cases"][35]).expect("capacity shortfall");
        assert_eq!(out["design_disposition"], "INFEASIBLE");
        assert_eq!(out["treatment_capacity_margin_l_per_h"], -1.0);
        assert_eq!(out["idealized_equalization_working_volume_l"], Value::Null);
    }

    #[test]
    fn sizing_treated_storage_uses_autonomy_reserve_and_usable_fraction() {
        let out = compute(&fixture("inputs")["cases"][36]).expect("storage sizing");
        assert_eq!(out["base_autonomy_volume_l"], 240.0);
        assert_eq!(out["usable_storage_volume_l"], 288.0);
        assert_eq!(out["gross_storage_volume_l"], 320.0);
    }

    #[test]
    fn sizing_pump_energy_uses_declared_flow_head_and_efficiency() {
        let out = compute(&fixture("inputs")["cases"][37]).expect("pump sizing");
        assert!((out["hydraulic_power_w"].as_f64().unwrap() - 35.66054545454545).abs() < 1.0e-10);
        assert!(
            (out["hydraulic_specific_energy_kwh_m3"].as_f64().unwrap() - 0.05943424242424241).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn sizing_rejects_efficiency_above_unity_and_zero_process_hours() {
        let efficiency = compute(&fixture("inputs")["cases"][38]).expect("invalid efficiency");
        let availability = compute(&fixture("inputs")["cases"][39]).expect("zero availability");
        assert_eq!(efficiency["design_disposition"], "REJECT");
        assert_eq!(availability["reason"], "ZERO_TREATMENT_AVAILABILITY");
    }

    #[test]
    fn sizing_unadmitted_inputs_and_synthetic_claim_promotion_abstain() {
        let unadmitted = compute(&fixture("inputs")["cases"][40]).expect("unadmitted inputs");
        let promotion = compute(&fixture("inputs")["cases"][41]).expect("claim promotion control");
        assert_eq!(unadmitted["design_disposition"], "INDETERMINATE");
        assert_eq!(promotion["design_disposition"], "REJECT_CLAIM_PROMOTION");
        assert_eq!(promotion["qualification_status"], "NotRun");
    }

    #[test]
    fn non_synthetic_claim_ceiling_is_rejected() {
        let mut inputs = fixture("inputs");
        inputs["claim_ceiling"] = json!("physical_water_quality_qualified");
        assert!(evaluate(&inputs, &fixture("manifest"), &fixture("oracle")).is_err());
    }
}
