//! Analytical two-phase planar Stefan oracle with initially superheated liquid.
//!
//! Standalone research reference for the 1D isothermal benchmark in:
//! Peters et al., IJHMT 249 (2025) 127166, DOI 10.1016/j.ijheatmasstransfer.2025.127166.
//!
//! The admitted case has equal solid/liquid density, heat capacity and
//! conductivity. It is not a numerical PDE solver or a material model.
//!
//! Compile directly:
//!   rustc --edition=2024 -D warnings --test tools/casting-phase-reference/two_phase.rs -o /tmp/two-phase-tests
//!   rustc --edition=2024 -D warnings tools/casting-phase-reference/two_phase.rs -o /tmp/two-phase
//!
//! Model: initially liquid at T_infinity > T_m, cold Dirichlet wall T_w < T_m
//! at x=0, semi-infinite domain, no advection, no volumetric heat source.

use std::f64::consts::PI;

const SQRT_PI: f64 = 1.772_453_850_905_516_027_3;
const ERF_MAX_X: f64 = 1.0;
const ROOT_MAX_ITERATIONS: usize = 160;
const ROOT_WIDTH_TOL: f64 = 2.0 * f64::EPSILON;

#[derive(Debug, Clone, Copy)]
struct Case {
    density_kg_m3: f64,
    heat_capacity_j_kg_k: f64,
    conductivity_w_m_k: f64,
    latent_heat_j_kg: f64,
    wall_temperature_c: f64,
    melting_temperature_c: f64,
    initial_liquid_temperature_c: f64,
    domain_length_m: f64,
    time_s: f64,
}

impl Case {
    fn published_reference() -> Self {
        Self {
            density_kg_m3: 1.0,
            heat_capacity_j_kg_k: 2.5e6,
            conductivity_w_m_k: 2.0,
            latent_heat_j_kg: 1.0e8,
            wall_temperature_c: -10.0,
            melting_temperature_c: 0.0,
            initial_liquid_temperature_c: 2.0,
            domain_length_m: 6.0,
            time_s: 288.0 * 3600.0,
        }
    }

    fn validate(self) -> Result<ValidatedCase, String> {
        let positive = [
            self.density_kg_m3,
            self.heat_capacity_j_kg_k,
            self.conductivity_w_m_k,
            self.latent_heat_j_kg,
            self.domain_length_m,
            self.time_s,
        ];
        if positive.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err("density, heat capacity, conductivity, latent heat, domain and time must be finite and positive".into());
        }
        if !self.wall_temperature_c.is_finite()
            || !self.melting_temperature_c.is_finite()
            || !self.initial_liquid_temperature_c.is_finite()
            || !(self.wall_temperature_c < self.melting_temperature_c
                && self.melting_temperature_c < self.initial_liquid_temperature_c)
        {
            return Err("require finite wall temperature < melting temperature < initial liquid temperature".into());
        }

        let diffusivity_m2_s =
            self.conductivity_w_m_k / (self.density_kg_m3 * self.heat_capacity_j_kg_k);
        let stefan_solid = self.heat_capacity_j_kg_k
            * (self.melting_temperature_c - self.wall_temperature_c)
            / self.latent_heat_j_kg;
        let stefan_liquid = self.heat_capacity_j_kg_k
            * (self.initial_liquid_temperature_c - self.melting_temperature_c)
            / self.latent_heat_j_kg;

        if !diffusivity_m2_s.is_finite() || diffusivity_m2_s <= 0.0 {
            return Err("computed diffusivity must be finite and positive".into());
        }
        if !stefan_solid.is_finite()
            || !stefan_liquid.is_finite()
            || stefan_solid <= 0.0
            || stefan_liquid <= 0.0
        {
            return Err("both solid-side and liquid-side Stefan numbers must be finite and positive".into());
        }

        let lambda = solve_lambda(stefan_solid, stefan_liquid)?;
        let similarity_length_m = 2.0 * (diffusivity_m2_s * self.time_s).sqrt();
        let front_m = lambda * similarity_length_m;
        if !front_m.is_finite() || front_m <= 0.0 || front_m >= self.domain_length_m {
            return Err("interface is outside the admitted finite comparison domain".into());
        }

        // The published finite D=6 m computation is compared against the
        // semi-infinite exact solution. Require the boundary to lie at least
        // 3 similarity lengths from x=0 so the far boundary is remote.
        let far_field_coordinate = self.domain_length_m / similarity_length_m;
        if far_field_coordinate < 3.0 {
            return Err("finite comparison domain is not remote enough for this semi-infinite reference".into());
        }

