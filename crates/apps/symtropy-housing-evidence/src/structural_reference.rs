// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small, dependency-free analytical reference kernel for elementary linear-static
//! structural cases used by Symtropy's housing evidence tests.
//!
//! All numeric inputs are SI magnitudes (N, m, Pa, m², m⁴, N/m). This API uses
//! field names and documentation to make units visible; it does not provide a
//! compile-time dimensional type system.
//!
//! This module is an analytical reference implementation, not an FEM solver,
//! structural design check, building-code engine, or construction approval.

use std::fmt;

/// Failure mode for an input, intermediate, or computed output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralReferenceErrorKind {
    NonFiniteInput,
    NonPositiveInput,
    NonFiniteIntermediate,
    NonPositiveIntermediate,
    NonFiniteOutput,
    NonPositiveOutput,
}

/// A fail-closed calculation error with the field or intermediate that failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructuralReferenceError {
    pub field: &'static str,
    pub kind: StructuralReferenceErrorKind,
}

impl StructuralReferenceError {
    const fn new(field: &'static str, kind: StructuralReferenceErrorKind) -> Self {
        Self { field, kind }
    }
}

impl fmt::Display for StructuralReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {:?}", self.field, self.kind)
    }
}

impl std::error::Error for StructuralReferenceError {}

type Result<T> = std::result::Result<T, StructuralReferenceError>;

/// Public result type for fail-closed structural reference calculations.
pub type StructuralReferenceResult<T> = std::result::Result<T, StructuralReferenceError>;

fn positive_input(field: &'static str, value: f64) -> Result<f64> {
    if !value.is_finite() {
        return Err(StructuralReferenceError::new(
            field,
            StructuralReferenceErrorKind::NonFiniteInput,
        ));
    }
    if value <= 0.0 {
        return Err(StructuralReferenceError::new(
            field,
            StructuralReferenceErrorKind::NonPositiveInput,
        ));
    }
    Ok(value)
}

fn positive_intermediate(field: &'static str, value: f64) -> Result<f64> {
    if !value.is_finite() {
        return Err(StructuralReferenceError::new(
            field,
            StructuralReferenceErrorKind::NonFiniteIntermediate,
        ));
    }
    if value <= 0.0 {
        return Err(StructuralReferenceError::new(
            field,
            StructuralReferenceErrorKind::NonPositiveIntermediate,
        ));
    }
    Ok(value)
}

fn positive_output(field: &'static str, value: f64) -> Result<f64> {
    if !value.is_finite() {
        return Err(StructuralReferenceError::new(
            field,
            StructuralReferenceErrorKind::NonFiniteOutput,
        ));
    }
    if value <= 0.0 {
        return Err(StructuralReferenceError::new(
            field,
            StructuralReferenceErrorKind::NonPositiveOutput,
        ));
    }
    Ok(value)
}

fn checked_product(field: &'static str, left: f64, right: f64) -> Result<f64> {
    positive_intermediate(field, left * right)
}

fn checked_power3(field: &'static str, value: f64) -> Result<f64> {
    positive_intermediate(field, value.powi(3))
}

fn checked_power4(field: &'static str, value: f64) -> Result<f64> {
    positive_intermediate(field, value.powi(4))
}

fn checked_quotient(field: &'static str, numerator: f64, denominator: f64) -> Result<f64> {
    let numerator = positive_intermediate("numerator", numerator)?;
    let denominator = positive_intermediate("denominator", denominator)?;
    positive_output(field, numerator / denominator)
}

/// Rectangular section properties computed from primitive dimensions in metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RectangularSectionProperties {
    pub width_m: f64,
    pub depth_m: f64,
    pub area_m2: f64,
    pub second_moment_area_m4: f64,
    pub extreme_fiber_distance_m: f64,
}

/// Compute A = b h, I = b h³ / 12, and c = h / 2 from primitive dimensions.
///
/// Formal contract: `geometry.rectangular_section.second_moment.v1`.
pub fn rectangular_section_properties(
    width_m: f64,
    depth_m: f64,
) -> StructuralReferenceResult<RectangularSectionProperties> {
    let width_m = positive_input("section_width_m", width_m)?;
    let depth_m = positive_input("section_depth_m", depth_m)?;
    let area_m2 = checked_product("section_area_m2", width_m, depth_m)?;
    let depth_cubed = checked_power3("section_depth_cubed_m3", depth_m)?;
    let inertia_numerator =
        checked_product("section_inertia_numerator_m4", width_m, depth_cubed)?;
    let second_moment_area_m4 =
        positive_intermediate("second_moment_area_m4", inertia_numerator / 12.0)?;
    let extreme_fiber_distance_m =
        positive_intermediate("extreme_fiber_distance_m", depth_m / 2.0)?;

    Ok(RectangularSectionProperties {
        width_m,
        depth_m,
        area_m2,
        second_moment_area_m4,
        extreme_fiber_distance_m,
    })
}

