// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small, dependency-free analytical reference kernel for elementary linear-static
//! structural cases used by Symtropy's housing evidence tests.
//!
//! All numeric inputs are SI magnitudes (N, m, Pa, m², m⁴, N/m). This API uses
//! field names and private-payload newtypes provide compile-time distinctions
//! among supported SI quantity categories. Constructors validate positive finite
//! magnitudes; they do not perform unit conversion or model signed loads.
//!
//! This module is an analytical reference implementation, not an FEM solver,
//! structural design check, building-code engine, or construction approval.
//!
//! Quantity newtypes prevent category mistakes at compile time. For example,
//! the following deliberately swapped API call must fail to compile:
//!
//! ```compile_fail
//! use symtropy_housing_evidence::structural_reference::{axial_bar, Meters, Newtons, Pascals};
//! let load = Newtons::try_from_si(12_000.0).unwrap();
//! let length = Meters::try_from_si(3.0).unwrap();
//! let modulus = Pascals::try_from_si(200_000_000_000.0).unwrap();
//! let width = Meters::try_from_si(0.05).unwrap();
//! let depth = Meters::try_from_si(0.1).unwrap();
//! // The first two arguments are intentionally reversed.
//! let _ = axial_bar(length, load, modulus, width, depth);
//! ```

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

/// Define a positive finite SI-magnitude type with a checked constructor.
/// The private payload prevents callers from bypassing validation.
macro_rules! define_positive_si_quantity {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name(f64);

        impl $name {
            /// Construct from a positive finite magnitude already expressed in SI.
            pub fn try_from_si(value: f64) -> StructuralReferenceResult<Self> {
                positive_input($label, value).map(Self)
            }

            /// Return the represented SI magnitude.
            pub const fn get(self) -> f64 {
                self.0
            }
        }
    };
}

define_positive_si_quantity!(Meters, "meters");
define_positive_si_quantity!(SquareMeters, "square_meters");
define_positive_si_quantity!(FourthMomentOfArea, "fourth_moment_of_area");
define_positive_si_quantity!(Newtons, "newtons");
define_positive_si_quantity!(Pascals, "pascals");
define_positive_si_quantity!(NewtonsPerMeter, "newtons_per_meter");
define_positive_si_quantity!(NewtonMeters, "newton_meters");

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
    pub width_m: Meters,
    pub depth_m: Meters,
    pub area_m2: SquareMeters,
    pub second_moment_area_m4: FourthMomentOfArea,
    pub extreme_fiber_distance_m: Meters,
}

/// Compute A = b h, I = b h³ / 12, and c = h / 2 from primitive dimensions.
///
/// Formal contract: `geometry.rectangular_section.second_moment.v1`.
pub fn rectangular_section_properties(
    width_m: Meters,
    depth_m: Meters,
) -> StructuralReferenceResult<RectangularSectionProperties> {
    let width_m = positive_input("section_width_m", width_m.get())?;
    let depth_m = positive_input("section_depth_m", depth_m.get())?;
    let area_m2 = checked_product("section_area_m2", width_m, depth_m)?;
    let depth_cubed = checked_power3("section_depth_cubed_m3", depth_m)?;
    let inertia_numerator =
        checked_product("section_inertia_numerator_m4", width_m, depth_cubed)?;
    let second_moment_area_m4 =
        positive_intermediate("second_moment_area_m4", inertia_numerator / 12.0)?;
    let extreme_fiber_distance_m =
        positive_intermediate("extreme_fiber_distance_m", depth_m / 2.0)?;

    Ok(RectangularSectionProperties {
        width_m: Meters(width_m),
        depth_m: Meters(depth_m),
        area_m2: SquareMeters(area_m2),
        second_moment_area_m4: FourthMomentOfArea(second_moment_area_m4),
        extreme_fiber_distance_m: Meters(extreme_fiber_distance_m),
    })
}

/// Outputs for a prismatic, uniformly elastic axial bar under centered axial tension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxialBarResult {
    pub section: RectangularSectionProperties,
    pub axial_stress_pa: Pascals,
    pub free_end_extension_m: Meters,
    pub support_reaction_magnitude_n: Newtons,
}

