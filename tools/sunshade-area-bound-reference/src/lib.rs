//! Analytical lower bound on projected shade-mask union area.
//!
//! This reference is intentionally narrower than an optical-transfer solver.
//! It assumes parallel source, shade and target planes; a finite, uniformly
//! bright circular source; straight-line geometric optics; one static common
//! shade plane; and the same scalar module transmittance for every covered ray.
//! It consumes the spatially integrated target-plane attenuation as an input.
//!
//! It does not calculate that attenuation map, model diffraction or scattering,
//! infer the meaning of an abstract effectiveness factor, or estimate mass.

/// Inputs to the common-plane projected-mask-area lower-bound calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaBoundInput {
    /// Source plane to shade plane distance, metres.
    pub source_to_shade_distance_m: f64,
    /// Shade plane to target plane distance, metres.
    pub shade_to_target_distance_m: f64,
    /// Area of the finite target-plane domain, square metres.
    pub target_domain_area_m2: f64,
    /// Integral of local fractional target-plane attenuation over the target
    /// domain, in square metres: integral_T attenuation_fraction(x) dA_target.
    pub integrated_target_attenuation_area_m2: f64,
    /// Common scalar radiant transmittance of every module, in [0, 1).
    pub module_transmittance_fraction: f64,
}

/// Result of the analytical projected-mask-area bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaBoundEstimate {
    /// alpha = source_to_shade / (source_to_shade + shade_to_target).
    pub source_to_shade_fraction: f64,
    /// Mean fractional attenuation over the finite target domain.
    pub mean_target_attenuation_fraction: f64,
    /// Lower bound on the projected union area of the common-plane shade mask,
    /// square metres. This is not a sum of module areas or a system mass.
    pub projected_mask_union_area_lower_bound_m2: f64,
}

/// Compute a lower bound on the common-plane projected shade-mask union area.
///
/// Under the stated assumptions, let M be the union of opaque-equivalent
/// module footprints in the shade plane and B(x) the fraction of the finite
/// solar disk intercepted by M for target point x. Then
///
///     integral over the entire target plane B(x) dA = |M| / alpha^2,
///
/// where alpha = L_source-shade / (L_source-shade + L_shade-target).
///
/// For a finite target domain T, the integral can only be smaller:
///
///     |M| >= alpha^2 * integral_T B(x) dA.
///
/// With uniform module transmittance tau and local fractional attenuation
/// A(x) = (1 - tau) B(x), this becomes
///
///     |M| >= alpha^2 / (1 - tau) * integral_T A(x) dA.
///
/// The bound is on the projected union area of a single common shade plane.
/// A finite-source radiance map must be independently calculated or measured;
/// this function does not provide that map.
pub fn evaluate_projected_mask_area_bound(
    input: AreaBoundInput,
) -> Result<AreaBoundEstimate, &'static str> {
    if !finite_positive(input.source_to_shade_distance_m) {
        return Err("source_to_shade_distance_m must be finite and positive");
    }
    if !finite_positive(input.shade_to_target_distance_m) {
        return Err("shade_to_target_distance_m must be finite and positive");
    }
    if !finite_positive(input.target_domain_area_m2) {
        return Err("target_domain_area_m2 must be finite and positive");
    }
    if !finite_nonnegative(input.integrated_target_attenuation_area_m2) {
        return Err("integrated_target_attenuation_area_m2 must be finite and non-negative");
    }
    if input.integrated_target_attenuation_area_m2 > input.target_domain_area_m2 {
        return Err("integrated attenuation cannot exceed the finite target-domain area");
    }
    if !input.module_transmittance_fraction.is_finite()
        || !(0.0..1.0).contains(&input.module_transmittance_fraction)
    {
        return Err("module_transmittance_fraction must be finite and in [0, 1)");
    }

    // Scale before summation so that two large, finite distances do not
    // overflow merely while calculating their ratio.
    let distance_scale = input
        .source_to_shade_distance_m
        .max(input.shade_to_target_distance_m);
    let scaled_source_distance = input.source_to_shade_distance_m / distance_scale;
    let scaled_target_distance = input.shade_to_target_distance_m / distance_scale;
    let source_to_shade_fraction =
        scaled_source_distance / (scaled_source_distance + scaled_target_distance);

    let mean_target_attenuation_fraction =
        input.integrated_target_attenuation_area_m2 / input.target_domain_area_m2;
    let transmission_loss_fraction = 1.0 - input.module_transmittance_fraction;
    let projected_mask_union_area_lower_bound_m2 = source_to_shade_fraction.powi(2)
        * input.integrated_target_attenuation_area_m2
        / transmission_loss_fraction;

    if !source_to_shade_fraction.is_finite()
        || source_to_shade_fraction <= 0.0
        || source_to_shade_fraction >= 1.0
        || !mean_target_attenuation_fraction.is_finite()
        || !projected_mask_union_area_lower_bound_m2.is_finite()
    {
        return Err("projected mask area bound overflowed its finite domain");
    }

    Ok(AreaBoundEstimate {
        source_to_shade_fraction,
        mean_target_attenuation_fraction,
        projected_mask_union_area_lower_bound_m2,
    })
}

