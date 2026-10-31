//! Deterministic synthetic challenge-family generator for SIM-REL-017.
//!
//! Families vary physical carryover, sensor drift, instrument mapping and
//! intervention damage while preserving common action/observation semantics.
//! Seeds select a declared rational noise rate; they do not silently alter
//! priors, vocabularies or the meaning of an action.

use super::{
    JointKernel, JointMass, JointOutcome, KernelRow, Rational,
    robust::ModelWorld,
};

const NOISE_RATES: [(u128, u128); 3] = [(1, 8), (1, 4), (1, 3)];

/// A complete generated finite model family plus its nominal member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengeFamily {
    pub seed: u64,
    pub observation_noise_rate: Rational,
    pub nominal_world_id: String,
    pub worlds: Vec<ModelWorld>,
}

/// Generate a deterministic confounding/shift challenge family.
///
/// It contains nominal, sensor-drift, physical-carryover, instrument-reversal,
/// shifted-hazard, and damaging-calibration worlds. Each world uses the same
/// action and observation vocabularies, state dimensions, priors and costs.
pub fn generate_stateful_confounding_family(seed: u64) -> Result<ChallengeFamily, String> {
    let selection = (seed % NOISE_RATES.len() as u64) as usize;
    let (numerator, denominator) = NOISE_RATES[selection];
    let noise = Rational::new(numerator, denominator)?;
    let modes = [
        Mode::Nominal,
        Mode::SensorDrift,
        Mode::PhysicalCarryover,
        Mode::InstrumentReversal,
        Mode::ShiftedHazard,
        Mode::DamagingCalibration,
    ];
    let worlds = modes
        .into_iter()
        .map(|mode| build_world(mode, noise))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ChallengeFamily {
        seed,
        observation_noise_rate: noise,
        nominal_world_id: "nominal".into(),
        worlds,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Nominal,
    SensorDrift,
    PhysicalCarryover,
    InstrumentReversal,
    ShiftedHazard,
    DamagingCalibration,
}

impl Mode {
    fn id(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::SensorDrift => "sensor-drift",
            Self::PhysicalCarryover => "physical-carryover",
            Self::InstrumentReversal => "instrument-reversal",
            Self::ShiftedHazard => "shifted-hazard",
            Self::DamagingCalibration => "damaging-calibration",
        }
    }
}

fn build_world(mode: Mode, noise: Rational) -> Result<ModelWorld, String> {
    let action_ids = vec![
        "measure-combined-signal".to_string(),
        "calibrate-sensor".to_string(),
    ];
    let observation_ids = vec!["zero".to_string(), "one".to_string()];
    let mut rows = Vec::new();

    for hypothesis in 0..2 {
        for physical_state in 0..2 {
            for sensor_state in 0..2 {
                for action in 0..2 {
                    let outcomes = if action == 0 {
                        measurement_outcomes(
                            mode,
                            noise,
                            physical_state,
                            sensor_state,
                        )?
                    } else {
                        calibration_outcomes(mode, physical_state)?
                    };
                    rows.push(KernelRow {
                        hypothesis,
                        physical_state,
                        sensor_state,
                        action,
                        outcomes,
                    });
                }
            }
        }
    }

    let kernel = JointKernel {
        hypothesis_ids: vec!["h0".into(), "h1".into()],
        physical_state_count: 2,
        sensor_state_count: 2,
        action_ids,
        observation_ids,
        action_costs: vec![2, 1],
        initial_belief: vec![
            JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 1,
                mass: Rational::new(1, 2)?,
            },
            JointMass {
                hypothesis: 1,
                physical_state: 1,
                sensor_state: 0,
                mass: Rational::new(1, 2)?,
            },
        ],
        rows,
    };
    kernel.validate()?;
    Ok(ModelWorld {
        id: mode.id().to_string(),
        kernel,
    })
}

