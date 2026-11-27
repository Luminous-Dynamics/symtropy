use broadphase_tournament::shape_coverage::{
    ShapeCoverageBackendRow, ShapeCoverageRow, run_shape_coverage_backend_campaign,
    run_shape_coverage_campaign,
};

fn main() {
    let cases_per_dimension = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(4_096);

    let rows = run_shape_coverage_campaign(cases_per_dimension)
        .expect("deterministic transformed-OBB shape coverage campaign");
    println!("{}", ShapeCoverageRow::csv_header());
    for row in rows {
        println!("{}", row.to_csv());
        assert_eq!(
            row.missed_shape_contacts, 0,
            "AABB oracle omitted an exact shape contact in {}D",
            row.dimension
        );
        assert!(row.exact_shape_contacts > 0);
        assert_eq!(row.candidate_recall, Some(1.0));
    }

    let backend_rows = run_shape_coverage_backend_campaign(cases_per_dimension)
        .expect("same-frame backend shape coverage campaign");
    eprintln!("{}", ShapeCoverageBackendRow::csv_header());
    for row in &backend_rows {
        eprintln!("{}", row.to_csv());
        assert_eq!(
            row.backend_aabb_false_negatives, 0,
            "{} omitted an AABB oracle pair in {}D",
            row.backend,
            row.dimension
        );
        assert_eq!(row.derived_shape_contact_recall, Some(1.0));
    }
    assert_eq!(backend_rows.len(), 18);
}
