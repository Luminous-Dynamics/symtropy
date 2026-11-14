//! Unequal-property two-phase planar Stefan analytical oracle.
//!
//! Constant but distinct solid/liquid conductivity and heat capacity, equal
//! density, superheated initial liquid, fixed cold wall, no flow or volume change.
//!
//! This is a sharp-interface conduction reference, not an alloy/foundry model.
//! Compile directly with rustc; it is deliberately outside the Bevy workspace.

const SQRT_PI: f64 = 1.772_453_850_905_516_027_3;
const ERF_SERIES_MAX: f64 = 1.25;
const ROOT_MAX_UPPER: f64 = 8.0;
const ROOT_MAX_ITERATIONS: usize = 180;
const ROOT_RESIDUAL_TOL: f64 = 5.0e-14;
const ROOT_WIDTH_TOL: f64 = 2.0 * f64::EPSILON;
const MIN_DOMAIN_SIMILARITY_COORDINATE: f64 = 4.0;

#[derive(Clone, Copy, Debug)]
struct Case {
    density_kg_m3: f64,
    solid_cp_j_kg_k: f64,
    liquid_cp_j_kg_k: f64,
    solid_k_w_m_k: f64,
    liquid_k_w_m_k: f64,
    latent_heat_j_kg: f64,
    wall_temperature_c: f64,
    melting_temperature_c: f64,
    initial_liquid_temperature_c: f64,
    domain_length_m: f64,
    time_s: f64,
}

impl Case {
    fn unequal_property_fixture() -> Self {
        Self {
            density_kg_m3: 7_800.0,
            solid_cp_j_kg_k: 600.0,
            liquid_cp_j_kg_k: 800.0,
            solid_k_w_m_k: 25.0,
            liquid_k_w_m_k: 18.0,
            latent_heat_j_kg: 270_000.0,
            wall_temperature_c: -40.0,
            melting_temperature_c: 0.0,
            initial_liquid_temperature_c: 100.0,
            domain_length_m: 0.2,
            time_s: 100.0,
        }
    }

    fn equal_property_reduction() -> Self {
        Self {
            density_kg_m3: 1.0,
            solid_cp_j_kg_k: 2.5e6,
            liquid_cp_j_kg_k: 2.5e6,
            solid_k_w_m_k: 2.0,
            liquid_k_w_m_k: 2.0,
            latent_heat_j_kg: 1.0e8,
            wall_temperature_c: -10.0,
            melting_temperature_c: 0.0,
            initial_liquid_temperature_c: 2.0,
            domain_length_m: 10.0,
            time_s: 288.0 * 3600.0,
        }
    }

    fn validate(self) -> Result<ValidatedCase, String> {
        let positive = [
            self.density_kg_m3,
            self.solid_cp_j_kg_k,
            self.liquid_cp_j_kg_k,
            self.solid_k_w_m_k,
            self.liquid_k_w_m_k,
            self.latent_heat_j_kg,
            self.domain_length_m,
            self.time_s,
        ];
        if positive.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            return Err("MATERIAL_INVALID: densities, heat capacities, conductivities, latent heat, domain and time must be finite and positive".into());
        }
        if !self.wall_temperature_c.is_finite()
            || !self.melting_temperature_c.is_finite()
            || !self.initial_liquid_temperature_c.is_finite()
            || !(self.wall_temperature_c < self.melting_temperature_c
                && self.melting_temperature_c < self.initial_liquid_temperature_c)
        {
            return Err("TEMPERATURE_ORDER_INVALID: require finite Tw < Tm < T_initial".into());
        }

        let alpha_s = self.solid_k_w_m_k
            / (self.density_kg_m3 * self.solid_cp_j_kg_k);
        let alpha_l = self.liquid_k_w_m_k
            / (self.density_kg_m3 * self.liquid_cp_j_kg_k);
        if !alpha_s.is_finite() || alpha_s <= 0.0
            || !alpha_l.is_finite() || alpha_l <= 0.0 {
            return Err("DIFFUSIVITY_INVALID: both phase diffusivities must be finite and positive".into());
        }

