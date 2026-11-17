use factory_capability_resilience::{evaluate_frozen_corpus, render_tsv};

fn main() {
    match evaluate_frozen_corpus() {
        Ok(evaluations) => {
            print!("{}", render_tsv(&evaluations));
        }
        Err(error) => {
            eprintln!("factory capability resilience: {error}");
            std::process::exit(1);
        }
    }
}