        Ok(ValidatedCase {
            case: self,
            diffusivity_m2_s,
            stefan_solid,
            stefan_liquid,
            lambda,
            similarity_length_m,
            front_m,
            far_field_coordinate,
        })
    }
}

#[derive(Debug, Clone, Copy)]
struct ValidatedCase {
    case: Case,
    diffusivity_m2_s: f64,
    stefan_solid: f64,
    stefan_liquid: f64,
    lambda: f64,
    similarity_length_m: f64,
    front_m: f64,
    far_field_coordinate: f64,
}

#[derive(Debug, Clone, Copy)]
struct Summary {
    diffusivity_m2_s: f64,
    stefan_solid: f64,
    stefan_liquid: f64,
    lambda: f64,
    root_residual: f64,
    time_s: f64,
    front_m: f64,
    domain_length_m: f64,
    far_field_coordinate: f64,
    temperature_quarter_solid_c: f64,
    temperature_half_solid_c: f64,
    temperature_125_front_c: f64,
    temperature_2_front_c: f64,
    wall_heat_j_m2: f64,
    enthalpy_decrease_j_m2: f64,
    energy_residual_j_m2: f64,
    energy_residual_relative: f64,
}

fn erf_series(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x.abs() > ERF_MAX_X {
        return Err("erf series is admitted only for finite |x| <= 1".into());
    }
    if x == 0.0 {
        return Ok(0.0);
    }

    // erf(x) = 2/sqrt(pi) * sum_n (-1)^n x^(2n+1)/(n!(2n+1)).
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    for n in 1..=100 {
        term *= -x2 / (n as f64) * ((2 * n - 1) as f64) / ((2 * n + 1) as f64);
        sum += term;
        if term.abs() <= 0.2 * f64::EPSILON * sum.abs() {
            break;
        }
    }
    let value = 2.0 * sum / SQRT_PI;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("erf series produced a non-finite value".into())
    }
}

fn erfc_small(x: f64) -> Result<f64, String> {
    if !(0.0..=ERF_MAX_X).contains(&x) {
        return Err("this oracle evaluates erfc only for 0 <= x <= 1".into());
    }
    Ok(1.0 - erf_series(x)?)
}

fn lambda_residual(lambda: f64, stefan_solid: f64, stefan_liquid: f64) -> Result<f64, String> {
    if !lambda.is_finite() || !(0.0 < lambda && lambda <= 1.0) {
        return Err("similarity parameter trial must lie in (0, 1]".into());
    }
    let erf_lambda = erf_series(lambda)?;
    let erfc_lambda = erfc_small(lambda)?;
    if erf_lambda <= 0.0 || erfc_lambda <= 0.0 {
        return Err("invalid erf/erfc denominator in similarity equation".into());
    }

    // Two-phase Stefan condition for equal solid/liquid diffusivity and
    // conductivity:
    // lambda*exp(lambda^2) = (Ste_s/erf(lambda) - Ste_l/erfc(lambda))/sqrt(pi).
    Ok(lambda * (lambda * lambda).exp()
        - (stefan_solid / erf_lambda - stefan_liquid / erfc_lambda) / SQRT_PI)
}

fn solve_lambda(stefan_solid: f64, stefan_liquid: f64) -> Result<f64, String> {
    if !stefan_solid.is_finite()
        || !stefan_liquid.is_finite()
        || stefan_solid <= 0.0
        || stefan_liquid <= 0.0
    {
        return Err("both Stefan numbers must be finite and positive".into());
    }

    let mut lower = 1.0e-10;
    let mut upper = 1.0;
    let mut lower_residual = lambda_residual(lower, stefan_solid, stefan_liquid)?;
    let upper_residual = lambda_residual(upper, stefan_solid, stefan_liquid)?;
    if !(lower_residual < 0.0 && upper_residual > 0.0) {
        return Err("failed to bracket the positive two-phase similarity root".into());
    }

    for _ in 0..ROOT_MAX_ITERATIONS {
        let midpoint = lower + (upper - lower) * 0.5;
        let residual = lambda_residual(midpoint, stefan_solid, stefan_liquid)?;
        if residual < 0.0 {
            lower = midpoint;
            lower_residual = residual;
        } else {
            upper = midpoint;
        }
        if upper - lower <= ROOT_WIDTH_TOL * midpoint.max(1.0) {
            let root = lower + (upper - lower) * 0.5;
            let final_residual = lambda_residual(root, stefan_solid, stefan_liquid)?.abs();
            if final_residual > 5.0e-14 {
                return Err("two-phase similarity root failed its residual tolerance".into());
            }
            return Ok(root);
        }
    }
    let _ = lower_residual;
    Err("two-phase similarity root did not converge within the iteration budget".into())
}

