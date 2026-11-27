use std::collections::BTreeMap;

use maritime_capability_to_service::run_audit;

const OWNER_RECEIPT: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/mycelix-mpcb-tiny-v0.receipt.tsv"
);

#[derive(Clone, Debug)]
struct OwnerPlan {
    feasible: bool,
    pareto: bool,
    cost_units: u64,
    max_lead_days: u64,
    embodied_co2_units: u64,
    regional_value_units: u64,
    selected_options: Vec<String>,
    blocking_reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct OwnerReport {
    instance_id: String,
    version: String,
    work_packages: BTreeMap<String, String>,
    resources: BTreeMap<String, (u64, String)>,
    plans: BTreeMap<String, OwnerPlan>,
    pareto_plan_ids: Vec<String>,
    candidate_count: usize,
    feasible_count: usize,
    infeasible_count: usize,
}

fn whole_units(value: &str) -> u64 {
    let parsed: f64 = value.parse().expect("owner metric is numeric");
    assert!(
        (parsed - parsed.round()).abs() < 1e-9,
        "MCSC v0.4 micro-adapter accepts only whole synthetic units"
    );
    parsed.round() as u64
}

fn parse_owner_receipt(receipt: &str) -> OwnerReport {
    let mut lines = receipt.lines();
    assert_eq!(lines.next(), Some("MPCB_RECEIPT\t0.1"));

    let mut instance_id = None;
    let mut version = None;
    let mut work_packages = BTreeMap::new();
    let mut resources = BTreeMap::new();
    let mut plans = BTreeMap::<String, OwnerPlan>::new();
    let mut pareto_plan_ids = Vec::new();
    let mut summary = None;

    for line in lines {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields[0] {
            "INSTANCE" if fields.len() == 3 => {
                assert!(instance_id.replace(fields[1].to_owned()).is_none());
                version = Some(fields[2].to_owned());
            }
            "WORK_PACKAGE" if fields.len() == 4 => {
                assert!(
                    work_packages
                        .insert(fields[1].to_owned(), fields[2].to_owned())
                        .is_none()
                );
            }
            "RESOURCE" if fields.len() == 6 => {
                let resource = (whole_units(fields[4]), fields[5].to_owned());
                assert!(resources.insert(fields[1].to_owned(), resource).is_none());
            }
            "PLAN" if fields.len() == 8 => {
                let plan = OwnerPlan {
                    feasible: fields[2] == "FEASIBLE",
                    pareto: fields[3] == "PARETO",
                    cost_units: whole_units(fields[4]),
                    max_lead_days: whole_units(fields[5]),
                    embodied_co2_units: whole_units(fields[6]),
                    regional_value_units: whole_units(fields[7]),
                    selected_options: Vec::new(),
                    blocking_reasons: Vec::new(),
                };
                assert!(
                    plans.insert(fields[1].to_owned(), plan).is_none(),
                    "owner receipt duplicated a plan"
                );
            }
            "SELECTED" if fields.len() == 3 => {
                plans
                    .get_mut(fields[1])
                    .expect("SELECTED references a declared plan")
                    .selected_options
                    .push(fields[2].to_owned());
            }
            "BLOCK" if fields.len() == 3 => {
                plans
                    .get_mut(fields[1])
                    .expect("BLOCK references a declared plan")
                    .blocking_reasons
                    .push(fields[2].to_owned());
            }
            "SUMMARY" if fields.len() == 4 => {
                assert!(summary.is_none());
                summary = Some((
                    fields[1].parse().expect("candidate count is numeric"),
                    fields[2].parse().expect("feasible count is numeric"),
                    fields[3].parse().expect("infeasible count is numeric"),
                ));
            }
            "PARETO" if fields.len() == 2 => {
                pareto_plan_ids = if fields[1].is_empty() {
                    Vec::new()
                } else {
                    fields[1].split(';').map(str::to_owned).collect()
                };
            }
            other => panic!("unsupported owner receipt record: {other}"),
        }
    }

    let (candidate_count, feasible_count, infeasible_count) =
        summary.expect("owner receipt has exactly one SUMMARY");
    assert_eq!(candidate_count, plans.len());
    assert_eq!(
        feasible_count,
        plans.values().filter(|plan| plan.feasible).count()
    );
    assert_eq!(
        infeasible_count,
        plans.values().filter(|plan| !plan.feasible).count()
    );

    for plan in plans.values() {
        assert_eq!(plan.selected_options.len(), 2);
        if plan.feasible {
            assert!(plan.blocking_reasons.is_empty());
        } else {
            assert!(!plan.blocking_reasons.is_empty());
        }
    }

    let computed_pareto: Vec<String> = plans
        .iter()
        .filter(|(_, plan)| plan.pareto)
        .map(|(id, _)| id.clone())
        .collect();
    let mut published_pareto = pareto_plan_ids.clone();
    published_pareto.sort();
    assert_eq!(computed_pareto, published_pareto);

    OwnerReport {
        instance_id: instance_id.expect("owner receipt has INSTANCE"),
        version: version.expect("owner receipt has version"),
        work_packages,
        resources,
        plans,
        pareto_plan_ids,
        candidate_count,
        feasible_count,
        infeasible_count,
    }
}

fn build_service_case(owner: &OwnerReport) -> (String, BTreeMap<String, String>) {
    let mut rows = vec![
        "INSTANCE\tMCSC-MPCB-SERVICE-JOIN-001\tv0.4".to_owned(),
        "SERVICE\tSERVICE-001\t10\tcargo_unit\t20".to_owned(),
    ];
    let mut option_to_plan = BTreeMap::new();

    for (index, (plan_id, plan)) in owner
        .plans
        .iter()
        .filter(|(_, plan)| plan.feasible)
        .enumerate()
    {
        let option_id = format!("OWNER-PLAN-{index:02}");
        let has_local = plan
            .selected_options
            .iter()
            .any(|selection| selection.contains("@REGIONAL"));
        let has_import = plan
            .selected_options
            .iter()
            .any(|selection| selection.contains("@IMPORTED"));
        let region = match (has_local, has_import) {
            (true, true) => "HYBRID",
            (true, false) => "REGIONAL",
            (false, true) => "IMPORTED",
            (false, false) => panic!("owner plan has no sourced options"),
        };

        // The Mycelix owner receipt supplies procurement cost, max lead, emissions,
        // regional value and the hard feasibility decision. Three additional synthetic
        // days represent the separately declared inspection, commissioning and receiver
        // handoff stages in this service integration case.
        rows.push(format!(
            "OPTION\t{option_id}\t{region}\t{}\t{}\t1\t1\t1\t{}\t{}\t-\tREV-MPCB-001",
            plan.cost_units, plan.max_lead_days, plan.embodied_co2_units, plan.regional_value_units
        ));

        // These downstream dispositions are explicit synthetic scenario assumptions
        // applied equally to every owner-feasible plan. They are not emitted by Mycelix
        // and must never be inferred from a FEASIBLE sourcing receipt.
        rows.push(format!(
            "INSPECTION\t{option_id}\tPASS\t10\tREV-MPCB-001"
        ));
        rows.push(format!(
            "RECEIVER_ACCEPTANCE\t{option_id}\tACCEPTED\t10\tREV-MPCB-001"
        ));
        option_to_plan.insert(option_id, plan_id.clone());
    }

    (rows.join("\n") + "\n", option_to_plan)
}

#[test]
fn owner_capacity_receipt_is_composed_with_receiver_service_without_reoptimizing() {
    let owner = parse_owner_receipt(OWNER_RECEIPT);

    assert_eq!(owner.instance_id, "MPCB-TINY-001");
    assert_eq!(owner.version, "v0.1");
    assert_eq!(owner.work_packages.len(), 2);
    assert_eq!(owner.resources["REGIONAL-PLATE"], (3, "t".to_owned()));
    assert_eq!(owner.resources["REGIONAL-WELD"], (12, "h".to_owned()));
    assert_eq!(owner.resources["REGIONAL-NDT"], (2, "job".to_owned()));
    assert_eq!(owner.candidate_count, 4);
    assert_eq!(owner.feasible_count, 3);
    assert_eq!(owner.infeasible_count, 1);
    assert_eq!(owner.pareto_plan_ids.len(), 3);

    let all_local = owner
        .plans
        .iter()
        .find(|(id, _)| id.contains("A-LOCAL@REGIONAL+WP-B=B-LOCAL@REGIONAL"))
        .map(|(id, plan)| (id.clone(), plan))
        .expect("all-local plan exists");
    assert!(!all_local.1.feasible);
    assert!(
        all_local
            .1
            .blocking_reasons
            .iter()
            .any(|reason| reason.contains("REGIONAL-PLATE") && reason.contains("used=4.000"))
    );
    assert!(
        all_local
            .1
            .blocking_reasons
            .iter()
            .any(|reason| reason.contains("REGIONAL-WELD") && reason.contains("used=16.000"))
    );

    let (service_case, option_to_plan) = build_service_case(&owner);
    let receipt = run_audit(&service_case)
        .expect("owner-feasible plans produce a canonical receiver-service receipt");
    assert_eq!(
        option_to_plan.len(),
        3,
        "only owner-feasible plans enter service evaluation"
    );
    assert!(
        !option_to_plan
            .values()
            .any(|plan_id| plan_id == &all_local.0),
        "resource-infeasible all-local plan must not receive a hypothetical completion time"
    );
    let mut seen = BTreeMap::new();

    for line in receipt.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.first() != Some(&"OUTCOME") {
            continue;
        }
        let option_id = fields[1].to_owned();
        let plan_id = option_to_plan
            .get(&option_id)
            .expect("every outcome maps to an owner plan");
        let owner_plan = &owner.plans[plan_id];
        let receiver_day: u64 = fields[4].parse().expect("receiver day is numeric");
        let expected_units: u64 = fields[5].parse().expect("expected units are numeric");
        let inspection_qualified: u64 = fields[6].parse().expect("inspection units are numeric");
        let receiver_accepted: u64 = fields[7].parse().expect("accepted units are numeric");
        let on_time: u64 = fields[8].parse().expect("on-time units are numeric");

        assert_eq!(expected_units, 10);
        assert!(owner_plan.feasible);
        assert_eq!(inspection_qualified, 10);
        assert_eq!(receiver_accepted, 10);
        assert_eq!(on_time, if receiver_day <= 20 { 10 } else { 0 });
        assert_eq!(fields[12].parse::<u64>().unwrap(), owner_plan.cost_units);
        assert_eq!(
            fields[13].parse::<u64>().unwrap(),
            owner_plan.embodied_co2_units
        );
        assert_eq!(
            fields[14].parse::<u64>().unwrap(),
            owner_plan.regional_value_units
        );
        seen.insert(plan_id.clone(), (receiver_day, receiver_accepted, on_time));
    }

    assert_eq!(seen.len(), 3);
    assert!(
        seen.values().all(|(_, accepted, _)| *accepted == 10),
        "synthetic downstream PASS/ACCEPTED assumptions apply to all feasible owner plans"
    );

    // The faster all-local portfolio remains visible in the source receipt's blocking
    // witnesses, but is excluded from the service candidate set before timing is computed.

    let timely_hybrid = owner
        .plans
        .iter()
        .find(|(id, plan)| {
            plan.feasible && id.contains("A-LOCAL@REGIONAL") && id.contains("B-IMPORT@IMPORTED")
        })
        .map(|(id, _)| id)
        .expect("timely hybrid owner plan exists");
    assert_eq!(seen[timely_hybrid], (19, 10, 10));

    // Imported-heavy and the alternative hybrid are feasible but miss the declared
    // receiver deadline once inspection, commissioning and handoff time are included.
    assert_eq!(
        seen.iter()
            .filter(|(plan_id, (_, accepted, on_time))| {
                owner.plans[*plan_id].feasible && *accepted == 10 && *on_time == 0
            })
            .count(),
        2
    );
}