        let stefan_solid = self.solid_cp_j_kg_k
            * (self.melting_temperature_c - self.wall_temperature_c)
            / self.latent_heat_j_kg;
        let stefan_liquid = self.liquid_cp_j_kg_k
            * (self.initial_liquid_temperature_c - self.melting_temperature_c)
            / self.latent_heat_j_kg;
        if !stefan_solid.is_finite() || stefan_solid <= 0.0
            || !stefan_liquid.is_finite() || stefan_liquid <= 0.0 {
            return Err("STEFAN_NUMBER_INVALID: both phase Stefan numbers must be finite and positive".into());
        }

        let lambda_ratio = (alpha_s / alpha_l).sqrt();
        let lambda_s = solve_lambda_s(stefan_solid, stefan_liquid, lambda_ratio)?;
        let lambda_l = lambda_s * lambda_ratio;
        let similarity_length_s_m = 2.0 * (alpha_s * self.time_s).sqrt();
        let similarity_length_l_m = 2.0 * (alpha_l * self.time_s).sqrt();
        let front_m = lambda_s * similarity_length_s_m;
        let front_l_m = lambda_l * similarity_length_l_m;
        if !front_m.is_finite() || front_m <= 0.0 || front_m >= self.domain_length_m {
            return Err("DOMAIN_INVALID: front must lie inside the finite comparison domain".into());
        }
        if (front_m - front_l_m).abs() > 64.0 * f64::EPSILON * front_m.max(1.0) {
            return Err("INTERFACE_COORDINATES_INCONSISTENT: phase similarity fronts do not agree".into());
        }
        let far_field_coordinate_l = self.domain_length_m / similarity_length_l_m;
        if far_field_coordinate_l < MIN_DOMAIN_SIMILARITY_COORDINATE {
            return Err("DOMAIN_TOO_SHORT: far boundary is not sufficiently remote for the semi-infinite reference".into());
        }

