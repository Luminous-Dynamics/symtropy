//! Deterministic generator for exact, adversarial scientific experiment benchmarks.
//!
//! Complete finite joint kernels are generated from a versioned seed and
//! validated before return. This is a benchmark factory, not a proof that its
//! challenge families cover real laboratory failure modes.

use super::{
    JointKernel, JointMass, JointOutcome, KernelRow, Rational,
    robust::ModelWorld,
};

pub const GENERATOR_VERSION: &str = "sim-rel-017-v1";
const MAX_INSTANCES_PER_FAMILY: usize = 256;
const INSTANCE_SEED_STRIDE: u64 = 0x9E37_79B9_7F4A_7C15;

/// Deliberately small challenge families with known construction semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ChallengeFamily {
    SensorPhysicalConfounding,
    PersistentSensorDrift,
    PhysicalCarryover,
    CalibrationDamagesSpecimen,
    CorrelatedHazard,
}

impl ChallengeFamily {
    pub const ALL: [Self; 5] = [
        Self::SensorPhysicalConfounding,
        Self::PersistentSensorDrift,
        Self::PhysicalCarryover,
        Self::CalibrationDamagesSpecimen,
        Self::CorrelatedHazard,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SensorPhysicalConfounding => "sensor-physical-confounding",
            Self::PersistentSensorDrift => "persistent-sensor-drift",
            Self::PhysicalCarryover => "physical-carryover",
            Self::CalibrationDamagesSpecimen => "calibration-damages-specimen",
            Self::CorrelatedHazard => "correlated-hazard",
        }
    }
}

/// Reproducible benchmark generation settings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BenchmarkSuiteConfig {
    pub seed: u64,
    pub instances_per_family: usize,
}

impl Default for BenchmarkSuiteConfig {
    fn default() -> Self {
        Self {
            seed: 0x5349_4D52_454C_3031,
            instances_per_family: 2,
        }
    }
}

/// One complete generated challenge instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkInstance {
    pub id: String,
    pub seed: u64,
    pub generator_version: String,
    pub family: ChallengeFamily,
    pub nominal_world_id: String,
    pub fingerprint_hex: String,
    pub worlds: Vec<ModelWorld>,
}

