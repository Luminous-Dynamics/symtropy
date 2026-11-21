//! Research-only finite-volume material-state primitives for CMNI.
//!
//! This module is intentionally small: it provides a conservative 1-D
//! diffusion step with zero-flux boundaries and an exact local update for a
//! constant source plus first-order loss. It is not a complete Pd/Ti hydride
//! chemistry model. Coefficients, boundary conditions and source terms must
//! be calibrated or supplied by validated specialist solvers.

use crate::ModelError;

/// Explicit forward-Euler stability limit for 1-D diffusion on a uniform
/// cell-centred finite-volume grid.
///
/// Returns None when diffusion is exactly zero (no diffusion timestep limit).
pub fn diffusion_stability_limit_s(
    diffusion_cm2_s: f64,
    cell_width_cm: f64,
) -> Result<Option<f64>, ModelError> {
    if !diffusion_cm2_s.is_finite() || diffusion_cm2_s < 0.0 {
        return Err(ModelError::InvalidParameter(
            "diffusion_cm2_s_must_be_finite_and_nonnegative",
        ));
    }
    if !cell_width_cm.is_finite() {
        return Err(ModelError::NonFiniteValue("cell_width_cm"));
    }
    if cell_width_cm <= 0.0 {
        return Err(ModelError::NonPositiveValue("cell_width_cm"));
    }
    if diffusion_cm2_s == 0.0 {
        return Ok(None);
    }
    Ok(Some(
        cell_width_cm * cell_width_cm / (2.0 * diffusion_cm2_s),
    ))
}

/// Advance a cell-centred 1-D concentration profile by one explicit diffusion
/// step with zero-flux (Neumann) boundaries at both ends.
///
/// Concentration is in atoms/cm^3, diffusion in cm^2/s, cell width in cm and
/// timestep in seconds. The finite-volume boundary flux is zero at the outer
/// faces; internal face fluxes are equal and opposite, so total inventory is
/// conserved to floating-point accuracy.
///
/// The timestep must not exceed dx^2/(2D). This intentionally fails closed
/// rather than silently substepping or accepting a potentially unstable step.
pub fn diffuse_no_flux_1d(
    concentration_atoms_cm3: &[f64],
    diffusion_cm2_s: f64,
    cell_width_cm: f64,
    timestep_s: f64,
) -> Result<Vec<f64>, ModelError> {
    if concentration_atoms_cm3.len() < 2 {
        return Err(ModelError::InvalidParameter(
            "depth_profile_requires_at_least_two_cells",
        ));
    }
    for value in concentration_atoms_cm3 {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("concentration_atoms_cm3"));
        }
        if *value < 0.0 {
            return Err(ModelError::InvalidParameter(
                "concentration_must_be_nonnegative",
            ));
        }
    }
    if !timestep_s.is_finite() {
        return Err(ModelError::NonFiniteValue("timestep_s"));
    }
    if timestep_s <= 0.0 {
        return Err(ModelError::NonPositiveValue("timestep_s"));
    }

    let stability_limit = diffusion_stability_limit_s(diffusion_cm2_s, cell_width_cm)?;
    if diffusion_cm2_s == 0.0 {
        return Ok(concentration_atoms_cm3.to_vec());
    }
    match stability_limit {
        Some(limit) if timestep_s <= limit * (1.0 + 1.0e-12) => {}
        Some(_) => {
            return Err(ModelError::InvalidParameter(
                "diffusion_timestep_exceeds_explicit_stability_limit",
            ));
        }
        None => {
            return Err(ModelError::InvalidParameter(
                "positive_diffusivity_requires_finite_stability_limit",
            ));
        }
    }

    let n = concentration_atoms_cm3.len();
    let inverse_dx2 = 1.0 / (cell_width_cm * cell_width_cm);
    let coefficient = diffusion_cm2_s * timestep_s * inverse_dx2;
    let mut next = vec![0.0; n];

    for i in 0..n {
        let laplacian = if i == 0 {
            concentration_atoms_cm3[1] - concentration_atoms_cm3[0]
        } else if i == n - 1 {
            concentration_atoms_cm3[n - 2] - concentration_atoms_cm3[n - 1]
        } else {
            concentration_atoms_cm3[i - 1] - 2.0 * concentration_atoms_cm3[i]
                + concentration_atoms_cm3[i + 1]
        };
        next[i] = concentration_atoms_cm3[i] + coefficient * laplacian;
        if !next[i].is_finite() {
            return Err(ModelError::InvalidParameter(
                "diffusion_step_produced_nonfinite_concentration",
            ));
        }
        if next[i]
            < -1.0e-12
                * concentration_atoms_cm3
                    .iter()
                    .copied()
                    .fold(1.0_f64, f64::max)
        {
            return Err(ModelError::InvalidParameter(
                "diffusion_step_produced_negative_concentration",
            ));
        }
        if next[i] < 0.0 {
            next[i] = 0.0;
        }
    }

    Ok(next)
}

