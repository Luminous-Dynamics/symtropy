use std::collections::BTreeMap;

use maritime_capability_to_service::run_audit;

const BASE: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/mcsc-counterfactual-tiny-v0.tsv"
);
const CORPUS: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/mcsc-scenarios-v0.tsv"
);

fn scenario_input(
    id: &str,
    shock_spec: &str,
    availability_override: &str,
    deadline: u64,
    expected_units: u64,
    inspection_override: &str,
    receiver_override: &str,
) -> String {
    let mut records = Vec::new();
    for line in BASE.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields[0] {
            "INSTANCE" => records.push(format!("INSTANCE\tMCSC-{id}\tv0.2")),
            "SERVICE" => records.push(format!(
                "SERVICE\t{}\t{}\t{}\t{}",
                fields[1], expected_units, fields[3], deadline
            )),
            "DEPENDENCY" => {
                let (dependency, day) = availability_override
                    .split_once(':')
                    .unwrap_or(("NONE", "0"));
                let availability = if dependency == fields[1] {
                    day.parse::<u64>().expect("corpus dependency day is numeric")
                } else {
                    fields[2].parse::<u64>().expect("base dependency day is numeric")
                };
                records.push(format!("DEPENDENCY\t{}\t{availability}", fields[1]));
            }
            "SHOCK" => {}
            "INSPECTION" => {
                records.push(apply_disposition_override(line, &fields, inspection_override));
            }
            "RECEIVER_ACCEPTANCE" => {
                records.push(apply_disposition_override(line, &fields, receiver_override));
            }
            _ => records.push(line.to_owned()),
        }
    }

    if shock_spec != "NONE" {
        for shock in shock_spec.split(',') {
            let (dependency, end_day) = shock
                .split_once(':')
                .expect("corpus shock has dependency:end form");
            let end_day = end_day.parse::<u64>().expect("shock end is numeric");
            records.push(format!("SHOCK\t{dependency}\t0\t{end_day}"));
        }
    }
    records.join("\n") + "\n"
}

fn apply_disposition_override(line: &str, fields: &[&str], override_spec: &str) -> String {
    if override_spec == "NONE" {
        return line.to_owned();
    }
    let parts: Vec<&str> = override_spec.split(':').collect();
    assert_eq!(parts.len(), 4);
    if parts[0] == fields[1] {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            fields[0], parts[0], parts[1], parts[2], parts[3]
        )
    } else {
        line.to_owned()
    }
}

type ExpectedOutcome = (u64, u64, u64, u64, u64, u64, String, String, String);

