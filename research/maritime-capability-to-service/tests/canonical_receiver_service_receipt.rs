use std::collections::BTreeMap;

const RECEIPT: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/symtropy-receiver-service-v0.receipt.tsv"
);

#[derive(Clone, Debug, Eq, PartialEq)]
struct Cohort {
    cohort_id: String,
    unit: String,
    scheduled: u64,
    adjudicated: u64,
    on_time: u64,
    late: u64,
    not_accepted: u64,
    lost: u64,
    unresolved: u64,
    reconciliation_failure: u64,
    observed_numerator: u64,
    observed_denominator: u64,
    lower_numerator: u64,
    lower_denominator: u64,
    upper_numerator: u64,
    upper_denominator: u64,
    floor: Option<u16>,
    disposition: String,
}

fn parse_receipt(input: &str) -> Result<(BTreeMap<String, (usize, usize)>, BTreeMap<(String, String, String), Cohort>), String> {
    let mut lines = input.lines();
    if lines.next() != Some("RECEIVER_SERVICE_RECEIPT\t0.1") {
        return Err("unsupported receiver-service receipt header".to_owned());
    }
    if lines.next() != Some("CASE\tcase_id\tunit_count\tcohort_count") {
        return Err("missing receiver-service receipt column header".to_owned());
    }
    let mut cases = BTreeMap::new();
    let mut cohorts = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied().unwrap_or_default() {
            "CASE" if fields.len() == 4 => {
                let unit_count = fields[2]
                    .parse::<usize>()
                    .map_err(|_| format!("line {}: invalid unit count", index + 3))?;
                let cohort_count = fields[3]
                    .parse::<usize>()
                    .map_err(|_| format!("line {}: invalid cohort count", index + 3))?;
                if cases
                    .insert(fields[1].to_owned(), (unit_count, cohort_count))
                    .is_some()
                {
                    return Err(format!("line {}: duplicate case", index + 3));
                }
            }
            "COHORT" if fields.len() == 20 => {
                let n = |i: usize, name: &str| {
                    fields[i]
                        .parse::<u64>()
                        .map_err(|_| format!("line {}: invalid {name}", index + 3))
                };
                let floor = if fields[18] == "-" {
                    None
                } else {
                    Some(
                        fields[18]
                            .parse::<u16>()
                            .map_err(|_| format!("line {}: invalid floor", index + 3))?,
                    )
                };
                let row = Cohort {
                    cohort_id: fields[2].to_owned(),
                    unit: fields[3].to_owned(),
                    scheduled: n(4, "scheduled")?,
                    adjudicated: n(5, "adjudicated")?,
                    on_time: n(6, "on time")?,
                    late: n(7, "late")?,
                    not_accepted: n(8, "not accepted")?,
                    lost: n(9, "lost")?,
                    unresolved: n(10, "unresolved")?,
                    reconciliation_failure: n(11, "reconciliation failure")?,
                    observed_numerator: n(12, "observed numerator")?,
                    observed_denominator: n(13, "observed denominator")?,
                    lower_numerator: n(14, "lower numerator")?,
                    lower_denominator: n(15, "lower denominator")?,
                    upper_numerator: n(16, "upper numerator")?,
                    upper_denominator: n(17, "upper denominator")?,
                    floor,
                    disposition: fields[19].to_owned(),
                };
                if !cases.contains_key(fields[1]) {
                    return Err(format!("line {}: cohort precedes case", index + 3));
                }
                let key = (
                    fields[1].to_owned(),
                    row.cohort_id.clone(),
                    row.unit.clone(),
                );
                if cohorts.insert(key, row).is_some() {
                    return Err(format!("line {}: duplicate cohort/unit result", index + 3));
                }
            }
            "SUMMARY" if fields.len() == 3 => {
                if fields[1] != "PASS" || fields[2] != "10/10" {
                    return Err("receiver-service owner summary did not pass 10/10".to_owned());
                }
            }
            other => return Err(format!("line {}: unknown/malformed receipt row {other}", index + 3)),
        }
    }

    for (case_id, (_, expected_cohorts)) in &cases {
        let actual = cohorts.keys().filter(|key| &key.0 == case_id).count();
        if actual != *expected_cohorts {
            return Err(format!(
                "case {case_id} declared {expected_cohorts} cohorts but emitted {actual}"
            ));
        }
    }
    if cases.len() != 10 || cohorts.len() != 11 {
        return Err(format!(
            "expected 10 cases/11 cohorts, found {}/{}",
            cases.len(),
            cohorts.len()
        ));
    }

    // Verify summary arithmetic and the conservative floor classification.
    // Unit-level adjudication remains the canonical #7554 implementation.
    for ((case_id, _, _), row) in &cohorts {
        if row.adjudicated + row.unresolved + row.reconciliation_failure != row.scheduled {
            return Err(format!("{case_id}: adjudicated + unresolved + reconciliation must equal scheduled"));
        }
        if row.on_time + row.late + row.not_accepted + row.lost != row.adjudicated {
            return Err(format!("{case_id}: adjudicated outcome categories do not reconcile"));
        }
        if row.observed_numerator != row.on_time
            || row.observed_denominator != row.adjudicated
            || row.lower_numerator != row.on_time
            || row.lower_denominator != row.scheduled
            || row.upper_numerator != row.on_time + row.unresolved
            || row.upper_denominator != row.scheduled
        {
            return Err(format!("{case_id}: observed or conservative service bounds do not reconcile"));
        }
        let expected_disposition = match row.floor {
            None => "NOT_CONFIGURED",
            Some(_) if row.reconciliation_failure > 0 => "INVALID_EVIDENCE",
            Some(floor) => {
                let lower_passes =
                    u128::from(row.lower_numerator) * 10_000
                        >= u128::from(floor) * u128::from(row.lower_denominator);
                let upper_fails =
                    u128::from(row.upper_numerator) * 10_000
                        < u128::from(floor) * u128::from(row.upper_denominator);
                if lower_passes {
                    "PASS"
                } else if upper_fails {
                    "FAIL"
                } else {
                    "UNKNOWN"
                }
            }
        };
        if row.disposition != expected_disposition {
            return Err(format!(
                "{case_id}: floor disposition {} does not match conservative bounds ({expected_disposition})",
                row.disposition
            ));
        }
    }
    Ok((cases, cohorts))
}

