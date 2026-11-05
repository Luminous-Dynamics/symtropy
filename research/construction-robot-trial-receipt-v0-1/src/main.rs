use construction_robot_trial_receipt::{read_json, validate_receipt};
use std::path::{Path, PathBuf};

fn fixture_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot_read_directory:{}:{error}", directory.display()))?;
    let mut paths = entries
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| format!("cannot_read_directory_entry:{error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().and_then(|value| value.to_str()) == Some("json"));
    paths.sort();
    Ok(paths)
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = root.join("schema/construction-robot-physical-trial-v0.1.schema.json");
    let schema = match read_json(&schema_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };

    let mut failures = 0usize;
    let mut total = 0usize;
    println!("fixture|expected|actual|reason_codes");

    for (directory, expected_valid) in [("examples", true), ("fixtures/invalid", false)] {
        let path = root.join(directory);
        let files = match fixture_files(&path) {
            Ok(files) => files,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        };

        for fixture_path in files {
            total += 1;
            let relative = fixture_path
                .strip_prefix(&root)
                .unwrap_or(&fixture_path)
                .display()
                .to_string();
            let receipt = match read_json(&fixture_path) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    failures += 1;
                    println!(
                        "{relative}|{}|parse-error|invalid_json",
                        if expected_valid { "valid" } else { "invalid" }
                    );
                    continue;
                }
            };

            let errors = validate_receipt(&schema, &receipt);
            let actual_valid = errors.is_empty();
            let matched = actual_valid == expected_valid;
            if !matched {
                failures += 1;
            }
            println!(
                "{relative}|{}|{}|{}",
                if expected_valid { "valid" } else { "invalid" },
                if actual_valid { "valid" } else { "invalid" },
                if errors.is_empty() {
                    "none".to_owned()
                } else {
                    errors.join(";")
                }
            );
        }
    }

    println!("summary|fixtures={total}|mismatches={failures}");
    if total == 0 || failures > 0 {
        std::process::exit(1);
    }
}