        Ok(ValidatedCase {
            case: self,
            alpha_s,
            alpha_l,
            stefan_solid,
            stefan_liquid,
            lambda_ratio,
            lambda_s,
            lambda_l,
            front_m,
            far_field_coordinate_l,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct ValidatedCase {
    case: Case,
    alpha_s: f64,
    alpha_l: f64,
    stefan_solid: f64,
    stefan_liquid: f64,
    lambda_ratio: f64,
    lambda_s: f64,
    lambda_l: f64,
    front_m: f64,
    far_field_coordinate_l: f64,
}

#[derive(Clone, Copy, Debug)]
struct Summary {
    alpha_s: f64,
    alpha_l: f64,
    stefan_solid: f64,
    stefan_liquid: f64,
    lambda_s: f64,
    lambda_l: f64,
    root_residual: f64,
    time_s: f64,
    front_m: f64,
    front_from_liquid_m: f64,
    domain_m: f64,
    far_field_coordinate_l: f64,
    temperature_quarter_front_c: f64,
    temperature_half_front_c: f64,
    temperature_125_front_c: f64,
    temperature_2_front_c: f64,
    far_boundary_temperature_c: f64,
    wall_heat_removed_j_m2: f64,
    enthalpy_decrease_j_m2: f64,
    energy_residual_j_m2: f64,
    energy_residual_relative: f64,
}

fn erf_series(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x.abs() > ERF_SERIES_MAX {
        return Err("ERF_RANGE: series is restricted to finite |x| <= 1.25".into());
    }
    if x == 0.0 {
        return Ok(0.0);
    }
    let x2 = x * x;
    let mut term = x;
    let mut sum = x;
    let mut compensation = 0.0;
    for n in 1..=160 {
        term *= -x2 / n as f64 * (2 * n - 1) as f64 / (2 * n + 1) as f64;
        let corrected = term - compensation;
        let next = sum + corrected;
        compensation = (next - sum) - corrected;
        sum = next;
        if term.abs() <= 0.2 * f64::EPSILON * sum.abs() {
            break;
        }
    }
    let result = 2.0 * sum / SQRT_PI;
    if result.is_finite() {
        Ok(result)
    } else {
        Err("ERF_NONFINITE: series produced a non-finite result".into())
    }
}

// erfc(x) = Q(1/2, x^2), evaluated with a continued fraction outside
// the small-x erf series so the far-liquid tail avoids 1-erf cancellation.
fn erfc_positive(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x < 0.0 {
        return Err("ERFC_RANGE: argument must be finite and nonnegative".into());
    }
    if x <= ERF_SERIES_MAX {
        return Ok(1.0 - erf_series(x)?);
    }
    let xx = x * x;
    if !xx.is_finite() {
        return Ok(0.0);
    }
    const A: f64 = 0.5;
    const FPMIN: f64 = 1.0e-300;
    const MAX_ITER: usize = 256;
    const EPS: f64 = 3.0e-15;
    const LOG_GAMMA_HALF: f64 = 0.572_364_942_924_700_087_1;

    let mut b = xx + 1.0 - A;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..=MAX_ITER {
        let fi = i as f64;
        let an = -fi * (fi - A);
        b += 2.0;
        d = an * d + b;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = b + an / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() <= EPS {
            break;
        }
        if i == MAX_ITER {
            return Err("ERFC_NO_CONVERGENCE: continued fraction exceeded iteration budget".into());
        }
    }
    let value = (-xx + A * xx.ln() - LOG_GAMMA_HALF).exp() * h;
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err("ERFC_NONFINITE: continued fraction produced an invalid result".into())
    }
}

fn erf_positive(x: f64) -> Result<f64, String> {
    if !x.is_finite() || x < 0.0 {
        return Err("ERF_RANGE: argument must be finite and nonnegative".into());
    }
    if x <= ERF_SERIES_MAX {
        erf_series(x)
    } else {
        Ok(1.0 - erfc_positive(x)?)
    }
}

fn lambda_residual(
    lambda_s: f64,
    stefan_solid: f64,
    stefan_liquid: f64,
    lambda_ratio: f64,
) -> Result<f64, String> {
    if !lambda_s.is_finite() || lambda_s <= 0.0
        || !lambda_ratio.is_finite() || lambda_ratio <= 0.0 {
        return Err("ROOT_ARGUMENT_INVALID: lambda_s and phase ratio must be finite and positive".into());
    }
    let lambda_l = lambda_s * lambda_ratio;
    let erf_s = erf_positive(lambda_s)?;
    let erfc_l = erfc_positive(lambda_l)?;
    if erf_s <= 0.0 || erfc_l <= 0.0 {
        return Err("ROOT_SPECIAL_FUNCTION_UNDERFLOW: erf/erfc denominator is not positive".into());
    }

    let solid_term = stefan_solid * (-lambda_s * lambda_s).exp()
        / (lambda_s * erf_s);
    let liquid_term = stefan_liquid * (-lambda_l * lambda_l).exp()
        / (lambda_l * erfc_l);
    let residual = solid_term - liquid_term - SQRT_PI;
    if residual.is_finite() {
        Ok(residual)
    } else {
        Err("ROOT_RESIDUAL_NONFINITE: transcendental residual is invalid".into())
    }
}

fn solve_lambda_s(
    stefan_solid: f64,
    stefan_liquid: f64,
    lambda_ratio: f64,
) -> Result<f64, String> {
    if !stefan_solid.is_finite() || stefan_solid <= 0.0
        || !stefan_liquid.is_finite() || stefan_liquid <= 0.0 {
        return Err("STEFAN_NUMBER_INVALID: both Stefan numbers must be finite and positive".into());
    }

    let mut lower = 1.0e-10;
    let mut upper = 0.25;
    let lower_residual = lambda_residual(lower, stefan_solid, stefan_liquid, lambda_ratio)?;
    if lower_residual <= 0.0 {
        return Err("ROOT_BRACKET_INVALID: lower endpoint is not below the positive root".into());
    }
    let mut upper_residual = lambda_residual(upper, stefan_solid, stefan_liquid, lambda_ratio)?;
    while upper_residual > 0.0 && upper < ROOT_MAX_UPPER {
        upper *= 2.0;
        upper_residual = lambda_residual(upper, stefan_solid, stefan_liquid, lambda_ratio)?;
    }
    if upper_residual >= 0.0 {
        return Err("ROOT_BRACKET_FAILED: no sign change inside the admitted lambda_s range".into());
    }

    for _ in 0..ROOT_MAX_ITERATIONS {
        let midpoint = lower + 0.5 * (upper - lower);
        let residual = lambda_residual(midpoint, stefan_solid, stefan_liquid, lambda_ratio)?;
        if residual > 0.0 {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
        if upper - lower <= ROOT_WIDTH_TOL * midpoint.max(1.0) {
            let root = lower + 0.5 * (upper - lower);
            let final_residual = lambda_residual(root, stefan_solid, stefan_liquid, lambda_ratio)?.abs();
            if final_residual > ROOT_RESIDUAL_TOL {
                return Err("ROOT_RESIDUAL_EXCEEDED: bisection root failed residual tolerance".into());
            }
            return Ok(root);
        }
    }
    Err("ROOT_NO_CONVERGENCE: bisection exceeded iteration budget".into())
}

impl ValidatedCase {
    fn temperature_c(self, position_m: f64) -> Result<f64, String> {
        if !position_m.is_finite() || position_m < 0.0 {
            return Err("POSITION_INVALID: require finite x >= 0".into());
        }
        let case = self.case;
        if position_m <= self.front_m {
            let eta_s = position_m / (2.0 * (self.alpha_s * case.time_s).sqrt());
            let value = case.wall_temperature_c
                + (case.melting_temperature_c - case.wall_temperature_c)
                    * erf_positive(eta_s)? / erf_positive(self.lambda_s)?;
            if value.is_finite() {
                Ok(value)
            } else {
                Err("SOLID_TEMPERATURE_NONFINITE".into())
            }
        } else {
            let eta_l = position_m / (2.0 * (self.alpha_l * case.time_s).sqrt());
            let denominator = erfc_positive(self.lambda_l)?;
            if denominator <= 0.0 {
                return Err("LIQUID_INTERFACE_ERFC_UNDERFLOW".into());
            }
            let value = case.initial_liquid_temperature_c
                + (case.melting_temperature_c - case.initial_liquid_temperature_c)
                    * erfc_positive(eta_l)? / denominator;
            if value.is_finite() {
                Ok(value)
            } else {
                Err("LIQUID_TEMPERATURE_NONFINITE".into())
            }
        }
    }

    fn wall_heat_removed_j_m2(self) -> Result<f64, String> {
        let case = self.case;
        let heat = 2.0 * case.solid_k_w_m_k
            * (case.melting_temperature_c - case.wall_temperature_c)
            * case.time_s.sqrt()
            / (SQRT_PI * self.alpha_s.sqrt() * erf_positive(self.lambda_s)?);
        if heat.is_finite() && heat > 0.0 {
            Ok(heat)
        } else {
            Err("WALL_HEAT_INVALID: integrated heat must be finite and positive".into())
        }
    }

    fn total_enthalpy_decrease_j_m2(self) -> Result<f64, String> {
        let c = self.case;
        let s = self.front_m;
        let a_s = 2.0 * (self.alpha_s * c.time_s).sqrt();
        let a_l = 2.0 * (self.alpha_l * c.time_s).sqrt();
        let erf_s = erf_positive(self.lambda_s)?;
        let erfc_l = erfc_positive(self.lambda_l)?;

        let integral_erf_s = s * erf_s
            + a_s / SQRT_PI * ((-self.lambda_s * self.lambda_s).exp() - 1.0);
        let integral_temperature_s = c.wall_temperature_c * s
            + (c.melting_temperature_c - c.wall_temperature_c) * integral_erf_s / erf_s;
        let integral_solid_sensible = integral_temperature_s - c.melting_temperature_c * s;

        let integral_liquid_superheat = (c.initial_liquid_temperature_c - c.melting_temperature_c)
            / erfc_l
            * (a_l / SQRT_PI * (-self.lambda_l * self.lambda_l).exp() - s * erfc_l);

        let initial_liquid_enthalpy_in_solid_region = c.density_kg_m3
            * (c.liquid_cp_j_kg_k * (c.initial_liquid_temperature_c - c.melting_temperature_c)
                + c.latent_heat_j_kg)
            * s;
        let final_solid_sensible_enthalpy = c.density_kg_m3
            * c.solid_cp_j_kg_k
            * integral_solid_sensible;
        let remaining_liquid_sensible_drop = c.density_kg_m3
            * c.liquid_cp_j_kg_k
            * integral_liquid_superheat;

        let total = initial_liquid_enthalpy_in_solid_region
            - final_solid_sensible_enthalpy
            + remaining_liquid_sensible_drop;
        if total.is_finite() && total > 0.0 {
            Ok(total)
        } else {
            Err("ENTHALPY_DECREASE_INVALID: integrated enthalpy change must be finite and positive".into())
        }
    }

    fn summary(self) -> Result<Summary, String> {
        let wall_heat = self.wall_heat_removed_j_m2()?;
        let enthalpy = self.total_enthalpy_decrease_j_m2()?;
        let residual = wall_heat - enthalpy;
        let relative = residual.abs() / wall_heat;
        let c = self.case;
        let front_from_liquid = 2.0 * self.lambda_l * (self.alpha_l * c.time_s).sqrt();

        Ok(Summary {
            alpha_s: self.alpha_s,
            alpha_l: self.alpha_l,
            stefan_solid: self.stefan_solid,
            stefan_liquid: self.stefan_liquid,
            lambda_s: self.lambda_s,
            lambda_l: self.lambda_l,
            root_residual: lambda_residual(
                self.lambda_s, self.stefan_solid, self.stefan_liquid, self.lambda_ratio
            )?,
            time_s: c.time_s,
            front_m: self.front_m,
            front_from_liquid_m: front_from_liquid,
            domain_m: c.domain_length_m,
            far_field_coordinate_l: self.far_field_coordinate_l,
            temperature_quarter_front_c: self.temperature_c(0.25 * self.front_m)?,
            temperature_half_front_c: self.temperature_c(0.5 * self.front_m)?,
            temperature_125_front_c: self.temperature_c(1.25 * self.front_m)?,
            temperature_2_front_c: self.temperature_c(2.0 * self.front_m)?,
            far_boundary_temperature_c: self.temperature_c(c.domain_length_m)?,
            wall_heat_removed_j_m2: wall_heat,
            enthalpy_decrease_j_m2: enthalpy,
            energy_residual_j_m2: residual,
            energy_residual_relative: relative,
        })
    }
}

fn verify() -> Result<(), String> {
    let unequal = Case::unequal_property_fixture().validate()?.summary()?;
    const EXPECTED_LAMBDA_S: f64 = 0.146_746_679_540_970_45;
    const EXPECTED_LAMBDA_L: f64 = 0.199_696_936_846_165_1;
    const EXPECTED_FRONT: f64 = 0.006_783_367_661_398_094;
    const EXPECTED_WALL_HEAT: f64 = 29_695_671.555_602_343;

    if (unequal.alpha_s - 5.341_880_341_880_342e-6).abs() > 1.0e-20
        || (unequal.alpha_l - 2.884_615_384_615_384_6e-6).abs() > 1.0e-20 {
        return Err("UNEQUAL_DIFFUSIVITY_KNOWN_ANSWER_MISMATCH".into());
    }
    if (unequal.lambda_s - EXPECTED_LAMBDA_S).abs() > 2.0e-14
        || (unequal.lambda_l - EXPECTED_LAMBDA_L).abs() > 2.0e-14 {
        return Err("UNEQUAL_SIMILARITY_PARAMETER_MISMATCH".into());
    }
    if unequal.root_residual.abs() > ROOT_RESIDUAL_TOL {
        return Err("UNEQUAL_ROOT_RESIDUAL_EXCEEDED".into());
    }
    if (unequal.front_m - EXPECTED_FRONT).abs() > 2.0e-13
        || (unequal.front_from_liquid_m - unequal.front_m).abs() > 2.0e-14 {
        return Err("UNEQUAL_FRONT_KNOWN_ANSWER_MISMATCH".into());
    }
    if (unequal.wall_heat_removed_j_m2 - EXPECTED_WALL_HEAT).abs() > 2.0e-5 {
        return Err("UNEQUAL_WALL_HEAT_KNOWN_ANSWER_MISMATCH".into());
    }
    if unequal.energy_residual_relative > 5.0e-12 {
        return Err("UNEQUAL_ENERGY_CLOSURE_FAILED".into());
    }
    if (unequal.temperature_quarter_front_c - (-29.932_683_869_813_198)).abs() > 2.0e-10
        || (unequal.temperature_half_front_c - (-19.892_424_674_940_905)).abs() > 2.0e-10
        || (unequal.temperature_125_front_c - 6.886_444_213_064_353).abs() > 2.0e-10
        || (unequal.temperature_2_front_c - 26.418_277_221_933_892).abs() > 2.0e-10 {
        return Err("UNEQUAL_TEMPERATURE_PROFILE_MISMATCH".into());
    }
    if (unequal.far_boundary_temperature_c - 100.0).abs() > 1.0e-10 {
        return Err("UNEQUAL_FAR_FIELD_BOUNDARY_MISMATCH".into());
    }

    // Metamorphic equal-property reduction: the two coupled parameters collapse
    // to the existing published two-phase oracle's one lambda.
    let equal = Case::equal_property_reduction().validate()?.summary()?;
    if (equal.lambda_s - 0.322_390_632_922_300_86).abs() > 2.0e-14
        || (equal.lambda_l - equal.lambda_s).abs() > 2.0e-14
        || (equal.front_m - 0.587_225_489_090_536_2).abs() > 2.0e-13
        || equal.energy_residual_relative > 5.0e-12 {
        return Err("EQUAL_PROPERTY_LIMIT_DID_NOT_REDUCE_TO_PUBLISHED_ORACLE".into());
    }
    Ok(())
}

fn emit_csv() -> Result<(), String> {
    println!("case_id,rho_kg_m3,cp_s_j_kg_k,cp_l_j_kg_k,k_s_w_m_k,k_l_w_m_k,L_j_kg,Tw_c,Tm_c,Tinf_c,alpha_s_m2_s,alpha_l_m2_s,Ste_s,Ste_l,lambda_s,lambda_l,root_residual,time_s,front_s_m,front_l_m,domain_m,far_eta_l,T_025front_c,T_05front_c,T_125front_c,T_2front_c,T_far_c,Q_wall_j_m2,enthalpy_drop_j_m2,energy_residual_j_m2,energy_residual_relative");
    for (case_id, case) in [
        ("unequal", Case::unequal_property_fixture()),
        ("equal_reduction", Case::equal_property_reduction()),
    ] {
        let v = case.validate()?;
        let s = v.summary()?;
        println!(
            "{},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e},{:.8e}",
            case_id,
            case.density_kg_m3,
            case.solid_cp_j_kg_k,
            case.liquid_cp_j_kg_k,
            case.solid_k_w_m_k,
            case.liquid_k_w_m_k,
            case.latent_heat_j_kg,
            case.wall_temperature_c,
            case.melting_temperature_c,
            case.initial_liquid_temperature_c,
            s.alpha_s,
            s.alpha_l,
            s.stefan_solid,
            s.stefan_liquid,
            s.lambda_s,
            s.lambda_l,
            s.root_residual,
            s.time_s,
            s.front_m,
            s.front_from_liquid_m,
            s.domain_m,
            s.far_field_coordinate_l,
            s.temperature_quarter_front_c,
            s.temperature_half_front_c,
            s.temperature_125_front_c,
            s.temperature_2_front_c,
            s.far_boundary_temperature_c,
            s.wall_heat_removed_j_m2,
            s.enthalpy_decrease_j_m2,
            s.energy_residual_j_m2,
            s.energy_residual_relative
        );
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("UNEQUAL_TWO_PHASE_STEFAN_OK: coupled phase similarity, profile and energy closure")),
        [arg] if arg == "--verify" => verify().map(|()| println!("UNEQUAL_TWO_PHASE_STEFAN_OK: coupled phase similarity, profile and energy closure")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: two-phase-unequal [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("UNEQUAL_TWO_PHASE_STEFAN_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unequal_property_similarity_known_answers() {
        let v = Case::unequal_property_fixture().validate().unwrap();
        assert!((v.alpha_s - 5.341_880_341_880_342e-6).abs() < 1.0e-20);
        assert!((v.alpha_l - 2.884_615_384_615_384_6e-6).abs() < 1.0e-20);
        assert!((v.lambda_s - 0.146_746_679_540_970_45).abs() < 2.0e-14);
        assert!((v.lambda_l - 0.199_696_936_846_165_1).abs() < 2.0e-14);
        assert!((v.front_m - 0.006_783_367_661_398_094).abs() < 2.0e-13);
    }

    #[test]
    fn interface_continuity_wall_and_far_field_conditions_hold() {
        let v = Case::unequal_property_fixture().validate().unwrap();
        assert!((v.temperature_c(0.0).unwrap() - (-40.0)).abs() < 1.0e-13);
        assert!(v.temperature_c(v.front_m).unwrap().abs() < 1.0e-12);
        assert!((v.temperature_c(0.2).unwrap() - 100.0).abs() < 1.0e-10);
        assert!((v.front_m - 2.0 * v.lambda_l * (v.alpha_l * v.case.time_s).sqrt()).abs() < 2.0e-14);
    }

    #[test]
    fn solid_and_liquid_profiles_are_monotone() {
        let v = Case::unequal_property_fixture().validate().unwrap();
        let s = v.front_m;
        let ts0 = v.temperature_c(0.0).unwrap();
        let ts1 = v.temperature_c(0.25 * s).unwrap();
        let ts2 = v.temperature_c(0.5 * s).unwrap();
        let tsm = v.temperature_c(s).unwrap();
        let tl1 = v.temperature_c(1.25 * s).unwrap();
        let tl2 = v.temperature_c(2.0 * s).unwrap();
        let tlf = v.temperature_c(v.case.domain_length_m).unwrap();
        assert!(ts0 < ts1 && ts1 < ts2 && ts2 < tsm);
        assert!(tsm < tl1 && tl1 < tl2 && tl2 < tlf);
    }

    #[test]
    fn wall_heat_matches_independent_total_enthalpy_decrease() {
        let summary = Case::unequal_property_fixture().validate().unwrap().summary().unwrap();
        assert!((summary.wall_heat_removed_j_m2 - 29_695_671.555_602_343).abs() < 2.0e-5);
        assert!(summary.energy_residual_relative < 5.0e-12);
    }

    #[test]
    fn interface_scales_with_square_root_of_time() {
        let case = Case::unequal_property_fixture();
        let full = case.validate().unwrap().front_m;
        let quarter = Case { time_s: case.time_s / 4.0, ..case }.validate().unwrap().front_m;
        let sixteenth = Case { time_s: case.time_s / 16.0, ..case }.validate().unwrap().front_m;
        assert!((quarter / full - 0.5).abs() < 2.0e-14);
        assert!((sixteenth / full - 0.25).abs() < 2.0e-14);
    }

    #[test]
    fn equal_property_limit_matches_existing_two_phase_reference() {
        let summary = Case::equal_property_reduction().validate().unwrap().summary().unwrap();
        assert!((summary.lambda_s - 0.322_390_632_922_300_86).abs() < 2.0e-14);
        assert!((summary.lambda_l - summary.lambda_s).abs() < 2.0e-14);
        assert!((summary.front_m - 0.587_225_489_090_536_2).abs() < 2.0e-13);
    }

    #[test]
    fn invalid_properties_temperatures_time_and_geometry_reject() {
        let mut c = Case::unequal_property_fixture();
        c.solid_k_w_m_k = 0.0;
        assert!(c.validate().is_err());

        let mut c = Case::unequal_property_fixture();
        c.liquid_cp_j_kg_k = f64::NAN;
        assert!(c.validate().is_err());

        let mut c = Case::unequal_property_fixture();
        c.initial_liquid_temperature_c = -10.0;
        assert!(c.validate().is_err());

        let mut c = Case::unequal_property_fixture();
        c.time_s = 0.0;
        assert!(c.validate().is_err());

        let mut c = Case::unequal_property_fixture();
        c.domain_length_m = 0.001;
        assert!(c.validate().is_err());
    }

    #[test]
    fn deterministic_both_fixture_verification() {
        verify().unwrap();
        let one = Case::unequal_property_fixture().validate().unwrap().summary().unwrap();
        let two = Case::unequal_property_fixture().validate().unwrap().summary().unwrap();
        assert_eq!(one.front_m, two.front_m);
        assert_eq!(one.wall_heat_removed_j_m2, two.wall_heat_removed_j_m2);
    }
}
