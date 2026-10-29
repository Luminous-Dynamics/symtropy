// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_physics::gjk::intersects_support_maps;
use symtropy_physics::support_map::WorldSupportMap;
use symtropy_physics::GjkTermination;

#[derive(Clone)]
struct AxisAlignedBox<const D: usize> {
    center: SVector<f64, D>,
    half_extents: SVector<f64, D>,
}

impl<const D: usize> WorldSupportMap<D> for AxisAlignedBox<D> {
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local = SVector::from_fn(|axis, _| {
            if direction[axis] >= 0.0 {
                self.half_extents[axis]
            } else {
                -self.half_extents[axis]
            }
        });
        self.center + local
    }

    fn center_world(&self) -> SVector<f64, D> {
        self.center
    }

    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64) {
        (self.center, self.half_extents.norm())
    }
}

fn aabb_oracle<const D: usize>(a: &AxisAlignedBox<D>, b: &AxisAlignedBox<D>) -> bool {
    (0..D).all(|axis| {
        (a.center[axis] - b.center[axis]).abs()
            <= a.half_extents[axis] + b.half_extents[axis]
    })
}

fn box_at<const D: usize>(center: SVector<f64, D>, half: f64) -> AxisAlignedBox<D> {
    AxisAlignedBox {
        center,
        half_extents: SVector::from_element(half),
    }
}

fn assert_nd_contract<const D: usize>() {
    let a = box_at(SVector::<f64, D>::zeros(), 1.0);
    let separated = box_at(
        SVector::from_fn(|axis, _| if axis == 0 { 3.5 } else { 0.0 }),
        1.0,
    );
    assert!(!aabb_oracle(&a, &separated));
    let separated_result = intersects_support_maps(&a, &separated);
    assert_eq!(
        separated_result.termination,
        GjkTermination::Separated,
        "D={D}: separated hyperboxes should be classified as separated"
    );
    assert!(!separated_result.intersecting);

    let overlapping = box_at(SVector::from_element(0.5), 1.0);
    assert!(aabb_oracle(&a, &overlapping));
    let overlap_result = intersects_support_maps(&a, &overlapping);
    assert_eq!(
        overlap_result.termination,
        GjkTermination::Intersecting,
        "D={D}: overlapping hyperboxes should be classified as intersecting"
    );
    assert!(overlap_result.intersecting);
    assert!(
        overlap_result.simplex.len() <= D + 1,
        "D={D}: the terminal simplex cannot have more than D+1 vertices"
    );

    let swapped = intersects_support_maps(&overlapping, &a);
    assert_eq!(
        swapped.termination,
        GjkTermination::Intersecting,
        "D={D}: swapping support maps must preserve overlap classification"
    );

    let translation = SVector::from_element(32.0);
    let translated_a = box_at(translation, 1.0);
    let translated_overlapping = box_at(translation + SVector::from_element(0.5), 1.0);
    let translated_result = intersects_support_maps(&translated_a, &translated_overlapping);
    assert_eq!(
        translated_result.termination,
        GjkTermination::Intersecting,
        "D={D}: common translation must preserve classification"
    );
}


#[derive(Clone)]
struct Ball<const D: usize> {
    center: SVector<f64, D>,
    radius: f64,
}

impl<const D: usize> WorldSupportMap<D> for Ball<D> {
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let norm = direction.norm();
        if norm <= f64::MIN_POSITIVE {
            let mut axis = SVector::<f64, D>::zeros();
            axis[0] = self.radius;
            self.center + axis
        } else {
            self.center + direction * (self.radius / norm)
        }
    }

    fn center_world(&self) -> SVector<f64, D> {
        self.center
    }

    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64) {
        (self.center, self.radius)
    }
}

#[test]
fn active_set_gjk_matches_analytic_sphere_oracle_in_five_dimensions() {
    let a = Ball::<5> {
        center: SVector::zeros(),
        radius: 1.0,
    };
    for (distance, expected) in [(1.5, GjkTermination::Intersecting), (3.0, GjkTermination::Separated)] {
        let b = Ball::<5> {
            center: SVector::from([distance, 0.0, 0.0, 0.0, 0.0]),
            radius: 1.0,
        };
        let analytical = distance <= a.radius + b.radius;
        assert_eq!(analytical, expected == GjkTermination::Intersecting);
        let result = intersects_support_maps(&a, &b);
        assert_eq!(
            result.termination,
            expected,
            "D=5: sphere distance {distance} disagrees with analytical reference"
        );
    }
}


#[derive(Clone)]
struct OrientedBox<const D: usize> {
    rotation: nalgebra::SMatrix<f64, D, D>,
    center: SVector<f64, D>,
    half_extents: SVector<f64, D>,
}