fn measurement_outcomes(
    mode: Mode,
    noise: Rational,
    physical_state: usize,
    sensor_state: usize,
) -> Result<Vec<JointOutcome>, String> {
    let reversed = mode == Mode::InstrumentReversal;
    let signal = (physical_state ^ sensor_state) ^ if reversed { 1 } else { 0 };
    let next_physical = if mode == Mode::PhysicalCarryover {
        physical_state ^ 1
    } else {
        physical_state
    };
    let next_sensor = if mode == Mode::SensorDrift {
        sensor_state ^ 1
    } else {
        sensor_state
    };

    let hazard = if mode == Mode::ShiftedHazard && physical_state == 1 {
        Rational::new(1, 4)?
    } else {
        Rational::zero()
    };
    let safe_mass = Rational::one().checked_sub(hazard)?;
    let correct_probability = Rational::one()
        .checked_sub(noise)?
        .checked_mul(safe_mass)?;
    let error_probability = noise.checked_mul(safe_mass)?;

    let mut outcomes = Vec::with_capacity(3);
    if !hazard.is_zero() {
        outcomes.push(JointOutcome {
            probability: hazard,
            next_physical_state,
            next_sensor_state: next_sensor,
            observation: None,
        });
    }
    outcomes.push(JointOutcome {
        probability: correct_probability,
        next_physical_state,
        next_sensor_state: next_sensor,
        observation: Some(signal),
    });
    outcomes.push(JointOutcome {
        probability: error_probability,
        next_physical_state,
        next_sensor_state: next_sensor,
        observation: Some(signal ^ 1),
    });
    Ok(outcomes)
}

fn calibration_outcomes(
    mode: Mode,
    physical_state: usize,
) -> Result<Vec<JointOutcome>, String> {
    let next_physical = if mode == Mode::DamagingCalibration {
        physical_state ^ 1
    } else {
        physical_state
    };
    Ok(vec![JointOutcome {
        probability: Rational::one(),
        next_physical_state: next_physical,
        next_sensor_state: 0,
        observation: Some(0),
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        benchmark::fingerprint_model_family,
        robust_optimizer::{optimize_robust_worlds, RobustSolverConfig},
    };

    #[test]
    fn generated_family_is_reproducible_and_covers_declared_modes() {
        let a = generate_stateful_confounding_family(17).unwrap();
        let b = generate_stateful_confounding_family(17).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.nominal_world_id, "nominal");
        assert_eq!(a.worlds.len(), 6);
        let ids: Vec<&str> = a.worlds.iter().map(|world| world.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "nominal",
                "sensor-drift",
                "physical-carryover",
                "instrument-reversal",
                "shifted-hazard",
                "damaging-calibration",
            ]
        );
        for world in &a.worlds {
            world.kernel.validate().unwrap();
        }
    }

    #[test]
    fn generated_noise_rate_is_seeded_and_visible_in_the_family_fingerprint() {
        let a = generate_stateful_confounding_family(0).unwrap();
        let b = generate_stateful_confounding_family(1).unwrap();
        let c = generate_stateful_confounding_family(0).unwrap();
        assert_eq!(a.observation_noise_rate, Rational::new(1, 8).unwrap());
        assert_eq!(b.observation_noise_rate, Rational::new(1, 4).unwrap());
        assert_ne!(
            fingerprint_model_family(&a.worlds).unwrap(),
            fingerprint_model_family(&b.worlds).unwrap()
        );
        assert_eq!(
            fingerprint_model_family(&a.worlds).unwrap(),
            fingerprint_model_family(&c.worlds).unwrap()
        );
    }

    #[test]
    fn generated_family_exercises_robust_optimizer_and_preserves_abstention() {
        let family = generate_stateful_confounding_family(31).unwrap();
        let result = optimize_robust_worlds(
            &family.worlds,
            &family.nominal_world_id,
            &RobustSolverConfig {
                horizon: 2,
                max_memo_states: 50_000,
                max_generated_combinations: 500_000,
                max_frontier_points: 10_000,
            },
        )
        .unwrap();
        let selected = result
            .minimum_worst_case_cost_under(
                Rational::zero(),
                Rational::zero(),
                Rational::one(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            selected.worst_case_false_identification_probability(),
            Rational::zero()
        );
        assert_eq!(
            selected.worst_case_abstention_probability(),
            Rational::one()
        );
    }

    #[test]
    fn invalid_seed_does_not_create_invalid_probability_mass() {
        for seed in [0, 1, 2, 3, u64::MAX] {
            let family = generate_stateful_confounding_family(seed).unwrap();
            for world in family.worlds {
                world.kernel.validate().unwrap();
            }
        }
    }
}
