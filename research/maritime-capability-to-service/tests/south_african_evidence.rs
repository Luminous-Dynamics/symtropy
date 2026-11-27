use std::collections::BTreeMap;

const EVIDENCE: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/south-african-port-energy-evidence-v0.tsv"
);

#[derive(Debug)]
struct EvidenceRow<'a> {
    source_id: &'a str,
    site: &'a str,
    period: &'a str,
    metric: &'a str,
    value: &'a str,
    unit: &'a str,
    evidence_class: &'a str,
    claim_ceiling: &'a str,
}

fn rows(input: &str) -> Vec<EvidenceRow<'_>> {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                8,
                "every public evidence row has eight fields"
            );
            EvidenceRow {
                source_id: fields[0],
                site: fields[1],
                period: fields[2],
                metric: fields[3],
                value: fields[4],
                unit: fields[5],
                evidence_class: fields[6],
                claim_ceiling: fields[7],
            }
        })
        .collect()
}

fn validate(rows: &[EvidenceRow<'_>]) -> Result<(), String> {
    if rows.len() != 36 {
        return Err(format!("expected 36 source rows, found {}", rows.len()));
    }

    for row in rows {
        match row.source_id {
            "LOAD_CONNECTION_2025" => {
                if !matches!(row.unit, "MVA" | "category") {
                    return Err(format!(
                        "connection metric {} at {} must remain MVA/category, not {}",
                        row.metric, row.site, row.unit
                    ));
                }
                if row.evidence_class != "PRE_FEASIBILITY_DESKTOP_ASSESSMENT" {
                    return Err(format!(
                        "connection metric {} at {} lost its pre-feasibility evidence class",
                        row.metric, row.site
                    ));
                }
                if row.metric == "ESTIMATED_SPARE_CAPACITY"
                    && !row.claim_ceiling.contains("Subject to supply authority")
                {
                    return Err(format!(
                        "spare capacity at {} must retain utility-confirmation caveat",
                        row.site
                    ));
                }
            }
            "SHORE_POWER_DEMAND_2025" => {
                if row.unit != "MW" {
                    return Err(format!(
                        "shore-power estimate {} at {} must remain MW, not {}",
                        row.metric, row.site, row.unit
                    ));
                }
                if row.value == "UNKNOWN" {
                    if row.evidence_class != "NOT_REPORTED_FOR_PORT_PERIOD"
                        || !row.claim_ceiling.contains("Do not treat dash as zero")
                    {
                        return Err(format!(
                            "unknown period {} at {} must retain NOT_REPORTED state",
                            row.period, row.site
                        ));
                    }
                } else {
                    if row.evidence_class != "ESTIMATED_FROM_TNPA_VESSEL_LOGS_AND_REFERENCE_POWER"
                        || row.claim_ceiling.contains("Do not treat dash as zero")
                        || !row
                            .claim_ceiling
                            .contains("Not an interval-metered demand observation")
                    {
                        return Err(format!(
                            "shore-power value {} at {} must preserve its model-estimate claim ceiling",
                            row.period, row.site
                        ));
                    }
                    row.value
                        .parse::<f64>()
                        .map_err(|_| format!("non-numeric shore-power value {}", row.value))?;
                }
            }
            other => return Err(format!("unrecognized source identifier {other}")),
        }
    }
    Ok(())
}

fn exactly_one<'a>(
    rows: &'a [EvidenceRow<'a>],
    source: &str,
    site: &str,
    period: &str,
    metric: &str,
) -> &'a EvidenceRow<'a> {
    let found: Vec<&EvidenceRow<'a>> = rows
        .iter()
        .filter(|row| {
            row.source_id == source
                && row.site == site
                && row.period == period
                && row.metric == metric
        })
        .collect();
    assert_eq!(found.len(), 1, "expected exactly one source metric row");
    found[0]
}

#[test]
fn source_evidence_preserves_units_statuses_and_claim_ceilings() {
    let evidence = rows(EVIDENCE);
    validate(&evidence).expect("source evidence passes its typed claim contract");

    let mut counts = BTreeMap::<(&str, &str), usize>::new();
    for row in &evidence {
        *counts.entry((row.source_id, row.unit)).or_default() += 1;
    }
    assert_eq!(counts.get(&("LOAD_CONNECTION_2025", "MVA")), Some(&6));
    assert_eq!(counts.get(&("LOAD_CONNECTION_2025", "category")), Some(&2));
    assert_eq!(counts.get(&("SHORE_POWER_DEMAND_2025", "MW")), Some(&28));

    let ngqura_spare = exactly_one(
        &evidence,
        "LOAD_CONNECTION_2025",
        "NGQURA",
        "2025_STUDY",
        "ESTIMATED_SPARE_CAPACITY",
    );
    assert_eq!((ngqura_spare.value, ngqura_spare.unit), ("19", "MVA"));
    assert_eq!(
        ngqura_spare.evidence_class,
        "PRE_FEASIBILITY_DESKTOP_ASSESSMENT"
    );

    let durban_supply = exactly_one(
        &evidence,
        "LOAD_CONNECTION_2025",
        "DURBAN",
        "2025_STUDY_FIRM_SUPPLY",
        "FIRM_SUPPLY",
    );
    let durban_nmd = exactly_one(
        &evidence,
        "LOAD_CONNECTION_2025",
        "DURBAN",
        "2025_STUDY_NMD",
        "NOTIFIED_MAXIMUM_DEMAND",
    );
    let durban_spare = exactly_one(
        &evidence,
        "LOAD_CONNECTION_2025",
        "DURBAN",
        "2025_STUDY",
        "ESTIMATED_SPARE_CAPACITY",
    );
    assert_eq!((durban_supply.value, durban_supply.unit), ("17", "MVA"));
    assert_eq!((durban_nmd.value, durban_nmd.unit), ("13", "MVA"));
    assert_eq!((durban_spare.value, durban_spare.unit), ("4", "MVA"));

    let durban_peak = exactly_one(
        &evidence,
        "SHORE_POWER_DEMAND_2025",
        "DURBAN",
        "2018-2019",
        "ABSOLUTE_PEAK_UTILIZATION_MAX",
    );
    assert_eq!((durban_peak.value, durban_peak.unit), ("47", "MW"));
    let ngqura_peak = exactly_one(
        &evidence,
        "SHORE_POWER_DEMAND_2025",
        "NGQURA",
        "2024-2025",
        "ABSOLUTE_PEAK_UTILIZATION_MAX",
    );
    assert_eq!(ngqura_peak.value, "UNKNOWN");
    assert_eq!(ngqura_peak.evidence_class, "NOT_REPORTED_FOR_PORT_PERIOD");
}

#[test]
fn mva_to_mw_promotion_is_rejected_without_a_source_power_factor_contract() {
    let invalid = EVIDENCE.replace(
        "ESTIMATED_SPARE_CAPACITY\t19\tMVA",
        "ESTIMATED_SPARE_CAPACITY\t17.1\tMW",
    );
    let parsed = rows(&invalid);
    assert!(
        validate(&parsed)
            .unwrap_err()
            .contains("must remain MVA/category")
    );
}

#[test]
fn missing_shore_power_periods_cannot_be_relabelled_as_zero() {
    let invalid = EVIDENCE.replace(
        "UNKNOWN\tMW\tNOT_REPORTED_FOR_PORT_PERIOD",
        "0\tMW\tESTIMATED_FROM_TNPA_VESSEL_LOGS_AND_REFERENCE_POWER",
    );
    let parsed = rows(&invalid);
    assert!(
        validate(&parsed).is_err(),
        "replacing explicit unknown with zero must violate frozen source cardinality"
    );
}