impl<const D: usize> WorldSupportMap<D> for OrientedBox<D> {
    fn support_world(&self, direction: &SVector<f64, D>) -> SVector<f64, D> {
        let local_direction = self.rotation.transpose() * direction;
        let local = SVector::from_fn(|axis, _| {
            if local_direction[axis] >= 0.0 {
                self.half_extents[axis]
            } else {
                -self.half_extents[axis]
            }
        });
        self.rotation * local + self.center
    }

    fn center_world(&self) -> SVector<f64, D> {
        self.center
    }

    fn bounding_sphere_world(&self) -> (SVector<f64, D>, f64) {
        (self.center, self.half_extents.norm())
    }
}

fn determinant(matrix: &mut [f64], size: usize) -> f64 {
    if size == 0 || matrix.len() != size * size {
        return 0.0;
    }
    let mut det = 1.0;
    for column in 0..size {
        let mut pivot = column;
        for row in (column + 1)..size {
            if matrix[row * size + column].abs() > matrix[pivot * size + column].abs() {
                pivot = row;
            }
        }
        let pivot_value = matrix[pivot * size + column];
        if !pivot_value.is_finite() || pivot_value.abs() <= 1e-14 {
            return 0.0;
        }
        if pivot != column {
            for entry in 0..size {
                matrix.swap(column * size + entry, pivot * size + entry);
            }
            det = -det;
        }
        let diagonal = matrix[column * size + column];
        det *= diagonal;
        for row in (column + 1)..size {
            let factor = matrix[row * size + column] / diagonal;
            for entry in (column + 1)..size {
                matrix[row * size + entry] -= factor * matrix[column * size + entry];
            }
        }
    }
    det
}

/// Generalized cross product of D-1 generators in D dimensions.
/// This reference routine is only used by tests and never calls GJK.
fn normal_from_generators<const D: usize>(
    generators: &[SVector<f64, D>],
    selected: &[usize],
) -> SVector<f64, D> {
    let mut normal = SVector::<f64, D>::zeros();
    for omitted_axis in 0..D {
        let mut minor = vec![0.0; (D - 1) * (D - 1)];
        for row in 0..(D - 1) {
            let generator = &generators[selected[row]];
            let mut col = 0;
            for axis in 0..D {
                if axis == omitted_axis {
                    continue;
                }
                minor[row * (D - 1) + col] = generator[axis];
                col += 1;
            }
        }
        let sign = if omitted_axis % 2 == 0 { 1.0 } else { -1.0 };
        normal[omitted_axis] = sign * determinant(&mut minor, D - 1);
    }
    normal
}

/// Independent SAT oracle for oriented hyperboxes. Facet normals of the
/// Minkowski-difference zonotope are orthogonal to D-1 of its 2D edge generators.
fn sat_oracle<const D: usize>(a: &OrientedBox<D>, b: &OrientedBox<D>) -> bool {
    let mut generators: Vec<SVector<f64, D>> = Vec::with_capacity(2 * D);
    for axis in 0..D {
        generators.push(a.rotation.column(axis).into_owned());
        generators.push(b.rotation.column(axis).into_owned());
    }
    let center_delta = b.center - a.center;
    let geometry_scale = a
        .half_extents
        .norm()
        .max(b.half_extents.norm())
        .max(center_delta.norm());
    let mut selected = Vec::with_capacity(D - 1);

    fn visit<const D: usize>(
        a: &OrientedBox<D>,
        b: &OrientedBox<D>,
        generators: &[SVector<f64, D>],
        selected: &mut Vec<usize>,
        start: usize,
        center_delta: &SVector<f64, D>,
        geometry_scale: f64,
    ) -> bool {
        if selected.len() == D - 1 {
            let normal = normal_from_generators(generators, selected);
            let normal_norm = normal.norm();
            if !normal_norm.is_finite() || normal_norm * normal_norm <= 1e-20 {
                return true;
            }
            let radius_a: f64 = (0..D)
                .map(|axis| {
                    a.half_extents[axis]
                        * normal.dot(&a.rotation.column(axis).into_owned()).abs()
                })
                .sum();
            let radius_b: f64 = (0..D)
                .map(|axis| {
                    b.half_extents[axis]
                        * normal.dot(&b.rotation.column(axis).into_owned()).abs()
                })
                .sum();
            let tolerance = 128.0 * f64::EPSILON * normal_norm * geometry_scale;
            return center_delta.dot(&normal).abs() <= radius_a + radius_b + tolerance;
        }

        let needed = (D - 1) - selected.len();
        for index in start..=(generators.len() - needed) {
            selected.push(index);
            let possible = visit(
                a,
                b,
                generators,
                selected,
                index + 1,
                center_delta,
                geometry_scale,
            );
            selected.pop();
            if !possible {
                return false;
            }
        }
        true
    }

    visit(
        a,
        b,
        &generators,
        &mut selected,
        0,
        &center_delta,
        geometry_scale,
    )
}

