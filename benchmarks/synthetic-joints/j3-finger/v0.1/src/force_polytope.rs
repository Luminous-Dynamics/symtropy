//! Generic 3D force-polytope construction and volume-coverage primitives.
//!
//! This extends the J3 directional-force oracle without claiming a PACT
//! prototype reproduction. It projects the four-tendon/one-auxiliary-actuator
//! feasible set into fingertip force space by eliminating the auxiliary
//! actuator torque, then computes the volume of bounded convex H-polytopes.
//! Source-specific pose Jacobians and required-force-region halfspaces must be
//! transcribed and independently validated before source-equivalent coverage
//! can be reported.

const RELATIVE_PIVOT_EPSILON: f64 = 1.0e-12;
const FEASIBILITY_TOLERANCE: f64 = 1.0e-9;
const VERTEX_TOLERANCE: f64 = 1.0e-8;

type Matrix4 = [[f64; 4]; 4];
type Matrix3x4 = [[f64; 4]; 3];
type Vector4 = [f64; 4];
type Vector3 = [f64; 3];

#[derive(Clone, Copy, Debug)]
struct Inequality4 {
    force: Vector3,
    auxiliary_torque: f64,
    bound: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Halfspace3 {
    normal: Vector3,
    bound: f64,
}

fn dot3(a: Vector3, b: Vector3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add3(a: Vector3, b: Vector3) -> Vector3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: Vector3, b: Vector3) -> Vector3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale3(a: Vector3, scale: f64) -> Vector3 {
    [a[0] * scale, a[1] * scale, a[2] * scale]
}

fn cross3(a: Vector3, b: Vector3) -> Vector3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn norm3(a: Vector3) -> f64 {
    dot3(a, a).sqrt()
}

fn normalize3(a: Vector3) -> Option<Vector3> {
    let norm = norm3(a);
    if !norm.is_finite() || norm <= f64::MIN_POSITIVE {
        return None;
    }
    let result = scale3(a, 1.0 / norm);
    result
        .iter()
        .all(|value| value.is_finite())
        .then_some(result)
}

fn finite_matrix4(matrix: Matrix4) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn finite_matrix3x4(matrix: Matrix3x4) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn inverse4(matrix: Matrix4) -> Option<Matrix4> {
    if !finite_matrix4(matrix) {
        return None;
    }

    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return None;
    }

    let mut augmented = [[0.0_f64; 8]; 4];
    for row in 0..4 {
        for col in 0..4 {
            augmented[row][col] = matrix[row][col];
        }
        augmented[row][row + 4] = 1.0;
    }

    for pivot_col in 0..4 {
        let mut pivot_row = pivot_col;
        for row in (pivot_col + 1)..4 {
            if augmented[row][pivot_col].abs() > augmented[pivot_row][pivot_col].abs() {
                pivot_row = row;
            }
        }

        let pivot = augmented[pivot_row][pivot_col];
        if !pivot.is_finite() || pivot.abs() <= RELATIVE_PIVOT_EPSILON * scale {
            return None;
        }
        if pivot_row != pivot_col {
            augmented.swap(pivot_row, pivot_col);
        }

        let pivot_value = augmented[pivot_col][pivot_col];
        for col in 0..8 {
            augmented[pivot_col][col] /= pivot_value;
        }

        for row in 0..4 {
            if row == pivot_col {
                continue;
            }
            let factor = augmented[row][pivot_col];
            for col in 0..8 {
                augmented[row][col] -= factor * augmented[pivot_col][col];
            }
        }
    }

    let mut result = [[0.0_f64; 4]; 4];
    for row in 0..4 {
        for col in 0..4 {
            result[row][col] = augmented[row][col + 4];
        }
    }
    finite_matrix4(result).then_some(result)
}

