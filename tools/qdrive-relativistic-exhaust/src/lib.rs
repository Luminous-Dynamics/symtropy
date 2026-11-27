//! Relativistic particle and exhaust reference kernels.
//!
//! This crate is deliberately narrower than a q-drive model. It does not model
//! plasma interaction, electromagnetic harvesting, field dynamics, medium
//! recoil, orbital trajectories, or a complete spacecraft energy budget.
//!
//! Rate convention: exhaust mass flow is rest mass crossing the nozzle exit
//! per unit coordinate time in the instantaneous vehicle frame.

/// Exact speed of light in vacuum, m/s.
pub const SPEED_OF_LIGHT_M_S: f64 = 299_792_458.0;

/// Four-momentum of one particle in an explicitly declared inertial frame.
///
/// The temporal component is exposed as energy in joules and the spatial
/// components as momentum in kg m/s. The invariant uses the (+---) metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FourMomentum {
    energy_j: f64,
    momentum_kg_m_s: [f64; 3],
}

impl FourMomentum {
    /// Construct a particle four-momentum from rest mass and frame velocity.
    pub fn from_particle(rest_mass_kg: f64, velocity_m_s: [f64; 3]) -> Result<Self, &'static str> {
        if !rest_mass_kg.is_finite() || rest_mass_kg <= 0.0 {
            return Err("rest_mass_kg must be finite and positive");
        }
        if !finite_vector(velocity_m_s) {
            return Err("velocity_m_s must contain only finite values");
        }

        let speed_m_s = vector_norm(velocity_m_s);
        if !speed_m_s.is_finite() || speed_m_s >= SPEED_OF_LIGHT_M_S {
            return Err("particle speed must be finite and below the speed of light");
        }

        let beta = speed_m_s / SPEED_OF_LIGHT_M_S;
        let gamma = 1.0 / (1.0 - beta * beta).sqrt();
        let energy_j = gamma * rest_mass_kg * SPEED_OF_LIGHT_M_S.powi(2);
        let momentum_kg_m_s = velocity_m_s.map(|component| gamma * rest_mass_kg * component);

        if !energy_j.is_finite() || !finite_vector(momentum_kg_m_s) {
            return Err("four-momentum calculation overflowed its finite domain");
        }

        Ok(Self {
            energy_j,
            momentum_kg_m_s,
        })
    }

    /// Particle energy in the represented inertial frame, joules.
    pub fn energy_j(self) -> f64 {
        self.energy_j
    }

    /// Particle momentum in the represented inertial frame, kg m/s.
    pub fn momentum_kg_m_s(self) -> [f64; 3] {
        self.momentum_kg_m_s
    }

    /// Recover invariant rest mass using E²/c⁴ - |p|²/c².
    pub fn invariant_rest_mass_kg(self) -> Result<f64, &'static str> {
        if !self.energy_j.is_finite()
            || self.energy_j <= 0.0
            || !finite_vector(self.momentum_kg_m_s)
        {
            return Err("four-momentum must be finite and future-directed");
        }

        let energy_over_c = self.energy_j / SPEED_OF_LIGHT_M_S;
        let momentum_squared = dot(self.momentum_kg_m_s, self.momentum_kg_m_s);
        let energy_squared = energy_over_c * energy_over_c;
        let invariant_mass_times_c_squared = energy_squared - momentum_squared;

        if !invariant_mass_times_c_squared.is_finite() {
            return Err("four-momentum invariant overflowed its finite domain");
        }

        let roundoff_bound = 64.0 * f64::EPSILON * (energy_squared + momentum_squared).max(1.0);
        if invariant_mass_times_c_squared < -roundoff_bound {
            return Err("four-momentum is spacelike beyond numerical roundoff");
        }

        Ok(invariant_mass_times_c_squared.max(0.0).sqrt() / SPEED_OF_LIGHT_M_S)
    }

    /// Transform this particle four-momentum into a frame moving at
    /// frame_velocity_m_s relative to the represented frame.
    ///
    /// The sign convention is E' = gamma(E - v·p). Thus, a particle at rest in
    /// the original frame has negative momentum in a frame moving in the
    /// positive direction.
    pub fn boost_to_moving_frame(self, frame_velocity_m_s: [f64; 3]) -> Result<Self, &'static str> {
        if !self.energy_j.is_finite()
            || self.energy_j <= 0.0
            || !finite_vector(self.momentum_kg_m_s)
        {
            return Err("four-momentum must be finite and future-directed");
        }
        if !finite_vector(frame_velocity_m_s) {
            return Err("frame_velocity_m_s must contain only finite values");
        }

        let beta = frame_velocity_m_s.map(|component| component / SPEED_OF_LIGHT_M_S);
        let beta_squared = dot(beta, beta);
        if !beta_squared.is_finite() || beta_squared >= 1.0 {
            return Err("frame speed must be finite and below the speed of light");
        }
        if beta_squared == 0.0 {
            return Ok(self);
        }

        let gamma = 1.0 / (1.0 - beta_squared).sqrt();
        let beta_dot_momentum = dot(beta, self.momentum_kg_m_s);
        let energy_j = gamma * (self.energy_j - SPEED_OF_LIGHT_M_S * beta_dot_momentum);
        let momentum_coefficient = ((gamma - 1.0) / beta_squared) * beta_dot_momentum
            - gamma * self.energy_j / SPEED_OF_LIGHT_M_S;
        let momentum_kg_m_s = std::array::from_fn(|index| {
            self.momentum_kg_m_s[index] + momentum_coefficient * beta[index]
        });

        if !energy_j.is_finite() || energy_j <= 0.0 || !finite_vector(momentum_kg_m_s) {
            return Err("Lorentz transformation overflowed its finite domain");
        }

        Ok(Self {
            energy_j,
            momentum_kg_m_s,
        })
    }
}