fn next_random(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*seed >> 11) as f64) / ((1u64 << 53) as f64)
}

fn random_rotation<const D: usize>(seed: &mut u64) -> nalgebra::SMatrix<f64, D, D> {
    let mut rotation = nalgebra::SMatrix::<f64, D, D>::identity();
    for i in 0..D {
        for j in (i + 1)..D {
            let angle = (2.0 * next_random(seed) - 1.0) * std::f64::consts::PI;
            let (sin, cos) = angle.sin_cos();
            let mut plane = nalgebra::SMatrix::<f64, D, D>::identity();
            plane[(i, i)] = cos;
            plane[(j, j)] = cos;
            plane[(i, j)] = -sin;
            plane[(j, i)] = sin;
            rotation *= plane;
        }
    }
    rotation
}

fn seeded_oriented_box<const D: usize>(seed: &mut u64) -> OrientedBox<D> {
    OrientedBox {
        rotation: random_rotation(seed),
        center: SVector::from_fn(|_, _| 2.0 * next_random(seed) - 1.0),
        half_extents: SVector::from_fn(|_, _| 0.3 + 0.7 * next_random(seed)),
    }
}

fn assert_seeded_rotated_box_panel<const D: usize>() {
    let mut seed = 0x4e44_474a_4b5f_5341_u64 ^ (D as u64);
    let mut indeterminate_cases = 0usize;
    for case_id in 0..24 {
        let a = seeded_oriented_box::<D>(&mut seed);
        let b = seeded_oriented_box::<D>(&mut seed);
        let expected = sat_oracle(&a, &b);
        let result = intersects_support_maps(&a, &b);
        match result.termination {
            GjkTermination::Intersecting => assert!(
                expected,
                "D={D}: false positive against independent SAT at case {case_id}"
            ),
            GjkTermination::Separated => assert!(
                !expected,
                "D={D}: false negative against independent SAT at case {case_id}"
            ),
            GjkTermination::Indeterminate(_) => indeterminate_cases += 1,
        }
    }
    assert!(
        indeterminate_cases <= 4,
        "D={D}: indeterminate outcomes {indeterminate_cases}/24 exceeded the frozen budget of 4"
    );
}

#[test]
fn active_set_gjk_matches_sat_on_seeded_rotated_five_dimensional_boxes() {
    assert_seeded_rotated_box_panel::<5>();
}

#[test]
fn active_set_gjk_matches_sat_on_seeded_rotated_six_dimensional_boxes() {
    assert_seeded_rotated_box_panel::<6>();
}

#[test]
fn active_set_gjk_handles_five_dimensions() {
    assert_nd_contract::<5>();
}

#[test]
fn active_set_gjk_handles_six_dimensions() {
    assert_nd_contract::<6>();
}

#[test]
fn active_set_gjk_handles_eight_dimensions() {
    assert_nd_contract::<8>();
}

#[test]
fn five_dimensional_classification_is_uniform_scale_invariant() {
    let base_a = box_at(SVector::<f64, 5>::zeros(), 1.0);
    let base_b = box_at(SVector::from([3.5, 0.0, 0.0, 0.0, 0.0]), 1.0);
    assert!(!aabb_oracle(&base_a, &base_b));

    for scale in [1e-12, 1e-9, 1.0, 1e9, 1e12] {
        let a = AxisAlignedBox {
            center: base_a.center * scale,
            half_extents: base_a.half_extents * scale,
        };
        let b = AxisAlignedBox {
            center: base_b.center * scale,
            half_extents: base_b.half_extents * scale,
        };
        assert!(!aabb_oracle(&a, &b));
        let result = intersects_support_maps(&a, &b);
        assert_eq!(
            result.termination,
            GjkTermination::Separated,
            "D=5: scale {scale:e} changed separated classification"
        );
    }

    let overlap_base_b = box_at(SVector::from_element(0.5), 1.0);
    assert!(aabb_oracle(&base_a, &overlap_base_b));
    for scale in [1e-12, 1e-9, 1.0, 1e9, 1e12] {
        let a = AxisAlignedBox {
            center: base_a.center * scale,
            half_extents: base_a.half_extents * scale,
        };
        let b = AxisAlignedBox {
            center: overlap_base_b.center * scale,
            half_extents: overlap_base_b.half_extents * scale,
        };
        assert!(aabb_oracle(&a, &b));
        let result = intersects_support_maps(&a, &b);
        assert_eq!(
            result.termination,
            GjkTermination::Intersecting,
            "D=5: scale {scale:e} changed overlap classification"
        );
    }
}
