//! Independent analytical oracle tests for the public trajectory-audit API.
//!
//! These fixtures intentionally use synchronized parallel paths, so their relative position
//! is constant over the full shared time interval. The expected result is calculated directly
//! from the declared scalar bounds rather than by reusing the implementation's minimization logic.

use symtropy_airspace_trajectory_audit::{
    ConflictDisposition, SeparationCriteria, Trajectory, TrajectoryPairCandidate, TrajectoryPoint,
    audit_trajectory_pair, broadphase_candidate_pairs,
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

#[test]
fn exhaustive_sloped_segments_match_integer_rational_closest_approach_oracle() {
    // A moves 100 m east during the shared interval. B has an integer relative start
    // offset r0 and integer relative displacement d. Every point is integral, so the
    // expected minimum can be compared to an inclusive integer radius exactly:
    //
    //   q = r0·d, n = |d|², m = |r0|².
    //   If q >= 0, min² = m at t=0.
    //   If -q >= n, min² = |r0+d|² at t=1.
    //   Otherwise min² = (m*n - q²)/n, so min² <= R² iff
    //   m*n - q² <= R²*n. This integer comparison uses no floating-point
    // minimizer and is independent of the production segment interpolation.
    //
    // 121 start offsets × 120 nonzero relative displacements × 5 radii = 72,600
    // complete end-to-end checks across sloped/crossing tracks and exact boundaries.
    const STARTS: std::ops::RangeInclusive<i32> = -5..=5;
    const DISPLACEMENTS: std::ops::RangeInclusive<i32> = -5..=5;
    const T0: i64 = 1_000;
    const T1: i64 = 11_000;

    let mut criteria = criteria();
    criteria.minimum_vertical_separation_m = 5.0;

    for r0_x in STARTS.clone() {
        for r0_y in STARTS.clone() {
            for d_x in DISPLACEMENTS.clone() {
                for d_y in DISPLACEMENTS.clone() {
                    if d_x == 0 && d_y == 0 {
                        continue;
                    }

                    let dx = i64::from(d_x);
                    let dy = i64::from(d_y);
                    let rx = i64::from(r0_x);
                    let ry = i64::from(r0_y);
                    let dot = rx * dx + ry * dy;
                    let norm_d = dx * dx + dy * dy;
                    let norm_r = rx * rx + ry * ry;

                    let min_distance_sq_numerator = if dot >= 0 {
                        // The exact minimum lies at the start endpoint.
                        norm_r * norm_d
                    } else if -dot >= norm_d {
                        // The exact minimum lies at the end endpoint.
                        let end_x = rx + dx;
                        let end_y = ry + dy;
                        (end_x * end_x + end_y * end_y) * norm_d
                    } else {
                        // Strictly interior stationary point. Keep squared distance as
                        // a rational numerator/denominator instead of evaluating sqrt.
                        norm_r * norm_d - dot * dot
                    };

                    for radius_m in 1_i32..=5 {
                        let radius = i64::from(radius_m);
                        let expected_conflict =
                            min_distance_sq_numerator <= radius * radius * norm_d;

                        criteria.minimum_horizontal_separation_m = f64::from(radius_m);
                        let a = Trajectory {
                            trajectory_id: "A-rational-oracle".to_owned(),
                            revision: "rational-crossing-oracle-v1".to_owned(),
                            reference_frame_id: "RATIONAL-ORACLE-LOCAL-XYZ".to_owned(),
                            vertical_datum_id: "RATIONAL-ORACLE-VERTICAL".to_owned(),
                            source_reference: "fixture://rational-oracle/A".to_owned(),
                            source_sha256: "a".repeat(64),
                            horizontal_uncertainty_bound_m: 0.0,
                            vertical_uncertainty_bound_m: 0.0,
                            points: vec![point(T0, 0.0, 0.0, 100.0), point(T1, 100.0, 0.0, 100.0)],
                        };
                        let b = Trajectory {
                            trajectory_id: "B-rational-oracle".to_owned(),
                            revision: "rational-crossing-oracle-v1".to_owned(),
                            reference_frame_id: "RATIONAL-ORACLE-LOCAL-XYZ".to_owned(),
                            vertical_datum_id: "RATIONAL-ORACLE-VERTICAL".to_owned(),
                            source_reference: "fixture://rational-oracle/B".to_owned(),
                            source_sha256: "b".repeat(64),
                            horizontal_uncertainty_bound_m: 0.0,
                            vertical_uncertainty_bound_m: 0.0,
                            points: vec![
                                point(T0, f64::from(r0_x), f64::from(r0_y), 100.0),
                                point(
                                    T1,
                                    100.0 + f64::from(r0_x + d_x),
                                    f64::from(r0_y + d_y),
                                    100.0,
                                ),
                            ],
                        };

                        let report = audit_trajectory_pair(&a, &b, &criteria)
                            .expect("bounded integer-rational fixture must be valid");
                        let case = format!("r0=({r0_x},{r0_y}), d=({d_x},{d_y}), R={radius_m} m");
                        if expected_conflict {
                            assert_eq!(
                                report.disposition,
                                ConflictDisposition::ConflictDetected,
                                "exact rational oracle expected conflict for {case}"
                            );
                            assert!(
                                !report.conflicts.is_empty(),
                                "positive oracle case must emit an event for {case}"
                            );
                        } else {
                            assert_eq!(
                                report.disposition,
                                ConflictDisposition::NoConflictDetectedWithinEvaluatedWindow,
                                "exact rational oracle expected no conflict for {case}"
                            );
                            assert!(
                                report.conflicts.is_empty(),
                                "negative oracle case must not emit an event for {case}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn broadphase_has_full_recall_against_brute_force_pairwise_audits() {
    use std::collections::BTreeSet;

    // Frozen deterministic fleet with varied time, slopes, altitude, and uncertainty.
    let mut fleet = Vec::new();
    for i in 0_i32..96 {
        let start = 1_000_i64 + i64::from(i % 9) * 100;
        let x0 = f64::from((i * 47) % 211 - 105);
        let y0 = f64::from((i * 31) % 197 - 98);
        let z0 = 100.0 + f64::from((i * 11) % 7) * 25.0;
        let drift_x = f64::from((i * 13) % 31 - 15);
        let drift_y = f64::from((i * 17) % 29 - 14);
        let drift_z = f64::from((i * 7) % 5 - 2) * 10.0;
        fleet.push(Trajectory {
            trajectory_id: format!("fleet-{i:03}"),
            revision: "broadphase-oracle-v1".to_owned(),
            reference_frame_id: "BROADPHASE-LOCAL-XYZ".to_owned(),
            vertical_datum_id: "BROADPHASE-VERTICAL".to_owned(),
            source_reference: format!("fixture://broadphase/fleet-{i:03}"),
            source_sha256: "a".repeat(64),
            horizontal_uncertainty_bound_m: f64::from(i % 4) * 2.5,
            vertical_uncertainty_bound_m: f64::from(i % 3) * 1.5,
            points: vec![
                point(start, x0, y0, z0),
                point(start + 10_000, x0 + drift_x, y0 + drift_y, z0 + drift_z),
                point(
                    start + 20_000,
                    x0 + 2.0 * drift_x,
                    y0 + 2.0 * drift_y,
                    z0 + 2.0 * drift_z,
                ),
            ],
        });
    }

    let mut rule = criteria();
    rule.valid_from_utc_ms = 0;
    rule.valid_until_utc_ms = 30_000;
    let candidates: BTreeSet<(usize, usize)> = broadphase_candidate_pairs(&fleet, &rule)
        .expect("common-frame, fully covered fleet should be assessable")
        .into_iter()
        .map(|pair: TrajectoryPairCandidate| (pair.first_index, pair.second_index))
        .collect();

    let mut brute_force_conflicts = 0_usize;
    for i in 0..fleet.len() {
        for j in (i + 1)..fleet.len() {
            let report = audit_trajectory_pair(&fleet[i], &fleet[j], &rule)
                .expect("brute-force pairwise comparison must be valid");
            if report.disposition == ConflictDisposition::ConflictDetected {
                brute_force_conflicts += 1;
                assert!(
                    candidates.contains(&(i, j)),
                    "broadphase omitted brute-force conflict pair ({i}, {j})"
                );
            }
        }
    }
    assert!(
        brute_force_conflicts > 0,
        "fixture must contain actual conflicts"
    );
    assert!(
        candidates.len() < fleet.len() * (fleet.len() - 1) / 2,
        "broadphase fixture should demonstrate pruning versus all-pairs enumeration"
    );
}

#[test]
fn broadphase_fails_closed_when_criteria_do_not_cover_fleet_time_span() {
    let (a, b) = crossing_pair_for_metamorphic_tests();
    let mut rule = criteria();
    rule.valid_from_utc_ms = 2_000;
    rule.valid_until_utc_ms = 10_000;
    assert_eq!(
        broadphase_candidate_pairs(&[a, b], &rule),
        Err(
            symtropy_airspace_trajectory_audit::AuditInputError::InvalidCriteria(
                "broadphase requires criteria validity to cover the full fleet time span"
            )
        )
    );
}

#[test]
fn broadphase_rejects_mixed_reference_frames_and_vertical_datums() {
    let (a, mut b) = crossing_pair_for_metamorphic_tests();
    let rule = criteria();
    b.reference_frame_id = "OTHER-REFERENCE-FRAME".to_owned();
    assert_eq!(
        broadphase_candidate_pairs(&[a.clone(), b.clone()], &rule),
        Err(symtropy_airspace_trajectory_audit::AuditInputError::ReferenceFrameMismatch)
    );

    b.reference_frame_id = a.reference_frame_id.clone();
    b.vertical_datum_id = "OTHER-VERTICAL-DATUM".to_owned();
    assert_eq!(
        broadphase_candidate_pairs(&[a, b], &rule),
        Err(symtropy_airspace_trajectory_audit::AuditInputError::VerticalDatumMismatch)
    );
}

#[test]
fn deterministic_wide_integer_oracle_stresses_floating_point_rounding() {
    // The expected answer is computed with i128 integer arithmetic over exact integer-valued
    // coordinates. Inputs reach 1e8 m, making squared products exceed exact f64 integer
    // precision while remaining far below i128 overflow. This is a regression stress test for
    // rounding in dot products, squared velocities, minimizer division and hypot comparison.
    // It is still a finite synthetic corpus, not a proof for arbitrary f64 inputs.
    const T0: i64 = 1_000;
    const T1: i64 = 11_000;
    let mut seed = 0x6a09_e667_f3bc_c909_u64;
    let mut checked = 0_usize;

    fn next(seed: &mut u64) -> u64 {
        // Deterministic xorshift64*; no external RNG/dependency and reproducible on all hosts.
        *seed ^= *seed >> 12;
        *seed ^= *seed << 25;
        *seed ^= *seed >> 27;
        seed.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn signed_coordinate(seed: &mut u64) -> i64 {
        (next(seed) % 200_000_001) as i64 - 100_000_000
    }

    let mut rule = criteria();
    rule.minimum_vertical_separation_m = 5.0;

    for case_index in 0..20_000_usize {
        let rx = signed_coordinate(&mut seed);
        let ry = signed_coordinate(&mut seed);
        let dx = signed_coordinate(&mut seed);
        let dy = signed_coordinate(&mut seed);
        if dx == 0 && dy == 0 {
            continue;
        }
        let radius = (next(&mut seed) % 100_000_000 + 1) as i64;

        let dot = i128::from(rx) * i128::from(dx) + i128::from(ry) * i128::from(dy);
        let norm_d = i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy);
        let norm_r = i128::from(rx) * i128::from(rx) + i128::from(ry) * i128::from(ry);
        let minimum_numerator = if dot >= 0 {
            norm_r * norm_d
        } else if -dot >= norm_d {
            let ex = i128::from(rx) + i128::from(dx);
            let ey = i128::from(ry) + i128::from(dy);
            (ex * ex + ey * ey) * norm_d
        } else {
            norm_r * norm_d - dot * dot
        };
        let r = i128::from(radius);
        let expected_conflict = minimum_numerator <= r * r * norm_d;

        let a = Trajectory {
            trajectory_id: "A-wide-integer-oracle".to_owned(),
            revision: "wide-integer-oracle-v1".to_owned(),
            reference_frame_id: "WIDE-ORACLE-LOCAL-XYZ".to_owned(),
            vertical_datum_id: "WIDE-ORACLE-VERTICAL".to_owned(),
            source_reference: "fixture://wide-oracle/A".to_owned(),
            source_sha256: "a".repeat(64),
            horizontal_uncertainty_bound_m: 0.0,
            vertical_uncertainty_bound_m: 0.0,
            points: vec![point(T0, 0.0, 0.0, 100.0), point(T1, 0.0, 0.0, 100.0)],
        };
        let b = Trajectory {
            trajectory_id: "B-wide-integer-oracle".to_owned(),
            revision: "wide-integer-oracle-v1".to_owned(),
            reference_frame_id: "WIDE-ORACLE-LOCAL-XYZ".to_owned(),
            vertical_datum_id: "WIDE-ORACLE-VERTICAL".to_owned(),
            source_reference: "fixture://wide-oracle/B".to_owned(),
            source_sha256: "b".repeat(64),
            horizontal_uncertainty_bound_m: 0.0,
            vertical_uncertainty_bound_m: 0.0,
            points: vec![
                point(T0, rx as f64, ry as f64, 100.0),
                point(T1, (rx + dx) as f64, (ry + dy) as f64, 100.0),
            ],
        };

        rule.minimum_horizontal_separation_m = radius as f64;
        let report = audit_trajectory_pair(&a, &b, &rule).unwrap_or_else(|error| {
            panic!("wide integer oracle case {case_index} invalid: {error}")
        });
        let diagnostic =
            format!("case={case_index}, r=({rx},{ry}), d=({dx},{dy}), radius={radius}");
        if expected_conflict {
            assert_eq!(
                report.disposition,
                ConflictDisposition::ConflictDetected,
                "integer oracle found a conflict: {diagnostic}"
            );
        } else {
            assert_eq!(
                report.disposition,
                ConflictDisposition::NoConflictDetectedWithinEvaluatedWindow,
                "integer oracle found no conflict: {diagnostic}"
            );
        }
        checked += 1;
    }
    assert!(checked >= 19_999);
}

#[test]
fn exact_tangency_at_large_coordinate_scale_must_not_be_reported_clear() {
    // Integer geometry has an exact 5 m minimum. A stationary A path and B path are constructed
    // so the closest parameter is exactly 1/3, but the f64 dot/norm ratio can round to the next
    // representable value and evaluate a separation above 5 m. This test guards that false-safe.
    let m = 10_000_000_000_i64;
    let rx = -3 * m - 4;
    let ry = -4 * m + 3;
    let dx = 9 * m;
    let dy = 12 * m;
    let end_x = rx + dx;
    let end_y = ry + dy;
    let criteria = SeparationCriteria {
        criteria_id: "EXACT-LARGE-TANGENCY".to_owned(),
        revision: "tangency-v1".to_owned(),
        jurisdiction: "fictional-integer-tangency".to_owned(),
        source_reference: "fixture://large-tangency/criteria".to_owned(),
        source_sha256: "c".repeat(64),
        valid_from_utc_ms: 1_000,
        valid_until_utc_ms: 11_000,
        minimum_horizontal_separation_m: 5.0,
        minimum_vertical_separation_m: 5.0,
    };
    let a = Trajectory {
        trajectory_id: "A-large-tangency".to_owned(),
        revision: "tangency-v1".to_owned(),
        reference_frame_id: "LARGE-TANGENCY-LOCAL-XYZ".to_owned(),
        vertical_datum_id: "LARGE-TANGENCY-VERTICAL".to_owned(),
        source_reference: "fixture://large-tangency/A".to_owned(),
        source_sha256: "a".repeat(64),
        horizontal_uncertainty_bound_m: 0.0,
        vertical_uncertainty_bound_m: 0.0,
        points: vec![
            point(1_000, 0.0, 0.0, 100.0),
            point(11_000, 0.0, 0.0, 100.0),
        ],
    };
    let b = Trajectory {
        trajectory_id: "B-large-tangency".to_owned(),
        revision: "tangency-v1".to_owned(),
        reference_frame_id: "LARGE-TANGENCY-LOCAL-XYZ".to_owned(),
        vertical_datum_id: "LARGE-TANGENCY-VERTICAL".to_owned(),
        source_reference: "fixture://large-tangency/B".to_owned(),
        source_sha256: "b".repeat(64),
        horizontal_uncertainty_bound_m: 0.0,
        vertical_uncertainty_bound_m: 0.0,
        points: vec![
            point(1_000, rx as f64, ry as f64, 100.0),
            point(11_000, end_x as f64, end_y as f64, 100.0),
        ],
    };

    // Exact integer arithmetic proves cross(r0,d)^2 / |d|^2 = 25 m²:
    // cross(r0,d) = 75*m and |d|² = 225*m².
    let cross = i128::from(rx) * i128::from(dy) - i128::from(ry) * i128::from(dx);
    let norm_d = i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy);
    assert_eq!(cross * cross, 25_i128 * norm_d);

    let report = audit_trajectory_pair(&a, &b, &criteria)
        .expect("exact integer tangency fixture must be structurally valid");
    assert_eq!(
        report.disposition,
        ConflictDisposition::ConflictDetected,
        "an exact inclusive 5 m tangency must never be reported clear"
    );
    assert!(!report.conflicts.is_empty());
}
