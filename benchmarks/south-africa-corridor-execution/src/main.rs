use sa_corridor_execution_bench::load_reference_scorecard;

fn main() {
    let rows = load_reference_scorecard().unwrap_or_else(|error| {
        eprintln!("scorecard validation failed: {error}");
        std::process::exit(2);
    });

    println!("South African freight-corridor reference scorecard");
    println!("Source period: FY2025/26 | Unit: percentage points (pp) for target gaps");
    println!();
    println!(
        "{:<22} {:>10} {:>11} {:>12} {:>12} {:>12}",
        "Corridor", "Plan ran", "OTD incl80s", "OTD excl80s", "OTA", "Rows"
    );

    for row in &rows {
        println!(
            "{:<22} {:>+9}pp {:>+10}pp {:>+10}pp {:>+9}pp {:>12}",
            row.observation.corridor,
            row.gaps.train_execution_pp,
            row.gaps.otd_including_80s_pp,
            row.gaps.otd_excluding_80s_pp,
            row.gaps.ota_pp,
            "source-reported"
        );
    }

    println!();
    println!("Positive gap = actual above target; negative gap = actual below target.");
    println!("The two OTD series preserve the source labels 'including 80s' and 'excluding 80s'.");
    println!(
        "This is an aggregate published reference, not route capacity or a live dispatch feed."
    );
}