#[test]
fn canonical_receiver_service_receipt_preserves_unknown_and_protected_floor_bounds() {
    let (cases, cohorts) = parse_receipt(RECEIPT).expect("owner receipt parses");
    assert_eq!(cases.len(), 10);
    assert_eq!(cohorts.len(), 11);

    let accepted = &cohorts[&(
        "S00_ACCEPTED_ON_TIME".to_owned(),
        "essential".to_owned(),
        "container".to_owned(),
    )];
    assert_eq!((accepted.scheduled, accepted.adjudicated, accepted.on_time), (2, 2, 2));
    assert_eq!(accepted.disposition, "PASS");

    let terminal_only = &cohorts[&(
        "S01_TERMINAL_EXIT_ONLY".to_owned(),
        "essential".to_owned(),
        "container".to_owned(),
    )];
    assert_eq!((terminal_only.scheduled, terminal_only.unresolved), (1, 1));
    assert_eq!(terminal_only.lower_numerator, 0);
    assert_eq!(terminal_only.upper_numerator, 1);
    assert_eq!(terminal_only.disposition, "UNKNOWN");

    let late = &cohorts[&(
        "S02_ACCEPTED_LATE".to_owned(),
        "essential".to_owned(),
        "container".to_owned(),
    )];
    assert_eq!((late.scheduled, late.adjudicated, late.late, late.on_time), (1, 1, 1, 0));
    assert_eq!(late.disposition, "FAIL");

    let censored = &cohorts[&(
        "S03_CENSORED_BOUNDS".to_owned(),
        "essential".to_owned(),
        "container".to_owned(),
    )];
    assert_eq!((censored.scheduled, censored.adjudicated, censored.unresolved), (10, 6, 4));
    assert_eq!((censored.lower_numerator, censored.lower_denominator), (6, 10));
    assert_eq!((censored.upper_numerator, censored.upper_denominator), (10, 10));
    assert_eq!(censored.disposition, "UNKNOWN");

    let container = &cohorts[&(
        "S07_UNIT_TYPES_SEPARATE".to_owned(),
        "mixed".to_owned(),
        "container".to_owned(),
    )];
    let teu = &cohorts[&(
        "S07_UNIT_TYPES_SEPARATE".to_owned(),
        "mixed".to_owned(),
        "TEU".to_owned(),
    )];
    assert_ne!(container.unit, teu.unit);

    let invalid = &cohorts[&(
        "S08_RECONCILIATION_FAILURE".to_owned(),
        "protected".to_owned(),
        "container".to_owned(),
    )];
    assert_eq!(invalid.disposition, "INVALID_EVIDENCE");
    assert_eq!(invalid.reconciliation_failure, 1);
}

#[test]
fn contradictory_receiver_service_summary_is_rejected() {
    let original = "COHORT\\tS01_TERMINAL_EXIT_ONLY\\tessential\\tcontainer\\t1\\t0\\t0\\t0\\t0\\t0\\t1\\t0\\t0\\t0\\t0\\t1\\t1\\t1\\t10000\\tUNKNOWN";
    let contradictory = "COHORT\\tS01_TERMINAL_EXIT_ONLY\\tessential\\tcontainer\\t1\\t0\\t1\\t0\\t0\\t0\\t1\\t0\\t1\\t0\\t0\\t1\\t1\\t1\\t10000\\tUNKNOWN";
    let invalid = RECEIPT.replace(original, contradictory);
    assert!(
        parse_receipt(&invalid)
            .unwrap_err()
            .contains("adjudicated outcome categories do not reconcile")
    );
}
