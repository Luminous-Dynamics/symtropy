// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent two-DOF analytical oracle for the published NAFEMS NBC05
//! single-element-per-member axial-truss frequency table.
//!
//! This is a deliberately narrow reproduction of the reported axial-bar
//! model, not a model of the full physical structure. It excludes bending,
//! rotary inertia, joint flexibility, material nonlinearity and damping.

use crate::Check;
use crate::modal_bar::MassMatrixFormulation;

/// Parameters transcribed from the NAFEMS NBC05 figure with explicitly stated
/// assumptions. The section annotation is interpreted as a circular radius;
/// for this same-section axial-only frequency calculation, the area cancels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nbc05AxialTrussParameters {
    pub top_span_m: f64,
    pub vertical_offset_m: f64,
    pub youngs_modulus_pa: f64,
    pub poisson_ratio: f64,
    pub density_kg_m3: f64,
    pub section_radius_m: f64,
}

/// Frequencies and reduced-system diagnostics for the two free-joint modes.
#[derive(Clone, Debug, PartialEq)]
pub struct Nbc05AxialTrussResult {
    /// Frequencies in ascending order (Hz).
    pub frequencies_hz: [f64; 2],
    /// Reduced stiffness matrix for the free joint (N/m).
    pub reduced_stiffness_n_per_m: [[f64; 2]; 2],
    /// Scalar effective translational mass applied to each free-joint DOF (kg).
    pub effective_mass_kg: f64,
    /// Circular section area inferred from the annotated radius (m²).
    pub section_area_m2: f64,
    /// Horizontal and diagonal member lengths (m).
    pub member_lengths_m: [f64; 2],
    /// Relative errors against the rounded reported CS1/CS2 table values.
    pub reference_relative_errors: [f64; 2],
}

/// Parameters as read from the official NAFEMS NBC05 figure:
/// 1 m top span, 1 m vertical offset, E=210 GPa, nu=0.3,
/// rho=7800 kg/m³ and section annotation r=0.01 m.
pub fn published_nbc05_parameters() -> Nbc05AxialTrussParameters {
    Nbc05AxialTrussParameters {
        top_span_m: 1.0,
        vertical_offset_m: 1.0,
        youngs_modulus_pa: 210.0e9,
        poisson_ratio: 0.3,
        density_kg_m3: 7800.0,
        section_radius_m: 0.01,
    }
}

/// Canonical source-attributed input string, stable across platforms.
pub fn canonical_request(parameters: Nbc05AxialTrussParameters) -> String {
    format!(
        concat!(
            "schema=nafems-nbc05-axial-truss-request-v0.1",
            "|fixture=PHYS-NAFEMS-NBC05-AXIAL-001",
            "|geometry=top_span_m:{},vertical_offset_m:{}",
            "|node_A=(0,vertical_offset),fixed_x=true,fixed_y=true",
            "|node_B=(0,0),fixed_x=true,fixed_y=true",
            "|node_C=(top_span,vertical_offset),fixed_x=false,fixed_y=false",
            "|member_AC=horizontal|member_BC=diagonal",
            "|youngs_modulus_pa={}|poisson_ratio={}",
            "|density_kg_m3={}|section_radius_m={}",
            "|mass_formulations=consistent,lumped",
            "|reference_cs1_hz=452.09,1114.6|reference_cs2_hz=369.13,910.05",
            "|reference_model=single_element_per_member_axial_bar_only",
            "|claim_ceiling=reported_table_reproduction_not_bending_or_physical_validation"
        ),
        parameters.top_span_m,
        parameters.vertical_offset_m,
        parameters.youngs_modulus_pa,
        parameters.poisson_ratio,
        parameters.density_kg_m3,
        parameters.section_radius_m
    )
}

