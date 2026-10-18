use port_call_value_sharing_benchmark::{
    cost_ledger::run_demo_component_ledger,
    run_demo,
};

fn usage() -> ! {
    eprintln!(
        "usage: port-call-value-sharing-benchmark [--json|--component-ledger-json]"
    );
    std::process::exit(2);
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 1 {
        usage();
    }

    match args.first().map(String::as_str) {
        None | Some("--json") => match run_demo() {
            Ok(report) => println!("{}", report.to_json()),
            Err(error) => {
                eprintln!("value-sharing reference failed closed: {error}");
                std::process::exit(1);
            }
        },
        Some("--component-ledger-json") => match run_demo_component_ledger() {
            Ok(report) => println!("{}", report.to_json()),
            Err(error) => {
                eprintln!("component cost ledger failed closed: {error}");
                std::process::exit(1);
            }
        },
        _ => usage(),
    }
}