/// Exact local update for a constant source and first-order loss over one
/// timestep:
///
/// dc/dt = source_rate - loss_rate * c
///
/// Source has units atoms cm^-3 s^-1; loss has units s^-1. This operator is
/// separate from diffusion so a caller can choose and document an operator
/// splitting scheme and its timestep/convergence tests.
pub fn source_loss_step(
    concentration_atoms_cm3: f64,
    source_rate_atoms_cm3_s: f64,
    loss_rate_s: f64,
    timestep_s: f64,
) -> Result<f64, ModelError> {
    for (value, name) in [
        (concentration_atoms_cm3, "concentration_atoms_cm3"),
        (source_rate_atoms_cm3_s, "source_rate_atoms_cm3_s"),
        (loss_rate_s, "loss_rate_s"),
    ] {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue(name));
        }
        if value < 0.0 {
            return Err(ModelError::InvalidParameter(name));
        }
    }
    if !timestep_s.is_finite() {
        return Err(ModelError::NonFiniteValue("timestep_s"));
    }
    if timestep_s <= 0.0 {
        return Err(ModelError::NonPositiveValue("timestep_s"));
    }

    let updated = if loss_rate_s == 0.0 {
        concentration_atoms_cm3 + source_rate_atoms_cm3_s * timestep_s
    } else {
        let decay = (-loss_rate_s * timestep_s).exp();
        concentration_atoms_cm3 * decay + (source_rate_atoms_cm3_s / loss_rate_s) * (1.0 - decay)
    };
    if !updated.is_finite() || updated < 0.0 {
        return Err(ModelError::InvalidParameter(
            "source_loss_step_produced_invalid_concentration",
        ));
    }
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diffusion_preserves_uniform_profile() {
        let initial = vec![2.0e20; 16];
        let next =
            diffuse_no_flux_1d(&initial, 1.0e-8, 1.0e-6, 1.0e-5).expect("stable diffusion step");
        for value in next {
            assert!((value - 2.0e20).abs() / 2.0e20 < 1.0e-14);
        }
    }

    #[test]
    fn diffusion_conserves_total_inventory_with_no_flux_boundaries() {
        let initial = vec![0.5e20, 1.0e20, 2.0e20, 4.0e20, 3.0e20, 1.5e20];
        let next =
            diffuse_no_flux_1d(&initial, 2.0e-9, 2.0e-6, 5.0e-4).expect("stable diffusion step");
        let before = initial.iter().sum::<f64>();
        let after = next.iter().sum::<f64>();
        assert!((after - before).abs() / before < 1.0e-14);
    }

    #[test]
    fn diffusion_smooths_a_nonuniform_profile() {
        let initial = vec![0.0, 0.0, 10.0, 0.0, 0.0];
        let next = diffuse_no_flux_1d(&initial, 1.0, 1.0, 0.2).expect("stable diffusion step");
        let initial_range = initial.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - initial.iter().copied().fold(f64::INFINITY, f64::min);
        let next_range = next.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - next.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(next_range < initial_range);
        assert!(next.iter().all(|value| *value >= 0.0));
    }

    #[test]
    fn diffusion_matches_discrete_neumann_cosine_mode() {
        let n = 32usize;
        let dx = 1.0e-6;
        let diffusion = 1.0e-8;
        let mode = 1.0;
        let theta = std::f64::consts::PI * mode / n as f64;
        let initial: Vec<f64> = (0..n)
            .map(|i| 1.0e20 + 0.2e20 * (theta * (i as f64 + 0.5)).cos())
            .collect();

        let dt = 0.2 * diffusion_stability_limit_s(diffusion, dx).unwrap().unwrap();
        let steps = 100usize;
        let mut state = initial.clone();
        for _ in 0..steps {
            state = diffuse_no_flux_1d(&state, diffusion, dx, dt).expect("stable step");
        }

        let eigenvalue = -4.0 * diffusion / (dx * dx) * (theta / 2.0).sin().powi(2);
        let expected_amplitude = 0.2e20 * (1.0 + eigenvalue * dt).powi(steps as i32);
        for (i, actual) in state.iter().enumerate() {
            let expected = 1.0e20 + expected_amplitude * (theta * (i as f64 + 0.5)).cos();
            assert!((*actual - expected).abs() / 1.0e20 < 1.0e-12);
        }
    }

    #[test]
    fn diffusion_rejects_unstable_timestep() {
        let result = diffuse_no_flux_1d(&[1.0, 2.0, 3.0], 1.0, 1.0, 0.6);
        assert_eq!(
            result,
            Err(ModelError::InvalidParameter(
                "diffusion_timestep_exceeds_explicit_stability_limit"
            ))
        );
    }

    #[test]
    fn zero_diffusivity_returns_unchanged_profile() {
        let initial = vec![1.0, 2.0, 3.0];
        let next = diffuse_no_flux_1d(&initial, 0.0, 1.0, 10.0).expect("zero-diffusivity profile");
        assert_eq!(next, initial);
    }

    #[test]
    fn source_loss_step_matches_no_loss_and_pure_decay_limits() {
        let sourced = source_loss_step(10.0, 2.0, 0.0, 3.0).expect("no-loss source");
        assert!((sourced - 16.0).abs() < 1.0e-12);
        let decayed = source_loss_step(10.0, 0.0, 0.5, 2.0).expect("pure decay");
        assert!((decayed - 10.0 * (-1.0_f64).exp()).abs() < 1.0e-12);
    }

    #[test]
    fn source_loss_step_matches_exact_constant_source_solution() {
        let c0 = 4.0;
        let source = 3.0;
        let loss = 0.2;
        let dt = 1.5;
        let actual = source_loss_step(c0, source, loss, dt).expect("exact source-loss update");
        let decay = (-loss * dt).exp();
        let expected = c0 * decay + source / loss * (1.0 - decay);
        assert!((actual - expected).abs() < 1.0e-12);
    }
}
