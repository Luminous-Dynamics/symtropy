const SNAPSHOT: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/busa-cargo-movement-update-306-snapshot-v0.tsv"
);

#[test]
fn snapshot_has_exact_eight_dwell_and_seven_weekly_flow_rows() {
    let mut lines = SNAPSHOT.lines();
    let header = lines.next().expect("header");
    let columns: Vec<&str> = header.split('\t').collect();
    assert_eq!(columns.len(), 15);
    assert_eq!(columns[0], "record_type");
    assert_eq!(columns[10], "report_date");
    assert_eq!(columns[11], "source_updated_date");

    let rows: Vec<Vec<&str>> = lines.map(|line| line.split('\t').collect()).collect();
    assert_eq!(rows.len(), 15, "all published rows remain present");
    assert!(rows.iter().all(|row| row.len() == columns.len()));

    let dwell: Vec<&Vec<&str>> = rows.iter().filter(|row| row[0] == "vessel_dwell").collect();
    let flow: Vec<&Vec<&str>> = rows.iter().filter(|row| row[0] == "weekly_flow").collect();
    assert_eq!(dwell.len(), 8);
    assert_eq!(flow.len(), 7);

    for row in &dwell {
        assert_eq!(row[1], "", "dwell observation window is not specified");
        assert_eq!(row[2], "", "dwell observation window is not specified");
        assert_eq!(row[10], "2026-11-01");
        assert_eq!(row[11], "2026-11-03");
        assert_eq!(row[12], "MarineTraffic AIS");
        assert_eq!(row[13], "PUBLISHED_AGGREGATE / AIS_DERIVED");
    }

    for row in &flow {
        assert_eq!(row[1], "2026-10-26");
        assert_eq!(row[2], "2026-11-01");
        assert_eq!(row[10], "2026-11-01");
        assert_eq!(row[11], "2026-11-01");
        assert_eq!(row[13], "PUBLISHED_AGGREGATE / OPERATOR_DERIVED");
    }
}

#[test]
fn weekly_teu_total_is_reproduced_without_reallocating_other_or_averages() {
    let records: Vec<Vec<&str>> = SNAPSHOT
        .lines()
        .skip(1)
        .map(|line| line.split('\t').collect())
        .filter(|row: &Vec<&str>| row[0] == "weekly_flow")
        .collect();

    let terminal_rows: Vec<&Vec<&str>> = records
        .iter()
        .filter(|row| row[3] != "South Africa")
        .collect();
    let weekly_terminal_total: u64 = terminal_rows
        .iter()
        .map(|row| row[8].parse::<u64>().expect("weekly terminal TEU total"))
        .sum();
    assert_eq!(weekly_terminal_total, 100_423);

    let terminal_daily_average_sum: u64 = terminal_rows
        .iter()
        .map(|row| row[7].parse::<u64>().expect("terminal daily average TEU"))
        .sum();
    let national_row = records
        .iter()
        .find(|row| row[3] == "South Africa")
        .expect("national total row");
    let published_total_daily_average = national_row[7]
        .parse::<u64>()
        .expect("national daily average");

    // Independently rounded terminal averages sum to 14,347; the published
    // national total is 14,346. Preserve both source values; do not "fix" either.
    assert_eq!(terminal_daily_average_sum, 14_347);
    assert_eq!(published_total_daily_average, 14_346);

    let national_weekly_total = national_row[8]
        .parse::<u64>()
        .expect("national weekly total");
    assert_eq!(national_weekly_total, 100_423);
}

#[test]
fn terminal_dwell_known_answers_and_asynchronous_dates_are_preserved() {
    let records: Vec<Vec<&str>> = SNAPSHOT
        .lines()
        .skip(1)
        .map(|line| line.split('\t').collect())
        .filter(|row: &Vec<&str>| row[0] == "vessel_dwell")
        .collect();

    let expected = [
        ("CTCT", 0.2, 1.8),
        ("FPT", 0.1, 3.2),
        ("MPT", 4.9, 4.1),
        ("DGT", 10.3, 4.9),
        ("Pier 1", 5.4, 3.3),
        ("MPT", 0.1, 2.0),
        ("NCT", 1.8, 2.6),
        ("PECT", 0.7, 1.5),
    ];

    for (terminal, anchorage, berth) in expected {
        assert!(records.iter().any(|row| {
            row[4] == terminal
                && row[5].parse::<f64>().ok() == Some(anchorage)
                && row[6].parse::<f64>().ok() == Some(berth)
        }));
    }

    assert!(
        records
            .iter()
            .all(|row| { row[10] == "2026-11-01" && row[11] == "2026-11-03" })
    );
}
