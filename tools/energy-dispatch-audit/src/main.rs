use std::{env, error::Error, fs, io};

use symtropy_energy_dispatch_audit::audit_bytes;

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy-dispatch-audit <dispatch-request.json> <dispatch-run-report.json>",
        )
        .into());
    }

    let request_bytes = fs::read(&args[1])?;
    let report_bytes = fs::read(&args[2])?;
    let audit = audit_bytes(&request_bytes, &report_bytes);
    println!("{}", serde_json::to_string_pretty(&audit)?);

    Ok(match audit.status.as_str() {
        "pass" => 0,
        "unknown" => 3,
        _ => 2,
    })
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("energy dispatch independent audit failed to run: {error}");
            std::process::exit(2);
        }
    }
}
