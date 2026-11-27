use std::collections::BTreeMap;

use maritime_receiver_service::{
    evaluate_receiver_service, CargoUnitOutcome, FloorDisposition, OutcomeEvidence,
    ProtectedServiceFloor,
};

const OWNER_RECEIPT: &str = include_str!("../fixtures/mycelix-mpcb-tiny-v0.receipt.tsv");

#[derive(Clone, Debug)]
struct OwnerPlan {
    feasible: bool,
    pareto: bool,
    cost: u64,
    lead_days: u64,
    emissions: u64,
    regional_value: u64,
    selected: Vec<String>,
    blockers: Vec<String>,
}

fn whole_units(value: &str) -> u64 {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    assert!(fraction.chars().all(|digit| digit == '0'));
    whole.parse().expect("whole numeric source value")
}

fn owner_plans(receipt: &str) -> BTreeMap<String, OwnerPlan> {
    let mut plans = BTreeMap::new();
    for line in receipt.lines().skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        match f[0] {
            "PLAN" => {
                assert_eq!(f.len(), 8);
                let plan = OwnerPlan {
                    feasible: f[2] == "FEASIBLE",
                    pareto: f[3] == "PARETO",
                    cost: whole_units(f[4]),
                    lead_days: whole_units(f[5]),
                    emissions: whole_units(f[6]),
                    regional_value: whole_units(f[7]),
                    selected: Vec::new(),
                    blockers: Vec::new(),
                };
                assert!(plans.insert(f[1].to_owned(), plan).is_none());
            }
            "SELECTED" => plans
                .get_mut(f[1])
                .expect("plan exists")
                .selected
                .push(f[2].to_owned()),
            "BLOCK" => plans
                .get_mut(f[1])
                .expect("plan exists")
                .blockers
                .push(f[2].to_owned()),
            _ => {}
        }
    }
    for plan in plans.values() {
        assert_eq!(plan.selected.len(), 2);
        assert_eq!(plan.feasible, plan.blockers.is_empty());
    }
    plans
}

fn make_records(
    cohort_id: &str,
    unit_count: u64,
    receiver_day: i64,
    deadline: i64,
) -> Vec<CargoUnitOutcome> {
    (0..unit_count)
        .map(|index| CargoUnitOutcome {
            unit_id: format!("{cohort_id}#UNIT-{index:02}"),
            cohort_id: cohort_id.to_owned(),
            quantity: 1,
            unit: "cargo_unit".to_owned(),
            deadline_at: deadline,
            evidence: OutcomeEvidence::ReceiverAccepted {
                accepted_at: receiver_day,
                independently_verified: true,
            },
        })
        .collect()
}

