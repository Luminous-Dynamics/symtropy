//! Conservative 1D finite-volume enthalpy baseline for planar Stefan freezing.
//!
//! Standalone research utility; deliberately not part of the Bevy/Cargo workspace.
//! Compile with:
//!   rustc --edition=2024 -D warnings --test tools/casting-phase-reference/finite_volume.rs -o /tmp/stefan-fv-tests
//!   rustc --edition=2024 -D warnings tools/casting-phase-reference/finite_volume.rs -o /tmp/stefan-fv
//!
//! Scope: constant-property, conduction-only one-phase freezing; explicit time
//! integration; cell-centred finite volumes; cold Dirichlet wall and insulated far end.

const RHO: f64 = 1_000.0;
const CP: f64 = 2_000.0;
const CONDUCTIVITY: f64 = 0.5;
const LATENT_HEAT: f64 = 200_000.0;
const WALL_TEMPERATURE: f64 = -20.0;
const MELTING_TEMPERATURE: f64 = 0.0;
const DOMAIN_LENGTH_M: f64 = 0.01;
const END_TIME_S: f64 = 60.0;
const CFL: f64 = 0.2;
const REFERENCE_FRONT_M: f64 = 0.002_373_549_364_687_617_3;
const MAX_STEPS: usize = 5_000_000;
const MAX_CFL: f64 = 0.25;

#[derive(Debug, Clone, Copy)]
struct Case {
    density_kg_m3: f64,
    heat_capacity_j_kg_k: f64,
    conductivity_w_m_k: f64,
    latent_heat_j_kg: f64,
    wall_temperature_c: f64,
    melting_temperature_c: f64,
    domain_length_m: f64,
    end_time_s: f64,
    cells: usize,
    cfl: f64,
    reference_front_m: f64,
}

impl Case {
    fn reference(cells: usize) -> Self {
        Self {
            density_kg_m3: RHO,
            heat_capacity_j_kg_k: CP,
            conductivity_w_m_k: CONDUCTIVITY,
            latent_heat_j_kg: LATENT_HEAT,
            wall_temperature_c: WALL_TEMPERATURE,
            melting_temperature_c: MELTING_TEMPERATURE,
            domain_length_m: DOMAIN_LENGTH_M,
            end_time_s: END_TIME_S,
            cells,
            cfl: CFL,
            reference_front_m: REFERENCE_FRONT_M,
        }
    }

    fn validate(self) -> Result<ValidatedCase, String> {
        let positive = [
            self.density_kg_m3,
            self.heat_capacity_j_kg_k,
            self.conductivity_w_m_k,
            self.latent_heat_j_kg,
            self.domain_length_m,
            self.end_time_s,
        ];
        if positive.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err("material properties, domain length and end time must be finite and positive".into());
        }
        if !self.wall_temperature_c.is_finite()
            || !self.melting_temperature_c.is_finite()
            || self.wall_temperature_c >= self.melting_temperature_c
        {
            return Err("freezing requires finite wall temperature below melting temperature".into());
        }
        if self.cells < 8 {
            return Err("at least 8 finite-volume cells are required".into());
        }
        if !self.cfl.is_finite() || !(0.0 < self.cfl && self.cfl <= MAX_CFL) {
            return Err(format!("CFL must be in (0, {MAX_CFL}]"));
        }
        if !self.reference_front_m.is_finite() || self.reference_front_m <= 0.0 {
            return Err("analytical reference front must be finite and positive".into());
        }

        let diffusivity_m2_s =
            self.conductivity_w_m_k / (self.density_kg_m3 * self.heat_capacity_j_kg_k);
        if !diffusivity_m2_s.is_finite() || diffusivity_m2_s <= 0.0 {
            return Err("computed diffusivity must be finite and positive".into());
        }
        let stefan_number = self.heat_capacity_j_kg_k
            * (self.melting_temperature_c - self.wall_temperature_c)
            / self.latent_heat_j_kg;
        if !stefan_number.is_finite() || !(0.0 < stefan_number && stefan_number <= 1.0) {
            return Err("this reference profile admits only 0 < Stefan number <= 1".into());
        }