fn jacobian_has_full_force_rank(jacobian: Matrix3x4) -> bool {
    if !finite_matrix3x4(jacobian) {
        return false;
    }

    let mut column_scale = 0.0_f64;
    for col in 0..4 {
        let column = [jacobian[0][col], jacobian[1][col], jacobian[2][col]];
        column_scale = column_scale.max(norm3(column));
    }
    if !column_scale.is_finite() || column_scale == 0.0 {
        return false;
    }

    let mut maximum_minor = 0.0_f64;
    for a in 0..4 {
        for b in (a + 1)..4 {
            for c in (b + 1)..4 {
                let ca = [jacobian[0][a], jacobian[1][a], jacobian[2][a]];
                let cb = [jacobian[0][b], jacobian[1][b], jacobian[2][b]];
                let cc = [jacobian[0][c], jacobian[1][c], jacobian[2][c]];
                maximum_minor = maximum_minor.max(dot3(ca, cross3(cb, cc)).abs());
            }
        }
    }

    maximum_minor > RELATIVE_PIVOT_EPSILON * column_scale.powi(3)
}

fn normalize_halfspaces(input: &[Halfspace3]) -> Option<Vec<Halfspace3>> {
    let mut normalized: Vec<Halfspace3> = Vec::with_capacity(input.len());

    for halfspace in input {
        if halfspace.normal.iter().any(|value| !value.is_finite()) || !halfspace.bound.is_finite() {
            return None;
        }

        let norm = norm3(halfspace.normal);
        if !norm.is_finite() {
            return None;
        }
        if norm <= f64::MIN_POSITIVE {
            // 0 <= b is redundant; 0 <= negative b makes the set empty.
            if halfspace.bound < -FEASIBILITY_TOLERANCE {
                return None;
            }
            continue;
        }

        let candidate = Halfspace3 {
            normal: scale3(halfspace.normal, 1.0 / norm),
            bound: halfspace.bound / norm,
        };

        let duplicate = normalized.iter().position(|existing| {
            dot3(existing.normal, candidate.normal) >= 1.0 - 1.0e-10
                && (existing.bound - candidate.bound).abs()
                    <= FEASIBILITY_TOLERANCE
                        * 1.0_f64.max(existing.bound.abs()).max(candidate.bound.abs())
        });

        if let Some(index) = duplicate {
            // Same outward normal and plane; retaining either is equivalent.
            if candidate.bound < normalized[index].bound {
                normalized[index] = candidate;
            }
            continue;
        }

        // Parallel constraints with the same normal but different bounds:
        // retain the stricter (smaller) upper bound.
        let parallel = normalized
            .iter()
            .position(|existing| dot3(existing.normal, candidate.normal) >= 1.0 - 1.0e-10);
        if let Some(index) = parallel {
            if candidate.bound < normalized[index].bound {
                normalized[index] = candidate;
            }
            continue;
        }

        normalized.push(candidate);
    }

    Some(normalized)
}