/// Evaluate sigma = F/A, delta = F L/(E A), and R = F.
/// All loads/material values are positive scalar magnitudes; compression/sign
/// conventions and more complex bar models are intentionally outside this API.
///
/// Formal contract: `mechanics.axial_bar.extension.v1`.
pub fn axial_bar(
    load_n: Newtons,
    length_m: Meters,
    youngs_modulus_pa: Pascals,
    width_m: Meters,
    depth_m: Meters,
) -> StructuralReferenceResult<AxialBarResult> {
    let load_n = positive_input("load_N", load_n.get())?;
    let length_m = positive_input("length_m", length_m.get())?;
    let youngs_modulus_pa = positive_input("elastic_modulus_Pa", youngs_modulus_pa.get())?;
    let section = rectangular_section_properties(width_m, depth_m)?;

    let axial_stress_pa = checked_quotient("axial_stress_Pa", load_n, section.area_m2.get())?;
    let extension_numerator = checked_product("extension_numerator_Nm", load_n, length_m)?;
    let axial_stiffness = checked_product(
        "axial_stiffness_N",
        youngs_modulus_pa,
        section.area_m2.get(),
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
        axial_stress_pa: Pascals(axial_stress_pa),
        free_end_extension_m: Meters(free_end_extension_m),
        support_reaction_magnitude_n: Newtons(support_reaction_magnitude_n),
    })
}

/// Outputs for an idealized elementary beam analytical reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamResult {
    pub section: RectangularSectionProperties,
    pub maximum_deflection_m: Meters,
    pub maximum_bending_moment_nm: NewtonMeters,
    pub maximum_bending_stress_pa: Pascals,
    pub support_reaction_magnitude_n: Newtons,
}

/// Positive-magnitude load cases supported by the simply supported beam reference.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimplySupportedLoadCase {
    /// Concentrated force at midspan (N).
    MidpointPointLoad { force_n: Newtons },
    /// Constant vertical load intensity over the full span (N/m).
    UniformDistributedLoad { load_n_per_m: NewtonsPerMeter },
}

fn bending_stress(
    maximum_bending_moment_nm: f64,
    section: RectangularSectionProperties,
) -> Result<f64> {
    let moment_times_fiber = checked_product(
        "bending_stress_numerator_Nm2",
        maximum_bending_moment_nm,
        section.extreme_fiber_distance_m.get(),
    )?;
    checked_quotient(
        "maximum_bending_stress_Pa",
        moment_times_fiber,
        section.second_moment_area_m4.get(),
    )
}

/// Evaluate the cantilever free-end point-load reference:
/// delta = F L³/(3 E I), M = F L, sigma = M c/I.
///
/// Formal contract: `mechanics.cantilever.tip_deflection.v1`.
pub fn cantilever_tip_load(
    load_n: Newtons,
    span_m: Meters,
    youngs_modulus_pa: Pascals,
    width_m: Meters,
    depth_m: Meters,
) -> StructuralReferenceResult<BeamResult> {
    let load_n = positive_input("load_N", load_n.get())?;
    let span_m = positive_input("span_m", span_m.get())?;
    let youngs_modulus_pa = positive_input("elastic_modulus_Pa", youngs_modulus_pa.get())?;
    let section = rectangular_section_properties(width_m, depth_m)?;

    let span_cubed = checked_power3("span_cubed_m3", span_m)?;
    let deflection_numerator =
        checked_product("deflection_numerator_Nm3", load_n, span_cubed)?;
    let flexural_stiffness = checked_product(
        "flexural_stiffness_Pa_m4",
        youngs_modulus_pa,
        section.second_moment_area_m4.get(),
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
        maximum_deflection_m: Meters(maximum_deflection_m),
        maximum_bending_moment_nm: NewtonMeters(maximum_bending_moment_nm),
        maximum_bending_stress_pa: Pascals(maximum_bending_stress_pa),
        support_reaction_magnitude_n: Newtons(support_reaction_magnitude_n),
    })
}

