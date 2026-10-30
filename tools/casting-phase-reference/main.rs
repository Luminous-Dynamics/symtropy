//! Dependency-free analytical oracle for one-phase planar Stefan freezing.
//!
//! This is a research reference, not a numerical PDE solver or a material model.
//! Compile directly with:
//!   rustc --edition=2024 -D warnings tools/casting-phase-reference/main.rs -o /tmp/stefan-reference
//!
//! Model: semi-infinite initially liquid domain at T_m, a wall stepped to
//! T_wall < T_m at x=0, constant solid properties, zero liquid-side gradient,
//! no fluid advection, and no volumetric heat source.

const SQRT_PI: f64 = 1.772_453_850_905_516;
const ERF_SERIES_MAX_X: f64 = 1.0;
const MAX_ROOT_ITERATIONS: usize = 128;
const ROOT_INTERVAL_TOLERANCE: f64 = 4.0 * f64::EPSILON;

#[derive(Debug, Clone, Copy)]
struct StefanCase {
    density_kg_m3: f64,
    solid_heat_capacity_j_kg_k: f64,
    solid_conductivity_w_m_k: f64,
    latent_heat_j_kg: f64,
    wall_temperature_c: f64,
    melting_temperature_c: f64,
}

impl StefanCase {
    fn reference() -> Self {
        Self {
            density_kg_m3: 1_000.0,
            solid_heat_capacity_j_kg_k: 2_000.0,
            solid_conductivity_w_m_k: 0.5,
            latent_heat_j_kg: 200_000.0,
            wall_temperature_c: -20.0,
            melting_temperature_c: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct StefanSolution {
    case: StefanCase,
    diffusivity_m2_s: f64,
    stefan_number: f64,
    similarity_lambda: f64,
    root_residual: f64,
}

impl StefanSolution {
    fn new(case: StefanCase) -> Result<Self, String> {
        let positive_inputs = [
            case.density_kg_m3,
            case.solid_heat_capacity_j_kg_k,
            case.solid_conductivity_w_m_k,
            case.latent_heat_j_kg,
        ];
        if positive_inputs.iter().any(|value| !value.is_finite() || *value <= 0.0) {
            return Err("density, heat capacity, conductivity and latent heat must be finite and positive".into());
        }
        if !case.wall_temperature_c.is_finite()
            || !case.melting_temperature_c.is_finite()
            || case.wall_temperature_c >= case.melting_temperature_c
        {
            return Err("freezing reference requires finite wall_temperature < melting_temperature".into());
        }

        let diffusivity_m2_s = case.solid_conductivity_w_m_k
            / (case.density_kg_m3 * case.solid_heat_capacity_j_kg_k);
        let stefan_number = case.solid_heat_capacity_j_kg_k
            * (case.melting_temperature_c - case.wall_temperature_c)
            / case.latent_heat_j_kg;

        if !diffusivity_m2_s.is_finite() || diffusivity_m2_s <= 0.0 {
            return Err("computed thermal diffusivity is not finite and positive".into());
        }
        // The dependency-free erf series is deliberately bounded to |x| <= 1.
        // For 0 < Ste <= 1, the positive similarity root is strictly below 1.
        if !stefan_number.is_finite() || !(0.0 < stefan_number && stefan_number <= 1.0) {
            return Err("reference profile requires 0 < Stefan number <= 1".into());
        }

        let similarity_lambda = solve_similarity_root(stefan_number)?;
        let root_residual = similarity_residual(similarity_lambda, stefan_number)?;

        Ok(Self {
            case,
            diffusivity_m2_s,
            stefan_number,
            similarity_lambda,
            root_residual,
        })
    }

    fn front_position_m(self, time_s: f64) -> Result<f64, String> {
        if !time_s.is_finite() || time_s <= 0.0 {
            return Err("time must be finite and strictly positive".into());
        }
        let front = 2.0 * self.similarity_lambda * (self.diffusivity_m2_s * time_s).sqrt();
        if !front.is_finite() || front <= 0.0 {
            return Err("computed phase-front position is invalid".into());
        }
        Ok(front)
    }

    /// Exact one-phase similarity temperature in Celsius.
    ///
    /// The solid occupies 0 <= x < s(t); the liquid is isothermal at T_m
    /// for x >= s(t). This sharp-interface model has no mushy zone.
    fn temperature_c(self, position_m: f64, time_s: f64) -> Result<f64, String> {
        if !position_m.is_finite() || position_m < 0.0 {
            return Err("position must be finite and nonnegative".into());
        }
        let front = self.front_position_m(time_s)?;
        if position_m >= front {
            return Ok(self.case.melting_temperature_c);
        }

        let similarity_coordinate = position_m / (2.0 * (self.diffusivity_m2_s * time_s).sqrt());
        let numerator = erf_series(similarity_coordinate)?;
        let denominator = erf_series(self.similarity_lambda)?;
        let temperature = self.case.wall_temperature_c
            + (self.case.melting_temperature_c - self.case.wall_temperature_c)
                * numerator
                / denominator;

        if !temperature.is_finite() {
            return Err("computed temperature is non-finite".into());
        }
        Ok(temperature)
    }
}

/// Error function by its power series. Its use is restricted to |x| <= 1.
///
/// erf(x) = 2/sqrt(pi) * sum_n [(-1)^n x^(2n+1) / (n! (2n+1))].
fn erf_series(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x.abs() > ERF_SERIES_MAX_X {
        return Err("erf series is only admitted for finite |x| <= 1".into());
    }
    if x == 0.0 {
        return Ok(0.0);
    }

    let x_squared = x * x;
    let mut term = x;
    let mut sum = term;

    for n in 1..=64 {
        let n_f64 = n as f64;
        term *= -x_squared / n_f64 * ((2 * n - 1) as f64) / ((2 * n + 1) as f64);
        sum += term;
        if term.abs() <= 0.25 * f64::EPSILON * sum.abs() {
            break;
        }
    }

    let result = 2.0 / SQRT_PI * sum;
    if result.is_finite() {
        Ok(result)
    } else {
        Err("erf series produced a non-finite value".into())
    }
}

fn similarity_residual(lambda: f64, stefan_number: f64) -> Result<f64, String> {
    if !lambda.is_finite() || !(0.0 < lambda && lambda <= 1.0) {
        return Err("similarity root trial must be finite and in (0, 1]".into());
    }
    if !stefan_number.is_finite() || !(0.0 < stefan_number && stefan_number <= 1.0) {
        return Err("Stefan number must be in (0, 1]".into());
    }
    let erf_lambda = erf_series(lambda)?;
    Ok(lambda * (lambda * lambda).exp() * erf_lambda - stefan_number / SQRT_PI)
}

/// Bisection for lambda * exp(lambda^2) * erf(lambda) = Ste / sqrt(pi).
fn solve_similarity_root(stefan_number: f64) -> Result<f64, String> {
    if !stefan_number.is_finite() || !(0.0 < stefan_number && stefan_number <= 1.0) {
        return Err("Stefan number must be in (0, 1]".into());
    }

    let mut lower = 0.0;
    let mut upper = 1.0;
    let lower_residual = -stefan_number / SQRT_PI;
    let upper_residual = similarity_residual(upper, stefan_number)?;
    if !(lower_residual < 0.0 && upper_residual > 0.0) {
        return Err("failed to bracket the positive similarity root".into());
    }

    for _ in 0..MAX_ROOT_ITERATIONS {
        let midpoint = lower + (upper - lower) * 0.5;
        let residual = similarity_residual(midpoint, stefan_number)?;
        if residual < 0.0 {
            lower = midpoint;
        } else {
            upper = midpoint;
        }

        if upper - lower <= ROOT_INTERVAL_TOLERANCE * midpoint.max(1.0) {
            let root = lower + (upper - lower) * 0.5;
            let final_residual = similarity_residual(root, stefan_number)?.abs();
            if final_residual > 2.0e-14 {
                return Err("similarity root failed the residual tolerance".into());
            }
            return Ok(root);
        }
    }

    Err("similarity root did not converge within the iteration budget".into())
}

fn verify_reference() -> Result<(), String> {
    let solution = StefanSolution::new(StefanCase::reference())?;

    const EXPECTED_LAMBDA: f64 = 0.306_423_905_361_211_247;
    const EXPECTED_FRONT_60_S_M: f64 = 0.002_373_549_364_687_617_429;
    const EXPECTED_HALF_FRONT_TEMPERATURE_C: f64 = -9.766_220_473_731_098;

    if (solution.diffusivity_m2_s - 2.5e-7).abs() > 1.0e-20 {
        return Err("known-answer diffusivity mismatch".into());
    }
    if (solution.stefan_number - 0.2).abs() > 1.0e-14 {
        return Err("known-answer Stefan-number mismatch".into());
    }
    if (solution.similarity_lambda - EXPECTED_LAMBDA).abs() > 2.0e-14 {
        return Err("known-answer similarity-root mismatch".into());
    }
    if solution.root_residual.abs() > 2.0e-14 {
        return Err("similarity-root residual exceeds tolerance".into());
    }

    let front_60 = solution.front_position_m(60.0)?;
    if (front_60 - EXPECTED_FRONT_60_S_M).abs() > 2.0e-15 {
        return Err("known-answer front-position mismatch at 60 s".into());
    }

    let half_temperature = solution.temperature_c(front_60 * 0.5, 60.0)?;
    if (half_temperature - EXPECTED_HALF_FRONT_TEMPERATURE_C).abs() > 2.0e-12 {
        return Err("known-answer mid-solid temperature mismatch".into());
    }

    for (time_s, expected_front) in [
        (60.0, EXPECTED_FRONT_60_S_M),
        (240.0, 2.0 * EXPECTED_FRONT_60_S_M),
        (960.0, 4.0 * EXPECTED_FRONT_60_S_M),
    ] {
        let front = solution.front_position_m(time_s)?;
        if (front - expected_front).abs() > 8.0e-15 {
            return Err(format!("front-position scale check failed at {time_s} s"));
        }
    }

    Ok(())
}

fn emit_csv() -> Result<(), String> {
    let solution = StefanSolution::new(StefanCase::reference())?;
    println!("time_s,lambda,stefan_number,alpha_m2_s,front_m,wall_temperature_c,half_front_temperature_c,front_temperature_c");
    for time_s in [60.0, 240.0, 960.0] {
        let front = solution.front_position_m(time_s)?;
        let wall_temperature = solution.temperature_c(0.0, time_s)?;
        let half_temperature = solution.temperature_c(0.5 * front, time_s)?;
        let front_temperature = solution.temperature_c(front, time_s)?;
        println!(
            "{:.0},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}",
            time_s,
            solution.similarity_lambda,
            solution.stefan_number,
            solution.diffusivity_m2_s,
            front,
            wall_temperature,
            half_temperature,
            front_temperature
        );
    }
    Ok(())
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [] => verify_reference().map(|()| println!("REFERENCE_OK: one-phase Stefan known answers and invariants")),
        [arg] if arg == "--verify" => {
            verify_reference().map(|()| println!("REFERENCE_OK: one-phase Stefan known answers and invariants"))
        }
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: stefan-reference [--verify|--csv|--help]");
            println!("Default action: --verify");
            Ok(())
        }
        _ => Err("expected at most one argument: --verify, --csv or --help".into()),
    };

    if let Err(error) = result {
        eprintln!("REFERENCE_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference() -> StefanSolution {
        StefanSolution::new(StefanCase::reference()).expect("reference case is valid")
    }

    #[test]
    fn frozen_known_answer_root_and_front() {
        let solution = reference();
        assert!((solution.similarity_lambda - 0.306_423_905_361_211_247).abs() < 2.0e-14);
        assert!((solution.front_position_m(60.0).unwrap() - 0.002_373_549_364_687_617_429).abs() < 2.0e-15);
        assert!(solution.root_residual.abs() < 2.0e-14);
    }

    #[test]
    fn boundary_and_interface_conditions_hold() {
        let solution = reference();
        let front = solution.front_position_m(60.0).unwrap();
        assert!((solution.temperature_c(0.0, 60.0).unwrap() - (-20.0)).abs() < 1.0e-13);
        assert!((solution.temperature_c(front, 60.0).unwrap()).abs() < 1.0e-13);
        assert!((solution.temperature_c(front * 1.1, 60.0).unwrap()).abs() < 1.0e-13);
    }

    #[test]
    fn solid_temperature_is_monotone_and_bounded() {
        let solution = reference();
        let time_s = 60.0;
        let front = solution.front_position_m(time_s).unwrap();
        let t0 = solution.temperature_c(0.0, time_s).unwrap();
        let t1 = solution.temperature_c(0.25 * front, time_s).unwrap();
        let t2 = solution.temperature_c(0.5 * front, time_s).unwrap();
        let t3 = solution.temperature_c(0.75 * front, time_s).unwrap();
        let t4 = solution.temperature_c(front, time_s).unwrap();
        assert!(t0 <= t1 && t1 <= t2 && t2 <= t3 && t3 <= t4);
        assert!(((-20.0)..=0.0).contains(&t2));
    }

    #[test]
    fn front_obeys_square_root_time_similarity() {
        let solution = reference();
        let s60 = solution.front_position_m(60.0).unwrap();
        let s240 = solution.front_position_m(240.0).unwrap();
        let s960 = solution.front_position_m(960.0).unwrap();
        assert!((s240 / s60 - 2.0).abs() < 1.0e-14);
        assert!((s960 / s60 - 4.0).abs() < 1.0e-14);
    }

    #[test]
    fn invalid_physical_inputs_and_times_are_rejected() {
        let mut invalid = StefanCase::reference();
        invalid.solid_conductivity_w_m_k = 0.0;
        assert!(StefanSolution::new(invalid).is_err());

        let mut invalid = StefanCase::reference();
        invalid.wall_temperature_c = 1.0;
        assert!(StefanSolution::new(invalid).is_err());

        let mut invalid = StefanCase::reference();
        invalid.latent_heat_j_kg = 1.0;
        assert!(StefanSolution::new(invalid).is_err());

        let solution = reference();
        assert!(solution.front_position_m(0.0).is_err());
        assert!(solution.front_position_m(f64::NAN).is_err());
        assert!(solution.temperature_c(-1.0, 60.0).is_err());
    }

    #[test]
    fn erf_reference_domain_is_enforced() {
        assert!(erf_series(1.000_001).is_err());
        assert!(erf_series(f64::NAN).is_err());
        assert!(erf_series(0.0).unwrap().abs() < 1.0e-15);
    }
}