impl ValidatedCase {
    fn temperature_c(self, position_m: f64) -> Result<f64, String> {
        if !position_m.is_finite() || position_m < 0.0 {
            return Err("position must be finite and nonnegative".into());
        }
        let eta = position_m / self.similarity_length_m;
        if eta <= self.lambda {
            let numerator = erf_series(eta)?;
            let denominator = erf_series(self.lambda)?;
            Ok(self.case.wall_temperature_c
                + (self.case.melting_temperature_c - self.case.wall_temperature_c)
                    * numerator
                    / denominator)
        } else {
            let numerator = erfc_small(eta)?;
            let denominator = erfc_small(self.lambda)?;
            Ok(self.case.initial_liquid_temperature_c
                + (self.case.melting_temperature_c - self.case.initial_liquid_temperature_c)
                    * numerator
                    / denominator)
        }
    }

    fn wall_heat_removed_j_m2(self) -> Result<f64, String> {
        let erf_lambda = erf_series(self.lambda)?;
        let delta_t = self.case.melting_temperature_c - self.case.wall_temperature_c;
        let heat = 2.0 * self.case.conductivity_w_m_k * delta_t * self.case.time_s.sqrt()
            / (SQRT_PI * self.diffusivity_m2_s.sqrt() * erf_lambda);
        if heat.is_finite() && heat > 0.0 {
            Ok(heat)
        } else {
            Err("integrated wall heat is invalid".into())
        }
    }

    fn total_enthalpy_decrease_j_m2(self) -> Result<f64, String> {
        let a = self.similarity_length_m;
        let s = self.front_m;
        let t = self.case.time_s;
        let lambda = self.lambda;
        let erf_lambda = erf_series(lambda)?;
        let erfc_lambda = erfc_small(lambda)?;

        // Integral_0^s erf(x/a) dx.
        let integral_erf = s * erf_lambda
            + a / SQRT_PI * ((-lambda * lambda).exp() - 1.0);
        let integral_solid_temperature = self.case.wall_temperature_c * s
            + (self.case.melting_temperature_c - self.case.wall_temperature_c)
                * integral_erf
                / erf_lambda;

        // Integral_s^infinity (T_infinity - T_l) dx.
        // Integral_s^infinity erfc(x/a) dx = a/sqrt(pi)*exp(-lambda^2) - s*erfc(lambda).
        let liquid_superheat_removed = (self.case.initial_liquid_temperature_c
            - self.case.melting_temperature_c)
            / erfc_lambda
            * (a / SQRT_PI * (-lambda * lambda).exp() - s * erfc_lambda);

        let sensible_capacity = self.case.density_kg_m3 * self.case.heat_capacity_j_kg_k;
        let latent = self.case.density_kg_m3 * self.case.latent_heat_j_kg * s;
        let solid_region_sensible = sensible_capacity
            * (self.case.initial_liquid_temperature_c * s - integral_solid_temperature);
        let remaining_liquid_sensible = sensible_capacity * liquid_superheat_removed;
        let total = latent + solid_region_sensible + remaining_liquid_sensible;

        if total.is_finite() && total > 0.0 && t.is_finite() {
            Ok(total)
        } else {
            Err("integrated total enthalpy decrease is invalid".into())
        }
    }

    fn summary(self) -> Result<Summary, String> {
        let wall_heat = self.wall_heat_removed_j_m2()?;
        let enthalpy_decrease = self.total_enthalpy_decrease_j_m2()?;
        let residual = wall_heat - enthalpy_decrease;
        let relative = residual.abs() / wall_heat;

        Ok(Summary {
            diffusivity_m2_s: self.diffusivity_m2_s,
            stefan_solid: self.stefan_solid,
            stefan_liquid: self.stefan_liquid,
            lambda: self.lambda,
            root_residual: lambda_residual(
                self.lambda,
                self.stefan_solid,
                self.stefan_liquid,
            )?,
            time_s: self.case.time_s,
            front_m: self.front_m,
            domain_length_m: self.case.domain_length_m,
            far_field_coordinate: self.far_field_coordinate,
            temperature_quarter_solid_c: self.temperature_c(0.25 * self.front_m)?,
            temperature_half_solid_c: self.temperature_c(0.5 * self.front_m)?,
            temperature_125_front_c: self.temperature_c(1.25 * self.front_m)?,
            temperature_2_front_c: self.temperature_c(2.0 * self.front_m)?,
            wall_heat_j_m2: wall_heat,
            enthalpy_decrease_j_m2: enthalpy_decrease,
            energy_residual_j_m2: residual,
            energy_residual_relative: relative,
        })
    }
}