fn force_set_halfspaces(
    fingertip_jacobian_m: Matrix3x4,
    routing_matrix_m: Matrix4,
    tendon_min_tensions_n: Vector4,
    tendon_max_tensions_n: Vector4,
    maximum_abduction_torque_nm: f64,
) -> Option<Vec<Halfspace3>> {
    if !finite_matrix3x4(fingertip_jacobian_m)
        || tendon_min_tensions_n
            .iter()
            .zip(tendon_max_tensions_n.iter())
            .any(|(minimum, maximum)| {
                !minimum.is_finite()
                    || !maximum.is_finite()
                    || *minimum < 0.0
                    || *maximum < *minimum
            })
        || !maximum_abduction_torque_nm.is_finite()
        || maximum_abduction_torque_nm < 0.0
        || !jacobian_has_full_force_rank(fingertip_jacobian_m)
    {
        return None;
    }

    let routing_inverse = inverse4(routing_matrix_m)?;
    let mut inequalities = Vec::with_capacity(10);

    for tendon in 0..4 {
        // T_i = A_i f + b_i tau_AA, with
        // A_i[a] = (R^-1 J^T)[i,a], b_i = -(R^-1)[i,0].
        let mut force_coefficients = [0.0_f64; 3];
        for axis in 0..3 {
            for joint in 0..4 {
                force_coefficients[axis] +=
                    routing_inverse[tendon][joint] * fingertip_jacobian_m[axis][joint];
            }
        }
        let auxiliary_coefficient = -routing_inverse[tendon][0];

        // T_i >= T_min  <=> -A_i f - b_i tau <= -T_min.
        inequalities.push(Inequality4 {
            force: scale3(force_coefficients, -1.0),
            auxiliary_torque: -auxiliary_coefficient,
            bound: -tendon_min_tensions_n[tendon],
        });

        // T_i <= T_max.
        inequalities.push(Inequality4 {
            force: force_coefficients,
            auxiliary_torque: auxiliary_coefficient,
            bound: tendon_max_tensions_n[tendon],
        });
    }

    // Bidirectional A/A actuator torque: -tau_max <= tau_AA <= tau_max.
    inequalities.push(Inequality4 {
        force: [0.0; 3],
        auxiliary_torque: 1.0,
        bound: maximum_abduction_torque_nm,
    });
    inequalities.push(Inequality4 {
        force: [0.0; 3],
        auxiliary_torque: -1.0,
        bound: maximum_abduction_torque_nm,
    });

    // Fourier–Motzkin elimination of the scalar auxiliary torque.
    // Classify coefficients relative to this problem's scale; an absolute
    // epsilon would silently erase a physically small but meaningful channel.
    // For p with b_p > 0 and n with b_n < 0:
    // (A_p/b_p - A_n/b_n) f <= c_p/b_p - c_n/b_n.
    let auxiliary_scale = inequalities.iter().fold(0.0_f64, |maximum, constraint| {
        maximum.max(constraint.auxiliary_torque.abs())
    });
    if !auxiliary_scale.is_finite() || auxiliary_scale == 0.0 {
        return None;
    }
    let auxiliary_tolerance = RELATIVE_PIVOT_EPSILON * auxiliary_scale;

    let positive: Vec<Inequality4> = inequalities
        .iter()
        .copied()
        .filter(|constraint| constraint.auxiliary_torque > auxiliary_tolerance)
        .collect();
    let negative: Vec<Inequality4> = inequalities
        .iter()
        .copied()
        .filter(|constraint| constraint.auxiliary_torque < -auxiliary_tolerance)
        .collect();
    let zero: Vec<Inequality4> = inequalities
        .iter()
        .copied()
        .filter(|constraint| constraint.auxiliary_torque.abs() <= auxiliary_tolerance)
        .collect();

    if positive.is_empty() || negative.is_empty() {
        return None;
    }

    let mut projected = Vec::with_capacity(zero.len() + positive.len() * negative.len());
    for constraint in zero {
        projected.push(Halfspace3 {
            normal: constraint.force,
            bound: constraint.bound,
        });
    }

    for upper in &positive {
        for lower in &negative {
            projected.push(Halfspace3 {
                normal: sub3(
                    scale3(upper.force, 1.0 / upper.auxiliary_torque),
                    scale3(lower.force, 1.0 / lower.auxiliary_torque),
                ),
                bound: upper.bound / upper.auxiliary_torque - lower.bound / lower.auxiliary_torque,
            });
        }
    }

    normalize_halfspaces(&projected)
}

fn halfspace_satisfied(halfspace: Halfspace3, point: Vector3) -> bool {
    let lhs = dot3(halfspace.normal, point);
    let tolerance = FEASIBILITY_TOLERANCE
        * 1.0_f64
            .max(lhs.abs())
            .max(halfspace.bound.abs())
            .max(norm3(point));
    lhs <= halfspace.bound + tolerance
}

