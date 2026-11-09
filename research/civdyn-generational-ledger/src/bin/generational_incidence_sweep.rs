// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_generational_ledger::{
    CapitalClass, CohortAssignment, CohortIncidenceLedger, FinancialObligation,
    FinancialObligationKind, InheritedCapitalRecord, InheritedStockAllocation,
    PhysicalObligation, QuantityUncertainty, Resource,
    ServiceAllocation, UnassignedReason,
};
use civdyn_resource_ledger::{
    Production, Region, ResourceVector, StepInput, TransferPolicy, World,
};

const CSV_COLUMNS: usize = 24;

fn resource_name(resource: Resource) -> &'static str {
    match resource {
        Resource::Water => "water",
        Resource::Energy => "energy",
        Resource::Food => "food",
    }
}

fn number(value: f64) -> String {
    format!("{value:.6}")
}

fn blank_row() -> Vec<String> {
    vec![String::new(); CSV_COLUMNS]
}

fn emit(mut fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "CSV column contract changed");
    for field in &mut fields {
        if field.contains(',') || field.contains('"') {
            *field = format!("\"{}\"", field.replace('"', "\"\""));
        }
    }
    println!("{}", fields.join(","));
}

fn main() {
    let mut world = World::new(vec![Region {
        id: "district-a".into(),
        population: 4.0,
        opening_stock: ResourceVector::new(100.0, 200.0, 80.0),
        essential_demand_per_capita: ResourceVector::new(5.0, 10.0, 4.0),
    }])
    .expect("frozen source world is valid");

    let source = world
        .step(
            &StepInput {
                production: vec![Production::default()],
                transfers: vec![],
            },
            TransferPolicy::Unrestricted,
        )
        .expect("source ledger step is valid");

    let present = CohortAssignment::assigned("working-age-2026")
        .expect("cohort identity is non-empty");
    let future = CohortAssignment::assigned("future-born-2040")
        .expect("cohort identity is non-empty");
    let shared = CohortAssignment::unassigned(UnassignedReason::SharedPublicAsset);
    let source_ref = "ledger:tick-1:district-a";
    let action_id = "policy:baseline";

    let service = vec![
        ServiceAllocation {
            allocation_id: "service-water-present".into(),
            assignment: present.clone(),
            region_id: "district-a".into(),
            resource: Resource::Water,
            demand: 5.0,
            served: 5.0,
            originating_action_id: action_id.into(),
            source_evidence_ref: source_ref.into(),
        },
        ServiceAllocation {
            allocation_id: "service-water-young".into(),
            assignment: future.clone(),
            region_id: "district-a".into(),
            resource: Resource::Water,
            demand: 15.0,
            served: 15.0,
            originating_action_id: action_id.into(),
            source_evidence_ref: source_ref.into(),
        },
        ServiceAllocation {
            allocation_id: "service-energy-present".into(),
            assignment: present.clone(),
            region_id: "district-a".into(),
            resource: Resource::Energy,
            demand: 10.0,
            served: 10.0,
            originating_action_id: action_id.into(),
            source_evidence_ref: source_ref.into(),
        },
        ServiceAllocation {
            allocation_id: "service-energy-young".into(),
            assignment: future.clone(),
            region_id: "district-a".into(),
            resource: Resource::Energy,
            demand: 30.0,
            served: 30.0,
            originating_action_id: action_id.into(),
            source_evidence_ref: source_ref.into(),
        },
        ServiceAllocation {
            allocation_id: "service-food-present".into(),
            assignment: present,
            region_id: "district-a".into(),
            resource: Resource::Food,
            demand: 4.0,
            served: 4.0,
            originating_action_id: action_id.into(),
            source_evidence_ref: source_ref.into(),
        },
        ServiceAllocation {
            allocation_id: "service-food-young".into(),
            assignment: future.clone(),
            region_id: "district-a".into(),
            resource: Resource::Food,
            demand: 12.0,
            served: 12.0,
            originating_action_id: action_id.into(),
            source_evidence_ref: source_ref.into(),
        },
    ];

    let stocks = vec![
        InheritedStockAllocation {
            allocation_id: "stock-water-future".into(),
            assignment: future.clone(),
            region_id: "district-a".into(),
            asset_id: "reservoir-a".into(),
            resource: Resource::Water,
            closing_stock: 40.0,
            source_evidence_ref: source_ref.into(),
        },
        InheritedStockAllocation {
            allocation_id: "stock-water-shared".into(),
            assignment: shared.clone(),
            region_id: "district-a".into(),
            asset_id: "reservoir-a".into(),
            resource: Resource::Water,
            closing_stock: 40.0,
            source_evidence_ref: source_ref.into(),
        },
        InheritedStockAllocation {
            allocation_id: "stock-energy-future".into(),
            assignment: future.clone(),
            region_id: "district-a".into(),
            asset_id: "grid-storage-a".into(),
            resource: Resource::Energy,
            closing_stock: 80.0,
            source_evidence_ref: source_ref.into(),
        },
        InheritedStockAllocation {
            allocation_id: "stock-energy-shared".into(),
            assignment: shared.clone(),
            region_id: "district-a".into(),
            asset_id: "grid-storage-a".into(),
            resource: Resource::Energy,
            closing_stock: 80.0,
            source_evidence_ref: source_ref.into(),
        },
        InheritedStockAllocation {
            allocation_id: "stock-food-future".into(),
            assignment: future.clone(),
            region_id: "district-a".into(),
            asset_id: "food-reserve-a".into(),
            resource: Resource::Food,
            closing_stock: 32.0,
            source_evidence_ref: source_ref.into(),
        },
        InheritedStockAllocation {
            allocation_id: "stock-food-shared".into(),
            assignment: shared,
            region_id: "district-a".into(),
            asset_id: "food-reserve-a".into(),
            resource: Resource::Food,
            closing_stock: 32.0,
            source_evidence_ref: source_ref.into(),
        },
    ];

    let physical_obligations = vec![PhysicalObligation {
        obligation_id: "restoration-water-1".into(),
        originating_action_id: "policy:defer-restoration".into(),
        created_tick: 1,
        due_tick: 3,
        region_id: "district-a".into(),
        assignment: future.clone(),
        measure_id: "restoration-water".into(),
        unit: "m3".into(),
        quantity: 12.0,
        uncertainty: QuantityUncertainty::Interval {
            lower: 10.0,
            upper: 15.0,
        },
        source_evidence_ref: "policy-receipt:1".into(),
    }];

    let financial_obligations = vec![FinancialObligation {
        obligation_id: "debt-principal-1".into(),
        originating_action_id: "policy:debt-financed-asset".into(),
        created_tick: 1,
        due_tick: 8,
        liable_cohort: future.clone(),
        recipient_cohort: Some(CohortAssignment::assigned("working-age-2026").unwrap()),
        amount: 500.0,
        currency_code: "USD".into(),
        price_base_year: 2026,
        kind: FinancialObligationKind::Principal,
        uncertainty: QuantityUncertainty::Known,
        source_evidence_ref: "financial-receipt:1".into(),
    }];

    let capital_records = vec![InheritedCapitalRecord {
        record_id: "reservoir-condition-1".into(),
        assignment: future.clone(),
        region_id: "district-a".into(),
        asset_id: "reservoir-a".into(),
        capital_class: CapitalClass::Natural,
        measure_id: "asset-condition".into(),
        unit: "fraction".into(),
        quantity: 1.0,
        condition: Some(0.80),
        uncertainty: QuantityUncertainty::Interval {
            lower: 0.75,
            upper: 0.85,
        },
        source_evidence_ref: "asset-inspection:1".into(),
    }];

    let mut ledger = CohortIncidenceLedger::new();
    ledger
        .assemble_tick(
            &source,
            service,
            stocks,
            physical_obligations,
            financial_obligations,
            capital_records,
        )
        .expect("cohort incidence reconciles with source ledger");

    println!(
        concat!(
            "record_type,tick,region_id,resource,assignment,demand,served,unmet,inherited_stock,",
            "ledger_demand,allocated_demand,ledger_served,allocated_served,ledger_unmet,",
            "allocated_unmet,ledger_closing_stock,allocated_closing_stock,record_id,",
            "measure_id,unit,quantity,currency_code,amount,price_base_year"
        )
    );

    let report = ledger.last().expect("one incidence report exists");
    for row in &report.regional_reconciliations {
        let mut fields = blank_row();
        fields[0] = "reconciliation".into();
        fields[1] = report.tick.to_string();
        fields[2] = row.region_id.clone();
        fields[3] = resource_name(row.resource).into();
        fields[9] = number(row.ledger_demand);
        fields[10] = number(row.allocated_demand);
        fields[11] = number(row.ledger_served);
        fields[12] = number(row.allocated_served);
        fields[13] = number(row.ledger_unmet);
        fields[14] = number(row.allocated_unmet);
        fields[15] = number(row.ledger_closing_stock);
        fields[16] = number(row.allocated_closing_stock);
        emit(fields);
    }

    for summary in ledger.cohort_summaries(report.tick).unwrap() {
        for resource in [Resource::Water, Resource::Energy, Resource::Food] {
            let mut fields = blank_row();
            fields[0] = "cohort_service".into();
            fields[1] = report.tick.to_string();
            fields[3] = resource_name(resource).into();
            fields[4] = summary.assignment.label();
            fields[5] = number(summary.service_demand.get(resource));
            fields[6] = number(summary.service_served.get(resource));
            fields[7] = number(summary.service_unmet.get(resource));
            fields[8] = number(summary.inherited_stock.get(resource));
            emit(fields);
        }
    }

    for obligation in &report.physical_obligations {
        let mut fields = blank_row();
        fields[0] = "physical_obligation".into();
        fields[1] = report.tick.to_string();
        fields[2] = obligation.region_id.clone();
        fields[4] = obligation.assignment.label();
        fields[17] = obligation.obligation_id.clone();
        fields[18] = obligation.measure_id.clone();
        fields[19] = obligation.unit.clone();
        fields[20] = number(obligation.quantity);
        emit(fields);
    }

    for obligation in &report.financial_obligations {
        let mut fields = blank_row();
        fields[0] = "financial_obligation".into();
        fields[1] = report.tick.to_string();
        fields[4] = obligation.liable_cohort.label();
        fields[17] = obligation.obligation_id.clone();
        fields[18] = format!("{:?}", obligation.kind);
        fields[19] = obligation.currency_code.clone();
        fields[21] = obligation.currency_code.clone();
        fields[22] = number(obligation.amount);
        fields[23] = obligation.price_base_year.to_string();
        emit(fields);
    }
}
