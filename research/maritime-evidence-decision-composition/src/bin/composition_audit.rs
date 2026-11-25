use maritime_evidence_decision_composition::{apply_named_fault, EvidenceBundle};
use std::process::ExitCode;

fn main() -> ExitCode {
    let manifest = include_str!("../../fixtures/composition-cases.tsv");
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0usize;
    let mut failed = 0usize;

    println!("case_id|actual_disposition|actual_reason");
    for (line_number, line) in manifest.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("case_id|") {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() != 4 {
            eprintln!("line {}: expected four fields", line_number + 1);
            return ExitCode::FAILURE;
        }
        let case_id = fields[0];
        if case_id.is_empty() || !seen.insert(case_id.to_owned()) {
            eprintln!("line {}: empty or duplicate case id", line_number + 1);
            return ExitCode::FAILURE;
        }

        let mut bundle = EvidenceBundle::synthetic_valid_fixture();
        if let Err(error) = apply_named_fault(&mut bundle, fields[1]) {
            eprintln!("{case_id}: {error}");
            return ExitCode::FAILURE;
        }
        let outcome = bundle.evaluate();
        let actual = outcome.disposition.as_str();
        let reason = outcome.primary_reason();
        println!("{case_id}|{actual}|{reason}");
        total += 1;

        if actual != fields[2] || reason != fields[3] {
            failed += 1;
            eprintln!(
                "{case_id}: expected {}/{}, got {actual}/{reason}",
                fields[2], fields[3]
            );
        }
    }

    eprintln!(
        "composition-corpus: cases={total} passed={} failed={failed}",
        total.saturating_sub(failed)
    );
    if total == 30 && failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
