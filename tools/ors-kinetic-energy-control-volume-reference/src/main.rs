use std::env;
use std::fmt::Write as _;
use std::process::ExitCode;

use ors_kinetic_energy_control_volume_reference::{CaseEvaluation, FlowDirection, evaluate_corpus};

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
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

fn flow_direction(direction: FlowDirection) -> &'static str {
    match direction {
        FlowDirection::Inflow => "Inflow",
        FlowDirection::Outflow => "Outflow",
    }
}

fn write_case(output: &mut String, case: &CaseEvaluation, is_last: bool) {
    writeln!(output, "    {{").unwrap();
    writeln!(output, "      \"case_id\": {},", json_string(&case.case_id)).unwrap();
    writeln!(
        output,
        "      \"frame_id\": {},",
        json_string(&case.frame_id)
    )
    .unwrap();
    writeln!(
        output,
        "      \"boundary_id\": {},",
        json_string(&case.boundary_id)
    )
    .unwrap();
    writeln!(output, "      \"epoch_s\": {:.15e},", case.epoch_s).unwrap();
    writeln!(output, "      \"port_count\": {},", case.port_count).unwrap();
    writeln!(
        output,
        "      \"mass_accumulation_kg_s\": {:.15e},",
        case.mass_accumulation_kg_s
    )
    .unwrap();
    writeln!(
        output,
        "      \"mass_flow_in_kg_s\": {:.15e},",
        case.mass_flow_in_kg_s
    )
    .unwrap();
    writeln!(
        output,
        "      \"mass_flow_out_kg_s\": {:.15e},",
        case.mass_flow_out_kg_s
    )
    .unwrap();
    writeln!(
        output,
        "      \"mass_balance_residual_kg_s\": {:.15e},",
        case.mass_balance_residual_kg_s
    )
    .unwrap();
    writeln!(
        output,
        "      \"mass_balance_tolerance_kg_s\": {:.15e},",
        case.mass_balance_tolerance_kg_s
    )
    .unwrap();
    writeln!(
        output,
        "      \"kinetic_energy_accumulation_w\": {:.15e},",
        case.kinetic_energy_accumulation_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"port_kinetic_energy_flow_in_w\": {:.15e},",
        case.port_kinetic_energy_flow_in_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"port_kinetic_energy_flow_out_w\": {:.15e},",
        case.port_kinetic_energy_flow_out_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"net_advective_kinetic_energy_flux_w\": {:.15e},",
        case.net_advective_kinetic_energy_flux_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"nonport_external_power_on_cv_w\": {:.15e},",
        case.nonport_external_power_on_cv_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"boundary_work_on_cv_w\": {:.15e},",
        case.boundary_work_on_cv_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"mechanical_dissipation_rate_w\": {:.15e},",
        case.mechanical_dissipation_rate_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"net_declared_power_on_cv_w\": {:.15e},",
        case.net_declared_power_on_cv_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"power_balance_residual_w\": {:.15e},",
        case.power_balance_residual_w
    )
    .unwrap();
    writeln!(
        output,
        "      \"power_balance_tolerance_w\": {:.15e},",
        case.power_balance_tolerance_w
    )
    .unwrap();
    writeln!(output, "      \"port_contributions\": [").unwrap();
    for (index, port) in case.port_contributions.iter().enumerate() {
        let comma = if index + 1 == case.port_contributions.len() {
            ""
        } else {
            ","
        };
        writeln!(output, "        {{").unwrap();
        writeln!(
            output,
            "          \"port_id\": {},",
            json_string(&port.port_id)
        )
        .unwrap();
        writeln!(
            output,
            "          \"direction\": {},",
            json_string(flow_direction(port.direction))
        )
        .unwrap();
        writeln!(
            output,
            "          \"mass_flow_kg_s\": {:.15e},",
            port.mass_flow_kg_s
        )
        .unwrap();
        writeln!(
            output,
            "          \"specific_kinetic_energy_j_kg\": {:.15e},",
            port.specific_kinetic_energy_j_kg
        )
        .unwrap();
        writeln!(
            output,
            "          \"signed_kinetic_energy_flow_w\": {:.15e}",
            port.signed_kinetic_energy_flow_w
        )
        .unwrap();
        writeln!(output, "        }}{comma}").unwrap();
    }
    writeln!(output, "      ]").unwrap();
    if is_last {
        writeln!(output, "    }}").unwrap();
    } else {
        writeln!(output, "    }},").unwrap();
    }
}