/// Compute the two generalized eigenfrequencies without calling the production
/// generalized-eigenvalue solver. The reduced 2x2 stiffness eigenvalues are
/// evaluated analytically from trace and discriminant; both mass formulations
/// yield a scalar multiple of the identity at the one free joint.
pub fn solve_published_axial_truss(
    parameters: Nbc05AxialTrussParameters,
    mass_formulation: MassMatrixFormulation,
) -> Check<Nbc05AxialTrussResult> {
    validate_positive("top_span_m", parameters.top_span_m)?;
    validate_positive("vertical_offset_m", parameters.vertical_offset_m)?;
    validate_positive("youngs_modulus_pa", parameters.youngs_modulus_pa)?;
    validate_positive("density_kg_m3", parameters.density_kg_m3)?;
    validate_positive("section_radius_m", parameters.section_radius_m)?;
    if !parameters.poisson_ratio.is_finite()
        || parameters.poisson_ratio <= -1.0
        || parameters.poisson_ratio >= 0.5
    {
        return Err("poisson_ratio must be finite and inside (-1, 0.5)".to_owned());
    }

    let area_m2 = std::f64::consts::PI * parameters.section_radius_m.powi(2);
    validate_positive("section_area_m2", area_m2)?;
    let horizontal_length = parameters.top_span_m;
    let diagonal_length = parameters.top_span_m.hypot(parameters.vertical_offset_m);
    validate_positive("diagonal_length_m", diagonal_length)?;

    // Supports A=(0,h) and B=(0,0) are fixed in x/y; joint C=(span,h)
    // is the only free joint. Member AC is horizontal; member BC is diagonal.
    let cosine = parameters.top_span_m / diagonal_length;
    let sine = parameters.vertical_offset_m / diagonal_length;
    let axial_factor = parameters.youngs_modulus_pa * area_m2;
    let horizontal_factor = axial_factor / horizontal_length;
    let diagonal_factor = axial_factor / diagonal_length;

    let kxx = horizontal_factor + diagonal_factor * cosine * cosine;
    let kxy = diagonal_factor * cosine * sine;
    let kyy = diagonal_factor * sine * sine;
    let stiffness = [[kxx, kxy], [kxy, kyy]];
    let trace = kxx + kyy;
    let discriminant = (kxx - kyy).hypot(2.0 * kxy);
    let stiffness_eigenvalues = [0.5 * (trace - discriminant), 0.5 * (trace + discriminant)];
    if stiffness_eigenvalues
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("reduced stiffness must have two finite positive eigenvalues".to_owned());
    }

    let mass_multiplier = match mass_formulation {
        // Consistent 2-node translational mass contributes m/3 per free-node
        // translational DOF for each member.
        MassMatrixFormulation::Consistent => 1.0 / 3.0,
        // Row-sum lumped translational mass contributes m/2 per free-node DOF.
        MassMatrixFormulation::Lumped => 1.0 / 2.0,
    };
    let effective_mass = parameters.density_kg_m3
        * area_m2
        * (horizontal_length + diagonal_length)
        * mass_multiplier;
    validate_positive("effective_mass_kg", effective_mass)?;

    let frequencies = stiffness_eigenvalues.map(|stiffness_eigenvalue| {
        (stiffness_eigenvalue / effective_mass).sqrt() / (2.0 * std::f64::consts::PI)
    });
    if frequencies
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("frequency result must be finite and positive".to_owned());
    }

    let targets = match mass_formulation {
        MassMatrixFormulation::Consistent => [452.09, 1114.6],
        MassMatrixFormulation::Lumped => [369.13, 910.05],
    };
    let reference_relative_errors = [
        (frequencies[0] - targets[0]).abs() / targets[0],
        (frequencies[1] - targets[1]).abs() / targets[1],
    ];

    Ok(Nbc05AxialTrussResult {
        frequencies_hz: frequencies,
        reduced_stiffness_n_per_m: stiffness,
        effective_mass_kg: effective_mass,
        section_area_m2: area_m2,
        member_lengths_m: [horizontal_length, diagonal_length],
        reference_relative_errors,
    })
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and positive"))
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_request, published_nbc05_parameters, solve_published_axial_truss};
    use crate::modal_bar::MassMatrixFormulation;

    const REFERENCE_RELATIVE_TOLERANCE: f64 = 1.0e-4;

    #[test]
    fn reported_nbc05_cs1_cs2_frequency_table_matches_analytic_two_dof_oracle() {
        let parameters = published_nbc05_parameters();
        let consistent = solve_published_axial_truss(parameters, MassMatrixFormulation::Consistent)
            .expect("consistent-mass axial truss");
        let lumped = solve_published_axial_truss(parameters, MassMatrixFormulation::Lumped)
            .expect("lumped-mass axial truss");

        assert!((consistent.member_lengths_m[0] - 1.0).abs() < 1.0e-14);
        assert!((consistent.member_lengths_m[1] - 2.0_f64.sqrt()).abs() < 1.0e-14);
        for error in consistent.reference_relative_errors {
            assert!(
                error < REFERENCE_RELATIVE_TOLERANCE,
                "CS1 relative error {error:.17e} exceeds pinned tolerance"
            );
        }
        for error in lumped.reference_relative_errors {
            assert!(
                error < REFERENCE_RELATIVE_TOLERANCE,
                "CS2 relative error {error:.17e} exceeds pinned tolerance"
            );
        }
        assert!((consistent.frequencies_hz[0] - 452.0919432369085).abs() < 1.0e-8);
        assert!((consistent.frequencies_hz[1] - 1114.5800375516237).abs() < 1.0e-8);
        assert!((lumped.frequencies_hz[0] - 369.1315259179073).abs() < 1.0e-8);
        assert!((lumped.frequencies_hz[1] - 910.0507898311973).abs() < 1.0e-8);
    }

    #[test]
    fn consistent_to_lumped_frequency_ratio_is_sqrt_two_thirds() {
        let parameters = published_nbc05_parameters();
        let consistent = solve_published_axial_truss(parameters, MassMatrixFormulation::Consistent)
            .expect("consistent");
        let lumped =
            solve_published_axial_truss(parameters, MassMatrixFormulation::Lumped).expect("lumped");
        let expected_ratio = (2.0_f64 / 3.0).sqrt();
        for mode in 0..2 {
            assert!(
                ((lumped.frequencies_hz[mode] / consistent.frequencies_hz[mode]) - expected_ratio)
                    .abs()
                    < 1.0e-12
            );
        }
    }

    #[test]
    fn uniform_area_change_does_not_change_axial_only_modes() {
        let mut parameters = published_nbc05_parameters();
        let baseline = solve_published_axial_truss(parameters, MassMatrixFormulation::Consistent)
            .expect("baseline");
        parameters.section_radius_m *= 100.0;
        let scaled = solve_published_axial_truss(parameters, MassMatrixFormulation::Consistent)
            .expect("scaled section");
        for mode in 0..2 {
            assert!(
                ((scaled.frequencies_hz[mode] - baseline.frequencies_hz[mode])
                    / baseline.frequencies_hz[mode])
                    .abs()
                    < 1.0e-12
            );
        }
    }

    #[test]
    fn axial_truss_frequencies_obey_modulus_density_and_length_scaling() {
        let parameters = published_nbc05_parameters();
        let baseline = solve_published_axial_truss(
            parameters,
            MassMatrixFormulation::Consistent,
        )
        .expect("baseline");
        let mut four_times_modulus = parameters;
        four_times_modulus.youngs_modulus_pa *= 4.0;
        let stiff = solve_published_axial_truss(
            four_times_modulus,
            MassMatrixFormulation::Consistent,
        )
        .expect("four-times modulus");
        let mut four_times_density = parameters;
        four_times_density.density_kg_m3 *= 4.0;
        let dense = solve_published_axial_truss(
            four_times_density,
            MassMatrixFormulation::Consistent,
        )
        .expect("four-times density");
        let mut double_geometry = parameters;
        double_geometry.top_span_m *= 2.0;
        double_geometry.vertical_offset_m *= 2.0;
        let long = solve_published_axial_truss(
            double_geometry,
            MassMatrixFormulation::Consistent,
        )
        .expect("double geometry");

        for mode in 0..2 {
            assert!(
                ((stiff.frequencies_hz[mode] / baseline.frequencies_hz[mode]) - 2.0).abs()
                    < 1.0e-12
            );
            assert!(
                ((dense.frequencies_hz[mode] / baseline.frequencies_hz[mode]) - 0.5).abs()
                    < 1.0e-12
            );
            assert!(
                ((long.frequencies_hz[mode] / baseline.frequencies_hz[mode]) - 0.5).abs()
                    < 1.0e-12
            );
        }
    }

    #[test]
    fn canonical_request_matches_frozen_source_fixture() {
        let actual = canonical_request(published_nbc05_parameters());
        let frozen = include_str!("../fixtures/nafems_nbc05_axial_truss_request_v0_1.txt")
            .trim_end_matches('\n');
        assert_eq!(actual, frozen);
    }

    #[test]
    fn invalid_geometry_and_material_domains_fail_closed() {
        let mut parameters = published_nbc05_parameters();
        parameters.top_span_m = 0.0;
        assert!(
            solve_published_axial_truss(parameters, MassMatrixFormulation::Consistent).is_err()
        );
        parameters = published_nbc05_parameters();
        parameters.poisson_ratio = 0.5;
        assert!(
            solve_published_axial_truss(parameters, MassMatrixFormulation::Consistent).is_err()
        );
    }
}