/// Generate a deterministic suite spanning every registered challenge family.
pub fn generate_suite(
    config: BenchmarkSuiteConfig,
) -> Result<Vec<BenchmarkInstance>, String> {
    if config.instances_per_family == 0
        || config.instances_per_family > MAX_INSTANCES_PER_FAMILY
    {
        return Err(format!(
            "instances_per_family must be in 1..={MAX_INSTANCES_PER_FAMILY}"
        ));
    }

    let total = ChallengeFamily::ALL
        .len()
        .checked_mul(config.instances_per_family)
        .ok_or_else(|| "benchmark instance count overflow".to_string())?;
    let mut instances = Vec::with_capacity(total);
    for (family_index, family) in ChallengeFamily::ALL.iter().copied().enumerate() {
        for instance_index in 0..config.instances_per_family {
            let seed = config.seed
                ^ (family_index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
                ^ (instance_index as u64).wrapping_mul(INSTANCE_SEED_STRIDE);
            instances.push(generate_instance(seed, family)?);
        }
    }
    Ok(instances)
}

fn generate_instance(
    seed: u64,
    family: ChallengeFamily,
) -> Result<BenchmarkInstance, String> {
    let mut rng = DeterministicRng::new(seed);
    let action_costs = vec![
        1 + rng.bounded(5) as u64,
        1 + rng.bounded(5) as u64,
        1 + rng.bounded(5) as u64,
    ];
    // The alternative world must always exercise its family-specific shift;
    // randomness changes the instance parameters, not whether the control exists.
    let alternative_sensor_toggle = true;
    let alternative_inverts_instrument = true;
    let hazard_mass = match rng.bounded(2) {
        0 => Rational::new(1, 4)?,
        _ => Rational::new(1, 3)?,
    };

    let worlds = vec![
        ModelWorld {
            id: "nominal".into(),
            kernel: build_kernel(
                family, false, false, false, hazard_mass, action_costs.clone(),
            ),
        },
        ModelWorld {
            id: "alternative".into(),
            kernel: build_kernel(
                family,
                true,
                alternative_sensor_toggle,
                alternative_inverts_instrument,
                hazard_mass,
                action_costs,
            ),
        },
    ];
    for world in &worlds {
        world.kernel.validate()?;
    }

    let mut instance = BenchmarkInstance {
        id: format!("{}-{:016x}", family.as_str(), seed),
        seed,
        generator_version: GENERATOR_VERSION.into(),
        family,
        nominal_world_id: "nominal".into(),
        fingerprint_hex: String::new(),
        worlds,
    };
    instance.fingerprint_hex = fingerprint_instance(&instance);
    Ok(instance)
}

/// Stable FNV-1a fingerprint of the complete benchmark manifest and kernels.
///
/// This is a reproducibility fingerprint, not a cryptographic commitment.
fn fingerprint_instance(instance: &BenchmarkInstance) -> String {
    let mut fingerprint = StableFingerprint::new();
    fingerprint.write_string(&instance.generator_version);
    fingerprint.write_u64(instance.seed);
    fingerprint.write_string(instance.family.as_str());
    fingerprint.write_string(&instance.nominal_world_id);
    for world in &instance.worlds {
        fingerprint.write_string(&world.id);
        let kernel = &world.kernel;
        fingerprint.write_string_slice(&kernel.hypothesis_ids);
        fingerprint.write_u64(kernel.physical_state_count as u64);
        fingerprint.write_u64(kernel.sensor_state_count as u64);
        fingerprint.write_string_slice(&kernel.action_ids);
        fingerprint.write_string_slice(&kernel.observation_ids);
        fingerprint.write_u64_slice(&kernel.action_costs);
        fingerprint.write_u64(kernel.initial_belief.len() as u64);
        for point in &kernel.initial_belief {
            fingerprint.write_u64(point.hypothesis as u64);
            fingerprint.write_u64(point.physical_state as u64);
            fingerprint.write_u64(point.sensor_state as u64);
            fingerprint.write_u128(point.mass.numerator());
            fingerprint.write_u128(point.mass.denominator());
        }
        fingerprint.write_u64(kernel.rows.len() as u64);
        for row in &kernel.rows {
            fingerprint.write_u64(row.hypothesis as u64);
            fingerprint.write_u64(row.physical_state as u64);
            fingerprint.write_u64(row.sensor_state as u64);
            fingerprint.write_u64(row.action as u64);
            fingerprint.write_u64(row.outcomes.len() as u64);
            for outcome in &row.outcomes {
                fingerprint.write_u128(outcome.probability.numerator());
                fingerprint.write_u128(outcome.probability.denominator());
                fingerprint.write_u64(outcome.next_physical_state as u64);
                fingerprint.write_u64(outcome.next_sensor_state as u64);
                match outcome.observation {
                    None => fingerprint.write_u64(u64::MAX),
                    Some(observation) => fingerprint.write_u64(observation as u64),
                }
            }
        }
    }
    format!("{:016x}", fingerprint.state)
}

#[derive(Clone, Copy, Debug)]
struct StableFingerprint {
    state: u64,
}

impl StableFingerprint {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    fn new() -> Self {
        Self {
            state: Self::OFFSET_BASIS,
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_u128(&mut self, value: u128) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_string(&mut self, value: &str) {
        self.write_u64(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    fn write_string_slice(&mut self, values: &[String]) {
        self.write_u64(values.len() as u64);
        for value in values {
            self.write_string(value);
        }
    }

    fn write_u64_slice(&mut self, values: &[u64]) {
        self.write_u64(values.len() as u64);
        for value in values {
            self.write_u64(*value);
        }
    }
}

fn build_kernel(
    family: ChallengeFamily,
    alternative_world: bool,
    alternative_sensor_toggle: bool,
    alternative_inverts_instrument: bool,
    nominal_hazard_mass: Rational,
    action_costs: Vec<u64>,
) -> JointKernel {
    let initial_belief = match family {
        ChallengeFamily::SensorPhysicalConfounding => vec![
            JointMass {
                hypothesis: 0, physical_state: 0, sensor_state: 1, mass: half(),
            },
            JointMass {
                hypothesis: 1, physical_state: 1, sensor_state: 0, mass: half(),
            },
        ],
        _ => vec![
            JointMass {
                hypothesis: 0, physical_state: 0, sensor_state: 0, mass: half(),
            },
            JointMass {
                hypothesis: 1, physical_state: 1, sensor_state: 0, mass: half(),
            },
        ],
    };

    let mut rows = Vec::with_capacity(24);
    for hypothesis in 0..2 {
        for physical_state in 0..2 {
            for sensor_state in 0..2 {
                for action in 0..3 {
                    let (next_physical, next_sensor, observation) = transition_for(
                        family,
                        alternative_world,
                        alternative_sensor_toggle,
                        alternative_inverts_instrument,
                        physical_state,
                        sensor_state,
                        action,
                    );
                    let hazard_mass = if family == ChallengeFamily::CorrelatedHazard
                        && alternative_world
                        && action == 0
                        && physical_state == 1
                    {
                        Rational::new(1, 2).expect("positive constant denominator")
                    } else {
                        nominal_hazard_mass
                    };
                    let outcomes = if family == ChallengeFamily::CorrelatedHazard
                        && action == 0
                        && physical_state == 1
                    {
                        let safe_mass = Rational::one().checked_sub(hazard_mass)
                            .expect("hazard probability is in [0, 1]");
                        vec![
                            JointOutcome {
                                probability: hazard_mass,
                                next_physical_state: next_physical,
                                next_sensor_state: next_sensor,
                                observation: None,
                            },
                            JointOutcome {
                                probability: safe_mass,
                                next_physical_state: next_physical,
                                next_sensor_state: next_sensor,
                                observation: Some(observation),
                            },
                        ]
                    } else {
                        vec![JointOutcome {
                            probability: Rational::one(),
                            next_physical_state: next_physical,
                            next_sensor_state: next_sensor,
                            observation: Some(observation),
                        }]
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

    JointKernel {
        hypothesis_ids: vec!["mechanism-a".into(), "mechanism-b".into()],
        physical_state_count: 2,
        sensor_state_count: 2,
        action_ids: vec![
            "measure".into(),
            "calibrate-sensor".into(),
            "reset-physical-state".into(),
        ],
        observation_ids: vec!["zero".into(), "one".into()],
        action_costs,
        initial_belief,
        rows,
    }
}

fn transition_for(
    family: ChallengeFamily,
    alternative_world: bool,
    alternative_sensor_toggle: bool,
    alternative_inverts_instrument: bool,
    physical_state: usize,
    sensor_state: usize,
    action: usize,
) -> (usize, usize, usize) {
    match action {
        0 => {
            let mut observation = physical_state ^ sensor_state;
            let mut next_physical = physical_state;
            let mut next_sensor = sensor_state;
            match family {
                ChallengeFamily::SensorPhysicalConfounding => {
                    if alternative_world && alternative_inverts_instrument {
                        observation ^= 1;
                    }
                }
                ChallengeFamily::PersistentSensorDrift => {
                    if alternative_world && alternative_sensor_toggle {
                        next_sensor ^= 1;
                    }
                }
                ChallengeFamily::PhysicalCarryover => {
                    if alternative_world {
                        next_physical ^= 1;
                    }
                }
                ChallengeFamily::CalibrationDamagesSpecimen
                | ChallengeFamily::CorrelatedHazard => {}
            }
            (next_physical, next_sensor, observation)
        }
        1 => {
            let mut next_physical = physical_state;
            if family == ChallengeFamily::CalibrationDamagesSpecimen && !alternative_world {
                next_physical ^= 1;
            }
            (next_physical, 0, 0)
        }
        2 => {
            // Physical reset is distinct from sensor calibration: it does not
            // silently erase a persistent sensor bias.
            (0, sensor_state, 0)
        }
        _ => unreachable!("the generator only emits its three declared actions"),
    }
}

fn half() -> Rational {
    Rational::new(1, 2).expect("positive constant denominator")
}

/// Small deterministic xorshift generator, stable across platforms and runs.
#[derive(Clone, Copy, Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xA076_1D64_78BD_642F
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn bounded(&mut self, upper_exclusive: u64) -> u64 {
        self.next_u64() % upper_exclusive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        baseline::rank_by_one_step_eig,
        robust::evaluate_policy_across_worlds,
        PolicyPlan,
    };
    use std::collections::BTreeSet;

    #[test]
    fn same_seed_and_generator_version_reproduce_identical_suite() {
        let config = BenchmarkSuiteConfig {
            seed: 0x0123_4567_89AB_CDEF,
            instances_per_family: 3,
        };
        let first = generate_suite(config).unwrap();
        let second = generate_suite(config).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), ChallengeFamily::ALL.len() * 3);
        assert!(first.iter().all(|instance| {
            instance.generator_version == GENERATOR_VERSION
                && instance.worlds.len() == 2
                && instance.worlds.iter().all(|world| world.kernel.validate().is_ok())
        }));
    }

    #[test]
    fn manifest_fingerprint_is_stable_and_sensitive_to_kernel_changes() {
        let first = generate_instance(
            0xF1_6E_2,
            ChallengeFamily::PersistentSensorDrift,
        ).unwrap();
        let same = generate_instance(
            0xF1_6E_2,
            ChallengeFamily::PersistentSensorDrift,
        ).unwrap();
        assert_eq!(first.fingerprint_hex, same.fingerprint_hex);
        assert_eq!(first.fingerprint_hex.len(), 16);

        let mut changed = first.clone();
        changed.worlds[1].kernel.action_costs[0] += 1;
        assert_ne!(
            first.fingerprint_hex,
            fingerprint_instance(&changed),
            "manifest fingerprint must cover costs and complete kernel state"
        );
    }

    #[test]
    fn every_family_generates_complete_normalized_joint_kernels() {
        let suite = generate_suite(BenchmarkSuiteConfig::default()).unwrap();
        assert_eq!(
            suite.iter().map(|case| case.family).collect::<BTreeSet<_>>().len(),
            ChallengeFamily::ALL.len()
        );
        for instance in suite {
            for world in &instance.worlds {
                world.kernel.validate().unwrap();
                assert_eq!(world.kernel.action_costs.len(), 3);
                assert_eq!(world.kernel.rows.len(), 24);
            }
            let abstention = evaluate_policy_across_worlds(
                &instance.worlds,
                &instance.nominal_world_id,
                &PolicyPlan::Abstain,
                0,
            ).unwrap();
            assert_eq!(
                abstention.worst_case_abstention_probability,
                Rational::one()
            );
        }
    }

    #[test]
    fn every_generated_alternative_world_changes_a_declared_failure_mode() {
        let suite = generate_suite(BenchmarkSuiteConfig {
            seed: 0xBAD5_EED,
            instances_per_family: 8,
        }).unwrap();
        for instance in suite {
            assert_ne!(
                instance.worlds[0].kernel,
                instance.worlds[1].kernel,
                "alternative world must differ for family {} seed {}",
                instance.family.as_str(),
                instance.seed
            );
        }
    }

    #[test]
    fn persistent_sensor_confounds_have_zero_myopic_measurement_eig() {
        let instance = generate_instance(
            0xC0FF_EE,
            ChallengeFamily::SensorPhysicalConfounding,
        ).unwrap();
        let nominal = &instance.worlds[0].kernel;
        let scores = rank_by_one_step_eig(nominal, &nominal.initial_belief).unwrap();
        let measure = scores.iter().find(|score| score.action_id == "measure").unwrap();
        assert!(measure.expected_information_gain_nats.abs() < 1e-12);
        assert!(measure.information_gain_per_cost.abs() < 1e-12);
    }

    #[test]
    fn invalid_suite_sizes_fail_closed() {
        assert!(generate_suite(BenchmarkSuiteConfig {
            seed: 1,
            instances_per_family: 0,
        }).is_err());
        assert!(generate_suite(BenchmarkSuiteConfig {
            seed: 1,
            instances_per_family: MAX_INSTANCES_PER_FAMILY + 1,
        }).is_err());
    }
}
