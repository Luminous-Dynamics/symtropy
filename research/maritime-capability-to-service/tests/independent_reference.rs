use std::collections::BTreeMap;

use maritime_capability_to_service::run_audit;

const FIXTURE: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/mcsc-counterfactual-tiny-v0.tsv"
);

#[test]
fn independent_parser_reconstructs_receiver_acceptance_and_deadline_receipt() {
    // This reference is intentionally separate from the crate parser/reducer.
    let mut expected_units = 0_u64;
    let mut deadline_day = 0_u64;
    let mut availability = BTreeMap::<String, u64>::new();
    let mut shock_end = BTreeMap::<String, u64>::new();
    let mut option_rows = Vec::<Vec<String>>::new();
    let mut inspections = BTreeMap::<String, (String, u64, String)>::new();

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
                shock_end.insert(
                    fields[1].clone(),
                    fields[3].parse().expect("shock end is numeric"),
                );
            }
            "OPTION" => option_rows.push(fields),
            "INSPECTION" => {
                assert!(inspections
                    .insert(
                        fields[1].clone(),
                        (
                            fields[2].clone(),
                            fields[3].parse().expect("accepted units are numeric"),
                            fields[4].clone(),
                        ),
                    )
                    .is_none());
            }
            "INSTANCE" => {}
            other => panic!("unexpected fixture record: {other}"),
        }
    }

    let mut expected =
        BTreeMap::<String, (u64, u64, u64, u64, u64, String, String)>::new();
    for option in option_rows {
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
                .expect("option dependency has source availability");
            normal_ready = normal_ready.max(available);
            disrupted_ready =
                disrupted_ready.max(available.max(*shock_end.get(dependency).unwrap_or(&0)));
        }

        let duration = option[4].parse::<u64>().unwrap()
            + option[5].parse::<u64>().unwrap()
            + option[6].parse::<u64>().unwrap()
            + option[7].parse::<u64>().unwrap();
        let normal_day = normal_ready + duration;
        let disrupted_day = disrupted_ready + duration;
        let (status, reported_units, inspection_revision) = inspections
            .get(&option[1])
            .expect("every portfolio has an inspection record");
        assert_eq!(inspection_revision, &option[11]);
        assert!(reported_units <= &expected_units);
        let accepted_units = if status == "PASS" { *reported_units } else { 0 };
        let on_time_units = if disrupted_day <= deadline_day && status == "PASS" {
            *reported_units
        } else {
            0
        };
        expected.insert(
            option[1].clone(),
            (
                normal_day,
                disrupted_day,
                expected_units,
                accepted_units,
                on_time_units,
                status.clone(),
                option[11].clone(),
            ),
        );
    }

    let receipt = run_audit(FIXTURE).expect("the frozen fixture audits");
    let mut observed = BTreeMap::new();
    for line in receipt.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.first() == Some(&"OUTCOME") {
            let value = (
                fields[3].parse().expect("normal day is numeric"),
                fields[4].parse().expect("disrupted day is numeric"),
                fields[5].parse().expect("expected units are numeric"),
                fields[6].parse().expect("receiver-accepted units are numeric"),
                fields[7].parse().expect("on-time accepted units are numeric"),
                fields[8].to_owned(),
                fields[9].to_owned(),
            );
            assert!(observed.insert(fields[1].to_owned(), value).is_none());
        }
    }

    assert_eq!(observed, expected);
    assert_eq!(observed.len(), 5, "no portfolio may disappear from the receipt");
}
