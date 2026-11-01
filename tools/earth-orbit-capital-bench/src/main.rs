fn main() {
    match earth_orbit_capital_bench::suite_summary() {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("benchmark error: {error}");
            std::process::exit(2);
        }
    }
}
