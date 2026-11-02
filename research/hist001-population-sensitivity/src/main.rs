use hist001_population_sensitivity::{
    estimate_k2, estimate_phase, HomesteadAssumptions, PUBLISHED_PHASES,
};

fn main() {
    let persons_per_homestead = [40.0, 50.0, 60.0];
    let homestead_use_years = [40.0, 50.0, 60.0];

    println!(
        "persons_per_homestead,homestead_use_years,phase,subinterval,duration_years,general_population,capital_population,total_if_capital_added"
    );

    for persons in persons_per_homestead {
        for duration in homestead_use_years {
            let assumptions = HomesteadAssumptions {
                persons_per_homestead: persons,
                homestead_use_years: duration,
            };

            for phase in PUBLISHED_PHASES {
                let estimate = estimate_phase(phase, assumptions)
                    .expect("predeclared sensitivity assumptions are valid");
                let capital = estimate
                    .capital_population
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "none".to_string());
                let total = estimate
                    .total_population_if_capital_added()
                    .map(|n| format!("{n:.2}"))
                    .unwrap_or_else(|| "not_applicable".to_string());

                println!(
                    "{persons:.0},{duration:.0},{},{},{:.0},{:.2},{capital},{total}",
                    estimate.phase,
                    estimate.subinterval,
                    estimate.duration_years,
                    estimate.general_population
                );
            }

            let k2 = estimate_k2(153, 150.0, 25.0, 300, 1500, assumptions)
                .expect("predeclared K2 temporal partition is valid");
            for estimate in k2 {
                let capital = estimate.capital_population.unwrap_or_default();
                let total = estimate
                    .total_population_if_capital_added()
                    .unwrap_or_default();
                println!(
                    "{persons:.0},{duration:.0},{},{},{:.0},{:.2},{capital},{total:.2}",
                    estimate.phase,
                    estimate.subinterval,
                    estimate.duration_years,
                    estimate.general_population
                );
            }
        }
    }
}
