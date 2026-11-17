use std::env;
use std::fmt::Write as _;
use std::process::ExitCode;

use ors_angular_momentum_control_volume_reference::{CaseEvaluation, Vector3, evaluate_corpus};

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

fn write_vector(output: &mut String, key: &str, value: Vector3) {
    writeln!(
        output,
        "      \"{}\": {{\"x\": {:.15e}, \"y\": {:.15e}, \"z\": {:.15e}}},",
        key, value.x, value.y, value.z
    )
    .unwrap();
}

fn write_case(output: &mut String, case: &CaseEvaluation, is_last: bool) {
    writeln!(output, "    {{").unwrap();
    writeln!(output, "      \"case_id\": {},", json_string(&case.case_id)).unwrap();
    writeln!(output, "      \"frame_id\": {},", json_string(&case.frame_id)).unwrap();
    writeln!(output, "      \"origin_id\": {},", json_string(&case.origin_id)).unwrap();
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

    write_vector(
        output,
        "linear_momentum_flux_n",
        case.linear_momentum_flux_n,
    );
    write_vector(
        output,
        "required_external_force_n",
        case.required_external_force_n,
    );
    write_vector(
        output,
        "support_on_stream_force_n",
        case.support_on_stream_force_n,
    );
    write_vector(
        output,
        "stream_on_support_reaction_n",
        case.stream_on_support_reaction_n,
    );
    write_vector(
        output,
        "force_reaction_closure_residual_n",
        case.force_reaction_closure_residual_n,
    );

    write_vector(
        output,
        "angular_momentum_accumulation_n_m",
        case.angular_momentum_accumulation_n_m,
    );
    write_vector(
        output,
        "other_external_force_n",
        case.other_external_force_n,
    );
    write_vector(
        output,
        "other_external_torque_n_m",
        case.other_external_torque_n_m,
    );
    write_vector(
        output,
        "advective_angular_momentum_flux_n_m",
        case.advective_angular_momentum_flux_n_m,
    );
    write_vector(
        output,
        "required_external_torque_n_m",
        case.required_external_torque_n_m,
    );
    write_vector(
        output,
        "support_on_stream_torque_n_m",
        case.support_on_stream_torque_n_m,
    );
    write_vector(
        output,
        "stream_on_support_reaction_torque_n_m",
        case.stream_on_support_reaction_torque_n_m,
    );

    writeln!(
        output,
        "      \"torque_reaction_closure_residual_n_m\": {{\"x\": {:.15e}, \"y\": {:.15e}, \"z\": {:.15e}}}",
        case.torque_reaction_closure_residual_n_m.x,
        case.torque_reaction_closure_residual_n_m.y,
        case.torque_reaction_closure_residual_n_m.z
    )
    .unwrap();
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
        "  \"validator\": \"ors-angular-momentum-control-volume-reference\","
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
        "  \"max_abs_force_reaction_closure_residual_n\": {:.15e},",
        report.max_abs_force_reaction_closure_residual_n
    )
    .unwrap();
    writeln!(
        output,
        "  \"max_abs_torque_reaction_closure_residual_n_m\": {:.15e},",
        report.max_abs_torque_reaction_closure_residual_n_m
    )
    .unwrap();
    writeln!(output, "  \"result\": \"pass\",").unwrap();
    let claim_ceiling = concat!(
        "Exact advective angular-momentum bookkeeping about a fixed inertial origin only; ",
        "no physical qualification."
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
            "usage: ors-angular-momentum-control-volume-reference --source-commit <40-char-sha> ",
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
            eprintln!("ors-angular-momentum-control-volume-reference: {error}");
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
        .expect("angular-momentum reference evaluates");
        let second = receipt(
            &"a".repeat(40),
            &"b".repeat(64),
            &"c".repeat(64),
            cases,
            ports,
        )
        .expect("angular-momentum reference evaluates");
        assert_eq!(first, second);
        assert!(first.contains("\"case_count\": 6"));
        assert!(first.contains("\"port_count\": 14"));
        assert!(first.contains("\"origin_id\": \"O0\""));
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