fn independent_outcomes(
    base: &str,
    shock_spec: &str,
    availability_override: &str,
    deadline: u64,
    expected_units: u64,
    inspection_override: &str,
    receiver_override: &str,
) -> BTreeMap<String, ExpectedOutcome> {
    let mut availability = BTreeMap::<String, u64>::new();
    let mut options = Vec::<Vec<String>>::new();
    let mut inspections = BTreeMap::<String, (String, u64, String)>::new();
    let mut receivers = BTreeMap::<String, (String, u64, String)>::new();

    for line in base.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(str::to_owned).collect();
        match fields[0].as_str() {
            "DEPENDENCY" => {
                availability.insert(
                    fields[1].clone(),
                    fields[2].parse().expect("base availability is numeric"),
                );
            }
            "OPTION" => options.push(fields),
            "INSPECTION" => {
                inspections.insert(
                    fields[1].clone(),
                    (
                        fields[2].clone(),
                        fields[3].parse().expect("qualified units are numeric"),
                        fields[4].clone(),
                    ),
                );
            }
            "RECEIVER_ACCEPTANCE" => {
                receivers.insert(
                    fields[1].clone(),
                    (
                        fields[2].clone(),
                        fields[3].parse().expect("receiver units are numeric"),
                        fields[4].clone(),
                    ),
                );
            }
            _ => {}
        }
    }

    if availability_override != "NONE" {
        let (dependency, day) = availability_override
            .split_once(':')
            .expect("override has dependency:day form");
        availability.insert(
            dependency.to_owned(),
            day.parse().expect("override day is numeric"),
        );
    }
    apply_reference_override(&mut inspections, inspection_override);
    apply_reference_override(&mut receivers, receiver_override);

    let shocks: BTreeMap<String, u64> = if shock_spec == "NONE" {
        BTreeMap::new()
    } else {
        shock_spec
            .split(',')
            .map(|item| {
                let (dependency, end_day) = item
                    .split_once(':')
                    .expect("shock has dependency:end form");
                (
                    dependency.to_owned(),
                    end_day.parse().expect("shock end is numeric"),
                )
            })
            .collect()
    };

    let mut expected = BTreeMap::new();
    for option in options {
        let dependencies: Vec<&str> = if option[10] == "-" {
            Vec::new()
        } else {
            option[10].split(',').collect()
        };
        let mut normal_ready = 0_u64;
        let mut disrupted_ready = 0_u64;
        for dependency in dependencies {
            let available = *availability
                .get(dependency)
                .expect("fixture option dependency is declared");
            normal_ready = normal_ready.max(available);
            disrupted_ready =
                disrupted_ready.max(available.max(*shocks.get(dependency).unwrap_or(&0)));
        }

        let duration = option[4].parse::<u64>().unwrap()
            + option[5].parse::<u64>().unwrap()
            + option[6].parse::<u64>().unwrap()
            + option[7].parse::<u64>().unwrap();
        let normal_day = normal_ready + duration;
        let disrupted_day = disrupted_ready + duration;
        let (inspection_status, qualified_reported, inspection_revision) =
            inspections.get(&option[1]).expect("inspection exists");
        let (receiver_status, accepted_reported, receiver_revision) =
            receivers.get(&option[1]).expect("receiver disposition exists");
        assert_eq!(inspection_revision, &option[11]);
        assert_eq!(receiver_revision, &option[11]);
        assert!(qualified_reported <= &expected_units);
        assert!(accepted_reported <= qualified_reported);
        assert!(
            inspection_status == "PASS" || *qualified_reported == 0,
            "failed/unknown inspection cannot qualify units"
        );
        assert!(
            receiver_status == "ACCEPTED" || *accepted_reported == 0,
            "rejected/unknown receiver cannot accept units"
        );
        let qualified = if inspection_status == "PASS" {
            *qualified_reported
        } else {
            0
        };
        let accepted = if receiver_status == "ACCEPTED" {
            *accepted_reported
        } else {
            0
        };
        let on_time = if disrupted_day <= deadline
            && inspection_status == "PASS"
            && receiver_status == "ACCEPTED"
        {
            *accepted_reported
        } else {
            0
        };
        expected.insert(
            option[1].clone(),
            (
                normal_day,
                disrupted_day,
                expected_units,
                qualified,
                accepted,
                on_time,
                inspection_status.clone(),
                receiver_status.clone(),
                option[11].clone(),
            ),
        );
    }
    expected
}

fn apply_reference_override(
    dispositions: &mut BTreeMap<String, (String, u64, String)>,
    override_spec: &str,
) {
    if override_spec == "NONE" {
        return;
    }
    let parts: Vec<&str> = override_spec.split(':').collect();
    assert_eq!(parts.len(), 4);
    dispositions.insert(
        parts[0].to_owned(),
        (
            parts[1].to_owned(),
            parts[2].parse().expect("override unit count is numeric"),
            parts[3].to_owned(),
        ),
    );
}

