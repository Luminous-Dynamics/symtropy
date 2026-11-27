use std::collections::BTreeMap;

use maritime_capability_to_service::run_audit;

const FIXTURE: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/mcsc-counterfactual-tiny-v0.tsv"
);

#[test]
fn independent_parser_reconstructs_the_complete_service_receipt() {
    // Deliberately separate from the library parser and evaluator. This tiny
    // reference reads the fixture rows directly and independently recomputes
    // both the no-shock and declared day-zero common-cause cases.
    let mut expected_units = 0_u64;
    let mut deadline_day = 0_u64;
    let mut availability = BTreeMap::<String, u64>::new();
    let mut outage_end = BTreeMap::<String, u64>::new();
    let mut option_rows = Vec::<Vec<String>>::new();

    for line in FIXTURE.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(str::to_owned).collect();
        match fields[0].as_str() {
            "SERVICE" => {
                expected_units = fields[2].parse().expect("expected units are numeric");
                deadline_day = fields[4].parse().expect("deadline is numeric");
            }
            "DEPENDENCY" => {
                availability.insert(
                    fields[1].clone(),
                    fields[2].parse().expect("availability is numeric"),
                );
            }
            "SHOCK" => {
                assert_eq!(fields[2], "0", "reference accepts only day-zero shocks");
                outage_end.insert(
                    fields[1].clone(),
                    fields[3].parse().expect("shock end is numeric"),
                );
            }
            "OPTION" => option_rows.push(fields),
            "INSTANCE" => {}
            other => panic!("unexpected fixture record: {other}"),
        }
    }

    assert!(expected_units > 0);
    let mut independently_expected = BTreeMap::<String, (u64, u64, u64, u64)>::new();

    for option in option_rows {
        let dependencies: Vec<&str> = if option[10] == "-" {
            Vec::new()
        } else {
            option[10].split(',').collect()
        };

        let base_ready = dependencies
            .iter()
            .map(|dependency| {
                *availability
                    .get(*dependency)
                    .expect("every listed dependency has a base availability")
            })
            .max()
            .unwrap_or(0);

        let outage_ready = dependencies
            .iter()
            .map(|dependency| {
                let base = *availability
                    .get(*dependency)
                    .expect("every listed dependency has a base availability");
                base.max(*outage_end.get(*dependency).unwrap_or(&0))
            })
            .max()
            .unwrap_or(0);

        let duration = option[4..8]
            .iter()
            .map(|value| value.parse::<u64>().expect("duration is numeric"))
            .sum::<u64>();
        let without_shock = base_ready + duration;
        let with_shock = outage_ready + duration;
        let on_time = if with_shock <= deadline_day {
            expected_units
        } else {
            0
        };

        independently_expected.insert(
            option[1].clone(),
            (without_shock, with_shock, expected_units, on_time),
        );
    }

    let receipt = run_audit(FIXTURE).expect("the frozen fixture audits");
    let mut observed = BTreeMap::<String, (u64, u64, u64, u64)>::new();
    for line in receipt.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.first() == Some(&"OUTCOME") {
            observed.insert(
                fields[1].to_owned(),
                (
                    fields[3].parse().expect("receipt base day is numeric"),
                    fields[4].parse().expect("receipt outage day is numeric"),
                    fields[5].parse().expect("receipt denominator is numeric"),
                    fields[6].parse().expect("receipt on-time units are numeric"),
                ),
            );
        }
    }

    assert_eq!(observed, independently_expected);
    assert_eq!(observed.len(), 5, "no portfolio may disappear from the receipt");
    assert!(receipt.lines().any(|line| line.starts_with("OPTION\tLOCAL-A\t")));
    assert!(receipt.lines().any(|line| line.starts_with("OPTION\tLOCAL-B\t")));
}