#[test]
fn owner_feasible_portfolios_use_the_canonical_receiver_service_evaluator() {
    let plans = owner_plans(OWNER_RECEIPT);
    let declared_resources = OWNER_RECEIPT.lines().collect::<Vec<_>>();
    assert!(declared_resources.contains(&"RESOURCE\\tREGIONAL-PLATE\\tYARD-A\\tP1\\t3.000\\tt"));
    assert!(declared_resources.contains(&"RESOURCE\\tREGIONAL-WELD\\tYARD-A\\tP1\\t12.000\\th"));
    assert!(declared_resources.contains(&"RESOURCE\\tREGIONAL-NDT\\tYARD-A\\tP1\\t2.000\\tjob"));
    assert_eq!(plans.len(), 4);
    assert_eq!(plans.values().filter(|plan| plan.feasible).count(), 3);
    assert_eq!(plans.values().filter(|plan| plan.pareto).count(), 3);
    assert_eq!(
        plans
            .values()
            .filter(|plan| plan.feasible)
            .map(|plan| plan.cost + plan.emissions + plan.regional_value)
            .sum::<u64>(),
        772
    );

    let all_local = plans
        .iter()
        .find(|(id, _)| id.contains("A-LOCAL@REGIONAL+WP-B=B-LOCAL@REGIONAL"))
        .expect("all-local alternative exists in owner receipt");
    assert!(!all_local.1.feasible);
    assert!(all_local.1.blockers.iter().any(|reason| {
        reason.contains("REGIONAL-PLATE")
            && reason.contains("used=4.000")
            && reason.contains("capacity=3.000")
            && reason.contains("unit=t")
    }));
    assert!(all_local.1.blockers.iter().any(|reason| {
        reason.contains("REGIONAL-WELD")
            && reason.contains("used=16.000")
            && reason.contains("capacity=12.000")
            && reason.contains("unit=h")
    }));

    let mut records = Vec::new();
    let mut floors = Vec::new();
    let mut expected_regions = BTreeMap::new();
    for (plan_id, plan) in plans.iter().filter(|(_, plan)| plan.feasible) {
        let local = plan.selected.iter().any(|item| item.contains("@REGIONAL"));
        let imported = plan.selected.iter().any(|item| item.contains("@IMPORTED"));
        let region = match (local, imported) {
            (true, true) => "HYBRID",
            (true, false) => "REGIONAL",
            (false, true) => "IMPORTED",
            (false, false) => panic!("source plan has no supplier option"),
        };
        // The +3 days and verified-receiver event are explicit synthetic downstream
        // assumptions for this experiment, not Mycelix facts or Mycelix receipt fields.
        let receiver_day = i64::try_from(plan.lead_days + 3).unwrap();
        records.extend(make_records(plan_id, 10, receiver_day, 20));
        floors.push(ProtectedServiceFloor {
            cohort_id: plan_id.clone(),
            unit: "cargo_unit".to_owned(),
            minimum_basis_points: 10_000,
        });
        expected_regions.insert(plan_id.clone(), region);
    }
    assert_eq!(expected_regions.len(), 3);
    assert!(!expected_regions.contains_key(&all_local.0));

    let report = evaluate_receiver_service(&records, &floors, 30)
        .expect("canonical receiver service evaluator accepts synthetic evidence");
    assert_eq!(report.unit_count, 30);
    assert_eq!(report.cohorts.len(), 3);
    assert!(!report.all_configured_floors_pass());

    for cohort in &report.cohorts {
        let plan = &plans[&cohort.cohort_id];
        assert!(plan.feasible);
        assert_eq!(cohort.scheduled_quantity, 10);
        assert_eq!(cohort.adjudicated_quantity, 10);
        assert_eq!(cohort.conservative_lower_denominator, 10);
        if plan.lead_days + 3 <= 20 {
            assert_eq!(cohort.on_time_quantity, 10);
            assert_eq!(cohort.floor_disposition, FloorDisposition::Pass);
        } else {
            assert_eq!(cohort.late_quantity, 10);
            assert_eq!(cohort.on_time_quantity, 0);
            assert_eq!(cohort.floor_disposition, FloorDisposition::Fail);
        }
    }

    let timely_hybrid = plans
        .iter()
        .find(|(id, plan)| {
            plan.feasible && id.contains("A-LOCAL@REGIONAL") && id.contains("B-IMPORT@IMPORTED")
        })
        .map(|(id, _)| id)
        .expect("timely hybrid exists");
    let timely = report
        .cohorts
        .iter()
        .find(|cohort| &cohort.cohort_id == timely_hybrid)
        .expect("timely hybrid has a service result");
    assert_eq!(timely.on_time_quantity, 10);
    assert_eq!(timely.floor_disposition, FloorDisposition::Pass);
}

#[test]
fn censored_receiver_units_produce_bounds_and_unknown_floor_not_assumed_success() {
    let cohort_id = "SYNTHETIC-CENSORED-PORTFOLIO";
    let mut records = make_records(cohort_id, 6, 19, 20);
    for index in 0..4 {
        records.push(CargoUnitOutcome {
            unit_id: format!("{cohort_id}#CENSORED-{index:02}"),
            cohort_id: cohort_id.to_owned(),
            quantity: 1,
            unit: "cargo_unit".to_owned(),
            deadline_at: 20,
            evidence: OutcomeEvidence::Censored {
                last_observed_at: 15,
            },
        });
    }
    let floor = ProtectedServiceFloor {
        cohort_id: cohort_id.to_owned(),
        unit: "cargo_unit".to_owned(),
        minimum_basis_points: 8_000,
    };
    let report = evaluate_receiver_service(&records, &[floor], 30).unwrap();
    let result = &report.cohorts[0];
    assert_eq!(
        (
            result.scheduled_quantity,
            result.on_time_quantity,
            result.unresolved_quantity,
        ),
        (10, 6, 4)
    );
    assert_eq!(
        (
            result.conservative_lower_numerator,
            result.conservative_lower_denominator
        ),
        (6, 10)
    );
    assert_eq!(
        (
            result.best_case_upper_numerator,
            result.best_case_upper_denominator
        ),
        (10, 10)
    );
    assert_eq!(result.floor_disposition, FloorDisposition::Unknown);
}
