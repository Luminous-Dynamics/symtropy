// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! One-tick matched intervention experiment backed by the shared resource ledger.

use civdyn_causal_path::{
    CausalEdge, CausalExperiment, CausalStage, CausalVariable, IndependentOutcomeReceipt,
    MechanismAblation, RunArm, TransitionReceipt,
};
use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, TransferPolicy, TransferRequest, World,
};
use std::collections::BTreeMap;

const CSV_COLUMNS: usize = 29;
const EXOGENOUS_SNAPSHOT: &str = "fixture:reservoir20m3_household_demand10m3_production0";

#[derive(Debug)]
struct ArmDiagnostics {
    arm: RunArm,
    reservoir_closing_water_m3: f64,
    household_served_water_m3: f64,
    max_water_conservation_residual_m3: f64,
}

fn variables() -> Vec<CausalVariable> {
    vec![
        CausalVariable {
            id: "safeguard_enabled".into(),
            stage: CausalStage::Intervention,
            unit: "boolean".into(),
        },
        CausalVariable {
            id: "mechanism_blocked".into(),
            stage: CausalStage::Intervention,
            unit: "boolean".into(),
        },
        CausalVariable {
            id: "transfer_eligible".into(),
            stage: CausalStage::Mechanism,
            unit: "boolean".into(),
        },
        CausalVariable {
            id: "authorized_transfer_m3".into(),
            stage: CausalStage::Intermediate,
            unit: "m3".into(),
        },
        CausalVariable {
            id: "delivered_water_m3".into(),
            stage: CausalStage::Intermediate,
            unit: "m3".into(),
        },
        CausalVariable {
            id: "household_water_coverage".into(),
            stage: CausalStage::Outcome,
            unit: "fraction".into(),
        },
        CausalVariable {
            id: "unrelated_intermediate".into(),
            stage: CausalStage::Intermediate,
            unit: "m3".into(),
        },
        CausalVariable {
            id: "reputation_counter".into(),
            stage: CausalStage::NegativeControl,
            unit: "count".into(),
        },
    ]
}

fn edges() -> Vec<CausalEdge> {
    vec![
        CausalEdge {
            id: "safeguard_to_eligibility".into(),
            from_variable: "safeguard_enabled".into(),
            to_variable: "transfer_eligible".into(),
        },
        CausalEdge {
            id: "eligibility_to_authorization".into(),
            from_variable: "transfer_eligible".into(),
            to_variable: "authorized_transfer_m3".into(),
        },
        CausalEdge {
            id: "authorization_to_delivery".into(),
            from_variable: "authorized_transfer_m3".into(),
            to_variable: "delivered_water_m3".into(),
        },
        CausalEdge {
            id: "delivery_to_service".into(),
            from_variable: "delivered_water_m3".into(),
            to_variable: "household_water_coverage".into(),
        },
    ]
}

