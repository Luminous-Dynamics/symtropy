// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Export the two matched synthetic cohort-inheritance policies.

use civdyn_generational_accounting::{SyntheticPolicy, run_synthetic_cohort_policy};

const COLUMNS: usize = 13;

fn row(policy: SyntheticPolicy) -> Vec<String> {
    let outcome = run_synthetic_cohort_policy(policy).expect("cohort policy must validate");
    vec![
        outcome.policy.name().to_string(),
        outcome.region_ledgers_conserved.to_string(),
        format!("{:.6}", outcome.current_household_water_served_m3),
        format!("{:.6}", outcome.current_household_water_unmet_m3),
        format!("{:.6}", outcome.current_maintenance_energy_served_kwh),
        format!("{:.6}", outcome.future_inherited_water_m3),
        format!("{:.6}", outcome.future_inherited_energy_kwh),
        format!("{:.6}", outcome.future_maintenance_energy_liability_kwh),
        format!("{:.6}", outcome.future_financial_liability_zar),
        format!("{:.6}", outcome.future_pump_condition_index),
        outcome.assigned_liability_records.to_string(),
        outcome.unassigned_liability_records.to_string(),
        outcome.cohort_service_reconciled.to_string(),
    ]
}

fn main() {
    println!(
        "policy,all_region_ledgers_conserved,current_household_water_served_m3,current_household_water_unmet_m3,current_maintenance_energy_served_kwh,future_inherited_water_m3,future_inherited_energy_kwh,future_maintenance_energy_liability_kwh,future_financial_liability_zar,future_pump_condition_index,assigned_liability_records,unassigned_liability_records,cohort_service_reconciled"
    );
    for policy in SyntheticPolicy::ALL {
        let fields = row(policy);
        assert_eq!(fields.len(), COLUMNS, "cohort CSV contract changed");
        println!("{}", fields.join(","));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_policy_emits_the_fixed_eleven_column_contract() {
        for policy in SyntheticPolicy::ALL {
            assert_eq!(row(policy).len(), COLUMNS);
        }
    }
}