/// Outputs for a prismatic, uniformly elastic axial bar under centered axial tension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxialBarResult {
    pub section: RectangularSectionProperties,
    pub axial_stress_pa: f64,
    pub free_end_extension_m: f64,
    pub support_reaction_magnitude_n: f64,
}

/// Evaluate sigma = F/A, delta = F L/(E A), and R = F.
/// All loads/material values are positive scalar magnitudes; compression/sign
/// conventions and more complex bar models are intentionally outside this API.
///
/// Formal contract: `mechanics.axial_bar.extension.v1`.
pub fn axial_bar(
    load_n: f64,
    length_m: f64,
    youngs_modulus_pa: f64,
    width_m: f64,
    depth_m: f64,
) -> StructuralReferenceResult<AxialBarResult> {
    let load_n = positive_input("load_N", load_n)?;
    let length_m = positive_input("length_m", length_m)?;
    let youngs_modulus_pa = positive_input("elastic_modulus_Pa", youngs_modulus_pa)?;
    let section = rectangular_section_properties(width_m, depth_m)?;

    let axial_stress_pa = checked_quotient("axial_stress_Pa", load_n, section.area_m2)?;
    let extension_numerator = checked_product("extension_numerator_Nm", load_n, length_m)?;
    let axial_stiffness = checked_product(
        "axial_stiffness_N",
        youngs_modulus_pa,
        section.area_m2,
    )?;
    let free_end_extension_m = checked_quotient(
        "free_end_extension_m",
        extension_numerator,
        axial_stiffness,
    )?;
    let support_reaction_magnitude_n =
        positive_output("support_reaction_magnitude_N", load_n)?;

    Ok(AxialBarResult {
        section,
        axial_stress_pa,
        free_end_extension_m,
        support_reaction_magnitude_n,
    })
}

/// Outputs for an idealized elementary beam analytical reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamResult {
    pub section: RectangularSectionProperties,
    pub maximum_deflection_m: f64,
    pub maximum_bending_moment_nm: f64,
    pub maximum_bending_stress_pa: f64,
    pub support_reaction_magnitude_n: f64,
}

/// Positive-magnitude load cases supported by the simply supported beam reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimplySupportedLoadCase {
    /// Concentrated force at midspan (N).
    MidpointPointLoad { force_n: f64 },
    /// Constant vertical load intensity over the full span (N/m).
    UniformDistributedLoad { load_n_per_m: f64 },
}

fn bending_stress(
    maximum_bending_moment_nm: f64,
    section: RectangularSectionProperties,
) -> Result<f64> {
    let moment_times_fiber =
        checked_product("bending_stress_numerator_Nm2", maximum_bending_moment_nm,
            section.extreme_fiber_distance_m)?;
    checked_quotient(
        "maximum_bending_stress_Pa",
        moment_times_fiber,
        section.second_moment_area_m4,
    )
}

/// Evaluate the cantilever free-end point-load reference:
/// delta = F L³/(3 E I), M = F L, sigma = M c/I.
///
/// Formal contract: `mechanics.cantilever.tip_deflection.v1`.
pub fn cantilever_tip_load(
    load_n: f64,
    span_m: f64,
    youngs_modulus_pa: f64,
    width_m: f64,
    depth_m: f64,
) -> StructuralReferenceResult<BeamResult> {
    let load_n = positive_input("load_N", load_n)?;
    let span_m = positive_input("span_m", span_m)?;
    let youngs_modulus_pa = positive_input("elastic_modulus_Pa", youngs_modulus_pa)?;
    let section = rectangular_section_properties(width_m, depth_m)?;

    let span_cubed = checked_power3("span_cubed_m3", span_m)?;
    let deflection_numerator =
        checked_product("deflection_numerator_Nm3", load_n, span_cubed)?;
    let flexural_stiffness = checked_product(
        "flexural_stiffness_Pa_m4",
        youngs_modulus_pa,
        section.second_moment_area_m4,
    )?;
    let deflection_denominator =
        checked_product("deflection_denominator", 3.0, flexural_stiffness)?;
    let maximum_deflection_m = checked_quotient(
        "maximum_deflection_m",
        deflection_numerator,
        deflection_denominator,
    )?;

    let maximum_bending_moment_nm =
        checked_product("maximum_bending_moment_Nm", load_n, span_m)?;
    let maximum_bending_stress_pa = bending_stress(maximum_bending_moment_nm, section)?;
    let support_reaction_magnitude_n =
        positive_output("support_reaction_magnitude_N", load_n)?;

    Ok(BeamResult {
        section,
        maximum_deflection_m,
        maximum_bending_moment_nm,
        maximum_bending_stress_pa,
        support_reaction_magnitude_n,
    })
}

