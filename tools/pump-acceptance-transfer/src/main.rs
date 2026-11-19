use pump_acceptance_transfer::{evaluate_corpus, render_report, CASES, CURVES, EXPECTED};
use std::process::ExitCode;

fn main() -> ExitCode {
    let results = match evaluate_corpus(CASES, CURVES, EXPECTED) {
        Ok(results) => results,
        Err(error) => {
            eprintln!("pump-acceptance-transfer: {error}");
            return ExitCode::FAILURE;
        }
    };
    match std::env::args().nth(1).as_deref() {
        None | Some("--report") => print!("{}", render_report(&results)),
        Some(other) => {
            eprintln!("unsupported argument: {other}");
            return ExitCode::FAILURE;
        }
    }
    let mut counts = BTreeCounts::default();
    for row in &results {
        counts.add(row.disposition.as_str());
    }
    eprintln!("scenarios={} dispositions={}", results.len(), counts.render());
    ExitCode::SUCCESS
}

#[derive(Default)]
struct BTreeCounts(std::collections::BTreeMap<&'static str, usize>);

impl BTreeCounts {
    fn add(&mut self, disposition: &'static str) {
        *self.0.entry(disposition).or_default() += 1;
    }
    fn render(&self) -> String {
        self.0.iter().map(|(k,v)| format!("{k}:{v}")).collect::<Vec<_>>().join(",")
    }
}