fn verify() -> Result<(), String> {
    let validated = Case::published_reference().validate()?;
    let summary = validated.summary()?;

    const EXPECTED_LAMBDA: f64 = 0.322_390_632_922_300_86;
    const EXPECTED_FRONT_M: f64 = 0.587_225_489_090_536_2;
    const EXPECTED_T_QUARTER: f64 = -7.418_693_664_209_443;
    const EXPECTED_T_HALF: f64 = -4.870_670_835_179_128;
    const EXPECTED_T_125: f64 = 0.245_830_171_289_429_97;
    const EXPECTED_T_2: f64 = 0.883_959_157_097_861_1;
    const EXPECTED_WALL_HEAT: f64 = 73_078_427.878_405_57;

    if (summary.diffusivity_m2_s - 8.0e-7).abs() > 1.0e-20 {
        return Err("published benchmark diffusivity mismatch".into());
    }
    if (summary.stefan_solid - 0.25).abs() > 1.0e-14
        || (summary.stefan_liquid - 0.05).abs() > 1.0e-14
    {
        return Err("published benchmark Stefan numbers mismatch".into());
    }
    if (summary.lambda - EXPECTED_LAMBDA).abs() > 2.0e-14 {
        return Err("two-phase similarity root mismatch".into());
    }
    if summary.root_residual.abs() > 5.0e-14 {
        return Err("two-phase similarity root residual exceeds tolerance".into());
    }
    if (summary.front_m - EXPECTED_FRONT_M).abs() > 2.0e-13 {
        return Err("two-phase interface known answer mismatch".into());
    }
    if (summary.temperature_quarter_solid_c - EXPECTED_T_QUARTER).abs() > 2.0e-12
        || (summary.temperature_half_solid_c - EXPECTED_T_HALF).abs() > 2.0e-12
        || (summary.temperature_125_front_c - EXPECTED_T_125).abs() > 2.0e-12
        || (summary.temperature_2_front_c - EXPECTED_T_2).abs() > 2.0e-12
    {
        return Err("two-phase temperature profile known answer mismatch".into());
    }
    if (summary.wall_heat_j_m2 - EXPECTED_WALL_HEAT).abs() > 2.0e-6 {
        return Err("two-phase wall heat known answer mismatch".into());
    }
    if summary.energy_residual_relative > 2.0e-14 {
        return Err("two-phase total-energy closure exceeds tolerance".into());
    }
    if summary.far_field_coordinate < 3.0 || summary.front_m >= summary.domain_length_m {
        return Err("finite-domain comparison is not sufficiently remote".into());
    }

    Ok(())
}

