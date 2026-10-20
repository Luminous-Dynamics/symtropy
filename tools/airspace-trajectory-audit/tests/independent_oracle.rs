//! Independent analytical oracle tests for the public trajectory-audit API.
//!
//! These fixtures intentionally use synchronized parallel paths, so their relative position
//! is constant over the full shared time interval. The expected result is calculated directly
//! from the declared scalar bounds rather than by reusing the implementation's minimization logic.

use symtropy_airspace_trajectory_audit::{
    ConflictDisposition, SeparationCriteria, Trajectory, TrajectoryPoint, audit_trajectory_pair,
};

fn point(time_utc_ms: i64, x_m: f64, y_m: f64, z_m: f64) -> TrajectoryPoint {
    TrajectoryPoint {
        time_utc_ms,
        x_m,
        y_m,
        z_m,
    }
}

fn trajectory(
    id: &str,
    y_offset_m: f64,
    z_offset_m: f64,
    horizontal_uncertainty_bound_m: f64,
    vertical_uncertainty_bound_m: f64,
) -> Trajectory {
    Trajectory {
        trajectory_id: id.to_owned(),
        revision: "oracle-grid-001".to_owned(),
        reference_frame_id: "ORACLE-LOCAL-XYZ-METRES".to_owned(),
        vertical_datum_id: "ORACLE-VERTICAL-DATUM".to_owned(),
        source_reference: format!("fixture://oracle-grid/{id}"),
        source_sha256: "a".repeat(64),
        horizontal_uncertainty_bound_m,
        vertical_uncertainty_bound_m,
        points: vec![
            point(1_000, 0.0, y_offset_m, z_offset_m),
            point(11_000, 100.0, y_offset_m, z_offset_m),
        ],
    }
}

fn criteria() -> SeparationCriteria {
    SeparationCriteria {
        criteria_id: "ORACLE-GRID-CRITERIA".to_owned(),
        revision: "oracle-grid-001".to_owned(),
        jurisdiction: "fictional-analytical-fixture".to_owned(),
        source_reference: "fixture://oracle-grid-criteria".to_owned(),
        source_sha256: "b".repeat(64),
        valid_from_utc_ms: 1_000,
        valid_until_utc_ms: 11_000,
        minimum_horizontal_separation_m: 10.0,
        minimum_vertical_separation_m: 5.0,
    }
}