fn is_bounded(halfspaces: &[Halfspace3]) -> bool {
    // A nonzero recession direction d satisfies n_i dot d <= 0 for every
    // outward half-space normal n_i. For 3D polyhedral cones, a feasible
    // direction is found among facet-pair intersections, lineality candidates,
    // or a cone interior direction aligned with a negative facet normal.
    let axes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut candidates: Vec<Vector3> = Vec::new();

    for axis in axes {
        candidates.push(axis);
        candidates.push(scale3(axis, -1.0));
    }

    for halfspace in halfspaces {
        candidates.push(scale3(halfspace.normal, -1.0));
        candidates.push(halfspace.normal);
        for axis in axes {
            let cross = cross3(halfspace.normal, axis);
            candidates.push(cross);
            candidates.push(scale3(cross, -1.0));
        }
    }

    for first in 0..halfspaces.len() {
        for second in (first + 1)..halfspaces.len() {
            let cross = cross3(halfspaces[first].normal, halfspaces[second].normal);
            candidates.push(cross);
            candidates.push(scale3(cross, -1.0));
        }
    }

    for direction in candidates {
        let norm = norm3(direction);
        if !norm.is_finite() || norm <= RELATIVE_PIVOT_EPSILON {
            continue;
        }
        let unit = scale3(direction, 1.0 / norm);
        if halfspaces
            .iter()
            .all(|halfspace| dot3(halfspace.normal, unit) <= FEASIBILITY_TOLERANCE)
        {
            return false;
        }
    }

    true
}

fn unique_vertices(halfspaces: &[Halfspace3]) -> Vec<Vector3> {
    let mut vertices: Vec<Vector3> = Vec::new();

    for first in 0..halfspaces.len() {
        for second in (first + 1)..halfspaces.len() {
            for third in (second + 1)..halfspaces.len() {
                let a = halfspaces[first];
                let b = halfspaces[second];
                let c = halfspaces[third];

                let cross_bc = cross3(b.normal, c.normal);
                let determinant = dot3(a.normal, cross_bc);
                let determinant_scale = norm3(a.normal) * norm3(b.normal) * norm3(c.normal);
                if !determinant.is_finite()
                    || determinant.abs()
                        <= RELATIVE_PIVOT_EPSILON * determinant_scale.max(f64::MIN_POSITIVE)
                {
                    continue;
                }

                let point = scale3(
                    add3(
                        add3(
                            scale3(cross_bc, a.bound),
                            scale3(cross3(c.normal, a.normal), b.bound),
                        ),
                        scale3(cross3(a.normal, b.normal), c.bound),
                    ),
                    1.0 / determinant,
                );

                if point.iter().any(|value| !value.is_finite())
                    || !halfspaces
                        .iter()
                        .all(|halfspace| halfspace_satisfied(*halfspace, point))
                {
                    continue;
                }

                let is_duplicate = vertices.iter().any(|existing| {
                    norm3(sub3(*existing, point))
                        <= VERTEX_TOLERANCE * 1.0_f64.max(norm3(*existing)).max(norm3(point))
                });
                if !is_duplicate {
                    vertices.push(point);
                }
            }
        }
    }

    vertices
}

