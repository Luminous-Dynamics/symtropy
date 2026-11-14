// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;
use std::fs;
use std::process;

use thermo_allocation_certificate::{
    verify, Agent, Certificate, EdgeFlow, FairnessObjective, IdAmount, ServiceValue, Well,
};

fn parse_u64(value: &str, line: usize, field: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("line {line}: {field} must be an unsigned 64-bit integer"))
}

fn parse_bool(value: &str, line: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("line {line}: boolean must be true or false")),
    }
}

fn parse_certificate(input: &str) -> Result<Certificate, String> {
    let mut lines = input.lines().enumerate().filter_map(|(index, raw)| {
        let trimmed = raw.trim();
        (!trimmed.is_empty() && !trimmed.starts_with('#')).then_some((index + 1, raw))
    });

    let Some((header_line, header)) = lines.next() else {
        return Err("certificate file is empty".to_owned());
    };
    if header != "THERMO-ALLOCATION-CERT\t1" {
        return Err(format!(
            "line {header_line}: expected THERMO-ALLOCATION-CERT<TAB>1 header"
        ));
    }

    let mut case_id = None;
    let mut objective = None;
    let mut claims_optimality = false;
    let mut claims_optimality_seen = false;
    let mut wells = Vec::new();
    let mut agents = Vec::new();
    let mut eligibility = Vec::<(u64, u64)>::new();
    let mut flows = Vec::new();
    let mut claimed_source_debits = Vec::new();
    let mut claimed_recipient_credits = Vec::new();
    let mut claimed_service_profile = Vec::new();

    for (line_number, raw) in lines {
        let fields: Vec<&str> = raw.split('\t').collect();
        match fields.first().copied().unwrap_or("") {
            "CASE" if fields.len() == 2 => {
                if case_id.replace(fields[1].to_owned()).is_some() {
                    return Err(format!("line {line_number}: duplicate CASE row"));
                }
            }
            "OBJECTIVE" if fields.len() == 2 => {
                if objective.is_some() {
                    return Err(format!("line {line_number}: duplicate OBJECTIVE row"));
                }
                objective = Some(match fields[1] {
                    "absolute" => FairnessObjective::AbsoluteAmount,
                    "capacity-normalized" => FairnessObjective::CapacityNormalized,
                    _ => return Err(format!("line {line_number}: unknown objective {}", fields[1])),
                });
            }
            "CLAIMS_OPTIMALITY" if fields.len() == 2 => {
                if claims_optimality_seen {
                    return Err(format!("line {line_number}: duplicate CLAIMS_OPTIMALITY row"));
                }
                claims_optimality = parse_bool(fields[1], line_number)?;
                claims_optimality_seen = true;
            }
            "WELL" if fields.len() == 3 => wells.push(Well {
                id: parse_u64(fields[1], line_number, "well ID")?,
                supply: parse_u64(fields[2], line_number, "well supply")?,
                eligible_agents: Vec::new(),
            }),
            "AGENT" if fields.len() == 3 => agents.push(Agent {
                id: parse_u64(fields[1], line_number, "agent ID")?,
                cap: parse_u64(fields[2], line_number, "agent cap")?,
            }),
            "ELIGIBLE" if fields.len() == 3 => eligibility.push((
                parse_u64(fields[1], line_number, "eligibility well ID")?,
                parse_u64(fields[2], line_number, "eligibility agent ID")?,
            )),
            "FLOW" if fields.len() == 4 => flows.push(EdgeFlow {
                well_id: parse_u64(fields[1], line_number, "flow well ID")?,
                agent_id: parse_u64(fields[2], line_number, "flow agent ID")?,
                amount: parse_u64(fields[3], line_number, "flow amount")?,
            }),
            "SOURCE_CLAIM" if fields.len() == 3 => claimed_source_debits.push(IdAmount {
                id: parse_u64(fields[1], line_number, "source claim ID")?,
                amount: parse_u64(fields[2], line_number, "source claim amount")?,
            }),
            "CREDIT_CLAIM" if fields.len() == 3 => claimed_recipient_credits.push(IdAmount {
                id: parse_u64(fields[1], line_number, "recipient claim ID")?,
                amount: parse_u64(fields[2], line_number, "recipient claim amount")?,
            }),
            "PROFILE_ABSOLUTE" if fields.len() == 2 => {
                claimed_service_profile.push(ServiceValue::Absolute(parse_u64(
                    fields[1],
                    line_number,
                    "profile amount",
                )?));
            }
            "PROFILE_RATIO" if fields.len() == 3 => {
                let numerator = parse_u64(fields[1], line_number, "ratio numerator")?;
                let denominator = parse_u64(fields[2], line_number, "ratio denominator")?;
                if denominator == 0 {
                    return Err(format!("line {line_number}: ratio denominator must be positive"));
                }
                claimed_service_profile.push(ServiceValue::Ratio {
                    numerator,
                    denominator,
                });
            }
            _ => {
                return Err(format!(
                    "line {line_number}: unknown row or wrong field count: {raw}"
                ));
            }
        }
    }

    let case_id = case_id.ok_or_else(|| "missing CASE row".to_owned())?;
    let objective = objective.ok_or_else(|| "missing OBJECTIVE row".to_owned())?;
    if !claims_optimality_seen {
        return Err("missing CLAIMS_OPTIMALITY row".to_owned());
    }

    for (well_id, agent_id) in eligibility {
        let Some(well) = wells.iter_mut().find(|well| well.id == well_id) else {
            return Err(format!("ELIGIBLE row references missing well {well_id}"));
        };
        well.eligible_agents.push(agent_id);
    }

    let expected_profile_type = match objective {
        FairnessObjective::AbsoluteAmount => 0,
        FairnessObjective::CapacityNormalized => 1,
    };
    if claimed_service_profile.iter().any(|value| {
        !matches!(
            (expected_profile_type, value),
            (0, ServiceValue::Absolute(_)) | (1, ServiceValue::Ratio { .. })
        )
    }) {
        return Err("profile row type does not match OBJECTIVE".to_owned());
    }

    Ok(Certificate {
        case_id,
        wells,
        agents,
        flows,
        claimed_source_debits,
        claimed_recipient_credits,
        objective,
        claimed_service_profile,
        claims_optimality,
    })
}

