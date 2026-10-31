//! Public, deterministic seed-to-scenario generator for scientific design benchmarks.
//!
//! The fingerprint here is a reproducibility key, not a cryptographic
//! integrity receipt. Final benchmark custody must keep holdout seeds private.

use super::{
    JointKernel, JointMass, JointOutcome, KernelRow, Rational, robust::ModelWorld,
};

/// Benchmark generator contract version. Increment when seed mapping or kernels change.
pub const GENERATOR_VERSION: &str = "sim-rel-017-v1";

/// Stateful stochastic challenge family.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BenchmarkFamily {
    /// Mechanism is directly observable up to independent measurement noise.
    Memoryless,
    /// Mechanism and specimen state cancel until a washout action.
    PhysicalCarryover,
    /// Mechanism and persistent sensor bias cancel until calibration.
    PersistentSensorBias,
    /// Mechanism, physical state, and sensor state jointly confound the first read.
    JointConfounding,
}

impl BenchmarkFamily {
    const fn tag(self) -> u8 {
        match self {
            Self::Memoryless => 0,
            Self::PhysicalCarryover => 1,
            Self::PersistentSensorBias => 2,
            Self::JointConfounding => 3,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Memoryless => "memoryless",
            Self::PhysicalCarryover => "physical-carryover",
            Self::PersistentSensorBias => "persistent-sensor-bias",
            Self::JointConfounding => "joint-confounding",
        }
    }
}

/// Expected structural property of a generated task; not an outcome label for the solver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BenchmarkControl {
    /// One measurement is informative.
    OneRead,
    /// A washout must precede the identifying measurement.
    WashoutThenRead,
    /// Sensor calibration must precede the identifying measurement.
    CalibrateThenRead,
    /// Both distinct interventions must precede the identifying measurement.
    CalibrateWashoutThenRead,
}

/// Reproducible development task with its full exact kernel and evaluation contract.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeededBenchmarkCase {
    pub seed: u64,
    pub generator_version: &'static str,
    pub family: BenchmarkFamily,
    pub case_id: String,
    /// Non-cryptographic reproducibility fingerprint; never use as an authenticity proof.
    pub replay_fingerprint: u64,
    pub measurement_accuracy: Rational,
    pub horizon: usize,
    pub max_false_identification: Rational,
    pub max_abstention: Rational,
    pub expected_control: BenchmarkControl,
    pub world: ModelWorld,
}

/// Generate one exact finite-state challenge from a public deterministic seed.
///
/// Seeds are suitable for development and reproducibility. They are not secret
/// test seeds and do not make a public benchmark resistant to memorization.
pub fn generate_seeded_case(
    seed: u64,
    family: BenchmarkFamily,
) -> Result<SeededBenchmarkCase, String> {
    let measurement_accuracy = if seed & 1 == 0 {
        Rational::new(3, 4)?
    } else {
        Rational::new(7, 8)?
    };
    let max_false_identification = Rational::one().checked_sub(measurement_accuracy)?;
    let (initial_physical, initial_sensor, horizon, expected_control) = match family {
        BenchmarkFamily::Memoryless => (0, 0, 1, BenchmarkControl::OneRead),
        BenchmarkFamily::PhysicalCarryover => (0, 0, 2, BenchmarkControl::WashoutThenRead),
        BenchmarkFamily::PersistentSensorBias => (0, 0, 2, BenchmarkControl::CalibrateThenRead),
        BenchmarkFamily::JointConfounding => {
            (0, 1, 3, BenchmarkControl::CalibrateWashoutThenRead)
        }
    };

    let world_id = format!("{}-{seed:016x}", family.name());
    let kernel = build_kernel(
        seed,
        family,
        measurement_accuracy,
        initial_physical,
        initial_sensor,
    )?;
    kernel.validate()?;

    let case_id = format!("{GENERATOR_VERSION}-{}-{seed:016x}", family.name());
    let replay_fingerprint = fingerprint(seed, family, measurement_accuracy);

    Ok(SeededBenchmarkCase {
        seed,
        generator_version: GENERATOR_VERSION,
        family,
        case_id,
        replay_fingerprint,
        measurement_accuracy,
        horizon,
        max_false_identification: max_false_identification,
        max_abstention: Rational::zero(),
        expected_control,
        world: ModelWorld {
            id: world_id,
            kernel,
        },
    })
}

