use broadphase_tournament::shape_coverage::{
    ShapeCoverageRow,
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
            "broadphase AABB candidates omitted an exact shape contact in {}D",
            row.dimension
        );
        assert!(row.exact_shape_contacts > 0);
        assert_eq!(row.candidate_recall, Some(1.0));
    }
}
