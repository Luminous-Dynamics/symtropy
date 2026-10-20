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
                    let b = trajectory(
                        "B",
                        f64::from(dy),
                        f64::from(dz),
                        horizontal_b,
                        vertical_b,
                    );

                    // For synchronized parallel tracks with constant relative position, this
                    // closed-form oracle is exact: the Euclidean horizontal distance is |dy|,
                    // the vertical distance is |dz|, and uncertainty expands each threshold.
                    let horizontal_limit =
                        criteria.minimum_horizontal_separation_m
                            + horizontal_a
                            + horizontal_b;
                    let vertical_limit =
                        criteria.minimum_vertical_separation_m + vertical_a + vertical_b;
                    let expected_conflict =
                        f64::from(dy).abs() <= horizontal_limit
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
