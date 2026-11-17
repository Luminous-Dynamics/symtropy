use factory_execution_reality::{render_tsv, validate_reference_corpus};

fn main() {
    match validate_reference_corpus() {
        Ok(results) => print!("{}", render_tsv(&results)),
        Err(error) => {
            eprintln!("factory execution reality: {error}");
            std::process::exit(1);
        }
    }
}