/// Evaluate an elementary simply supported beam with either midpoint force or
/// uniformly distributed load. Euler-Bernoulli, linear-static, small-deflection
/// assumptions are expected; this is not a general solver.
///
/// Load-family contracts are `mechanics.simply_supported.midpoint_point_load.v1`
/// and `mechanics.simply_supported.uniform_load.v1`.
pub fn simply_supported_beam(
    span_m: Meters,
    youngs_modulus_pa: Pascals,
    width_m: Meters,
    depth_m: Meters,
    load_case: SimplySupportedLoadCase,
) -> StructuralReferenceResult<BeamResult> {
    let span_m = positive_input("span_m", span_m.get())?;
    let youngs_modulus_pa = positive_input("elastic_modulus_Pa", youngs_modulus_pa.get())?;
    let section = rectangular_section_properties(width_m, depth_m)?;

    let (maximum_deflection_m, maximum_bending_moment_nm, support_reaction_magnitude_n) =
        match load_case {
            SimplySupportedLoadCase::MidpointPointLoad { force_n } => {
                let force_n = positive_input("load_N", force_n.get())?;
                let span_cubed = checked_power3("span_cubed_m3", span_m)?;
                let numerator =
                    checked_product("midpoint_deflection_numerator_Nm3", force_n, span_cubed)?;
                let flexural_stiffness = checked_product(
                    "flexural_stiffness_Pa_m4",
                    youngs_modulus_pa,
                    section.second_moment_area_m4.get(),
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
                let load_n_per_m = positive_input("load_N_per_m", load_n_per_m.get())?;
                let span_fourth = checked_power4("span_fourth_m4", span_m)?;
                let load_factor =
                    checked_product("uniform_deflection_load_factor", 5.0, load_n_per_m)?;
                let numerator =
                    checked_product("uniform_deflection_numerator_Nm3", load_factor, span_fourth)?;
                let flexural_stiffness = checked_product(
                    "flexural_stiffness_Pa_m4",
                    youngs_modulus_pa,
                    section.second_moment_area_m4.get(),
                )?;
                let denominator =
                    checked_product("uniform_deflection_denominator", 384.0, flexural_stiffness)?;
                let deflection =
                    checked_quotient("maximum_deflection_m", numerator, denominator)?;

                let span_squared = positive_intermediate("span_squared_m2", span_m.powi(2))?;
                let moment_numerator = checked_product(
                    "uniform_moment_numerator_Nm",
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
        maximum_deflection_m: Meters(maximum_deflection_m),
        maximum_bending_moment_nm: NewtonMeters(maximum_bending_moment_nm),
        maximum_bending_stress_pa: Pascals(maximum_bending_stress_pa),
        support_reaction_magnitude_n: Newtons(support_reaction_magnitude_n),
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

    fn m(value: f64) -> Meters {
        Meters::try_from_si(value).expect("positive SI length")
    }

    fn n(value: f64) -> Newtons {
        Newtons::try_from_si(value).expect("positive SI force")
    }

    fn pa(value: f64) -> Pascals {
        Pascals::try_from_si(value).expect("positive SI pressure")
    }

    fn n_per_m(value: f64) -> NewtonsPerMeter {
        NewtonsPerMeter::try_from_si(value).expect("positive SI line load")
    }

    #[test]
    fn axial_bar_matches_exact_reference_case() {
        let result = axial_bar(n(12_000.0), m(3.0), pa(200_000_000_000.0), m(0.05), m(0.1))
            .expect("reference input is valid");
        assert!(close(result.section.area_m2.get(), 0.005));
        assert!(close(result.axial_stress_pa.get(), 2_400_000.0));
        assert!(close(result.free_end_extension_m.get(), 0.000036));
        assert!(close(result.support_reaction_magnitude_n.get(), 12_000.0));
    }

    #[test]
    fn cantilever_matches_exact_reference_case() {
        let result = cantilever_tip_load(
            n(1_000.0),
            m(2.0),
            pa(200_000_000_000.0),
            m(0.096),
            m(0.1),
        )
        .expect("reference input is valid");
        assert!(close(result.section.second_moment_area_m4.get(), 8.0e-6));
        assert!(close(result.maximum_deflection_m.get(), 1.0 / 600.0));
        assert!(close(result.maximum_bending_stress_pa.get(), 12_500_000.0));
    }

    #[test]
    fn simply_supported_matches_both_load_families() {
        let point = simply_supported_beam(
            m(4.0),
            pa(200_000_000_000.0),
            m(0.1),
            m(0.2),
            SimplySupportedLoadCase::MidpointPointLoad { force_n: n(5_000.0) },
        )
        .expect("point-load case is valid");
        assert!(close(point.maximum_deflection_m.get(), 0.0005));
        assert!(close(point.maximum_bending_moment_nm.get(), 5_000.0));
        assert!(close(point.maximum_bending_stress_pa.get(), 7_500_000.0));
        assert!(close(point.support_reaction_magnitude_n.get(), 2_500.0));

        let uniform = simply_supported_beam(
            m(4.0),
            pa(200_000_000_000.0),
            m(0.1),
            m(0.2),
            SimplySupportedLoadCase::UniformDistributedLoad {
                load_n_per_m: n_per_m(1_000.0),
            },
        )
        .expect("uniform-load case is valid");
        assert!(close(uniform.maximum_deflection_m.get(), 0.00025));
        assert!(close(uniform.maximum_bending_moment_nm.get(), 2_000.0));
        assert!(close(uniform.maximum_bending_stress_pa.get(), 3_000_000.0));
        assert!(close(uniform.support_reaction_magnitude_n.get(), 2_000.0));
    }

    #[test]
    fn invalid_unit_magnitudes_fail_before_they_enter_equations() {
        assert_eq!(
            Newtons::try_from_si(0.0).unwrap_err().kind,
            StructuralReferenceErrorKind::NonPositiveInput
        );
        assert_eq!(
            Meters::try_from_si(f64::NAN).unwrap_err().kind,
            StructuralReferenceErrorKind::NonFiniteInput
        );
        assert_eq!(
            Pascals::try_from_si(-1.0).unwrap_err().kind,
            StructuralReferenceErrorKind::NonPositiveInput
        );
        assert_eq!(
            NewtonsPerMeter::try_from_si(f64::INFINITY).unwrap_err().kind,
            StructuralReferenceErrorKind::NonFiniteInput
        );
        assert_eq!(
            SquareMeters::try_from_si(0.0).unwrap_err().kind,
            StructuralReferenceErrorKind::NonPositiveInput
        );
        assert_eq!(
            FourthMomentOfArea::try_from_si(-1.0).unwrap_err().kind,
            StructuralReferenceErrorKind::NonPositiveInput
        );
        assert_eq!(
            NewtonMeters::try_from_si(f64::NEG_INFINITY).unwrap_err().kind,
            StructuralReferenceErrorKind::NonFiniteInput
        );
    }

    #[test]
    fn overflow_and_underflow_do_not_return_plausible_numbers() {
        let huge = m(f64::MAX);
        let tiny = m(f64::MIN_POSITIVE);
        assert!(rectangular_section_properties(huge, huge).is_err());
        assert!(rectangular_section_properties(tiny, tiny).is_err());
        assert!(axial_bar(
            n(f64::MAX),
            huge,
            pa(f64::MIN_POSITIVE),
            m(0.05),
            m(0.1)
        )
        .is_err());
    }

    #[test]
    fn scale_similarity_preserves_dimensionless_outputs() {
        let baseline = cantilever_tip_load(n(1_000.0), m(2.0), pa(200e9), m(0.096), m(0.1))
            .expect("baseline input valid");
        for scale in [0.5_f64, 0.75, 1.25, 2.0, 4.0] {
            let scaled = cantilever_tip_load(
                n(1_000.0 * scale.powi(2)),
                m(2.0 * scale),
                pa(200e9),
                m(0.096 * scale),
                m(0.1 * scale),
            )
            .expect("similarity transform remains valid");
            assert!(close(
                scaled.maximum_deflection_m.get() / (2.0 * scale),
                baseline.maximum_deflection_m.get() / 2.0
            ));
            assert!(close(
                scaled.maximum_bending_stress_pa.get(),
                baseline.maximum_bending_stress_pa.get()
            ));
        }
    }
}