/// Evaluate an elementary simply supported beam with either midpoint force or
/// uniformly distributed load. Euler-Bernoulli, linear-static, small-deflection
/// assumptions are expected; this is not a general solver.
///
/// Load-family contracts are `mechanics.simply_supported.midpoint_point_load.v1`
/// and `mechanics.simply_supported.uniform_load.v1`.
pub fn simply_supported_beam(
    span_m: f64,
    youngs_modulus_pa: f64,
    width_m: f64,
    depth_m: f64,
    load_case: SimplySupportedLoadCase,
) -> Result<BeamResult> {
    let span_m = positive_input("span_m", span_m)?;
    let youngs_modulus_pa = positive_input("elastic_modulus_Pa", youngs_modulus_pa)?;
    let section = rectangular_section_properties(width_m, depth_m)?;

    let (maximum_deflection_m, maximum_bending_moment_nm, support_reaction_magnitude_n) =
        match load_case {
            SimplySupportedLoadCase::MidpointPointLoad { force_n } => {
                let force_n = positive_input("load_N", force_n)?;
                let span_cubed = checked_power3("span_cubed_m3", span_m)?;
                let numerator =
                    checked_product("midpoint_deflection_numerator_Nm3", force_n, span_cubed)?;
                let flexural_stiffness = checked_product(
                    "flexural_stiffness_Pa_m4",
                    youngs_modulus_pa,
                    section.second_moment_area_m4,
                )?;
                let denominator =
                    checked_product("midpoint_deflection_denominator", 48.0, flexural_stiffness)?;
                let deflection =
                    checked_quotient("maximum_deflection_m", numerator, denominator)?;

                let moment_numerator =
                    checked_product("maximum_moment_numerator_Nm", force_n, span_m)?;
                let moment = positive_intermediate(
                    "maximum_bending_moment_Nm",
                    moment_numerator / 4.0,
                )?;
                let reaction = positive_intermediate(
                    "support_reaction_magnitude_N",
                    force_n / 2.0,
                )?;
                (deflection, moment, reaction)
            }
            SimplySupportedLoadCase::UniformDistributedLoad { load_n_per_m } => {
                let load_n_per_m = positive_input("load_N_per_m", load_n_per_m)?;
                let span_fourth = checked_power4("span_fourth_m4", span_m)?;
                let load_factor =
                    checked_product("uniform_deflection_load_factor", 5.0, load_n_per_m)?;
                let numerator =
                    checked_product("uniform_deflection_numerator_Nm4", load_factor, span_fourth)?;
                let flexural_stiffness = checked_product(
                    "flexural_stiffness_Pa_m4",
                    youngs_modulus_pa,
                    section.second_moment_area_m4,
                )?;
                let denominator =
                    checked_product("uniform_deflection_denominator", 384.0, flexural_stiffness)?;
                let deflection =
                    checked_quotient("maximum_deflection_m", numerator, denominator)?;

                let span_squared = positive_intermediate("span_squared_m2", span_m.powi(2))?;
                let moment_numerator = checked_product(
                    "uniform_moment_numerator_Nm2",
                    load_n_per_m,
                    span_squared,
                )?;
                let moment =
                    positive_intermediate("maximum_bending_moment_Nm", moment_numerator / 8.0)?;
                let reaction_numerator =
                    checked_product("support_reaction_numerator_N", load_n_per_m, span_m)?;
                let reaction = positive_intermediate(
                    "support_reaction_magnitude_N",
                    reaction_numerator / 2.0,
                )?;
                (deflection, moment, reaction)
            }
        };

    let maximum_bending_stress_pa =
        bending_stress(maximum_bending_moment_nm, section)?;

    Ok(BeamResult {
        section,
        maximum_deflection_m,
        maximum_bending_moment_nm,
        maximum_bending_stress_pa,
        support_reaction_magnitude_n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1.0e-12;

    fn close(actual: f64, expected: f64) -> bool {
        actual.is_finite()
            && expected.is_finite()
            && (actual - expected).abs()
                <= TOLERANCE * expected.abs().max(f64::MIN_POSITIVE)
    }

    #[test]
    fn axial_bar_matches_exact_reference_case() {
        let result = axial_bar(12_000.0, 3.0, 200_000_000_000.0, 0.05, 0.1)
            .expect("reference input is valid");
        assert!(close(result.section.area_m2, 0.005));
        assert!(close(result.axial_stress_pa, 2_400_000.0));
        assert!(close(result.free_end_extension_m, 0.000036));
        assert!(close(result.support_reaction_magnitude_n, 12_000.0));
    }

    #[test]
    fn cantilever_matches_exact_reference_case() {
        let result = cantilever_tip_load(1_000.0, 2.0, 200_000_000_000.0, 0.096, 0.1)
            .expect("reference input is valid");
        assert!(close(result.section.second_moment_area_m4, 8.0e-6));
        assert!(close(result.maximum_deflection_m, 1.0 / 600.0));
        assert!(close(result.maximum_bending_stress_pa, 12_500_000.0));
    }

    #[test]
    fn simply_supported_matches_both_load_families() {
        let point = simply_supported_beam(
            4.0,
            200_000_000_000.0,
            0.1,
            0.2,
            SimplySupportedLoadCase::MidpointPointLoad { force_n: 5_000.0 },
        )
        .expect("point-load case is valid");
        assert!(close(point.maximum_deflection_m, 0.0005));
        assert!(close(point.maximum_bending_moment_nm, 5_000.0));
        assert!(close(point.maximum_bending_stress_pa, 7_500_000.0));
        assert!(close(point.support_reaction_magnitude_n, 2_500.0));

        let uniform = simply_supported_beam(
            4.0,
            200_000_000_000.0,
            0.1,
            0.2,
            SimplySupportedLoadCase::UniformDistributedLoad {
                load_n_per_m: 1_000.0,
            },
        )
        .expect("uniform-load case is valid");
        assert!(close(uniform.maximum_deflection_m, 0.00025));
        assert!(close(uniform.maximum_bending_moment_nm, 2_000.0));
        assert!(close(uniform.maximum_bending_stress_pa, 3_000_000.0));
        assert!(close(uniform.support_reaction_magnitude_n, 2_000.0));
    }

    #[test]
    fn nonpositive_and_nonfinite_inputs_fail_closed() {
        assert_eq!(
            axial_bar(0.0, 3.0, 200e9, 0.05, 0.1).unwrap_err().kind,
            StructuralReferenceErrorKind::NonPositiveInput
        );
        assert_eq!(
            axial_bar(f64::NAN, 3.0, 200e9, 0.05, 0.1).unwrap_err().kind,
            StructuralReferenceErrorKind::NonFiniteInput
        );
        assert!(cantilever_tip_load(1_000.0, 0.0, 200e9, 0.096, 0.1).is_err());
        assert!(simply_supported_beam(
            4.0,
            200e9,
            0.1,
            0.2,
            SimplySupportedLoadCase::UniformDistributedLoad { load_n_per_m: -1.0 }
        ).is_err());
    }

    #[test]
    fn overflow_and_underflow_do_not_return_plausible_numbers() {
        assert!(rectangular_section_properties(f64::MAX, f64::MAX).is_err());
        assert!(rectangular_section_properties(f64::MIN_POSITIVE, f64::MIN_POSITIVE).is_err());
        assert!(axial_bar(f64::MAX, f64::MAX, f64::MIN_POSITIVE, 0.05, 0.1).is_err());
    }

    #[test]
    fn scale_similarity_preserves_dimensionless_outputs() {
        let baseline = cantilever_tip_load(1_000.0, 2.0, 200e9, 0.096, 0.1)
            .expect("baseline input valid");
        for scale in [0.5_f64, 0.75, 1.25, 2.0, 4.0] {
            let scaled = cantilever_tip_load(
                1_000.0 * scale.powi(2),
                2.0 * scale,
                200e9,
                0.096 * scale,
                0.1 * scale,
            )
            .expect("similarity transform remains valid");
            assert!(close(
                scaled.maximum_deflection_m / (2.0 * scale),
                baseline.maximum_deflection_m / 2.0
            ));
            assert!(close(scaled.maximum_bending_stress_pa, baseline.maximum_bending_stress_pa));
        }
    }
}
