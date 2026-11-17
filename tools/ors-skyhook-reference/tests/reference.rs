use ors_skyhook_reference::{FixtureStatus, evaluate_corpus, evaluate_fixture, parse_fixtures};

const CORPUS: &str = include_str!("../cases-v0.1.csv");

#[test]
fn all_36_birch_table_4_cases_reproduce_or_fail_closed_at_the_source_limit() {
    let report = evaluate_corpus(CORPUS).expect("frozen Table 4 corpus validates");
    assert_eq!(report.cases.len(), 36);
    assert_eq!(report.table_reproduced_count, 28);
    assert_eq!(report.outside_source_domain_count, 8);
    assert!(report.max_relative_error < 0.02);
}

#[test]
fn equation_35_and_equation_54_are_independent_algebraic_paths_to_the_same_force() {
    let fixtures = parse_fixtures(CORPUS).expect("corpus parses");
    let fixture = fixtures
        .iter()
        .find(|case| case.case_id == "H600_A08_DH300")
        .expect("representative 600 km case");
    let result = evaluate_fixture(fixture).expect("reference evaluates");
    let alpha = std::f64::consts::PI / 8.0;
    let earth_radius_m = 6_400_000.0;
    let g = 9.81;
    let line_density = 2.5 * std::f64::consts::PI * 1_000.0;
    let h = 600_000.0;
    let delta_h = 300_000.0;
    let radius_plus_height = earth_radius_m + h;
    let speed_denominator = radius_plus_height + delta_h * (1.0 - 2.0 / alpha.powi(2));
    let speed_squared = g * earth_radius_m.powi(2) / speed_denominator;
    let turn_angle = 4.0 * delta_h / (alpha * radius_plus_height);
    let from_eq35 = line_density * speed_squared * turn_angle;
    let from_eq54 = 4.0 * line_density * g * earth_radius_m.powi(2) * delta_h
        / (alpha * radius_plus_height * speed_denominator);
    assert!((from_eq35 - from_eq54).abs() / from_eq54 < 1.0e-12);
    assert_eq!(result.status, FixtureStatus::TableReproduced);
    assert!((result.predicted_force_n.unwrap() - from_eq35).abs() / from_eq35 < 1.0e-12);
}

#[test]
fn table_dashes_are_source_model_domain_rejections_not_physical_infeasibility_claims() {
    let fixtures = parse_fixtures(CORPUS).expect("corpus parses");
    let outside: Vec<_> = fixtures
        .iter()
        .filter(|fixture| fixture.expected_status == FixtureStatus::OutsideSourceDomain)
        .collect();
    assert_eq!(outside.len(), 8);
    for fixture in outside {
        let result = evaluate_fixture(fixture).expect("outside-domain fixture is classified");
        assert_eq!(result.status, FixtureStatus::OutsideSourceDomain);
        assert!(result.predicted_force_n.is_none());
    }
}

#[test]
fn rejects_a_force_scale_that_does_not_match_the_source_table_column() {
    let corrupted = CORPUS.replacen(
        "H300_A02_DH003,300000,3000,2,1000000000,0.54,table_reproduced",
        "H300_A02_DH003,300000,3000,2,10000000000,0.54,table_reproduced",
        1,
    );
    assert_ne!(corrupted, CORPUS);
    assert!(parse_fixtures(&corrupted).is_err());
}

#[test]
fn rejects_missing_duplicate_and_unexpected_fixtures() {
    let short = CORPUS.lines().take(36).collect::<Vec<_>>().join("\n");
    assert!(parse_fixtures(&short).is_err());

    let lines: Vec<_> = CORPUS.lines().collect();
    let duplicate = std::iter::once(lines[0])
        .chain(std::iter::once(lines[1]))
        .chain(std::iter::once(lines[1]))
        .chain(lines.iter().skip(3).copied())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(parse_fixtures(&duplicate).is_err());
}