fn polytope_volume(input: &[Halfspace3]) -> Option<f64> {
    let halfspaces = normalize_halfspaces(input)?;
    if halfspaces.is_empty() || !is_bounded(&halfspaces) {
        return None;
    }

    let vertices = unique_vertices(&halfspaces);
    if vertices.len() < 4 {
        // Empty or lower-dimensional intersections have zero 3D volume.
        return Some(0.0);
    }

    let mut interior = [0.0_f64; 3];
    for vertex in &vertices {
        interior = add3(interior, *vertex);
    }
    interior = scale3(interior, 1.0 / vertices.len() as f64);

    let mut volume = 0.0_f64;
    for halfspace in &halfspaces {
        let mut face: Vec<Vector3> = vertices
            .iter()
            .copied()
            .filter(|vertex| {
                let residual = (dot3(halfspace.normal, *vertex) - halfspace.bound).abs();
                residual
                    <= 10.0
                        * VERTEX_TOLERANCE
                        * 1.0_f64.max(halfspace.bound.abs()).max(norm3(*vertex))
            })
            .collect();

        if face.len() < 3 {
            continue;
        }

        let mut face_center = [0.0_f64; 3];
        for vertex in &face {
            face_center = add3(face_center, *vertex);
        }
        face_center = scale3(face_center, 1.0 / face.len() as f64);

        // Construct an orthonormal in-plane basis with u cross v = outward n.
        let reference = if halfspace.normal[0].abs() <= halfspace.normal[1].abs()
            && halfspace.normal[0].abs() <= halfspace.normal[2].abs()
        {
            [1.0, 0.0, 0.0]
        } else if halfspace.normal[1].abs() <= halfspace.normal[2].abs() {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let u = normalize3(cross3(reference, halfspace.normal))?;
        let v = cross3(halfspace.normal, u);

        face.sort_by(|left, right| {
            let left_delta = sub3(*left, face_center);
            let right_delta = sub3(*right, face_center);
            let left_angle = dot3(left_delta, v).atan2(dot3(left_delta, u));
            let right_angle = dot3(right_delta, v).atan2(dot3(right_delta, u));
            left_angle.total_cmp(&right_angle)
        });

        let anchor = face[0];
        for index in 1..(face.len() - 1) {
            let b = face[index];
            let c = face[index + 1];
            let a_rel = sub3(anchor, interior);
            let b_rel = sub3(b, interior);
            let c_rel = sub3(c, interior);
            let tetra_volume = dot3(a_rel, cross3(b_rel, c_rel)).abs() / 6.0;
            if !tetra_volume.is_finite() {
                return None;
            }
            volume += tetra_volume;
        }
    }

    volume.is_finite().then_some(volume)
}

fn required_box_halfspaces(minimum: Vector3, maximum: Vector3) -> Option<Vec<Halfspace3>> {
    if minimum
        .iter()
        .zip(maximum.iter())
        .any(|(low, high)| !low.is_finite() || !high.is_finite() || *low >= *high)
    {
        return None;
    }

    let mut constraints = Vec::with_capacity(6);
    for axis in 0..3 {
        let mut positive = [0.0; 3];
        positive[axis] = 1.0;
        let mut negative = [0.0; 3];
        negative[axis] = -1.0;
        constraints.push(Halfspace3 {
            normal: positive,
            bound: maximum[axis],
        });
        constraints.push(Halfspace3 {
            normal: negative,
            bound: -minimum[axis],
        });
    }
    normalize_halfspaces(&constraints)
}

fn force_region_coverage(
    force_set_halfspaces: &[Halfspace3],
    required_region_halfspaces: &[Halfspace3],
) -> Option<f64> {
    let required_volume = polytope_volume(required_region_halfspaces)?;
    if !required_volume.is_finite() || required_volume <= 0.0 {
        return None;
    }

    let mut intersection =
        Vec::with_capacity(force_set_halfspaces.len() + required_region_halfspaces.len());
    intersection.extend_from_slice(force_set_halfspaces);
    intersection.extend_from_slice(required_region_halfspaces);

    let intersection_volume = polytope_volume(&intersection)?;
    let coverage = intersection_volume / required_volume;
    if !coverage.is_finite() || coverage < -FEASIBILITY_TOLERANCE || coverage > 1.0 + 1.0e-6 {
        return None;
    }

    Some(coverage.clamp(0.0, 1.0))
}

fn identity4() -> Matrix4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn identity_jacobian() -> Matrix3x4 {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]
}

fn assert_close(actual: f64, expected: f64, label: &str) {
    let tolerance = 1.0e-9_f64.max(expected.abs() * 1.0e-9);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label}: expected {expected:.12e}, got {actual:.12e}, tolerance {tolerance:.3e}"
    );
}