fn receipt_outcomes(receipt: &str) -> BTreeMap<String, ExpectedOutcome> {
    let mut observed = BTreeMap::new();
    for line in receipt.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.first() == Some(&"OUTCOME") {
            let value = (
                fields[3].parse().expect("normal day is numeric"),
                fields[4].parse().expect("disrupted day is numeric"),
                fields[5].parse().expect("expected units are numeric"),
                fields[6].parse().expect("inspection-qualified units are numeric"),
                fields[7].parse().expect("receiver-accepted units are numeric"),
                fields[8].parse().expect("on-time accepted units are numeric"),
                fields[9].to_owned(),
                fields[10].to_owned(),
                fields[11].to_owned(),
            );
            assert!(observed.insert(fields[1].to_owned(), value).is_none());
        }
    }
    observed
}

#[test]
fn thirty_frozen_scenarios_match_the_separate_reference_or_reject() {
    let mut count = 0usize;
    for line in CORPUS.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields[0], "CASE");
        assert_eq!(fields.len(), 9);
        let id = fields[1];
        let shocks = fields[2];
        let availability = fields[3];
        let deadline = fields[4].parse::<u64>().expect("deadline is numeric");
        let units = fields[5].parse::<u64>().expect("expected units are numeric");
        let inspection = fields[6];
        let receiver = fields[7];
        let expected_result = fields[8];
        assert!(units > 0);

        let generated = scenario_input(
            id,
            shocks,
            availability,
            deadline,
            units,
            inspection,
            receiver,
        );
        let result = run_audit(&generated);
        match expected_result {
            "ERROR_REVISION" => assert!(
                result.unwrap_err().contains("revision"),
                "scenario {id} must reject a revision mismatch"
            ),
            "ERROR_RECEIVER_UNITS" => assert!(
                result.unwrap_err().contains("exceed inspection-qualified"),
                "scenario {id} must reject receiver over-acceptance"
            ),
            "ERROR_RECEIVER_STATUS" => assert!(
                result.unwrap_err().contains("without receiver acceptance"),
                "scenario {id} must reject accepted units from an UNKNOWN/REJECTED receiver"
            ),
            "PASS" => {
                let receipt = result.unwrap_or_else(|error| {
                    panic!("scenario {id} rejected: {error}");
                });
                let independently_expected = independent_outcomes(
                    BASE,
                    shocks,
                    availability,
                    deadline,
                    units,
                    inspection,
                    receiver,
                );
                let observed = receipt_outcomes(&receipt);
                assert_eq!(observed, independently_expected, "scenario {id}");
                assert_eq!(observed.len(), 5, "scenario {id} lost an option");
                assert!(
                    observed
                        .values()
                        .all(|(_, _, denominator, _, _, _, _, _, _)| *denominator == units),
                    "scenario {id} changed its expected-service denominator"
                );
            }
            other => panic!("unsupported expected result {other} in scenario {id}"),
        }
        count += 1;
    }
    assert_eq!(count, 30);
}

#[test]
fn equality_deadlines_and_large_denominators_remain_exact() {
    let equality = scenario_input("EQUALITY", "GRID-A:6", "NONE", 14, 10, "NONE", "NONE");
    let equality_receipt = receipt_outcomes(&run_audit(&equality).unwrap());
    assert_eq!(equality_receipt["LOCAL-A"].5, 10);

    let strict = scenario_input("STRICT", "GRID-A:6", "NONE", 13, 10, "NONE", "NONE");
    let strict_receipt = receipt_outcomes(&run_audit(&strict).unwrap());
    assert_eq!(strict_receipt["LOCAL-A"].5, 0);

    let large = scenario_input("LARGE", "GRID-A:6", "MILL-01:4", 15, 1_000, "NONE", "NONE");
    let large_receipt = receipt_outcomes(&run_audit(&large).unwrap());
    assert_eq!(large_receipt["LOCAL-B"].2, 1_000);
    assert_eq!(large_receipt["LOCAL-B"].3, 10);
    assert_eq!(large_receipt["LOCAL-B"].4, 10);
    assert_eq!(large_receipt["LOCAL-B"].5, 10);
}