fn execute_arm(
    arm_id: &str,
    safeguard_enabled: bool,
    decoy_counter: bool,
    mechanism_blocked: bool,
) -> ArmDiagnostics {
    let regions = vec![
        Region {
            id: "regional_reservoir".into(),
            population: 0.0,
            opening_stock: ResourceVector::new(20.0, 0.0, 0.0),
            essential_demand_per_capita: ResourceVector::default(),
        },
        Region {
            id: "household_1".into(),
            population: 1.0,
            opening_stock: ResourceVector::default(),
            essential_demand_per_capita: ResourceVector::new(10.0, 0.0, 0.0),
        },
    ];
    let mut world = World::new(regions).expect("synthetic resource ledger world must validate");
    let eligible = safeguard_enabled && !mechanism_blocked;
    let authorized_water_m3 = if eligible { 10.0 } else { 0.0 };
    let transfers = if authorized_water_m3 > 0.0 {
        vec![TransferRequest {
            from_region: 0,
            to_region: 1,
            amount: ResourceVector::new(authorized_water_m3, 0.0, 0.0),
        }]
    } else {
        Vec::new()
    };

    let report = world
        .step(
            &StepInput {
                production: vec![Production::default(); 2],
                transfers,
            },
            TransferPolicy::Unrestricted,
        )
        .expect("declared one-tick ledger scenario must conserve resources");

    let delivered_water_m3 = report
        .transfers
        .iter()
        .map(|receipt| receipt.delivered.water_m3)
        .sum::<f64>();
    let household = &report.regions[1];
    let household_water_coverage = household.coverage.water.unwrap_or(0.0);
    let initial_values = BTreeMap::from([
        (
            "safeguard_enabled".into(),
            if safeguard_enabled { 1.0 } else { 0.0 },
        ),
        (
            "mechanism_blocked".into(),
            if mechanism_blocked { 1.0 } else { 0.0 },
        ),
        ("transfer_eligible".into(), 0.0),
        ("authorized_transfer_m3".into(), 0.0),
        ("delivered_water_m3".into(), 0.0),
        ("household_water_coverage".into(), 0.0),
        ("unrelated_intermediate".into(), 0.0),
        ("reputation_counter".into(), 0.0),
    ]);
    let final_values = BTreeMap::from([
        (
            "safeguard_enabled".into(),
            if safeguard_enabled { 1.0 } else { 0.0 },
        ),
        (
            "mechanism_blocked".into(),
            if mechanism_blocked { 1.0 } else { 0.0 },
        ),
        ("transfer_eligible".into(), if eligible { 1.0 } else { 0.0 }),
        ("authorized_transfer_m3".into(), authorized_water_m3),
        ("delivered_water_m3".into(), delivered_water_m3),
        ("household_water_coverage".into(), household_water_coverage),
        ("unrelated_intermediate".into(), 0.0),
        (
            "reputation_counter".into(),
            if decoy_counter { 1.0 } else { 0.0 },
        ),
    ]);
    let transitions = edges()
        .iter()
        .map(|edge| {
            let before = initial_values[&edge.to_variable];
            let after = final_values[&edge.to_variable];
            TransitionReceipt {
                edge_id: edge.id.clone(),
                source_value: final_values[&edge.from_variable],
                target_before: before,
                target_after: after,
                applied: (after - before).abs() > 1e-9,
            }
        })
        .collect();

    let arm = RunArm {
        arm_id: arm_id.into(),
        exogenous_snapshot_id: EXOGENOUS_SNAPSHOT.into(),
        initial_values,
        final_values,
        transitions,
        independent_outcome: IndependentOutcomeReceipt {
            variable_id: "household_water_coverage".into(),
            value: household_water_coverage,
            unit: "fraction".into(),
            resource: "water".into(),
            region_id: "household_1".into(),
            tick: report.tick,
            source_record_id: format!("resource-ledger:{arm_id}:tick-{}:household_1", report.tick),
        },
    };

    let max_water_conservation_residual_m3 = report
        .regions
        .iter()
        .map(|region| region.ledger.conservation_residual().water_m3.abs())
        .fold(0.0_f64, f64::max);

    ArmDiagnostics {
        arm,
        reservoir_closing_water_m3: report.regions[0].closing_stock.water_m3,
        household_served_water_m3: household.served.water_m3,
        max_water_conservation_residual_m3,
    }
}

fn reference_experiment() -> (CausalExperiment, [ArmDiagnostics; 4]) {
    let control = execute_arm("control", false, false, false);
    let treatment = execute_arm("treatment", true, false, false);
    let negative_control = execute_arm("negative_control", false, true, false);
    let mechanism_ablation = execute_arm("mechanism_ablation", true, false, true);
    let edge_list = edges();

    let experiment = CausalExperiment {
        experiment_id: "CIVDYN-CAUSAL-PATH-001-water-safeguard".into(),
        intervention_variable_id: "safeguard_enabled".into(),
        negative_control_variable_id: "reputation_counter".into(),
        outcome_variable_id: "household_water_coverage".into(),
        required_path_edge_ids: edge_list.iter().map(|edge| edge.id.clone()).collect(),
        variables: variables(),
        edges: edge_list,
        control: control.arm.clone(),
        treatment: treatment.arm.clone(),
        negative_control: negative_control.arm.clone(),
        mechanism_ablation: MechanismAblation {
            block_variable_id: "mechanism_blocked".into(),
            blocked_mechanism_variable_id: "transfer_eligible".into(),
            arm: mechanism_ablation.arm.clone(),
        },
    };
    (
        experiment,
        [control, treatment, negative_control, mechanism_ablation],
    )
}

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "causal-path CSV schema must remain stable"
    );
    println!("{}", fields.join(","));
}