        let dx_m = self.domain_length_m / self.cells as f64;
        let maximum_dt_s = self.cfl * dx_m * dx_m / diffusivity_m2_s;
        let steps_f64 = (self.end_time_s / maximum_dt_s).ceil();
        if !steps_f64.is_finite() || steps_f64 < 1.0 || steps_f64 > MAX_STEPS as f64 {
            return Err("required timestep count is outside the supported budget".into());
        }
        let steps = steps_f64 as usize;
        let dt_s = self.end_time_s / steps as f64;
        if dt_s > maximum_dt_s * (1.0 + 32.0 * f64::EPSILON) {
            return Err("computed timestep exceeds the declared explicit stability bound".into());
        }

        Ok(ValidatedCase {
            diffusivity_m2_s,
            stefan_number,
            dx_m,
            dt_s,
            steps,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ValidatedCase {
    diffusivity_m2_s: f64,
    stefan_number: f64,
    dx_m: f64,
    dt_s: f64,
    steps: usize,
}

#[derive(Debug, Clone, Copy)]
struct Summary {
    cells: usize,
    dx_m: f64,
    dt_s: f64,
    steps: usize,
    stefan_number: f64,
    effective_fourier_number: f64,
    front_m: f64,
    reference_front_m: f64,
    front_abs_error_m: f64,
    front_relative_error: f64,
    wall_heat_removed_j_m2: f64,
    enthalpy_removed_j_m2: f64,
    energy_residual_j_m2: f64,
    energy_residual_relative: f64,
    far_boundary_solid_fraction: f64,
}

fn invert_enthalpy(case: Case, volumetric_enthalpy_j_m3: f64) -> Result<(f64, f64), String> {
    if !volumetric_enthalpy_j_m3.is_finite() {
        return Err("non-finite cell enthalpy".into());
    }
    let latent_per_volume = case.density_kg_m3 * case.latent_heat_j_kg;
    let sensible_capacity = case.density_kg_m3 * case.heat_capacity_j_kg_k;

    if volumetric_enthalpy_j_m3 > 0.0 {
        let temperature = case.melting_temperature_c
            + volumetric_enthalpy_j_m3 / sensible_capacity;
        if !temperature.is_finite() {
            return Err("liquid temperature inversion is non-finite".into());
        }
        Ok((temperature, 0.0))
    } else if volumetric_enthalpy_j_m3 >= -latent_per_volume {
        let solid_fraction = (-volumetric_enthalpy_j_m3 / latent_per_volume).clamp(0.0, 1.0);
        Ok((case.melting_temperature_c, solid_fraction))
    } else {
        let temperature = case.melting_temperature_c
            + (volumetric_enthalpy_j_m3 + latent_per_volume) / sensible_capacity;
        if !temperature.is_finite() {
            return Err("solid temperature inversion is non-finite".into());
        }
        Ok((temperature, 1.0))
    }
}

fn solve(case: Case) -> Result<Summary, String> {
    let validated = case.validate()?;
    let mut enthalpy = vec![0.0_f64; case.cells];
    let mut next_enthalpy = vec![0.0_f64; case.cells];
    let mut temperature = vec![case.melting_temperature_c; case.cells];
    let mut wall_heat_removed_j_m2 = 0.0_f64;

    for _step in 0..validated.steps {
        for i in 0..case.cells {
            temperature[i] = invert_enthalpy(case, enthalpy[i])?.0;
        }

        let wall_heat_rate_out =
            2.0 * case.conductivity_w_m_k
                * (temperature[0] - case.wall_temperature_c)
                / validated.dx_m;
        wall_heat_removed_j_m2 += wall_heat_rate_out * validated.dt_s;

        for i in 0..case.cells {
            let west_gradient = if i == 0 {
                2.0 * case.conductivity_w_m_k
                    * (case.wall_temperature_c - temperature[i])
                    / validated.dx_m
            } else {
                case.conductivity_w_m_k * (temperature[i - 1] - temperature[i])
                    / validated.dx_m
            };

            let east_gradient = if i + 1 == case.cells {
                0.0
            } else {
                case.conductivity_w_m_k * (temperature[i + 1] - temperature[i])
                    / validated.dx_m
            };

            let rate_j_m3_s = (west_gradient + east_gradient) / validated.dx_m;
            next_enthalpy[i] = enthalpy[i] + validated.dt_s * rate_j_m3_s;
            if !next_enthalpy[i].is_finite() {
                return Err("finite-volume update produced non-finite enthalpy".into());
            }
        }
        std::mem::swap(&mut enthalpy, &mut next_enthalpy);
    }

    let mut solid_equivalent_length_m = 0.0_f64;
    let mut enthalpy_sum_j_m3 = 0.0_f64;
    let mut far_boundary_solid_fraction = 0.0_f64;
    for (i, value) in enthalpy.iter().copied().enumerate() {
        let (_temperature_c, solid_fraction) = invert_enthalpy(case, value)?;
        if !(0.0..=1.0).contains(&solid_fraction) {
            return Err("solid fraction escaped [0, 1]".into());
        }
        solid_equivalent_length_m += solid_fraction * validated.dx_m;
        enthalpy_sum_j_m3 += value;
        if i + 1 == case.cells {
            far_boundary_solid_fraction = solid_fraction;
        }
    }

    if far_boundary_solid_fraction > 1.0e-12 {
        return Err("DOMAIN_TOO_SHORT: freezing reached the insulated far boundary".into());
    }
    if !wall_heat_removed_j_m2.is_finite() || wall_heat_removed_j_m2 <= 0.0 {
        return Err("wall heat removal must be finite and positive".into());
    }

    let enthalpy_removed_j_m2 = -enthalpy_sum_j_m3 * validated.dx_m;
    let energy_residual_j_m2 = enthalpy_removed_j_m2 - wall_heat_removed_j_m2;
    let energy_residual_relative = energy_residual_j_m2.abs() / wall_heat_removed_j_m2;
    let front_abs_error_m = (solid_equivalent_length_m - case.reference_front_m).abs();
    let front_relative_error = front_abs_error_m / case.reference_front_m;

    Ok(Summary {
        cells: case.cells,
        dx_m: validated.dx_m,
        dt_s: validated.dt_s,
        steps: validated.steps,
        stefan_number: validated.stefan_number,
        effective_fourier_number: validated.dt_s * validated.diffusivity_m2_s
            / (validated.dx_m * validated.dx_m),
        front_m: solid_equivalent_length_m,
        reference_front_m: case.reference_front_m,
        front_abs_error_m,
        front_relative_error,
        wall_heat_removed_j_m2,
        enthalpy_removed_j_m2,
        energy_residual_j_m2,
        energy_residual_relative,
        far_boundary_solid_fraction,
    })
}

fn refinement_summaries() -> Result<Vec<Summary>, String> {
    [32, 64, 128, 256]
        .into_iter()
        .map(|cells| solve(Case::reference(cells)))
        .collect()
}

fn verify() -> Result<(), String> {
    let summaries = refinement_summaries()?;
    let mut previous_error = f64::INFINITY;

    for summary in &summaries {
        if summary.front_abs_error_m >= previous_error {
            return Err("front error did not decrease under the frozen refinement ladder".into());
        }
        previous_error = summary.front_abs_error_m;

        if summary.energy_residual_relative > 1.0e-11 {
            return Err(format!(
                "energy closure relative residual {} exceeds tolerance",
                summary.energy_residual_relative
            ));
        }
        if summary.far_boundary_solid_fraction > 1.0e-12 {
            return Err("far boundary is not fully liquid".into());
        }
    }

    let finest = summaries.last().ok_or("empty refinement ladder")?;
    if finest.front_relative_error >= 5.0e-5 {
        return Err("finest-grid relative front error exceeds 5e-5".into());
    }
    if summaries[2].front_relative_error >= 5.0e-4 {
        return Err("128-cell relative front error exceeds 5e-4".into());
    }

    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("cells,stefan_number,dx_m,dt_s,steps,effective_fourier_number,front_m,reference_front_m,front_abs_error_m,front_relative_error,wall_heat_removed_j_m2,enthalpy_removed_j_m2,energy_residual_j_m2,energy_residual_relative,far_boundary_solid_fraction,status");
    for summary in refinement_summaries()? {
        println!(
            "{},{:.8e},{:.8e},{:.8e},{},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},PASS",
            summary.cells,
            summary.stefan_number,
            summary.dx_m,
            summary.dt_s,
            summary.steps,
            summary.effective_fourier_number,
            summary.front_m,
            summary.reference_front_m,
            summary.front_abs_error_m,
            summary.front_relative_error,
            summary.wall_heat_removed_j_m2,
            summary.enthalpy_removed_j_m2,
            summary.energy_residual_j_m2,
            summary.energy_residual_relative,
            summary.far_boundary_solid_fraction,
        );
    }
    Ok(())
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [] => verify().map(|()| println!("FINITE_VOLUME_OK: Stefan front refinement and energy closure")),
        [arg] if arg == "--verify" => {
            verify().map(|()| println!("FINITE_VOLUME_OK: Stefan front refinement and energy closure"))
        }
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: stefan-fv [--verify|--csv|--help]");
            println!("Default action: --verify");
            Ok(())
        }
        _ => Err("expected at most one argument: --verify, --csv or --help".into()),
    };

    if let Err(error) = result {
        eprintln!("FINITE_VOLUME_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_refinement_front_error_decreases() {
        let summaries = refinement_summaries().expect("reference refinement should run");
        for pair in summaries.windows(2) {
            assert!(pair[1].front_abs_error_m < pair[0].front_abs_error_m);
        }
        assert!(summaries[2].front_relative_error < 5.0e-4);
        assert!(summaries[3].front_relative_error < 5.0e-5);
    }

    #[test]
    fn energy_removed_matches_integrated_wall_heat() {
        for summary in refinement_summaries().expect("reference refinement should run") {
            assert!(summary.energy_residual_relative < 1.0e-11);
        }
    }

    #[test]
    fn actual_timestep_obeys_explicit_stability_bound() {
        for cells in [32, 64, 128, 256] {
            let case = Case::reference(cells);
            let validated = case.validate().unwrap();
            let maximum_dt =
                case.cfl * validated.dx_m * validated.dx_m / validated.diffusivity_m2_s;
            assert!(validated.dt_s <= maximum_dt * (1.0 + 32.0 * f64::EPSILON));
            assert!(validated.steps > 0);
            assert!(validated.stefan_number > 0.0 && validated.stefan_number <= 1.0);
        }
    }

    #[test]
    fn invalid_material_geometry_time_and_cfl_reject() {
        let mut case = Case::reference(32);
        case.conductivity_w_m_k = 0.0;
        assert!(case.validate().is_err());

        let mut case = Case::reference(32);
        case.domain_length_m = 0.0;
        assert!(case.validate().is_err());

        let mut case = Case::reference(32);
        case.end_time_s = f64::NAN;
        assert!(case.validate().is_err());

        let mut case = Case::reference(32);
        case.cfl = 0.5;
        assert!(case.validate().is_err());
    }

    #[test]
    fn domain_too_short_is_rejected() {
        let mut case = Case::reference(32);
        case.domain_length_m = 0.001;
        assert!(solve(case).is_err());
    }

    #[test]
    fn phase_inversion_is_bounded_and_piecewise() {
        let case = Case::reference(32);
        let latent_per_volume = case.density_kg_m3 * case.latent_heat_j_kg;

        let (t_liquid, fs_liquid) = invert_enthalpy(case, 0.0).unwrap();
        assert_eq!(t_liquid, case.melting_temperature_c);
        assert_eq!(fs_liquid, 0.0);

        let (t_mushy, fs_mushy) = invert_enthalpy(case, -0.5 * latent_per_volume).unwrap();
        assert_eq!(t_mushy, case.melting_temperature_c);
        assert!((fs_mushy - 0.5).abs() < f64::EPSILON);

        let (t_solid, fs_solid) = invert_enthalpy(case, -1.5 * latent_per_volume).unwrap();
        assert!((t_solid - (case.melting_temperature_c - 0.5 * case.latent_heat_j_kg / case.heat_capacity_j_kg_k)).abs() < 1.0e-12);
        assert_eq!(fs_solid, 1.0);
    }
}
