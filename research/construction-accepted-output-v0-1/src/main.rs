use construction_accepted_output::{
    derive_report, read_json, schema_is_valid, validate_ledger, validate_profile_binding,
    validate_report,
};
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema = match read_json(&root.join("schema/accepted-output-ledger-v0.1.schema.json")) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let report_schema =
        match read_json(&root.join("schema/accepted-output-report-v0.1.schema.json")) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        };

    if !schema_is_valid(&schema) || !schema_is_valid(&report_schema) {
        eprintln!("schema_meta_validation_failed");
        std::process::exit(1);
    }

    let valid_path = root.join("fixtures/layout-run.synthetic.json");
    let ledger = match read_json(&valid_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let profile = match read_json(&root.join("profile/layout-accepted-output-v0.1.json")) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let binding_errors = validate_profile_binding(&profile, &ledger);
    if !binding_errors.is_empty() {
        eprintln!("profile_binding_failed:{binding_errors:?}");
        std::process::exit(1);
    }

    let errors = validate_ledger(&schema, &ledger);
    if !errors.is_empty() {
        eprintln!("valid_fixture_rejected:{errors:?}");
        std::process::exit(1);
    }

    let report = derive_report(&ledger);
    let report_errors = validate_report(&report_schema, &report);
    if !report_errors.is_empty() {
        eprintln!("derived_report_rejected:{report_errors:?}");
        std::process::exit(1);
    }

    let invalid_path =
        root.join("fixtures/invalid/uninspected-output-counted-as-accepted.synthetic.json");
    let invalid = match read_json(&invalid_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let invalid_errors = validate_ledger(&schema, &invalid);
    if !invalid_errors
        .iter()
        .any(|error| error == "accepted_disposition_without_latest_accepted_inspection:p3")
    {
        eprintln!("semantic_negative_control_not_rejected:{invalid_errors:?}");
        std::process::exit(1);
    }

    println!(
        "accepted_output|valid_fixture=pass|derived_report=pass|uninspected_as_accepted=reject"
    );
    println!(
        "counts|attempted={}|first_pass_inspected={}|first_pass_accepted={}|finally_accepted={}|uninspected={}",
        report["derived_counts"]["attempted_units"],
        report["derived_counts"]["first_pass_inspected_units"],
        report["derived_counts"]["first_pass_accepted_units"],
        report["derived_counts"]["finally_accepted_units"],
        report["derived_counts"]["uninspected_units"]
    );
    println!("limitations|synthetic_fixture|no_physical_claim|no_safety_qualification");
}
