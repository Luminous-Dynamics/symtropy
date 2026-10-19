use port_call_value_sharing_benchmark::{component_ledger, run_demo};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mode = if args.is_empty() {
        "--json"
    } else if args.len() == 1 {
        args[0].as_str()
    } else {
        ""
    };

    match mode {
        "--json" => match run_demo() {
            Ok(report) => println!("{}", report.to_json()),
            Err(error) => {
                eprintln!("value-sharing reference failed closed: {error}");
                std::process::exit(1);
            }
        },
        "--cost-ledger-json" => match component_ledger::run_demo() {
            Ok(report) => println!("{}", report.to_json()),
            Err(error) => {
                eprintln!("component cost ledger failed closed: {error}");
                std::process::exit(1);
            }
        },
        _ => {
            eprintln!("usage: port-call-value-sharing-benchmark [--json|--cost-ledger-json]");
            std::process::exit(2);
        }
    }
}
