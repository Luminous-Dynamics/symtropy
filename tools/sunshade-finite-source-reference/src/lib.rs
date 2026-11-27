//! Finite-source geometric attenuation for one circular shade module.
//!
//! Geometry:
//! - The Sun is a uniformly bright circular disk in a source plane at z = -L_s.
//! - The shade module lies in the plane z = 0.
//! - A target point lies in the plane z = +L_t.
//! - The module is a circular disk parallel to the source and target planes.
//!
//! The source disk projects to a smaller disk on the shade plane. The fraction
//! of source rays intercepted by the module equals the overlap area divided by
//! the projected solar-disk area, under uniform radiance and geometric optics.
//!
//! This kernel does not model solar limb darkening, spectral variation,
//! diffraction, scattering, non-circular or multiple modules, module motion,
//! orbital dynamics, target-disk integration, or a full installed-system mass.

use std::f64::consts::PI;

/// One finite-source / single-module attenuation query in a common 2-D plane.
///
/// Coordinates are measured in metres in parallel source, shade and target
/// planes. The source disk is centred on the z-axis; the target point and module
/// centre use the same x-y coordinate frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularShadeInput {
    /// Sun radius in the source plane, metres.
    pub sun_radius_m: f64,
    /// Source-plane to shade-plane distance, metres.
    pub sun_to_shade_distance_m: f64,
    /// Shade-plane to target-plane distance, metres.
    pub shade_to_target_distance_m: f64,
    /// Target point position in the target plane, metres.
    pub target_point_xy_m: [f64; 2],
    /// Physical circular module centre in the shade plane, metres.
    pub module_center_xy_m: [f64; 2],
    /// Physical circular module radius in the shade plane, metres.
    pub module_radius_m: f64,
    /// Module's radiant transmittance, from 0 (opaque) to 1 (transparent).
    pub module_transmittance_fraction: f64,
}

/// Local target-plane attenuation for one circular module.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularShadeEstimate {
    /// Radius of the projected finite solar disk on the shade plane, metres.
    pub projected_sun_radius_at_shade_m: f64,
    /// Centre of the projected solar disk on the shade plane, metres.
    pub projected_sun_center_xy_at_shade_m: [f64; 2],
    /// Separation between the projected solar-disk centre and module centre.
    pub centre_separation_m: f64,
    /// Geometric overlap area on the shade plane, m².
    pub overlap_area_m2: f64,
    /// Fraction of the projected solar disk geometrically covered by the module.
    pub geometric_blocked_fraction: f64,
    /// Fraction of the local unshaded radiative flux attenuated by the module.
    pub attenuation_fraction: f64,
    /// Local transmitted flux divided by the unshaded local flux.
    pub target_transmittance_fraction: f64,
}

/// Evaluate the geometric shadow from a finite, uniformly bright solar disk.
///
/// The projected solar-disk radius and centre follow straight-line
/// interpolation between source, shade and target planes:
///
/// - r_projected = L_t / (L_s + L_t) * R_sun
/// - c_projected = L_s / (L_s + L_t) * x_target
///
/// The geometric blocked fraction is the circle-intersection area divided by
/// the projected solar-disk area. Module transmittance is then applied only to
/// the geometrically covered rays.
pub fn evaluate_circular_shade(
    input: CircularShadeInput,
) -> Result<CircularShadeEstimate, &'static str> {
    if !finite_positive(input.sun_radius_m) {
        return Err("sun_radius_m must be finite and positive");
    }
    if !finite_positive(input.sun_to_shade_distance_m) {
        return Err("sun_to_shade_distance_m must be finite and positive");
    }
    if !finite_positive(input.shade_to_target_distance_m) {
        return Err("shade_to_target_distance_m must be finite and positive");
    }
    if !finite_vector(input.target_point_xy_m) {
        return Err("target_point_xy_m must contain only finite values");
    }
    if !finite_vector(input.module_center_xy_m) {
        return Err("module_center_xy_m must contain only finite values");
    }
    if !finite_nonnegative(input.module_radius_m) {
        return Err("module_radius_m must be finite and non-negative");
    }
    if !input.module_transmittance_fraction.is_finite()
        || !(0.0..=1.0).contains(&input.module_transmittance_fraction)
    {
        return Err("module_transmittance_fraction must be finite and in [0, 1]");
    }

    let total_distance_m = input.sun_to_shade_distance_m + input.shade_to_target_distance_m;
    if !total_distance_m.is_finite() {
        return Err("source-to-target distance overflowed its finite domain");
    }

    let source_to_shade_fraction = input.sun_to_shade_distance_m / total_distance_m;
    let shade_to_target_fraction = input.shade_to_target_distance_m / total_distance_m;
    let projected_sun_radius_at_shade_m = shade_to_target_fraction * input.sun_radius_m;
    let projected_sun_center_xy_at_shade_m = input
        .target_point_xy_m
        .map(|component| source_to_shade_fraction * component);

    if !projected_sun_radius_at_shade_m.is_finite()
        || projected_sun_radius_at_shade_m <= 0.0
        || !finite_vector(projected_sun_center_xy_at_shade_m)
    {
        return Err("projected solar disk overflowed its finite domain");
    }

    let centre_delta = [
        projected_sun_center_xy_at_shade_m[0] - input.module_center_xy_m[0],
        projected_sun_center_xy_at_shade_m[1] - input.module_center_xy_m[1],
    ];
    let centre_separation_m = centre_delta[0].hypot(centre_delta[1]);
    if !centre_separation_m.is_finite() {
        return Err("projected centre separation overflowed its finite domain");
    }

    let overlap_area_m2 = circle_overlap_area(
        projected_sun_radius_at_shade_m,
        input.module_radius_m,
        centre_separation_m,
    )?;

    let projected_sun_area_m2 = PI * projected_sun_radius_at_shade_m.powi(2);
    if !projected_sun_area_m2.is_finite() || projected_sun_area_m2 <= 0.0 {
        return Err("projected solar-disk area overflowed its finite domain");
    }

    // Clamp only numerical roundoff; the analytic intersection is bounded by
    // both circle areas.
    let geometric_blocked_fraction = (overlap_area_m2 / projected_sun_area_m2).clamp(0.0, 1.0);
    let attenuation_fraction =
        geometric_blocked_fraction * (1.0 - input.module_transmittance_fraction);
    let target_transmittance_fraction = 1.0 - attenuation_fraction;

    let outputs = [
        centre_separation_m,
        overlap_area_m2,
        geometric_blocked_fraction,
        attenuation_fraction,
        target_transmittance_fraction,
    ];
    if !outputs.into_iter().all(f64::is_finite) {
        return Err("attenuation calculation overflowed its finite output domain");
    }

    Ok(CircularShadeEstimate {
        projected_sun_radius_at_shade_m,
        projected_sun_center_xy_at_shade_m,
        centre_separation_m,
        overlap_area_m2,
        geometric_blocked_fraction,
        attenuation_fraction,
        target_transmittance_fraction,
    })
}