fn build_kernel(
    _seed: u64,
    family: BenchmarkFamily,
    measurement_accuracy: Rational,
    initial_physical: usize,
    initial_sensor: usize,
) -> Result<JointKernel, String> {
    let hypothesis_ids = vec!["mechanism-a".to_string(), "mechanism-b".to_string()];
    let action_ids = vec![
        "measure".to_string(),
        "calibrate-sensor".to_string(),
        "washout-specimen".to_string(),
    ];
    let observation_ids = vec!["low".to_string(), "high".to_string()];

    let mut initial_belief = Vec::with_capacity(2);
    for hypothesis in 0..2 {
        let physical_state = match family {
            BenchmarkFamily::Memoryless | BenchmarkFamily::PersistentSensorBias => {
                initial_physical
            }
            BenchmarkFamily::PhysicalCarryover | BenchmarkFamily::JointConfounding => hypothesis,
        };
        let sensor_state = match family {
            BenchmarkFamily::Memoryless | BenchmarkFamily::PhysicalCarryover => initial_sensor,
            BenchmarkFamily::PersistentSensorBias => hypothesis,
            BenchmarkFamily::JointConfounding => initial_sensor,
        };
        initial_belief.push(JointMass {
            hypothesis,
            physical_state,
            sensor_state,
            mass: Rational::new(1, 2)?,
        });
    }

    let mut rows = Vec::with_capacity(2 * 2 * 2 * 3);
    for hypothesis in 0..2 {
        for physical_state in 0..2 {
            for sensor_state in 0..2 {
                for action in 0..3 {
                    let outcomes = match action {
                        0 => {
                            let latent_signal =
                                hypothesis ^ physical_state ^ sensor_state;
                            let next_physical = physical_state;
                            let next_sensor = sensor_state;
                            let incorrect_probability =
                                Rational::one().checked_sub(measurement_accuracy)?;
                            vec![
                                JointOutcome {
                                    probability: measurement_accuracy,
                                    next_physical_state: next_physical,
                                    next_sensor_state: next_sensor,
                                    observation: Some(latent_signal),
                                },
                                JointOutcome {
                                    probability: incorrect_probability,
                                    next_physical_state: next_physical,
                                    next_sensor_state: next_sensor,
                                    observation: Some(1 - latent_signal),
                                },
                            ]
                        }
                        1 => {
                            // Calibration resets only the sensor. It does not wash out
                            // specimen state; this distinction creates the joint case.
                            vec![JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: physical_state,
                                next_sensor_state: 0,
                                observation: Some(0),
                            }]
                        }
                        2 => {
                            // Washout resets only the specimen. It preserves sensor bias.
                            vec![JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: 0,
                                next_sensor_state: sensor_state,
                                observation: Some(0),
                            }]
                        }
                        _ => return Err("internal benchmark action index is invalid".into()),
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

    Ok(JointKernel {
        hypothesis_ids,
        physical_state_count: 2,
        sensor_state_count: 2,
        action_ids,
        observation_ids,
        action_costs: vec![2, 1, 1],
        initial_belief,
        rows,
    })
}

fn fingerprint(seed: u64, family: BenchmarkFamily, accuracy: Rational) -> u64 {
    // FNV-1a over versioned scenario inputs. This is deliberately not a
    // cryptographic hash and is not a substitute for a content-addressed receipt.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in GENERATOR_VERSION
        .bytes()
        .chain(seed.to_le_bytes())
        .chain([family.tag()])
        .chain(accuracy.numerator().to_le_bytes())
        .chain(accuracy.denominator().to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{baseline::rank_by_one_step_eig, optimize_joint_kernel, SolverConfig};

    fn select_case(seed: u64, family: BenchmarkFamily) -> SeededBenchmarkCase {
        generate_seeded_case(seed, family).unwrap()
    }

    fn selected_policy(case: &SeededBenchmarkCase) -> crate::PolicyPlan {
        let result = optimize_joint_kernel(
            &case.world.kernel,
            &SolverConfig {
                horizon: case.horizon,
                ..SolverConfig::default()
            },
        )
        .unwrap();
        result
            .minimum_cost_under(
                Rational::zero(),
                case.max_false_identification,
                case.max_abstention,
            )
            .unwrap()
            .unwrap()
            .plan
            .clone()
    }

    fn action_at<'a>(
        policy: &'a crate::PolicyPlan,
        observation: usize,
    ) -> Option<(usize, &'a crate::PolicyPlan)> {
        match policy {
            crate::PolicyPlan::Action { action, branches } => branches
                .iter()
                .find(|branch| branch.observation == observation)
                .map(|branch| (*action, branch.plan.as_ref())),
            _ => None,
        }
    }

    #[test]
    fn seed_and_generator_version_replay_exactly() {
        let first = select_case(42, BenchmarkFamily::JointConfounding);
        let second = select_case(42, BenchmarkFamily::JointConfounding);
        assert_eq!(first, second);
        assert_eq!(first.generator_version, GENERATOR_VERSION);
        assert_eq!(first.case_id, second.case_id);
        assert_eq!(first.replay_fingerprint, second.replay_fingerprint);
        first.world.kernel.validate().unwrap();
    }

    #[test]
    fn different_seeds_change_a_declared_noise_parameter() {
        let first = select_case(42, BenchmarkFamily::JointConfounding);
        let second = select_case(43, BenchmarkFamily::JointConfounding);
        assert_ne!(first.measurement_accuracy, second.measurement_accuracy);
        assert_ne!(first.replay_fingerprint, second.replay_fingerprint);
    }

    #[test]
    fn memoryless_case_is_solved_by_one_noisy_measurement() {
        let case = select_case(42, BenchmarkFamily::Memoryless);
        let policy = selected_policy(&case);
        match policy {
            crate::PolicyPlan::Action { action, .. } => {
                assert_eq!(case.world.kernel.action_ids[action], "measure");
            }
            _ => panic!("expected a single informative measurement"),
        }
    }

    #[test]
    fn physical_carryover_requires_washout_before_measurement() {
        let case = select_case(42, BenchmarkFamily::PhysicalCarryover);
        let policy = selected_policy(&case);
        let (first_action, after_first) = match &policy {
            crate::PolicyPlan::Action { action, branches } => {
                let branch = branches.iter().find(|branch| branch.observation == 0).unwrap();
                (*action, branch.plan.as_ref())
            }
            _ => panic!("expected washout and measurement"),
        };
        let (second_action, _) = action_at(after_first, 0).unwrap();
        assert_eq!(case.world.kernel.action_ids[first_action], "washout-specimen");
        assert_eq!(case.world.kernel.action_ids[second_action], "measure");
    }

    #[test]
    fn persistent_sensor_bias_requires_calibration_before_measurement() {
        let case = select_case(42, BenchmarkFamily::PersistentSensorBias);
        let policy = selected_policy(&case);
        let (first_action, after_first) = match &policy {
            crate::PolicyPlan::Action { action, branches } => {
                let branch = branches.iter().find(|branch| branch.observation == 0).unwrap();
                (*action, branch.plan.as_ref())
            }
            _ => panic!("expected calibration and measurement"),
        };
        let (second_action, _) = action_at(after_first, 0).unwrap();
        assert_eq!(case.world.kernel.action_ids[first_action], "calibrate-sensor");
        assert_eq!(case.world.kernel.action_ids[second_action], "measure");
    }

    #[test]
    fn joint_confounding_exposes_the_limit_of_myopic_eig() {
        let case = select_case(42, BenchmarkFamily::JointConfounding);
        let eig = rank_by_one_step_eig(
            &case.world.kernel,
            &case.world.kernel.initial_belief,
        )
        .unwrap();
        assert!(eig
            .iter()
            .all(|score| score.expected_information_gain_nats.abs() < 1e-12));

        let policy = selected_policy(&case);
        let (first_action, after_first) = match &policy {
            crate::PolicyPlan::Action { action, branches } => {
                let branch = branches.iter().find(|branch| branch.observation == 0).unwrap();
                (*action, branch.plan.as_ref())
            }
            _ => panic!("expected two enabling interventions then a measurement"),
        };
        let (second_action, after_second) = action_at(after_first, 0).unwrap();
        let (third_action, _) = action_at(after_second, 0).unwrap();
        let first_name = &case.world.kernel.action_ids[first_action];
        let second_name = &case.world.kernel.action_ids[second_action];
        let third_name = &case.world.kernel.action_ids[third_action];
        assert!(first_name == "calibrate-sensor" || first_name == "washout-specimen");
        assert!(second_name == "calibrate-sensor" || second_name == "washout-specimen");
        assert_ne!(first_name, second_name);
        assert_eq!(third_name, "measure");
    }
}