fn receipt(
    source_commit: &str,
    cases_sha256: &str,
    ports_sha256: &str,
    cases_csv: &str,
    ports_csv: &str,
) -> Result<String, String> {
    let report = evaluate_corpus(cases_csv, ports_csv)?;
    let mut output = String::new();
    writeln!(output, "{{").unwrap();
    writeln!(output, "  \"receipt_schema_version\": \"0.1.0\",").unwrap();
    writeln!(
        output,
        "  \"validator\": \"ors-kinetic-energy-control-volume-reference\","
    )
    .unwrap();
    writeln!(
        output,
        "  \"source_commit\": {},",
        json_string(source_commit)
    )
    .unwrap();
    writeln!(
        output,
        "  \"cases_corpus_sha256\": {},",
        json_string(cases_sha256)
    )
    .unwrap();
    writeln!(
        output,
        "  \"ports_corpus_sha256\": {},",
        json_string(ports_sha256)
    )
    .unwrap();
    writeln!(output, "  \"case_count\": {},", report.case_count).unwrap();
    writeln!(output, "  \"port_count\": {},", report.port_count).unwrap();
    writeln!(
        output,
        "  \"max_abs_mass_balance_residual_kg_s\": {:.15e},",
        report.max_abs_mass_balance_residual_kg_s
    )
    .unwrap();
    writeln!(
        output,
        "  \"max_abs_power_balance_residual_w\": {:.15e},",
        report.max_abs_power_balance_residual_w
    )
    .unwrap();
    writeln!(output, "  \"result\": \"pass\",").unwrap();
    let claim_ceiling = concat!(
        "Kinetic-energy storage and advective kinetic-energy flux only; ",
        "no total-energy or physical qualification."
    );
    writeln!(
        output,
        "  \"claim_ceiling\": {},",
        json_string(claim_ceiling)
    )
    .unwrap();
    writeln!(output, "  \"cases\": [").unwrap();
    for (index, case) in report.cases.iter().enumerate() {
        write_case(&mut output, case, index + 1 == report.cases.len());
    }
    writeln!(output, "  ]").unwrap();
    writeln!(output, "}}").unwrap();
    Ok(output)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 6
        || args[0] != "--source-commit"
        || args[2] != "--cases-sha256"
        || args[4] != "--ports-sha256"
    {
        let usage = concat!(
            "usage: ors-kinetic-energy-control-volume-reference --source-commit <40-char-sha> ",
            "--cases-sha256 <64-char-sha> --ports-sha256 <64-char-sha>"
        );
        return Err(usage.to_owned());
    }
    if !valid_hex(&args[1], 40) {
        return Err("source commit must be a 40-character lowercase hexadecimal SHA".to_owned());
    }
    if !valid_hex(&args[3], 64) || !valid_hex(&args[5], 64) {
        return Err(
            "corpus digests must be 64-character lowercase hexadecimal SHA-256 values".to_owned(),
        );
    }

    let cases_csv = include_str!("../cases-v0.1.csv");
    let ports_csv = include_str!("../ports-v0.1.csv");
    let output = receipt(&args[1], &args[3], &args[5], cases_csv, ports_csv)?;
    print!("{output}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ors-kinetic-energy-control-volume-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_is_deterministic_and_binds_both_corpora() {
        let cases = include_str!("../cases-v0.1.csv");
        let ports = include_str!("../ports-v0.1.csv");
        let first = receipt(
            &"a".repeat(40),
            &"b".repeat(64),
            &"c".repeat(64),
            cases,
            ports,
        )
        .expect("kinetic-energy reference evaluates");
        let second = receipt(
            &"a".repeat(40),
            &"b".repeat(64),
            &"c".repeat(64),
            cases,
            ports,
        )
        .expect("kinetic-energy reference evaluates");
        assert_eq!(first, second);
        assert!(first.contains("\"case_count\": 6"));
        assert!(first.contains("\"port_count\": 14"));
        assert!(first.contains("\"boundary_id\": \"CV0\""));
        assert!(first.contains("\"result\": \"pass\""));
    }

    #[test]
    fn rejects_invalid_source_and_corpus_digests() {
        assert!(!valid_hex("abc", 40));
        assert!(!valid_hex(&"g".repeat(64), 64));
        assert!(valid_hex(&"0".repeat(64), 64));
    }

    #[test]
    fn json_strings_escape_quotes_backslashes_and_control_characters() {
        assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(json_string("a\\b"), "\"a\\\\b\"");
        assert_eq!(json_string("a\nb"), "\"a\\nb\"");
    }
}