/// Exact area of intersection of two circles in a common plane.
///
/// Lengths are normalized by the largest input before squaring. This avoids
/// intermediate overflow for large but finite coordinates and radii.
pub fn circle_overlap_area(
    first_radius_m: f64,
    second_radius_m: f64,
    centre_separation_m: f64,
) -> Result<f64, &'static str> {
    if !finite_nonnegative(first_radius_m) || !finite_nonnegative(second_radius_m) {
        return Err("circle radii must be finite and non-negative");
    }
    if !finite_nonnegative(centre_separation_m) {
        return Err("centre separation must be finite and non-negative");
    }
    if first_radius_m == 0.0 || second_radius_m == 0.0 {
        return Ok(0.0);
    }

    let scale = first_radius_m.max(second_radius_m).max(centre_separation_m);
    if !finite_positive(scale) {
        return Err("circle geometry scale must be finite and positive");
    }

    let first_radius = first_radius_m / scale;
    let second_radius = second_radius_m / scale;
    let separation = centre_separation_m / scale;

    if separation >= first_radius + second_radius {
        return Ok(0.0);
    }

    let radius_difference = (first_radius - second_radius).abs();
    if separation <= radius_difference {
        let smaller_radius = first_radius.min(second_radius);
        let area = PI * smaller_radius.powi(2) * scale.powi(2);
        if !area.is_finite() {
            return Err("contained circle area overflowed its finite domain");
        }
        return Ok(area);
    }

    let first_squared = first_radius.powi(2);
    let second_squared = second_radius.powi(2);
    let distance_squared = separation.powi(2);
    let denominator_first = 2.0 * separation * first_radius;
    let denominator_second = 2.0 * separation * second_radius;

    let first_cosine =
        ((distance_squared + first_squared - second_squared) / denominator_first).clamp(-1.0, 1.0);
    let second_cosine =
        ((distance_squared + second_squared - first_squared) / denominator_second).clamp(-1.0, 1.0);

    let radicand = (-separation + first_radius + second_radius)
        * (separation + first_radius - second_radius)
        * (separation - first_radius + second_radius)
        * (separation + first_radius + second_radius);
    let unit_area = first_squared * first_cosine.acos() + second_squared * second_cosine.acos()
        - 0.5 * radicand.max(0.0).sqrt();
    let area = unit_area.max(0.0) * scale.powi(2);

    if !area.is_finite() {
        return Err("circle overlap area overflowed its finite domain");
    }
    Ok(area)
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn finite_vector(vector: [f64; 2]) -> bool {
    vector.into_iter().all(f64::is_finite)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual}, expected {expected}, tolerance {tolerance}"
        );
    }

    fn base_input() -> CircularShadeInput {
        CircularShadeInput {
            sun_radius_m: 1.0,
            sun_to_shade_distance_m: 100.0,
            shade_to_target_distance_m: 10.0,
            target_point_xy_m: [0.0, 0.0],
            module_center_xy_m: [0.0, 0.0],
            module_radius_m: 1.0,
            module_transmittance_fraction: 0.0,
        }
    }

    #[test]
    fn no_module_coverage_preserves_unshaded_target_flux() {
        let estimate = evaluate_circular_shade(CircularShadeInput {
            module_center_xy_m: [10.0, 0.0],
            ..base_input()
        })
        .unwrap();

        assert_eq!(estimate.overlap_area_m2, 0.0);
        assert_eq!(estimate.geometric_blocked_fraction, 0.0);
        assert_eq!(estimate.attenuation_fraction, 0.0);
        assert_eq!(estimate.target_transmittance_fraction, 1.0);
    }

    #[test]
    fn opaque_module_containing_projected_solar_disk_gives_full_blockage() {
        let estimate = evaluate_circular_shade(base_input()).unwrap();

        close(estimate.geometric_blocked_fraction, 1.0, 1e-14);
        close(estimate.attenuation_fraction, 1.0, 1e-14);
        close(estimate.target_transmittance_fraction, 0.0, 1e-14);
    }

    #[test]
    fn module_inside_projected_solar_disk_uses_module_area_fraction() {
        let estimate = evaluate_circular_shade(CircularShadeInput {
            module_radius_m: 0.5 * 10.0 / 110.0,
            ..base_input()
        })
        .unwrap();

        close(estimate.geometric_blocked_fraction, 0.25, 1e-14);
    }

    #[test]
    fn equal_circles_have_hand_checkable_partial_overlap() {
        let area = circle_overlap_area(1.0, 1.0, 1.0).unwrap();
        let expected = 2.0 * 0.5_f64.acos() - 0.5 * 3.0_f64.sqrt();

        close(area, expected, 1e-14);
        close(area / PI, 0.391_002_218_955_770_64, 1e-14);
    }

    #[test]
    fn external_tangent_has_zero_overlap() {
        assert_eq!(circle_overlap_area(1.0, 2.0, 3.0).unwrap(), 0.0);
    }

    #[test]
    fn internal_tangent_is_full_containment_of_smaller_circle() {
        let area = circle_overlap_area(2.0, 1.0, 1.0).unwrap();
        close(area, PI, 1e-14);
    }

    #[test]
    fn transparent_module_does_not_attenuate_geometrically_covered_rays() {
        let estimate = evaluate_circular_shade(CircularShadeInput {
            module_transmittance_fraction: 1.0,
            ..base_input()
        })
        .unwrap();

        close(estimate.geometric_blocked_fraction, 1.0, 1e-14);
        assert_eq!(estimate.attenuation_fraction, 0.0);
        assert_eq!(estimate.target_transmittance_fraction, 1.0);
    }

    #[test]
    fn partial_module_transmittance_scales_local_attenuation() {
        let estimate = evaluate_circular_shade(CircularShadeInput {
            module_transmittance_fraction: 0.3,
            ..base_input()
        })
        .unwrap();

        close(estimate.attenuation_fraction, 0.7, 1e-14);
        close(estimate.target_transmittance_fraction, 0.3, 1e-14);
    }

    #[test]
    fn finite_sun_projection_uses_both_source_and_target_distances() {
        let estimate = evaluate_circular_shade(CircularShadeInput {
            sun_radius_m: 5.0,
            sun_to_shade_distance_m: 100.0,
            shade_to_target_distance_m: 20.0,
            target_point_xy_m: [12.0, -6.0],
            module_center_xy_m: [0.0, 0.0],
            module_radius_m: 0.0,
            module_transmittance_fraction: 0.0,
        })
        .unwrap();

        close(estimate.projected_sun_radius_at_shade_m, 5.0 / 6.0, 1e-14);
        close(estimate.projected_sun_center_xy_at_shade_m[0], 10.0, 1e-14);
        close(estimate.projected_sun_center_xy_at_shade_m[1], -5.0, 1e-14);
    }

    #[test]
    fn normalized_circle_overlap_handles_large_finite_dimensions() {
        let area = circle_overlap_area(1.0e154, 1.0e154, 1.0e154).unwrap();
        let scaled_area = area / 1.0e308;
        close(scaled_area, 1.228_369_698_608_756_8, 1e-14);
    }

    #[test]
    fn normalized_circle_overlap_handles_small_finite_dimensions() {
        let area = circle_overlap_area(1.0e-150, 1.0e-150, 1.0e-150).unwrap();
        let scaled_area = area / 1.0e-300;
        close(scaled_area, 1.228_369_698_608_756_8, 1e-14);
    }

    #[test]
    fn invalid_geometry_and_non_finite_values_are_rejected() {
        assert!(
            evaluate_circular_shade(CircularShadeInput {
                sun_to_shade_distance_m: 0.0,
                ..base_input()
            })
            .is_err()
        );
        assert!(
            evaluate_circular_shade(CircularShadeInput {
                module_transmittance_fraction: 1.1,
                ..base_input()
            })
            .is_err()
        );
        assert!(
            evaluate_circular_shade(CircularShadeInput {
                target_point_xy_m: [f64::NAN, 0.0],
                ..base_input()
            })
            .is_err()
        );
        assert!(circle_overlap_area(f64::INFINITY, 1.0, 0.0).is_err());
    }
}
