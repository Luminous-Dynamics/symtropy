use std::env;
use std::fmt::Write as _;
use std::process::ExitCode;

use ors_skyhook_reference::evaluate_corpus;

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other if other.is_control() => output.push('?'),
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn receipt(source_commit: &str, corpus_sha256: &str, input: &str) -> Result<String, String> {
    let report = evaluate_corpus(input)?;
    let mut output = String::new();
    writeln!(&mut output, "{{").unwrap();
    writeln!(&mut output, "  \"receipt_schema_version\": \"0.1.0\",").unwrap();
    writeln!(&mut output, "  \"validator\": \"ors-skyhook-reference\",").unwrap();
    writeln!(
        &mut output,
        "  \"source_commit\": {},",
        json_string(source_commit)
    )
    .unwrap();
    writeln!(
        &mut output,
        "  \"corpus_sha256\": {},",
        json_string(corpus_sha256)
    )
    .unwrap();
    writeln!(&mut output, "  \"case_count\": {},", report.cases.len()).unwrap();
    writeln!(
        &mut output,
        "  \"table_reproduced_count\": {},",
        report.table_reproduced_count
    )
    .unwrap();
    writeln!(
        &mut output,
        "  \"outside_source_domain_count\": {},",
        report.outside_source_domain_count
    )
    .unwrap();
    writeln!(
        &mut output,
        "  \"max_relative_error\": {:.15e},",
        report.max_relative_error
    )
    .unwrap();
    writeln!(&mut output, "  \"result\": \"pass\",").unwrap();
    let claim_ceiling = concat!(
        "Source-arithmetic reproduction of Birch (1982) Eqs. 35/44/53/54 ",
        "and rounded Table 4 only; no physical qualification."
    );
    writeln!(
        &mut output,
        "  \"claim_ceiling\": {},",
        json_string(claim_ceiling)
    )
    .unwrap();
    writeln!(&mut output, "  \"cases\": [").unwrap();

    for (index, case) in report.cases.iter().enumerate() {
        let comma = if index + 1 == report.cases.len() {
            ""
        } else {
            ","
        };
        writeln!(&mut output, "    {{").unwrap();
        writeln!(
            &mut output,
            "      \"case_id\": {},",
            json_string(&case.case_id)
        )
        .unwrap();
        writeln!(
            &mut output,
            "      \"status\": {},",
            json_string(case.status.as_str())
        )
        .unwrap();
        writeln!(
            &mut output,
            "      \"turn_angle_rad\": {:.15e},",
            case.turn_angle_rad
        )
        .unwrap();
        writeln!(
            &mut output,
            "      \"turn_angle_limit_rad\": {:.15e},",
            case.turn_angle_limit_rad
        )
        .unwrap();
        match (
            case.orbital_speed_mps,
            case.predicted_force_n,
            case.table_force_n,
            case.relative_error,
        ) {
            (Some(speed), Some(force), Some(table_force), Some(error)) => {
                writeln!(&mut output, "      \"orbital_speed_mps\": {:.15e},", speed).unwrap();
                writeln!(&mut output, "      \"predicted_force_n\": {:.15e},", force).unwrap();
                writeln!(
                    &mut output,
                    "      \"table_force_n\": {:.15e},",
                    table_force
                )
                .unwrap();
                writeln!(&mut output, "      \"relative_error\": {:.15e}", error).unwrap();
            }
            _ => {
                writeln!(&mut output, "      \"orbital_speed_mps\": null,").unwrap();
                writeln!(&mut output, "      \"predicted_force_n\": null,").unwrap();
                writeln!(&mut output, "      \"table_force_n\": null,").unwrap();
                writeln!(&mut output, "      \"relative_error\": null").unwrap();
            }
        }
        write!(&mut output, "    }}{comma}").unwrap();
        output.push('\n');
    }
    writeln!(&mut output, "  ]").unwrap();
    writeln!(&mut output, "}}").unwrap();
    Ok(output)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 4 || args[0] != "--source-commit" || args[2] != "--corpus-sha256" {
        let usage = concat!(
            "usage: ors-skyhook-reference --source-commit ",
            "<40-char-sha> --corpus-sha256 <64-char-sha>"
        );
        return Err(usage.to_owned());
    }
    if !valid_hex(&args[1], 40) {
        return Err("source commit must be a 40-character lowercase hexadecimal SHA".to_owned());
    }
    if !valid_hex(&args[3], 64) {
        return Err(
            "corpus digest must be a 64-character lowercase hexadecimal SHA-256".to_owned(),
        );
    }

    let corpus = include_str!("../cases-v0.1.csv");
    let result = receipt(&args[1], &args[3], corpus)?;
    print!("{result}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ors-skyhook-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_a_deterministic_receipt_shape() {
        let corpus = include_str!("../cases-v0.1.csv");
        let first = receipt(&"a".repeat(40), &"b".repeat(64), corpus).expect("reference passes");
        let second = receipt(&"a".repeat(40), &"b".repeat(64), corpus).expect("reference passes");
        assert_eq!(first, second);
        assert!(first.contains("\"case_count\": 36"));
        assert!(first.contains("\"table_reproduced_count\": 28"));
        assert!(first.contains("\"outside_source_domain_count\": 8"));
    }

    #[test]
    fn emits_null_force_fields_for_source_domain_exclusions() {
        let corpus = include_str!("../cases-v0.1.csv");
        let output = receipt(&"a".repeat(40), &"b".repeat(64), corpus).expect("reference passes");
        assert!(output.contains("\"predicted_force_n\": null"));
        assert!(output.contains("\"table_force_n\": null"));
    }

    #[test]
    fn rejects_wrong_provenance_digest_lengths() {
        assert!(!valid_hex("abc", 40));
        assert!(!valid_hex(&"g".repeat(64), 64));
        assert!(valid_hex(&"0".repeat(64), 64));
    }

    #[test]
    fn json_strings_escape_control_and_quote_characters() {
        assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_string("a\\b"), "\"a\\\\b\"");
    }
}
