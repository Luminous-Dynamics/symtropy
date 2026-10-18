use port_call_value_sharing_benchmark::run_demo;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|argument| argument != "--json") {
        eprintln!("usage: port-call-value-sharing-benchmark [--json]");
        std::process::exit(2);
    }

    match run_demo() {
        Ok(report) => println!("{}", report.to_json()),
        Err(error) => {
            eprintln!("value-sharing reference failed closed: {error}");
            std::process::exit(1);
        }
    }
}