/// Exact full-target-plane integral of geometric blockage for a known mask
/// union area under the same idealized assumptions.
///
/// The finite-source disk radius cancels from this integral identity. It still
/// controls the spatial attenuation/penumbra map and finite-target edge spill.
pub fn full_plane_integrated_blockage_area(
    mask_union_area_m2: f64,
    source_to_shade_distance_m: f64,
    shade_to_target_distance_m: f64,
) -> Result<f64, &'static str> {
    if !finite_nonnegative(mask_union_area_m2) {
        return Err("mask_union_area_m2 must be finite and non-negative");
    }
    let estimate = evaluate_projected_mask_area_bound(AreaBoundInput {
        source_to_shade_distance_m,
        shade_to_target_distance_m,
        target_domain_area_m2: 1.0,
        integrated_target_attenuation_area_m2: 0.0,
        module_transmittance_fraction: 0.0,
    })?;
    let alpha = estimate.source_to_shade_fraction;
    let integrated_blockage_area_m2 = mask_union_area_m2 / alpha.powi(2);
    if !integrated_blockage_area_m2.is_finite() {
        return Err("full-plane blockage integral overflowed its finite domain");
    }
    Ok(integrated_blockage_area_m2)
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
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

    #[test]
    fn opaque_case_uses_the_analytic_finite_target_bound() {
        let estimate = evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 3.0,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: 100.0,
            integrated_target_attenuation_area_m2: 20.0,
            module_transmittance_fraction: 0.0,
        })
        .unwrap();

        close(estimate.source_to_shade_fraction, 0.75, 1e-15);
        close(estimate.mean_target_attenuation_fraction, 0.2, 1e-15);
        close(
            estimate.projected_mask_union_area_lower_bound_m2,
            11.25,
            1e-14,
        );
    }

    #[test]
    fn uniform_transmittance_increases_required_projected_mask_area() {
        let opaque = evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 3.0,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: 100.0,
            integrated_target_attenuation_area_m2: 20.0,
            module_transmittance_fraction: 0.0,
        })
        .unwrap();
        let half_transmitting = evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 3.0,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: 100.0,
            integrated_target_attenuation_area_m2: 20.0,
            module_transmittance_fraction: 0.5,
        })
        .unwrap();

        close(
            half_transmitting.mean_target_attenuation_fraction,
            opaque.mean_target_attenuation_fraction,
            1e-15,
        );
        close(
            half_transmitting.projected_mask_union_area_lower_bound_m2,
            2.0 * opaque.projected_mask_union_area_lower_bound_m2,
            1e-14,
        );
    }

    #[test]
    fn full_plane_integral_is_exactly_mask_area_over_alpha_squared() {
        let mask_area = 7.25;
        let source_distance = 4.0;
        let target_distance = 6.0;
        let integral = full_plane_integrated_blockage_area(
            mask_area,
            source_distance,
            target_distance,
        )
        .unwrap();

        close(integral, mask_area / 0.4_f64.powi(2), 1e-13);
    }

    #[test]
    fn source_and_target_distance_scaling_is_ratio_invariant() {
        let base = evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 3.0,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: 100.0,
            integrated_target_attenuation_area_m2: 20.0,
            module_transmittance_fraction: 0.2,
        })
        .unwrap();
        let scaled = evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 3.0e100,
            shade_to_target_distance_m: 1.0e100,
            target_domain_area_m2: 100.0,
            integrated_target_attenuation_area_m2: 20.0,
            module_transmittance_fraction: 0.2,
        })
        .unwrap();

        close(
            scaled.projected_mask_union_area_lower_bound_m2,
            base.projected_mask_union_area_lower_bound_m2,
            1e-14,
        );
    }

    #[test]
    fn zero_integrated_attenuation_has_zero_area_bound() {
        let estimate = evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 1.0,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: 10.0,
            integrated_target_attenuation_area_m2: 0.0,
            module_transmittance_fraction: 0.9,
        })
        .unwrap();

        assert_eq!(estimate.projected_mask_union_area_lower_bound_m2, 0.0);
    }

    #[test]
    fn invalid_distances_transmission_and_integrals_fail_closed() {
        let valid = AreaBoundInput {
            source_to_shade_distance_m: 1.0,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: 10.0,
            integrated_target_attenuation_area_m2: 1.0,
            module_transmittance_fraction: 0.0,
        };
        assert!(evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 0.0,
            ..valid
        })
        .is_err());
        assert!(evaluate_projected_mask_area_bound(AreaBoundInput {
            target_domain_area_m2: f64::NAN,
            ..valid
        })
        .is_err());
        assert!(evaluate_projected_mask_area_bound(AreaBoundInput {
            shade_to_target_distance_m: f64::INFINITY,
            ..valid
        })
        .is_err());
        assert!(evaluate_projected_mask_area_bound(AreaBoundInput {
            integrated_target_attenuation_area_m2: f64::NAN,
            ..valid
        })
        .is_err());
        assert!(evaluate_projected_mask_area_bound(AreaBoundInput {
            module_transmittance_fraction: 1.0,
            ..valid
        })
        .is_err());
        assert!(evaluate_projected_mask_area_bound(AreaBoundInput {
            module_transmittance_fraction: -0.1,
            ..valid
        })
        .is_err());
    }

    #[test]
    fn integrated_attenuation_cannot_exceed_target_domain_area() {
        let invalid = AreaBoundInput {
            source_to_shade_distance_m: 1.0,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: 10.0,
            integrated_target_attenuation_area_m2: 10.1,
            module_transmittance_fraction: 0.0,
        };
        assert!(evaluate_projected_mask_area_bound(invalid).is_err());
    }

    fn independent_circle_overlap_area(first_radius: f64, second_radius: f64, separation: f64) -> f64 {
        if separation >= first_radius + second_radius {
            return 0.0;
        }
        if separation <= (first_radius - second_radius).abs() {
            return std::f64::consts::PI * first_radius.min(second_radius).powi(2);
        }

        let first_cosine = ((separation.powi(2) + first_radius.powi(2)
            - second_radius.powi(2))
            / (2.0 * separation * first_radius))
            .clamp(-1.0, 1.0);
        let second_cosine = ((separation.powi(2) + second_radius.powi(2)
            - first_radius.powi(2))
            / (2.0 * separation * second_radius))
            .clamp(-1.0, 1.0);
        let radicand = (-separation + first_radius + second_radius)
            * (separation + first_radius - second_radius)
            * (separation - first_radius + second_radius)
            * (separation + first_radius + second_radius);

        first_radius.powi(2) * first_cosine.acos()
            + second_radius.powi(2) * second_cosine.acos()
            - 0.5 * radicand.max(0.0).sqrt()
    }

    fn integrate_disk_blockage_simpson(
        projected_sun_radius: f64,
        mask_radius: f64,
        alpha: f64,
        target_radius: f64,
        intervals: usize,
    ) -> f64 {
        assert!(intervals > 0 && (intervals & 1) == 0);
        let step = target_radius / intervals as f64;
        let sun_area = std::f64::consts::PI * projected_sun_radius.powi(2);
        let mut weighted_sum = 0.0;

        for index in 0..=intervals {
            let target_radius_at_point = index as f64 * step;
            let shade_plane_separation = alpha * target_radius_at_point;
            let overlap = independent_circle_overlap_area(
                projected_sun_radius,
                mask_radius,
                shade_plane_separation,
            );
            let blocked_fraction = overlap / sun_area;
            let radial_integrand =
                2.0 * std::f64::consts::PI * target_radius_at_point * blocked_fraction;
            let weight = if index == 0 || index == intervals {
                1.0
            } else if index & 1 == 0 {
                2.0
            } else {
                4.0
            };
            weighted_sum += weight * radial_integrand;
        }

        weighted_sum * step / 3.0
    }

    #[test]
    fn independent_radial_quadrature_recovers_full_plane_integral_identity() {
        let alpha: f64 = 0.75;
        let projected_sun_radius: f64 = 0.2;
        let mask_radius: f64 = 0.1;
        let support_radius = (projected_sun_radius + mask_radius) / alpha;
        let numerical_integral = integrate_disk_blockage_simpson(
            projected_sun_radius,
            mask_radius,
            alpha,
            support_radius,
            20_000,
        );
        let exact_integral =
            std::f64::consts::PI * mask_radius.powi(2) / alpha.powi(2);

        close(numerical_integral, exact_integral, 1.0e-10);
    }

    #[test]
    fn finite_target_quadrature_produces_a_valid_area_lower_bound() {
        let alpha: f64 = 0.75;
        let projected_sun_radius: f64 = 0.2;
        let mask_radius: f64 = 0.1;
        let finite_target_radius = 0.15;
        let integrated_blockage = integrate_disk_blockage_simpson(
            projected_sun_radius,
            mask_radius,
            alpha,
            finite_target_radius,
            20_000,
        );
        let projected_area_lower_bound = alpha.powi(2) * integrated_blockage;
        let actual_mask_union_area = std::f64::consts::PI * mask_radius.powi(2);

        assert!(projected_area_lower_bound > 0.0);
        assert!(projected_area_lower_bound <= actual_mask_union_area * (1.0 + 1.0e-10));
    }

    #[test]
    fn unrepresentable_bound_is_rejected() {
        let estimate = evaluate_projected_mask_area_bound(AreaBoundInput {
            source_to_shade_distance_m: 1.0e-200,
            shade_to_target_distance_m: 1.0,
            target_domain_area_m2: f64::MAX,
            integrated_target_attenuation_area_m2: f64::MAX,
            module_transmittance_fraction: 0.0,
        });
        assert!(estimate.is_err());
    }
}
