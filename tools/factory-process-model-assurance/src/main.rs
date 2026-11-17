use factory_process_model_assurance::{evaluate_reference_trace, render_tsv};

fn main() {
    match evaluate_reference_trace() {
        Ok(results) => print!("{}", render_tsv(&results)),
        Err(error) => {
            eprintln!("factory process model assurance: {error}");
            std::process::exit(1);
        }
    }
}
