use broadphase_tournament::shape_coverage::{
    ShapeCoverageBackendRow,
    run_shape_coverage_backend_campaign,
};

fn main() {
    let cases_per_dimension = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4_096);

    let rows = run_shape_coverage_backend_campaign(cases_per_dimension)
        .expect("same-frame transformed-OBB backend coverage campaign");
    println!("{}", ShapeCoverageBackendRow::csv_header());
    for row in rows {
        println!("{}", row.to_csv());
        assert_eq!(
            row.backend_aabb_false_negatives, 0,
            "{} omitted an AABB-oracle candidate in {}D",
            row.backend, row.dimension
        );
        assert_eq!(
            row.derived_shape_contact_recall,
            Some(1.0),
            "{} did not preserve exact OBB contact recall in {}D",
            row.backend,
            row.dimension
        );
        assert!(row.candidate_amplification.is_some());
    }
}
