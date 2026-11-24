// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic CSV smoke tournament for the aggregate planar patch oracle.

use planar_patch_limit_surface_oracle::{
    MaxDissipationResult, MaxDissipationTolerance, PlanarPatchModel, SlipTwist3, WrenchDirection3,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let patch = PlanarPatchModel {
        friction_coefficient: 0.5,
        normal_load: 2.0,
        ellipsoid_scales: [1.0, 0.8, 0.25],
    };

    println!(
        "schema,query,support_value,support_u,support_v,support_torsion,slide_u,slide_v,slide_torsion,power,dissipation,boundary_radius,disposition"
    );

    for (name, direction) in [
        ("tangent-u", [1.0, 0.0, 0.0]),
        ("tangent-v", [0.0, 1.0, 0.0]),
        ("torsion", [0.0, 0.0, 4.0]),
        ("mixed", [1.0, -2.0, 3.0]),
    ] {
        let support = patch.support(WrenchDirection3 {
            components: direction,
        })?;
        let response = patch.maximum_dissipation(
            SlipTwist3 {
                tangent_u_velocity: direction[0],
                tangent_v_velocity: direction[1],
                normal_angular_velocity: direction[2],
            },
            MaxDissipationTolerance::default(),
        )?;

        match response {
            MaxDissipationResult::Sliding {
                friction_wrench,
                power,
                dissipation_rate,
                normalized_boundary_radius,
                ..
            } => println!(
                "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},SLIDING",
                planar_patch_limit_surface_oracle::SCHEMA_VERSION,
                name,
                support.support_value,
                support.maximizing_wrench.tangent_u_force,
                support.maximizing_wrench.tangent_v_force,
                support.maximizing_wrench.normal_torsional_moment,
                friction_wrench.tangent_u_force,
                friction_wrench.tangent_v_force,
                friction_wrench.normal_torsional_moment,
                power,
                dissipation_rate,
                normalized_boundary_radius,
            ),
            MaxDissipationResult::StickingSet { .. } => println!(
                "{},{},{:.6},{:.6},{:.6},{:.6},NA,NA,NA,0,0,0,STICKING_SET",
                planar_patch_limit_surface_oracle::SCHEMA_VERSION,
                name,
                support.support_value,
                support.maximizing_wrench.tangent_u_force,
                support.maximizing_wrench.tangent_v_force,
                support.maximizing_wrench.normal_torsional_moment,
            ),
            MaxDissipationResult::ZeroFrictionCapacity { .. } => println!(
                "{},{},{:.6},{:.6},{:.6},{:.6},0,0,0,0,0,0,ZERO_CAPACITY",
                planar_patch_limit_surface_oracle::SCHEMA_VERSION,
                name,
                support.support_value,
                support.maximizing_wrench.tangent_u_force,
                support.maximizing_wrench.tangent_v_force,
                support.maximizing_wrench.normal_torsional_moment,
            ),
        }
    }
    Ok(())
}