/// Exhaust velocity components, expressed in the instantaneous vehicle frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativisticExhaustInput {
    /// Rest mass crossing the nozzle exit per vehicle-frame coordinate second.
    pub rest_mass_flow_kg_s: f64,
    /// Exhaust velocity vector in the instantaneous vehicle frame, m/s.
    pub exhaust_velocity_m_s: [f64; 3],
}

/// Relativistic exhaust accounting in the instantaneous vehicle frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativisticExhaustEstimate {
    pub beta: f64,
    pub gamma: f64,
    pub speed_m_s: f64,
    /// Exhaust momentum carried outward per second, expressed as a force vector.
    pub exhaust_momentum_flux_n: [f64; 3],
    /// Reaction force on the vehicle, opposite to the exhaust momentum flux.
    pub thrust_on_vehicle_n: [f64; 3],
    pub thrust_magnitude_n: f64,
    /// Rest-mass energy crossing the nozzle boundary per second.
    pub rest_mass_energy_flux_w: f64,
    /// Total exhaust energy flux, including rest-mass energy.
    pub total_energy_flux_w: f64,
    /// Exhaust kinetic-energy flux; excludes rest-mass energy.
    pub kinetic_power_w: f64,
}

/// Evaluate relativistic momentum and energy carried by a steady exhaust stream.
///
/// For the declared rate convention:
/// - momentum flux magnitude = mdot * gamma * speed;
/// - total energy flux = mdot * gamma * c²;
/// - kinetic power = mdot * (gamma - 1) * c².
///
/// The function is an exhaust submodel only. It does not establish where the
/// exhaust energy came from or close the vehicle + field + medium + radiation
/// four-momentum balance required to qualify a q-drive.
pub fn evaluate_relativistic_exhaust(
    input: RelativisticExhaustInput,
) -> Result<RelativisticExhaustEstimate, &'static str> {
    if !input.rest_mass_flow_kg_s.is_finite() || input.rest_mass_flow_kg_s < 0.0 {
        return Err("rest_mass_flow_kg_s must be finite and non-negative");
    }
    if !finite_vector(input.exhaust_velocity_m_s) {
        return Err("exhaust_velocity_m_s must contain only finite values");
    }

    let speed_m_s = vector_norm(input.exhaust_velocity_m_s);
    if !speed_m_s.is_finite() || speed_m_s >= SPEED_OF_LIGHT_M_S {
        return Err("exhaust speed must be finite and below the speed of light");
    }

    let beta = speed_m_s / SPEED_OF_LIGHT_M_S;
    let beta_squared = beta * beta;
    let gamma = 1.0 / (1.0 - beta_squared).sqrt();
    // Stable form of gamma - 1. Avoid catastrophic cancellation when beta is
    // small and gamma rounds to 1.0 in f64.
    let gamma_minus_one = beta_squared * gamma / (1.0 + (1.0 - beta_squared).sqrt());
    let momentum_scale = input.rest_mass_flow_kg_s * gamma;
    let exhaust_momentum_flux_n = input
        .exhaust_velocity_m_s
        .map(|component| momentum_scale * component);
    let thrust_on_vehicle_n = exhaust_momentum_flux_n.map(|component| -component);
    let thrust_magnitude_n = vector_norm(exhaust_momentum_flux_n);
    let rest_mass_energy_flux_w = input.rest_mass_flow_kg_s * SPEED_OF_LIGHT_M_S.powi(2);
    let total_energy_flux_w = rest_mass_energy_flux_w * gamma;
    let kinetic_power_w = input.rest_mass_flow_kg_s * gamma_minus_one * SPEED_OF_LIGHT_M_S.powi(2);

    let outputs = [
        beta,
        gamma,
        speed_m_s,
        thrust_magnitude_n,
        rest_mass_energy_flux_w,
        total_energy_flux_w,
        kinetic_power_w,
    ];
    if !outputs.into_iter().all(f64::is_finite)
        || !finite_vector(exhaust_momentum_flux_n)
        || !finite_vector(thrust_on_vehicle_n)
    {
        return Err("exhaust calculation overflowed its finite domain");
    }

    Ok(RelativisticExhaustEstimate {
        beta,
        gamma,
        speed_m_s,
        exhaust_momentum_flux_n,
        thrust_on_vehicle_n,
        thrust_magnitude_n,
        rest_mass_energy_flux_w,
        total_energy_flux_w,
        kinetic_power_w,
    })
}