#[test]
fn force_poly_001_cube_volume_is_eight() {
    let cube = required_box_halfspaces([-1.0; 3], [1.0; 3]).unwrap();
    assert_close(
        polytope_volume(&cube).unwrap(),
        8.0,
        "unit-radius cube volume",
    );
}

#[test]
fn force_poly_002_projection_of_synthetic_actuator_bounds_is_unit_cube() {
    let halfspaces = force_set_halfspaces(
        identity_jacobian(),
        identity4(),
        [0.0; 4],
        [1.0, 1.0, 1.0, 1.0],
        0.0,
    )
    .expect("bounded full-rank synthetic force set must project");

    let volume = polytope_volume(&halfspaces).expect("force set must be bounded");
    assert_close(volume, 1.0, "synthetic [0,1]^3 force-set volume");

    let expected = required_box_halfspaces([0.0; 3], [1.0; 3]).unwrap();
    assert_close(
        force_region_coverage(&halfspaces, &expected).unwrap(),
        1.0,
        "synthetic force set must cover identical required region",
    );
}

#[test]
fn force_poly_003_cube_inside_double_cube_has_one_eighth_coverage() {
    let force_set = required_box_halfspaces([0.0; 3], [1.0; 3]).unwrap();
    let required = required_box_halfspaces([-1.0; 3], [1.0; 3]).unwrap();

    assert_close(
        force_region_coverage(&force_set, &required).unwrap(),
        0.125,
        "coverage of unit cube in [-1,1]^3",
    );
}

#[test]
fn force_poly_004_partial_intersection_has_expected_coverage() {
    let force_set = required_box_halfspaces([-0.5; 3], [0.5; 3]).unwrap();
    let required = required_box_halfspaces([0.0; 3], [1.0; 3]).unwrap();

    // The intersection is [0,0.5]^3, whose volume is 0.125. The required
    // region has volume 1.
    assert_close(
        force_region_coverage(&force_set, &required).unwrap(),
        0.125,
        "partial intersection coverage",
    );
}

#[test]
fn force_poly_005_rank_deficient_jacobian_fails_closed() {
    let singular = [[1.0, 0.0, 0.0, 0.0]; 3];
    assert!(force_set_halfspaces(singular, identity4(), [0.0; 4], [1.0; 4], 1.0,).is_none());
}

#[test]
fn force_poly_006_empty_intersection_has_zero_coverage() {
    let force_set = required_box_halfspaces([2.0; 3], [3.0; 3]).unwrap();
    let required = required_box_halfspaces([0.0; 3], [1.0; 3]).unwrap();

    assert_close(
        force_region_coverage(&force_set, &required).unwrap(),
        0.0,
        "disjoint bounded regions",
    );
}

