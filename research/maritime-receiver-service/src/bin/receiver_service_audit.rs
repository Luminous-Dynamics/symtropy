use maritime_receiver_service::{
    evaluate_receiver_service, CargoUnitOutcome, OutcomeEvidence, ProtectedServiceFloor,
};
use std::collections::BTreeMap;
use std::error::Error;

const FIXTURE: &str = include_str!("../../fixtures/outcomes.tsv");

#[derive(Default)]
struct Case {
    horizon: Option<i64>,
    units: Vec<CargoUnitOutcome>,
    floors: Vec<ProtectedServiceFloor>,
}

fn parse_u64(value: &str, field: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_i64(value: &str, field: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("invalid {field}: {error}").into())
}

fn parse_bool(value: &str, field: &str) -> Result<bool, Box<dyn Error>> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("invalid {field}: {value}").into()),
    }
}

fn parse_outcome(
    kind: &str,
    time: &str,
    verified: &str,
    reason: &str,
) -> Result<OutcomeEvidence, Box<dyn Error>> {
    Ok(match kind {
        "ACCEPTED" => OutcomeEvidence::ReceiverAccepted {
            accepted_at: parse_i64(time, "accepted_at")?,
            independently_verified: parse_bool(verified, "accepted_verified")?,
        },
        "NOT_ACCEPTED" => OutcomeEvidence::NotAcceptedByHorizon {
            independently_verified: parse_bool(verified, "not_accepted_verified")?,
        },
        "TERMINAL_EXIT" => OutcomeEvidence::TerminalExitOnly {
            terminal_exit_at: parse_i64(time, "terminal_exit_at")?,
        },
        "CENSORED" => OutcomeEvidence::Censored {
            last_observed_at: parse_i64(time, "last_observed_at")?,
        },
        "DISPUTED" => OutcomeEvidence::Disputed,
        "LOST" => OutcomeEvidence::LostOrDamaged {
            observed_at: parse_i64(time, "lost_observed_at")?,
            independently_verified: parse_bool(verified, "lost_verified")?,
        },
        "RECON_FAILURE" => OutcomeEvidence::UnitReconciliationFailure {
            reason: reason.to_owned(),
        },
        other => return Err(format!("unknown outcome kind: {other}").into()),
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut cases: BTreeMap<String, Case> = BTreeMap::new();

    for (line_number, raw_line) in FIXTURE.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied() {
            Some("CASE") if fields.len() == 3 => {
                let case = cases.entry(fields[1].to_owned()).or_default();
                if case.horizon.is_some() {
                    return Err(format!("line {}: duplicate case header", line_number + 1).into());
                }
                case.horizon = Some(parse_i64(fields[2], "horizon")?);
            }
            Some("UNIT") if fields.len() == 11 => {
                let case = cases.entry(fields[1].to_owned()).or_default();
                let evidence = parse_outcome(fields[7], fields[8], fields[9], fields[10])?;
                case.units.push(CargoUnitOutcome {
                    unit_id: fields[2].to_owned(),
                    cohort_id: fields[3].to_owned(),
                    quantity: parse_u64(fields[4], "quantity")?,
                    unit: fields[5].to_owned(),
                    deadline_at: parse_i64(fields[6], "deadline")?,
                    evidence,
                });
            }
            Some("FLOOR") if fields.len() == 5 => {
                let case = cases.entry(fields[1].to_owned()).or_default();
                case.floors.push(ProtectedServiceFloor {
                    cohort_id: fields[2].to_owned(),
                    unit: fields[3].to_owned(),
                    minimum_basis_points: u16::try_from(parse_u64(
                        fields[4],
                        "minimum_basis_points",
                    )?)?,
                });
            }
            Some(kind) => {
                return Err(
                    format!("line {}: unknown/malformed record {kind}", line_number + 1).into(),
                );
            }
            None => return Err(format!("line {}: empty row", line_number + 1).into()),
        }
    }

    println!("RECEIVER_SERVICE_RECEIPT\t0.1");
    println!("CASE\tcase_id\tunit_count\tcohort_count");

    for (case_id, case) in &cases {
        let horizon = case
            .horizon
            .ok_or_else(|| format!("{case_id}: missing CASE header"))?;
        let report = evaluate_receiver_service(&case.units, &case.floors, horizon)?;
        println!(
            "CASE\t{}\t{}\t{}",
            case_id,
            report.unit_count,
            report.cohorts.len()
        );
        for cohort in report.cohorts {
            println!(
                "COHORT\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                case_id,
                cohort.cohort_id,
                cohort.unit,
                cohort.scheduled_quantity,
                cohort.adjudicated_quantity,
                cohort.on_time_quantity,
                cohort.late_quantity,
                cohort.not_accepted_quantity,
                cohort.lost_or_damaged_quantity,
                cohort.unresolved_quantity,
                cohort.reconciliation_failure_quantity,
                cohort.on_time_observed_numerator,
                cohort.on_time_observed_denominator,
                cohort.conservative_lower_numerator,
                cohort.conservative_lower_denominator,
                cohort.best_case_upper_numerator,
                cohort.best_case_upper_denominator,
                cohort.floor_basis_points.map_or_else(|| "-".to_owned(), |value| value.to_string()),
                cohort.floor_disposition.as_str()
            );
        }
    }
    println!("SUMMARY\tPASS\t{}/{}", cases.len(), cases.len());
    Ok(())
}