fn finite_vector(vector: [f64; 3]) -> bool {
    vector.into_iter().all(f64::is_finite)
}

fn vector_norm(vector: [f64; 3]) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
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
    fn gamma_reference_values_cover_qdrive_study_speeds() {
        let cases = [
            (0.05, 1.001_252_348_643_517_6),
            (0.10, 1.005_037_815_259_212),
            (0.20, 1.020_620_726_159_657_6),
        ];

        for (beta, expected_gamma) in cases {
            let result = evaluate_relativistic_exhaust(RelativisticExhaustInput {
                rest_mass_flow_kg_s: 1.0,
                exhaust_velocity_m_s: [beta * SPEED_OF_LIGHT_M_S, 0.0, 0.0],
            })
            .unwrap();
            close(result.gamma, expected_gamma, 1e-14);
        }
    }

    #[test]
    fn exhaust_energy_and_momentum_are_separate_and_close() {
        let flow = 2.5;
        let beta = 0.20;
        let speed = beta * SPEED_OF_LIGHT_M_S;
        let result = evaluate_relativistic_exhaust(RelativisticExhaustInput {
            rest_mass_flow_kg_s: flow,
            exhaust_velocity_m_s: [speed, 0.0, 0.0],
        })
        .unwrap();

        close(
            result.exhaust_momentum_flux_n[0],
            flow * result.gamma * speed,
            1e-8,
        );
        close(
            result.thrust_on_vehicle_n[0],
            -result.exhaust_momentum_flux_n[0],
            1e-8,
        );
        close(
            result.total_energy_flux_w,
            flow * result.gamma * SPEED_OF_LIGHT_M_S.powi(2),
            1e-6,
        );
        close(
            result.kinetic_power_w,
            flow * (result.gamma - 1.0) * SPEED_OF_LIGHT_M_S.powi(2),
            1e-6,
        );
        close(
            result.total_energy_flux_w - result.rest_mass_energy_flux_w,
            result.kinetic_power_w,
            64.0,
        );
    }

    #[test]
    fn low_beta_limit_recovers_newtonian_exhaust_relations() {
        let flow = 0.7;
        for beta in [1.0e-4, 1.0e-8, 1.0e-10] {
            let speed = beta * SPEED_OF_LIGHT_M_S;
            let result = evaluate_relativistic_exhaust(RelativisticExhaustInput {
                rest_mass_flow_kg_s: flow,
                exhaust_velocity_m_s: [speed, 0.0, 0.0],
            })
            .unwrap();

            let newtonian_thrust = flow * speed;
            let newtonian_power = 0.5 * flow * speed.powi(2);
            let thrust_relative_error =
                (result.thrust_magnitude_n - newtonian_thrust).abs() / newtonian_thrust;
            let power_relative_error =
                (result.kinetic_power_w - newtonian_power).abs() / newtonian_power;

            assert!(
                thrust_relative_error < 1.0e-8,
                "beta={beta}, error={thrust_relative_error}"
            );
            assert!(
                power_relative_error < 1.0e-7,
                "beta={beta}, error={power_relative_error}"
            );
        }
    }

    #[test]
    fn inverse_lorentz_boost_recovers_original_four_momentum() {
        let particle = FourMomentum::from_particle(
            2.3,
            [
                0.08 * SPEED_OF_LIGHT_M_S,
                -0.03 * SPEED_OF_LIGHT_M_S,
                0.02 * SPEED_OF_LIGHT_M_S,
            ],
        )
        .unwrap();
        let boost = [
            0.17 * SPEED_OF_LIGHT_M_S,
            -0.06 * SPEED_OF_LIGHT_M_S,
            0.04 * SPEED_OF_LIGHT_M_S,
        ];

        let transformed = particle.boost_to_moving_frame(boost).unwrap();
        let recovered = transformed
            .boost_to_moving_frame(boost.map(|component| -component))
            .unwrap();

        close(recovered.energy_j(), particle.energy_j(), 1e-6);
        for (actual, expected) in recovered
            .momentum_kg_m_s()
            .into_iter()
            .zip(particle.momentum_kg_m_s())
        {
            close(actual, expected, 1e-7);
        }
    }

    #[test]
    fn four_momentum_boost_composes_for_collinear_frames() {
        let mass = 1.7;
        let particle = FourMomentum::from_particle(mass, [0.0, 0.0, 0.0]).unwrap();
        let first_beta: f64 = 0.12;
        let second_beta: f64 = 0.08;
        let first = particle
            .boost_to_moving_frame([first_beta * SPEED_OF_LIGHT_M_S, 0.0, 0.0])
            .unwrap();
        let sequential = first
            .boost_to_moving_frame([second_beta * SPEED_OF_LIGHT_M_S, 0.0, 0.0])
            .unwrap();

        let combined_beta = (first_beta + second_beta) / (1.0 + first_beta * second_beta);
        let direct = particle
            .boost_to_moving_frame([combined_beta * SPEED_OF_LIGHT_M_S, 0.0, 0.0])
            .unwrap();

        close(sequential.energy_j(), direct.energy_j(), 1e-6);
        close(
            sequential.momentum_kg_m_s()[0],
            direct.momentum_kg_m_s()[0],
            1e-7,
        );
        close(sequential.invariant_rest_mass_kg().unwrap(), mass, 1e-11);
    }

    #[test]
    fn four_momentum_invariant_survives_general_lorentz_boost() {
        let mass = 3.2;
        let particle = FourMomentum::from_particle(
            mass,
            [
                0.12 * SPEED_OF_LIGHT_M_S,
                -0.04 * SPEED_OF_LIGHT_M_S,
                0.03 * SPEED_OF_LIGHT_M_S,
            ],
        )
        .unwrap();
        let boost = [
            0.21 * SPEED_OF_LIGHT_M_S,
            -0.07 * SPEED_OF_LIGHT_M_S,
            0.02 * SPEED_OF_LIGHT_M_S,
        ];
        let transformed = particle.boost_to_moving_frame(boost).unwrap();

        close(particle.invariant_rest_mass_kg().unwrap(), mass, 1e-12);
        close(transformed.invariant_rest_mass_kg().unwrap(), mass, 1e-11);
    }

    #[test]
    fn boost_from_rest_has_expected_energy_and_momentum_sign() {
        let mass = 1.0;
        let beta = 0.3;
        let particle = FourMomentum::from_particle(mass, [0.0, 0.0, 0.0]).unwrap();
        let transformed = particle
            .boost_to_moving_frame([beta * SPEED_OF_LIGHT_M_S, 0.0, 0.0])
            .unwrap();
        let gamma = 1.0 / (1.0 - beta * beta).sqrt();

        close(
            transformed.energy_j(),
            gamma * mass * SPEED_OF_LIGHT_M_S.powi(2),
            1e-6,
        );
        close(
            transformed.momentum_kg_m_s()[0],
            -gamma * mass * beta * SPEED_OF_LIGHT_M_S,
            1e-8,
        );
    }

    #[test]
    fn zero_exhaust_speed_has_rest_energy_but_no_kinetic_power_or_thrust() {
        let result = evaluate_relativistic_exhaust(RelativisticExhaustInput {
            rest_mass_flow_kg_s: 0.4,
            exhaust_velocity_m_s: [0.0, 0.0, 0.0],
        })
        .unwrap();

        assert_eq!(result.gamma, 1.0);
        assert_eq!(result.kinetic_power_w, 0.0);
        assert_eq!(result.thrust_magnitude_n, 0.0);
        assert!(result.rest_mass_energy_flux_w > 0.0);
    }

    #[test]
    fn zero_mass_flow_has_zero_energy_and_momentum_flux() {
        let result = evaluate_relativistic_exhaust(RelativisticExhaustInput {
            rest_mass_flow_kg_s: 0.0,
            exhaust_velocity_m_s: [0.1 * SPEED_OF_LIGHT_M_S, 0.0, 0.0],
        })
        .unwrap();

        assert_eq!(result.total_energy_flux_w, 0.0);
        assert_eq!(result.kinetic_power_w, 0.0);
        assert_eq!(result.thrust_magnitude_n, 0.0);
    }

    #[test]
    fn superluminal_speeds_and_non_finite_inputs_are_rejected() {
        assert!(FourMomentum::from_particle(1.0, [SPEED_OF_LIGHT_M_S, 0.0, 0.0]).is_err());
        assert!(
            evaluate_relativistic_exhaust(RelativisticExhaustInput {
                rest_mass_flow_kg_s: 1.0,
                exhaust_velocity_m_s: [1.001 * SPEED_OF_LIGHT_M_S, 0.0, 0.0],
            })
            .is_err()
        );
        assert!(
            evaluate_relativistic_exhaust(RelativisticExhaustInput {
                rest_mass_flow_kg_s: f64::NAN,
                exhaust_velocity_m_s: [1.0, 0.0, 0.0],
            })
            .is_err()
        );
        assert!(FourMomentum::from_particle(0.0, [0.0, 0.0, 0.0]).is_err());
    }

    #[test]
    fn superluminal_frame_boost_is_rejected() {
        let particle = FourMomentum::from_particle(1.0, [0.0, 0.0, 0.0]).unwrap();
        assert!(
            particle
                .boost_to_moving_frame([SPEED_OF_LIGHT_M_S, 0.0, 0.0])
                .is_err()
        );
    }

    #[test]
    fn vector_exhaust_direction_is_preserved() {
        let input = RelativisticExhaustInput {
            rest_mass_flow_kg_s: 1.0,
            exhaust_velocity_m_s: [0.0, 0.0, -0.1 * SPEED_OF_LIGHT_M_S],
        };
        let result = evaluate_relativistic_exhaust(input).unwrap();

        assert!(result.exhaust_momentum_flux_n[2] < 0.0);
        assert!(result.thrust_on_vehicle_n[2] > 0.0);
        assert_eq!(result.exhaust_momentum_flux_n[0], 0.0);
        assert_eq!(result.exhaust_momentum_flux_n[1], 0.0);
    }
}