#[test]
fn exhaustive_constant_relative_position_grid_matches_closed_form_oracle() {
    let criteria = criteria();
    let uncertainty_cases = [(0.0, 0.0), (0.5, 1.0), (2.0, 3.0)];

    // 41 horizontal offsets x 25 vertical offsets x 3 horizontal uncertainty pairs x
    // 3 vertical uncertainty pairs = 9,225 independent expected-result comparisons.
    for (horizontal_a, horizontal_b) in uncertainty_cases {
        for (vertical_a, vertical_b) in uncertainty_cases {
            for dy in -20_i32..=20 {
                for dz in -12_i32..=12 {
                    let a = trajectory("A", 0.0, 0.0, horizontal_a, vertical_a);
                    let b = trajectory("B", f64::from(dy), f64::from(dz), horizontal_b, vertical_b);

                    // For synchronized parallel tracks with constant relative position, this
                    // closed-form oracle is exact: the Euclidean horizontal distance is |dy|,
                    // the vertical distance is |dz|, and uncertainty expands each threshold.
                    let horizontal_limit =
                        criteria.minimum_horizontal_separation_m + horizontal_a + horizontal_b;
                    let vertical_limit =
                        criteria.minimum_vertical_separation_m + vertical_a + vertical_b;
                    let expected_conflict = f64::from(dy).abs() <= horizontal_limit
                        && f64::from(dz).abs() <= vertical_limit;

                    let report = audit_trajectory_pair(&a, &b, &criteria)
                        .expect("the bounded analytical fixture must be valid");

                    if expected_conflict {
                        assert_eq!(
                            report.disposition,
                            ConflictDisposition::ConflictDetected,
                            "expected conflict for dy={dy}, dz={dz}, h_unc=({horizontal_a},{horizontal_b}), v_unc=({vertical_a},{vertical_b})"
                        );
                        assert!(
                            !report.conflicts.is_empty(),
                            "positive oracle case must emit a conflict event"
                        );
                    } else {
                        assert_eq!(
                            report.disposition,
                            ConflictDisposition::NoConflictDetectedWithinEvaluatedWindow,
                            "expected no conflict for dy={dy}, dz={dz}, h_unc=({horizontal_a},{horizontal_b}), v_unc=({vertical_a},{vertical_b})"
                        );
                        assert!(
                            report.conflicts.is_empty(),
                            "negative oracle case must not emit a conflict event"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn exact_horizontal_and_vertical_limits_are_conflicts_in_the_oracle_grid() {
    let rule = criteria();
    let a = trajectory("A", 0.0, 0.0, 0.0, 0.0);
    let b = trajectory(
        "B",
        rule.minimum_horizontal_separation_m,
        rule.minimum_vertical_separation_m,
        0.0,
        0.0,
    );
    // Both criteria are inclusive; this case sits exactly on both bounds.
    let report = audit_trajectory_pair(&a, &b, &rule).expect("boundary fixture is valid");
    assert_eq!(report.disposition, ConflictDisposition::ConflictDetected);
    assert_eq!(report.conflicts.len(), 1);
}

// Metamorphic checks target properties that are orthogonal to the constant-relative-position
// oracle above. They do not replace an independent high-precision numerical oracle.
fn crossing_pair_for_metamorphic_tests() -> (Trajectory, Trajectory) {
    (
        Trajectory {
            trajectory_id: "A".to_owned(),
            revision: "metamorphic-001".to_owned(),
            reference_frame_id: "METAMORPHIC-LOCAL-XYZ".to_owned(),
            vertical_datum_id: "METAMORPHIC-VERTICAL-DATUM".to_owned(),
            source_reference: "fixture://metamorphic/A".to_owned(),
            source_sha256: "a".repeat(64),
            horizontal_uncertainty_bound_m: 0.0,
            vertical_uncertainty_bound_m: 0.0,
            points: vec![
                point(1_000, 0.0, 0.0, 100.0),
                point(11_000, 100.0, 0.0, 100.0),
            ],
        },
        Trajectory {
            trajectory_id: "B".to_owned(),
            revision: "metamorphic-001".to_owned(),
            reference_frame_id: "METAMORPHIC-LOCAL-XYZ".to_owned(),
            vertical_datum_id: "METAMORPHIC-VERTICAL-DATUM".to_owned(),
            source_reference: "fixture://metamorphic/B".to_owned(),
            source_sha256: "b".repeat(64),
            horizontal_uncertainty_bound_m: 0.0,
            vertical_uncertainty_bound_m: 0.0,
            points: vec![
                point(1_000, 50.0, -50.0, 100.0),
                point(11_000, 50.0, 50.0, 100.0),
            ],
        },
    )
}

fn assert_same_geometric_decision(
    baseline: &symtropy_airspace_trajectory_audit::ConflictAuditReport,
    transformed: &symtropy_airspace_trajectory_audit::ConflictAuditReport,
) {
    assert_eq!(baseline.disposition, transformed.disposition);
    assert_eq!(
        baseline.temporal_overlap_utc_ms,
        transformed.temporal_overlap_utc_ms
    );
    assert_eq!(
        baseline.evaluated_window_utc_ms,
        transformed.evaluated_window_utc_ms
    );
    assert_eq!(
        baseline.criteria_covers_declared_overlap,
        transformed.criteria_covers_declared_overlap
    );
    assert_eq!(
        baseline.effective_horizontal_bound_m,
        transformed.effective_horizontal_bound_m
    );
    assert_eq!(
        baseline.effective_vertical_bound_m,
        transformed.effective_vertical_bound_m
    );
    assert_eq!(baseline.conflicts.len(), transformed.conflicts.len());

    for (left, right) in baseline.conflicts.iter().zip(&transformed.conflicts) {
        assert_eq!(left.closest_approach_utc_ms, right.closest_approach_utc_ms);
        assert_eq!(left.horizontal_separation_m, right.horizontal_separation_m);
        assert_eq!(left.vertical_separation_m, right.vertical_separation_m);
    }
}

#[test]
fn reversing_pair_argument_order_preserves_the_complete_report() {
    let (a, b) = crossing_pair_for_metamorphic_tests();
    let rule = criteria();

    let forward = audit_trajectory_pair(&a, &b, &rule).expect("crossing fixture must be valid");
    let reverse = audit_trajectory_pair(&b, &a, &rule).expect("reversed fixture must be valid");

    assert_eq!(forward, reverse);
    assert_eq!(forward.disposition, ConflictDisposition::ConflictDetected);
}

#[test]
fn collinear_waypoint_subdivision_preserves_the_geometric_decision() {
    let (a, b) = crossing_pair_for_metamorphic_tests();
    let rule = criteria();
    let baseline = audit_trajectory_pair(&a, &b, &rule).expect("continuous paths must be valid");

    // Both trajectories are split at their exact midpoint. The physical piecewise-linear
    // path is unchanged; only its representation and segment indices differ.
    let mut subdivided_a = a.clone();
    subdivided_a
        .points
        .insert(1, point(6_000, 50.0, 0.0, 100.0));
    let mut subdivided_b = b.clone();
    subdivided_b
        .points
        .insert(1, point(6_000, 50.0, 0.0, 100.0));

    let subdivided = audit_trajectory_pair(&subdivided_a, &subdivided_b, &rule)
        .expect("subdivided paths must be valid");

    assert_same_geometric_decision(&baseline, &subdivided);
    assert_eq!(baseline.disposition, ConflictDisposition::ConflictDetected);
    assert_eq!(baseline.conflicts[0].closest_approach_utc_ms, 6_000);
}

#[test]
fn translating_all_timestamps_and_criteria_preserves_geometry_and_shifts_event_time() {
    let (a, b) = crossing_pair_for_metamorphic_tests();
    let rule = criteria();
    let baseline = audit_trajectory_pair(&a, &b, &rule).expect("base fixture must be valid");
    let shift_ms = 7_777_000_i64;

    let mut shifted_a = a.clone();
    for trajectory_point in &mut shifted_a.points {
        trajectory_point.time_utc_ms += shift_ms;
    }
    let mut shifted_b = b.clone();
    for trajectory_point in &mut shifted_b.points {
        trajectory_point.time_utc_ms += shift_ms;
    }
    let mut shifted_rule = rule.clone();
    shifted_rule.valid_from_utc_ms += shift_ms;
    shifted_rule.valid_until_utc_ms += shift_ms;

    let shifted = audit_trajectory_pair(&shifted_a, &shifted_b, &shifted_rule)
        .expect("uniformly shifted fixture must be valid");

    assert_same_geometric_decision_except_time(&baseline, &shifted, shift_ms);
}

fn assert_same_geometric_decision_except_time(
    baseline: &symtropy_airspace_trajectory_audit::ConflictAuditReport,
    shifted: &symtropy_airspace_trajectory_audit::ConflictAuditReport,
    shift_ms: i64,
) {
    assert_eq!(baseline.disposition, shifted.disposition);
    assert_eq!(
        baseline.effective_horizontal_bound_m,
        shifted.effective_horizontal_bound_m
    );
    assert_eq!(
        baseline.effective_vertical_bound_m,
        shifted.effective_vertical_bound_m
    );
    assert_eq!(baseline.conflicts.len(), shifted.conflicts.len());

    let (overlap_start, overlap_end) = baseline.temporal_overlap_utc_ms.unwrap();
    assert_eq!(
        shifted.temporal_overlap_utc_ms,
        Some((overlap_start + shift_ms, overlap_end + shift_ms))
    );
    let (window_start, window_end) = baseline.evaluated_window_utc_ms.unwrap();
    assert_eq!(
        shifted.evaluated_window_utc_ms,
        Some((window_start + shift_ms, window_end + shift_ms))
    );

    for (left, right) in baseline.conflicts.iter().zip(&shifted.conflicts) {
        assert_eq!(
            right.closest_approach_utc_ms,
            left.closest_approach_utc_ms + shift_ms
        );
        assert_eq!(left.horizontal_separation_m, right.horizontal_separation_m);
        assert_eq!(left.vertical_separation_m, right.vertical_separation_m);
    }
}
