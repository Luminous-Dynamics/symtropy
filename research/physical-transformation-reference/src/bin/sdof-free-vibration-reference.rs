// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic analytical/Newmark receipt for undamped and 5%-
//! damped SDOF free vibration. This is a synthetic verification fixture, not
//! a claim that the full SOFiSTiK BE23/BE24 input decks have been reproduced.

use physical_transformation_reference::sdof_dynamics::{
    SdofSystem, canonical_request, simulate_free_vibration,
};
use std::env;
use std::process::ExitCode;

const FIXTURE: &str =
    include_str!("../../fixtures/sdof_free_vibration_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sdof-free-vibration-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let request = canonical_request();
    if request != FIXTURE.trim_end_matches('\n') {
        return Err("canonical SDOF request differs from frozen fixture".to_owned());
    }

    let cases = [
        (
            "undamped",
            SdofSystem {
                mass_kg: 2.0,
                stiffness_n_per_m: 800.0,
                damping_n_s_per_m: 0.0,
                initial_displacement_m: 0.1,
                initial_velocity_m_per_s: 0.0,
            },
        ),
        (
            "underdamped",
            SdofSystem {
                mass_kg: 2.0,
                stiffness_n_per_m: 800.0,
                damping_n_s_per_m: 2.0 * 0.05 * (2.0_f64 * 800.0).sqrt(),
                initial_displacement_m: 0.1,
                initial_velocity_m_per_s: 0.0,
            },
        ),
    ];
    let mut summaries = Vec::with_capacity(cases.len());
    for (case_name, system) in cases {
        let simulation = simulate_free_vibration(system, 0.01, 1000)?;
        let initial_energy = simulation.samples[0].mechanical_energy_j;
        let final_energy = simulation.samples[1000].mechanical_energy_j;
        summaries.push(format!(
            concat!(
                "{{",
                "\"case_name\":{},",
                "\"mass_kg\":{},",
                "\"stiffness_n_per_m\":{},",
                "\"damping_n_s_per_m\":{},",
                "\"natural_frequency_hz\":{},",
                "\"damping_ratio\":{},",
                "\"damped_natural_frequency_hz\":{},",
                "\"time_step_s\":{},",
                "\"step_count\":{},",
                "\"final_time_s\":{},",
                "\"maximum_normalized_displacement_error\":{},",
                "\"maximum_normalized_velocity_error\":{},",
                "\"maximum_phase_error_radians\":{},",
                "\"maximum_relative_energy_balance_residual\":{},",
                "\"initial_energy_j\":{},",
                "\"final_energy_j\":{},",
                "\"final_to_initial_energy_ratio\":{}",
                "}}"
            ),
            quote(case_name),
            number(system.mass_kg),
            number(system.stiffness_n_per_m),
            number(system.damping_n_s_per_m),
            number(simulation.natural_frequency_hz),
            number(simulation.damping_ratio),
            number(simulation.damped_natural_frequency_hz),
            number(simulation.time_step_s),
            simulation.step_count,
            number(simulation.time_step_s * simulation.step_count as f64),
            number(simulation.maximum_normalized_displacement_error),
            number(simulation.maximum_normalized_velocity_error),
            number(simulation.maximum_phase_error_radians),
            number(simulation.maximum_relative_energy_balance_residual),
            number(initial_energy),
            number(final_energy),
            number(final_energy / initial_energy),
        ));
    }

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"sdof-free-vibration-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-SDOF-FREE-VIBRATION-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"analytical-numerical-verification\",",
            "\"result\":\"verification-candidate-complete\",",
            "\"integrator\":\"Newmark average acceleration, beta=0.25, gamma=0.5\",",
            "\"case_count\":2,",
            "\"cases\":[{}],",
            "\"claim_ceiling\":\"synthetic linear SDOF free vibration only; not full SOFiSTiK BE23/BE24 reproduction, physical validation or safety evidence\"",
            "}}"
        ),
        quote(&source_commit),
        quote(request),
        summaries.join(","),
    ))
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: sdof-free-vibration-reference --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn quote(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::{parse_source_commit, quote};

    #[test]
    fn source_commit_is_strictly_validated() {
        assert!(parse_source_commit().is_err());
    }

    #[test]
    fn json_quote_escapes_control_characters() {
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
