use shipyard_jsonschema_validator::{
    audit_benchmark_documents, validate_instance_text, validate_schema_document,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn usage() {
    eprintln!("usage: shipyard-jsonschema-validator meta <schema.json>");
    eprintln!("   or: shipyard-jsonschema-validator instance <schema.json> <instance.json>");
    eprintln!("   or: shipyard-jsonschema-validator audit <manifest.json> <report.json>");
}

fn read(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))
}

fn fail(messages: impl IntoIterator<Item = String>) -> ExitCode {
    for message in messages {
        eprintln!("{message}");
    }
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage();
        return ExitCode::from(2);
    }

    match args[1].as_str() {
        "meta" if args.len() == 3 => match read(&args[2]) {
            Ok(text) => match validate_schema_document(&text) {
                Ok(_) => {
                    println!("VALID Draft 2020-12 schema");
                    ExitCode::SUCCESS
                }
                Err(errors) => fail(errors.into_iter().map(|error| error.to_string())),
            },
            Err(error) => fail([error]),
        },
        "instance" if args.len() == 4 => {
            let schema = match read(&args[2]) {
                Ok(text) => text,
                Err(error) => return fail([error]),
            };
            let instance = match read(&args[3]) {
                Ok(text) => text,
                Err(error) => return fail([error]),
            };
            match validate_instance_text(&schema, &instance) {
                Ok(()) => {
                    println!("VALID instance");
                    ExitCode::SUCCESS
                }
                Err(errors) => fail(errors.into_iter().map(|error| error.to_string())),
            }
        }
        "audit" if args.len() == 4 => {
            let manifest = match read(&args[2]) {
                Ok(text) => text,
                Err(error) => return fail([error]),
            };
            let report = match read(&args[3]) {
                Ok(text) => text,
                Err(error) => return fail([error]),
            };
            match audit_benchmark_documents(&manifest, &report) {
                Ok(()) => {
                    println!("VALID benchmark schema + semantic audit");
                    ExitCode::SUCCESS
                }
                Err(errors) => fail(errors.into_iter().map(|error| error.to_string())),
            }
        }
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