fn empty_row(record_type: &str, arm_id: &str) -> Vec<String> {
    vec![
        record_type.into(),
        arm_id.into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn main() {
    println!(
        "record_type,arm_id,metric,value,unit,ledger_record_key,exogenous_snapshot_id,qualified,path_complete,first_broken_edge,transition_applied,control_outcome,treatment_outcome,reservoir_closing_water_m3,household_served_water_m3,max_water_conservation_residual_m3,treatment_source_value,control_source_value,source_changed,ablation_block_variable_id,ablation_block_switch_changed,ablation_primary_intervention_preserved,ablation_blocked_mediator_variable_id,ablation_mediator_suppressed,ablation_protocol_valid,mechanism_necessity_supported,ablation_outcome,ablation_effect,observed_treatment_effect"
    );

    let (experiment, arms) = reference_experiment();
    let result = experiment
        .evaluate()
        .expect("matched resource-ledger causal path must validate");

    for arm in &arms {
        let mut row = empty_row("arm_outcome", &arm.arm.arm_id);
        row[2] = "household_water_coverage".into();
        row[3] = fmt(arm.arm.independent_outcome.value);
        row[4] = arm.arm.independent_outcome.unit.clone();
        row[5] = arm.arm.independent_outcome.source_record_id.clone();
        row[6] = arm.arm.exogenous_snapshot_id.clone();
        row[13] = fmt(arm.reservoir_closing_water_m3);
        row[14] = fmt(arm.household_served_water_m3);
        row[15] = fmt(arm.max_water_conservation_residual_m3);
        emit(row);
    }

    for edge in &result.path_edges {
        let mut row = empty_row("path_edge", "treatment");
        row[2] = edge.edge_id.clone();
        row[3] = fmt(edge.treatment_target_delta);
        row[4] = "native_variable_unit".into();
        row[7] = result.qualified.to_string();
        row[8] = edge.complete.to_string();
        row[10] = edge.treatment_transition_applied.to_string();
        row[11] = fmt(result.control_outcome);
        row[12] = fmt(result.treatment_outcome);
        row[16] = fmt(edge.treatment_source_value);
        row[17] = fmt(edge.control_source_value);
        row[18] = edge.source_changed.to_string();
        emit(row);
    }

    let mut effect = empty_row("paired_effect", "control_vs_treatment");
    effect[2] = "qualified_treatment_effect".into();
    effect[3] = result
        .qualified_treatment_effect
        .map_or_else(|| "null".into(), fmt);
    effect[4] = "fraction".into();
    effect[7] = result.qualified.to_string();
    effect[8] = result.mechanism_path_complete.to_string();
    effect[9] = result.first_broken_edge.clone().unwrap_or_default();
    effect[11] = fmt(result.control_outcome);
    effect[12] = fmt(result.treatment_outcome);
    emit(effect);

    let mut negative = empty_row("negative_control", "negative_control");
    negative[2] = "negative_control_outcome_effect".into();
    negative[3] = fmt(result.negative_control_effect);
    negative[4] = "fraction".into();
    negative[7] = result.negative_control_isolated.to_string();
    negative[8] = result.negative_control_isolated.to_string();
    negative[11] = fmt(result.control_outcome);
    negative[12] = fmt(result.negative_control_outcome);
    emit(negative);

    let mut ablation = empty_row("mechanism_ablation", "mechanism_ablation");
    ablation[2] = "mechanism_necessity_supported".into();
    ablation[3] = result.mechanism_necessity_supported.to_string();
    ablation[4] = "boolean".into();
    ablation[7] = result.qualified.to_string();
    ablation[8] = result.mechanism_path_complete.to_string();
    ablation[11] = fmt(result.control_outcome);
    ablation[12] = fmt(result.treatment_outcome);
    ablation[19] = experiment.mechanism_ablation.block_variable_id.clone();
    ablation[20] = result.ablation_inputs_matched.to_string();
    ablation[21] = result.primary_intervention_preserved.to_string();
    ablation[22] = experiment
        .mechanism_ablation
        .blocked_mechanism_variable_id
        .clone();
    ablation[23] = result.blocked_mechanism_suppressed.to_string();
    ablation[24] = result.ablation_protocol_valid.to_string();
    ablation[25] = result.mechanism_necessity_supported.to_string();
    ablation[26] = fmt(result.mechanism_ablation_outcome);
    ablation[27] = fmt(result.mechanism_ablation_effect);
    ablation[28] = fmt(result.observed_treatment_effect);
    emit(ablation);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_effect_is_ledger_backed_and_path_qualified() {
        let (experiment, arms) = reference_experiment();
        let result = experiment.evaluate().unwrap();
        assert!(result.qualified);
        assert_eq!(result.qualified_treatment_effect, Some(1.0));
        assert_eq!(arms[0].household_served_water_m3, 0.0);
        assert_eq!(arms[1].household_served_water_m3, 10.0);
        assert_eq!(arms[2].household_served_water_m3, 0.0);
        assert_eq!(arms[3].household_served_water_m3, 0.0);
        assert_eq!(arms[0].reservoir_closing_water_m3, 20.0);
        assert_eq!(arms[1].reservoir_closing_water_m3, 10.0);
        assert_eq!(arms[2].reservoir_closing_water_m3, 20.0);
        assert!(
            arms.iter()
                .all(|arm| arm.max_water_conservation_residual_m3 < 1e-9)
        );
    }
}