fn emit_csv() -> Result<(), String> {
    let summary = Case::published_reference().validate()?.summary()?;
    println!("rho_kg_m3,cp_j_kg_k,k_w_m_k,L_j_kg,Tw_c,Tm_c,Tinfinity_c,alpha_m2_s,Ste_solid,Ste_liquid,lambda,root_residual,time_s,front_m,domain_m,far_field_coordinate,T_quarter_solid_c,T_half_solid_c,T_125_front_liquid_c,T_2_front_liquid_c,Q_wall_j_m2,enthalpy_decrease_j_m2,energy_residual_j_m2,energy_residual_relative");
    let c = Case::published_reference();
    println!(
        "{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
        c.density_kg_m3,
        c.heat_capacity_j_kg_k,
        c.conductivity_w_m_k,
        c.latent_heat_j_kg,
        c.wall_temperature_c,
        c.melting_temperature_c,
        c.initial_liquid_temperature_c,
        summary.diffusivity_m2_s,
        summary.stefan_solid,
        summary.stefan_liquid,
        summary.lambda,
        summary.root_residual,
        summary.time_s,
        summary.front_m,
        summary.domain_length_m,
        summary.far_field_coordinate,
        summary.temperature_quarter_solid_c,
        summary.temperature_half_solid_c,
        summary.temperature_125_front_c,
        summary.temperature_2_front_c,
        summary.wall_heat_j_m2,
        summary.enthalpy_decrease_j_m2,
        summary.energy_residual_j_m2,
        summary.energy_residual_relative
    );
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("TWO_PHASE_STEFAN_OK: analytical front, profile and energy closure")),
        [arg] if arg == "--verify" => {
            verify().map(|()| println!("TWO_PHASE_STEFAN_OK: analytical front, profile and energy closure"))
        }
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: two-phase-stefan [--verify|--csv|--help]");
            println!("Default action: --verify");
            Ok(())
        }
        _ => Err("expected at most one argument: --verify, --csv or --help".into()),
    };

    if let Err(error) = result {
        eprintln!("TWO_PHASE_STEFAN_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> ValidatedCase {
        Case::published_reference().validate().expect("published case is valid")
    }

    #[test]
    fn published_two_phase_root_and_front_known_answers() {
        let v = reference();
        assert!((v.lambda - 0.322_390_632_922_300_86).abs() < 2.0e-14);
        assert!((v.front_m - 0.587_225_489_090_536_2).abs() < 2.0e-13);
        assert!(lambda_residual(v.lambda, v.stefan_solid, v.stefan_liquid).unwrap().abs() < 5.0e-14);
    }

    #[test]
    fn solid_and_liquid_profiles_join_at_the_moving_interface() {
        let v = reference();
        let solid_side = v.temperature_c(v.front_m).unwrap();
        let liquid_side = v.temperature_c(v.front_m + 1.0e-12).unwrap();
        assert!(solid_side.abs() < 1.0e-12);
        assert!(liquid_side.abs() < 1.0e-10);
        assert!((v.temperature_c(0.0).unwrap() - (-10.0)).abs() < 1.0e-13);
    }

    #[test]
    fn both_phase_profiles_are_monotone_and_bounded() {
        let v = reference();
        let front = v.front_m;
        let s0 = v.temperature_c(0.0).unwrap();
        let s1 = v.temperature_c(0.25 * front).unwrap();
        let s2 = v.temperature_c(0.5 * front).unwrap();
        let sm = v.temperature_c(front).unwrap();
        let l1 = v.temperature_c(1.25 * front).unwrap();
        let l2 = v.temperature_c(2.0 * front).unwrap();
        assert!(s0 < s1 && s1 < s2 && s2 <= sm);
        assert!(sm < l1 && l1 < l2 && l2 < v.case.initial_liquid_temperature_c);
        assert!((-10.0..=0.0).contains(&s1));
        assert!((0.0..=2.0).contains(&l2));
    }

    #[test]
    fn similarity_front_scales_with_square_root_of_time() {
        let v = reference();
        let s_t = v.front_m;
        let half_time = Case { time_s: v.case.time_s / 4.0, ..v.case }.validate().unwrap().front_m;
        let quarter_time = Case { time_s: v.case.time_s / 16.0, ..v.case }.validate().unwrap().front_m;
        assert!((half_time / s_t - 0.5).abs() < 2.0e-14);
        assert!((quarter_time / s_t - 0.25).abs() < 2.0e-14);
    }

    #[test]
    fn integrated_wall_heat_matches_total_enthalpy_decrease() {
        let s = reference().summary().unwrap();
        assert!((s.wall_heat_j_m2 - 73_078_427.878_405_57).abs() < 2.0e-6);
        assert!(s.energy_residual_relative < 2.0e-14);
    }

    #[test]
    fn finite_comparison_domain_is_remote_from_the_front() {
        let v = reference();
        assert!(v.front_m < 0.1 * v.case.domain_length_m);
        assert!(v.far_field_coordinate >= 3.0);
    }

    #[test]
    fn invalid_material_temperature_domain_and_time_reject() {
        let mut c = Case::published_reference();
        c.conductivity_w_m_k = 0.0;
        assert!(c.validate().is_err());

        let mut c = Case::published_reference();
        c.initial_liquid_temperature_c = -1.0;
        assert!(c.validate().is_err());

        let mut c = Case::published_reference();
        c.time_s = f64::NAN;
        assert!(c.validate().is_err());

        let mut c = Case::published_reference();
        c.domain_length_m = 0.001;
        assert!(c.validate().is_err());
    }
}