#[test]
fn force_poly_007_invalid_or_unbounded_required_region_is_rejected() {
    let unbounded = vec![Halfspace3 {
        normal: [1.0, 0.0, 0.0],
        bound: 1.0,
    }];
    assert!(polytope_volume(&unbounded).is_none());

    let zero_volume = vec![
        Halfspace3 {
            normal: [1.0, 0.0, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [-1.0, 0.0, 0.0],
            bound: -1.0,
        },
        Halfspace3 {
            normal: [0.0, 1.0, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [0.0, -1.0, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [0.0, 0.0, 1.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [0.0, 0.0, -1.0],
            bound: 1.0,
        },
    ];
    assert!(force_region_coverage(&zero_volume, &zero_volume).is_none());
}
#[test]
fn force_poly_008_minimum_tendon_tension_is_preserved_as_pretension() {
    let force_set = force_set_halfspaces(
        identity_jacobian(),
        identity4(),
        [0.25, 0.25, 0.25, 0.0],
        [1.0; 4],
        0.0,
    )
    .expect("compatible positive pretension bounds must remain feasible");
    let required = required_box_halfspaces([0.0; 3], [1.0; 3]).unwrap();

    // The feasible set is [0.25,1]^3, volume 0.75^3.
    assert_close(
        force_region_coverage(&force_set, &required).unwrap(),
        0.421875,
        "coverage with nonzero minimum tendon tensions",
    );
}
#[test]
fn force_poly_009_oblique_tetrahedron_volume_is_one_sixth() {
    // T = {x >= 0, y >= 0, z >= 0, x+y+z <= 1}.
    let tetrahedron = vec![
        Halfspace3 {
            normal: [-1.0, 0.0, 0.0],
            bound: 0.0,
        },
        Halfspace3 {
            normal: [0.0, -1.0, 0.0],
            bound: 0.0,
        },
        Halfspace3 {
            normal: [0.0, 0.0, -1.0],
            bound: 0.0,
        },
        Halfspace3 {
            normal: [1.0, 1.0, 1.0],
            bound: 1.0,
        },
    ];
    assert_close(
        polytope_volume(&tetrahedron).unwrap(),
        1.0 / 6.0,
        "unit-coordinate tetrahedron volume",
    );
}

#[test]
fn force_poly_010_rotated_cube_volume_and_coverage_are_invariant() {
    // Rotate a [-1,1]^3 cube by an orthonormal 3-4-5 basis in the xy plane.
    let rotated_cube = vec![
        Halfspace3 {
            normal: [0.6, 0.8, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [-0.6, -0.8, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [-0.8, 0.6, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [0.8, -0.6, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [0.0, 0.0, 1.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [0.0, 0.0, -1.0],
            bound: 1.0,
        },
    ];
    let enclosing = required_box_halfspaces([-2.0; 3], [2.0; 3]).unwrap();
    assert_close(
        polytope_volume(&rotated_cube).unwrap(),
        8.0,
        "rotated cube volume",
    );
    assert_close(
        force_region_coverage(&rotated_cube, &enclosing).unwrap(),
        0.125,
        "rotated cube coverage inside enclosing [-2,2]^3",
    );
}

#[test]
fn force_poly_011_nonuniform_scaling_has_expected_volume() {
    let box_polytope = required_box_halfspaces([0.0, 0.0, 0.0], [2.0, 3.0, 4.0]).unwrap();
    assert_close(
        polytope_volume(&box_polytope).unwrap(),
        24.0,
        "2x3x4 box volume",
    );

    let scaled_force = required_box_halfspaces([0.0; 3], [3.0; 3]).unwrap();
    let scaled_required = required_box_halfspaces([0.0; 3], [6.0; 3]).unwrap();
    assert_close(
        force_region_coverage(&scaled_force, &scaled_required).unwrap(),
        0.125,
        "coverage remains dimensionless under uniform scale",
    );
}

#[test]
fn force_poly_012_parallel_redundant_halfspaces_do_not_double_count_faces() {
    let mut box_polytope = required_box_halfspaces([0.0; 3], [2.0, 3.0, 4.0]).unwrap();
    box_polytope.push(Halfspace3 {
        normal: [2.0, 0.0, 0.0],
        bound: 4.0,
    });
    box_polytope.push(Halfspace3 {
        normal: [1.0, 0.0, 0.0],
        bound: 2.0,
    });
    assert_close(
        polytope_volume(&box_polytope).unwrap(),
        24.0,
        "duplicate and scaled parallel constraints",
    );
}

#[test]
fn force_poly_013_oblique_lineality_cone_is_rejected_as_unbounded() {
    // The feasible recession directions include both signs of (1,1,1),
    // which is not a coordinate axis. All four normals are orthogonal to
    // that lineality direction, so the intersection is unbounded.
    let unbounded = vec![
        Halfspace3 {
            normal: [1.0, -1.0, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [-1.0, 1.0, 0.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [1.0, 1.0, -2.0],
            bound: 1.0,
        },
        Halfspace3 {
            normal: [-1.0, -1.0, 2.0],
            bound: 1.0,
        },
    ];
    assert!(polytope_volume(&unbounded).is_none());
}
