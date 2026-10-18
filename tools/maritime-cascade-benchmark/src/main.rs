use maritime_cascade_benchmark::run_demo;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|argument| argument != "--json") {
        eprintln!("usage: maritime-cascade-benchmark [--json]");
        std::process::exit(2);
    }

    match run_demo() {
        Ok(report) => println!("{}", report.to_json()),
        Err(error) => {
            eprintln!("cascade reference failed closed: {error}");
            std::process::exit(1);
        }
    }
}
