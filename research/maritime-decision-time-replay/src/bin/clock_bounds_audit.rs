use maritime_decision_time_replay::clock_bounds::{
    qualify_clock_bounds, ClockCutoffs, ClockEvidence, TimeInterval,
};
use std::error::Error;

const FIXTURE: &str = include_str!("../../fixtures/clock-cases.tsv");

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

fn parse_interval(
    fields: &[&str],
    lower_index: usize,
    name: &str,
) -> Result<TimeInterval, Box<dyn Error>> {
    let lower = parse_i64(fields[lower_index], name)?;
    let upper = parse_i64(fields[lower_index + 1], name)?;
    TimeInterval::new(lower, upper).map_err(Into::into)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut output = Vec::new();
    let mut count = 0_usize;

    for (line_number, raw_line) in FIXTURE.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id\t") {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 20 {
            return Err(format!(
                "fixture line {}: expected 20 fields, got {}",
                line_number + 1,
                fields.len()
            )
            .into());
        }

        let computed = (|| -> Result<(String, String), Box<dyn Error>> {
            let physical_at = parse_interval(&fields, 1, "physical_at")?;
            let decision_at = parse_interval(&fields, 3, "decision_at")?;
            let evidence = ClockEvidence {
                valid_from: parse_interval(&fields, 5, "valid_from")?,
                valid_until: parse_interval(&fields, 7, "valid_until")?,
                published_at: parse_interval(&fields, 9, "published_at")?,
                retrieved_at: parse_interval(&fields, 11, "retrieved_at")?,
                rights_clear: parse_bool(fields[13], "rights_clear")?,
                rights_valid_from: parse_interval(&fields, 14, "rights_valid_from")?,
                rights_valid_until: parse_interval(&fields, 16, "rights_valid_until")?,
            };
            let result = qualify_clock_bounds(
                &evidence,
                ClockCutoffs {
                    physical_at,
                    decision_at,
                },
            );
            let reasons = result.reasons.join("|");
            Ok((result.admission.as_str().to_owned(), reasons))
        })();

        let (admission, reasons) = match computed {
            Ok(value) => value,
            Err(_) => ("INVALID_INTERVAL".to_owned(), "INVALID_INTERVAL".to_owned()),
        };

        if admission != fields[18] || reasons != fields[19] {
            return Err(format!(
                "{}: expected {}/{}, got {}/{}",
                fields[0], fields[18], fields[19], admission, reasons
            )
            .into());
        }
        output.push((fields[0].to_owned(), admission, reasons));
        count += 1;
    }

    if count == 0 {
        return Err("empty clock uncertainty fixture".into());
    }

    println!("MDTR_CLOCK_RECEIPT\t0.1");
    println!("case_id\tadmission\treasons");
    for (case_id, admission, reasons) in output {
        println!("{case_id}\t{admission}\t{reasons}");
    }
    println!("SUMMARY\tPASS\t{count}/{count}");
    Ok(())
}