fn usage() -> &'static str {
    "usage: thermo-allocation-certificate --verify <certificate.tsv>"
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 3 || arguments[1] != "--verify" {
        eprintln!("{}", usage());
        process::exit(2);
    }

    let input = match fs::read_to_string(&arguments[2]) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("input_error: cannot read {}: {error}", arguments[2]);
            process::exit(2);
        }
    };
    let certificate = match parse_certificate(&input) {
        Ok(certificate) => certificate,
        Err(error) => {
            eprintln!("input_error: {error}");
            process::exit(2);
        }
    };

    let report = verify(&certificate);
    println!("case_id={}", certificate.case_id);
    println!("feasibility={}", if report.feasible { "PASS" } else { "FAIL" });
    println!("total_debited={}", report.total_debited);
    println!("total_credited={}", report.total_credited);
    println!("optimality_proven={}", report.optimality_proven);
    println!("computed_source_debits={:?}", report.computed_source_debits);
    println!("unused_source_supply={:?}", report.unused_source_supply);
    println!("computed_recipient_credits={:?}", report.computed_recipient_credits);
    println!("remaining_recipient_caps={:?}", report.remaining_recipient_caps);
    println!("computed_service_profile={:?}", report.computed_service_profile);
    for warning in &report.warnings {
        println!("warning={warning}");
    }
    for violation in &report.violations {
        println!("violation={} detail={}", violation.code, violation.detail);
    }

    if !report.feasible {
        process::exit(1);
    }
}
