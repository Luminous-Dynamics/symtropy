use std::env;
use std::process;

const SEEDS: u64 = 100;
const PARTS_PER_RUN: u64 = 500;
const AR_COEFFICIENT: f64 = 0.94;
const BIAS_LIMIT: f64 = 0.22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    NoMaintenance,
    FixedCycle,
    ConditionBased,
}

impl Policy {
    const ALL: [Self; 3] = [Self::NoMaintenance, Self::FixedCycle, Self::ConditionBased];

    fn key(self) -> &'static str {
        match self {
            Self::NoMaintenance => "no_maintenance",
            Self::FixedCycle => "fixed_cycle",
            Self::ConditionBased => "condition_based",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Scenario {
    id: &'static str,
    description: &'static str,
    innovation_half_width: f64,
    part_noise_half_width: f64,
    jump_probability: f64,
    jump_magnitude: f64,
    wear_per_part: f64,
    sensor_noise_half_width: f64,
    sensor_bias: f64,
    tolerance: f64,
    monitor_threshold: f64,
    wear_trigger: f64,
    fixed_interval: u64,
    fixed_downtime: f64,
    monitor_downtime: f64,
    monitor_overhead: f64,
    inspection_detection: f64,
    rework_success: f64,
    rework_time: f64,
}

const SCENARIOS: [Scenario; 4] = [
    Scenario {
        id: "nominal",
        description: "Reference drift, wear, part variation, inspection and repair assumptions.",
        innovation_half_width: 0.006,
        part_noise_half_width: 0.045,
        jump_probability: 0.020,
        jump_magnitude: 0.070,
        wear_per_part: 0.0018,
        sensor_noise_half_width: 0.008,
        sensor_bias: 0.0,
        tolerance: 0.100,
        monitor_threshold: 0.075,
        wear_trigger: 0.085,
        fixed_interval: 25,
        fixed_downtime: 2.0,
        monitor_downtime: 1.4,
        monitor_overhead: 0.030,
        inspection_detection: 0.94,
        rework_success: 0.72,
        rework_time: 1.4,
    },
    Scenario {
        id: "gradual_wear",
        description: "Lower process noise and fewer shifts; wear accumulation remains active.",
        innovation_half_width: 0.003,
        part_noise_half_width: 0.025,
        jump_probability: 0.008,
        jump_magnitude: 0.045,
        wear_per_part: 0.0018,
        sensor_noise_half_width: 0.006,
        sensor_bias: 0.0,
        tolerance: 0.110,
        monitor_threshold: 0.080,
        wear_trigger: 0.085,
        fixed_interval: 25,
        fixed_downtime: 2.0,
        monitor_downtime: 1.4,
        monitor_overhead: 0.030,
        inspection_detection: 0.94,
        rework_success: 0.72,
        rework_time: 1.4,
    },
    Scenario {
        id: "shock_stress",
        description: "More frequent persistent process shifts and larger variation.",
        innovation_half_width: 0.008,
        part_noise_half_width: 0.050,
        jump_probability: 0.040,
        jump_magnitude: 0.090,
        wear_per_part: 0.0023,
        sensor_noise_half_width: 0.009,
        sensor_bias: 0.0,
        tolerance: 0.100,
        monitor_threshold: 0.085,
        wear_trigger: 0.075,
        fixed_interval: 25,
        fixed_downtime: 2.0,
        monitor_downtime: 1.4,
        monitor_overhead: 0.030,
        inspection_detection: 0.94,
        rework_success: 0.72,
        rework_time: 1.4,
    },
    Scenario {
        id: "biased_sensor_negative_control",
        description: "A negative control: sensor bias masks some true process drift.",
        innovation_half_width: 0.006,
        part_noise_half_width: 0.045,
        jump_probability: 0.020,
        jump_magnitude: 0.070,
        wear_per_part: 0.0018,
        sensor_noise_half_width: 0.008,
        sensor_bias: -0.060,
        tolerance: 0.100,
        monitor_threshold: 0.075,
        wear_trigger: 0.085,
        fixed_interval: 25,
        fixed_downtime: 2.0,
        monitor_downtime: 1.4,
        monitor_overhead: 0.030,
        inspection_detection: 0.94,
        rework_success: 0.72,
        rework_time: 1.4,
    },
];

#[derive(Clone, Copy, Debug)]
struct Inputs {
    drift_innovation: f64,
    part_noise: f64,
    jump_draw: f64,
    jump_direction: f64,
    sensor_noise: f64,
    detection_draw: f64,
    rework_draw: f64,
}

/// Small pinned PRNG avoids third-party dependencies and makes replay independent
/// of platform RNG APIs. It is not a cryptographic RNG.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
        if state == 0 {
            state = 0xA5A5_A5A5_A5A5_A5A5;
        }
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    fn between(&mut self, low: f64, high: f64) -> f64 {
        low + (high - low) * self.unit()
    }
}

fn inputs_for(seed: u64, scenario: Scenario, parts: u64) -> Vec<Inputs> {
    let mut rng = Rng::new(seed);
    (0..parts)
        .map(|_| Inputs {
            drift_innovation: rng.between(
                -scenario.innovation_half_width,
                scenario.innovation_half_width,
            ),
            part_noise: rng.between(
                -scenario.part_noise_half_width,
                scenario.part_noise_half_width,
            ),
            jump_draw: rng.unit(),
            jump_direction: rng.between(-0.008, 0.008),
            sensor_noise: rng.between(
                -scenario.sensor_noise_half_width,
                scenario.sensor_noise_half_width,
            ),
            detection_draw: rng.unit(),
            rework_draw: rng.unit(),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
struct RunMetrics {
    parts: u64,
    conforming_units: u64,
    initial_defects: u64,
    escaped_defects: u64,
    rework_count: u64,
    maintenance_events: u64,
    maintenance_downtime: f64,
    total_time: f64,
}

impl RunMetrics {
    fn first_pass_yield(&self) -> f64 {
        (self.parts - self.initial_defects) as f64 / self.parts as f64
    }

    fn escape_rate(&self) -> f64 {
        self.escaped_defects as f64 / self.parts as f64
    }

    fn rework_rate(&self) -> f64 {
        self.rework_count as f64 / self.parts as f64
    }

    fn throughput(&self) -> f64 {
        self.conforming_units as f64 / self.total_time
    }
}

fn simulate(scenario: Scenario, policy: Policy, seed: u64, parts: u64) -> RunMetrics {
    // Each policy receives identical exogenous inputs for a given (scenario, seed).
    // This common-random-numbers design reduces variance in paired comparisons.
    let inputs = inputs_for(seed, scenario, parts);
    let mut process_bias = 0.0;
    let mut tool_wear = 0.0;
    let mut age_since_service = 0_u64;
    let mut metrics = RunMetrics {
        parts,
        conforming_units: 0,
        initial_defects: 0,
        escaped_defects: 0,
        rework_count: 0,
        maintenance_events: 0,
        maintenance_downtime: 0.0,
        total_time: 0.0,
    };

    for sample in inputs {
        if sample.jump_draw < scenario.jump_probability {
            process_bias += if sample.jump_direction >= 0.0 {
                scenario.jump_magnitude
            } else {
                -scenario.jump_magnitude
            };
        }
        process_bias = (AR_COEFFICIENT * process_bias + sample.drift_innovation)
            .clamp(-BIAS_LIMIT, BIAS_LIMIT);
        tool_wear += scenario.wear_per_part;
        age_since_service += 1;

        match policy {
            Policy::NoMaintenance => {}
            Policy::FixedCycle if age_since_service >= scenario.fixed_interval => {
                process_bias = 0.0;
                tool_wear = 0.0;
                age_since_service = 0;
                metrics.maintenance_events += 1;
                metrics.maintenance_downtime += scenario.fixed_downtime;
            }
            Policy::ConditionBased => {
                let observed_offset =
                    process_bias + tool_wear + scenario.sensor_bias + sample.sensor_noise;
                if observed_offset.abs() > scenario.monitor_threshold
                    || tool_wear > scenario.wear_trigger
                {
                    process_bias = 0.0;
                    tool_wear = 0.0;
                    age_since_service = 0;
                    metrics.maintenance_events += 1;
                    metrics.maintenance_downtime += scenario.monitor_downtime;
                }
            }
            Policy::FixedCycle => {}
        }

        metrics.total_time += 1.0
            + if policy == Policy::ConditionBased {
                scenario.monitor_overhead
            } else {
                0.0
            };

        let dimensional_error = process_bias + tool_wear + sample.part_noise;
        if dimensional_error.abs() > scenario.tolerance {
            metrics.initial_defects += 1;
            if sample.detection_draw < scenario.inspection_detection {
                metrics.rework_count += 1;
                metrics.total_time += scenario.rework_time;
                if sample.rework_draw < scenario.rework_success {
                    metrics.conforming_units += 1;
                } else {
                    metrics.escaped_defects += 1;
                }
            } else {
                metrics.escaped_defects += 1;
            }
        } else {
            metrics.conforming_units += 1;
        }
    }

    metrics.total_time += metrics.maintenance_downtime;
    metrics
}

#[derive(Clone, Copy, Debug)]
struct Summary {
    conforming_units: f64,
    first_pass_yield: f64,
    escape_rate: f64,
    rework_rate: f64,
    maintenance_events: f64,
    maintenance_downtime: f64,
    total_time: f64,
    throughput: f64,
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let values: Vec<f64> = values.collect();
    values.iter().sum::<f64>() / values.len() as f64
}

fn summarize(runs: &[RunMetrics]) -> Summary {
    Summary {
        conforming_units: mean(runs.iter().map(|r| r.conforming_units as f64)),
        first_pass_yield: mean(runs.iter().map(RunMetrics::first_pass_yield)),
        escape_rate: mean(runs.iter().map(RunMetrics::escape_rate)),
        rework_rate: mean(runs.iter().map(RunMetrics::rework_rate)),
        maintenance_events: mean(runs.iter().map(|r| r.maintenance_events as f64)),
        maintenance_downtime: mean(runs.iter().map(|r| r.maintenance_downtime)),
        total_time: mean(runs.iter().map(|r| r.total_time)),
        throughput: mean(runs.iter().map(RunMetrics::throughput)),
    }
}

fn paired_delta(
    treatment: &[RunMetrics],
    control: &[RunMetrics],
    metric: fn(&RunMetrics) -> f64,
) -> (f64, f64, f64) {
    let differences: Vec<f64> = treatment
        .iter()
        .zip(control.iter())
        .map(|(a, b)| metric(a) - metric(b))
        .collect();
    let avg = mean(differences.iter().copied());
    let variance = differences
        .iter()
        .map(|v| (v - avg) * (v - avg))
        .sum::<f64>()
        / (differences.len().saturating_sub(1).max(1) as f64);
    let standard_error = variance.sqrt() / (differences.len() as f64).sqrt();
    (
        avg,
        avg - 1.96 * standard_error,
        avg + 1.96 * standard_error,
    )
}

fn healthy_run(run: &RunMetrics) -> bool {
    run.conforming_units + run.escaped_defects == run.parts
        && run.initial_defects <= run.parts
        && run.rework_count <= run.initial_defects
        && run.escaped_defects <= run.parts
        && run.total_time.is_finite()
        && run.total_time > 0.0
        && run.maintenance_downtime.is_finite()
}

fn verify_benchmark() -> Result<(), String> {
    let mut checked_policy_pairs = 0usize;

    for scenario in SCENARIOS {
        let mut policy_runs = Vec::new();
        for policy in Policy::ALL {
            let runs: Vec<RunMetrics> = (1..=SEEDS)
                .map(|seed| simulate(scenario, policy, seed, PARTS_PER_RUN))
                .collect();

            if !runs.iter().all(healthy_run) {
                return Err(format!(
                    "mass-balance or finite-value invariant failed for {}/{}",
                    scenario.id,
                    policy.key()
                ));
            }
            policy_runs.push((policy, runs));
        }

        let fixed = &policy_runs[1].1;
        let condition = &policy_runs[2].1;
        let fixed_summary = summarize(fixed);
        let condition_summary = summarize(condition);

        if scenario.id == "biased_sensor_negative_control"
            && (fixed_summary.first_pass_yield <= condition_summary.first_pass_yield
                || fixed_summary.escape_rate >= condition_summary.escape_rate
                || fixed_summary.throughput <= condition_summary.throughput)
        {
            return Err("biased-sensor negative control no longer exposes monitor failure".into());
        }
        if scenario.id != "biased_sensor_negative_control"
            && (condition_summary.first_pass_yield <= fixed_summary.first_pass_yield
                || condition_summary.escape_rate >= fixed_summary.escape_rate
                || condition_summary.throughput <= fixed_summary.throughput)
        {
            return Err(format!(
                "condition-based policy did not beat fixed-cycle baseline on declared scenario {}",
                scenario.id
            ));
        }

        checked_policy_pairs += policy_runs.len();
    }

    // Fail if the same seed and policy cannot be replayed byte-for-byte as metrics.
    let repeat_a = simulate(SCENARIOS[0], Policy::ConditionBased, 17, PARTS_PER_RUN);
    let repeat_b = simulate(SCENARIOS[0], Policy::ConditionBased, 17, PARTS_PER_RUN);
    if repeat_a != repeat_b {
        return Err("deterministic replay failed".into());
    }
    if checked_policy_pairs != SCENARIOS.len() * Policy::ALL.len() {
        return Err("scenario/policy coverage is incomplete".into());
    }
    Ok(())
}

fn print_summary(summary: Summary) {
    print!(
        "{{\"conforming_units_per_run\":{:.6},\"first_pass_yield\":{:.6},\"escaped_defects_per_unit\":{:.6},\"rework_per_unit\":{:.6},\"maintenance_events_per_run\":{:.6},\"maintenance_downtime_per_run\":{:.6},\"total_simulated_time_per_run\":{:.6},\"conforming_units_per_time_unit\":{:.6}}}",
        summary.conforming_units,
        summary.first_pass_yield,
        summary.escape_rate,
        summary.rework_rate,
        summary.maintenance_events,
        summary.maintenance_downtime,
        summary.total_time,
        summary.throughput,
    );
}

fn print_delta(label: &str, delta: (f64, f64, f64)) {
    print!(
        "\"{}\":{{\"mean\":{:.6},\"descriptive_95pct_interval\":[{:.6},{:.6}]}}",
        label, delta.0, delta.1, delta.2
    );
}

fn report() {
    println!(
        "{{\"schema_version\":\"0.1.0\",\"model_id\":\"QDRIFT-SYNTH-001\",\"evidence_class\":\"synthetic_reference_simulation_not_calibrated\",\"replications_per_scenario\":{},\"parts_per_replication\":{},\"common_random_numbers\":true,\"interval_method\":\"paired normal approximation across deterministic Monte Carlo seeds; descriptive only\",\"time_unit\":\"arbitrary simulated process-time unit\",\"scenarios\":[",
        SEEDS, PARTS_PER_RUN
    );

    for (scenario_index, scenario) in SCENARIOS.iter().enumerate() {
        if scenario_index > 0 {
            println!(",");
        }
        let mut result_sets: Vec<(Policy, Vec<RunMetrics>)> = Vec::new();
        for policy in Policy::ALL {
            let runs: Vec<RunMetrics> = (1..=SEEDS)
                .map(|seed| simulate(*scenario, policy, seed, PARTS_PER_RUN))
                .collect();
            result_sets.push((policy, runs));
        }

        print!(
            "{{\"id\":\"{}\",\"description\":\"{}\",\"results\":{{",
            scenario.id, scenario.description
        );
        for (index, (policy, runs)) in result_sets.iter().enumerate() {
            if index > 0 {
                print!(",");
            }
            print!("\"{}\":", policy.key());
            print_summary(summarize(runs));
        }

        let fixed = &result_sets[1].1;
        let condition = &result_sets[2].1;
        print!("}},\"condition_based_minus_fixed_cycle\":{{");
        print_delta(
            "throughput",
            paired_delta(condition, fixed, RunMetrics::throughput),
        );
        print!(",");
        print_delta(
            "first_pass_yield",
            paired_delta(condition, fixed, RunMetrics::first_pass_yield),
        );
        print!(",");
        print_delta(
            "escape_rate",
            paired_delta(condition, fixed, RunMetrics::escape_rate),
        );
        print!("}}}}");
    }

    println!("]}}");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let check_only = args.iter().any(|arg| arg == "--check");
    if check_only {
        match verify_benchmark() {
            Ok(()) => println!(
                "{{\"check\":\"PASS\",\"scenarios\":{},\"policies_per_scenario\":{},\"seeds_per_policy\":{},\"parts_per_seed\":{},\"evidence_class\":\"synthetic_only\"}}",
                SCENARIOS.len(),
                Policy::ALL.len(),
                SEEDS,
                PARTS_PER_RUN
            ),
            Err(error) => {
                eprintln!("quality-drift-sim check failed: {error}");
                process::exit(2);
            }
        }
    } else {
        report();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_is_deterministic() {
        let first = simulate(SCENARIOS[0], Policy::FixedCycle, 1234, PARTS_PER_RUN);
        let second = simulate(SCENARIOS[0], Policy::FixedCycle, 1234, PARTS_PER_RUN);
        assert_eq!(first, second);
    }

    #[test]
    fn material_balance_holds_for_every_policy_and_scenario() {
        for scenario in SCENARIOS {
            for policy in Policy::ALL {
                for seed in 1..=10 {
                    let run = simulate(scenario, policy, seed, PARTS_PER_RUN);
                    assert!(healthy_run(&run), "{} / {}", scenario.id, policy.key());
                }
            }
        }
    }

    #[test]
    fn condition_monitoring_wins_only_inside_its_declared_validity_domain() {
        assert!(verify_benchmark().is_ok());
    }

    #[test]
    fn biased_sensor_fixture_is_a_real_negative_control() {
        let fixed: Vec<_> = (1..=SEEDS)
            .map(|seed| simulate(SCENARIOS[3], Policy::FixedCycle, seed, PARTS_PER_RUN))
            .collect();
        let condition: Vec<_> = (1..=SEEDS)
            .map(|seed| simulate(SCENARIOS[3], Policy::ConditionBased, seed, PARTS_PER_RUN))
            .collect();
        let fixed_summary = summarize(&fixed);
        let condition_summary = summarize(&condition);
        assert!(fixed_summary.first_pass_yield > condition_summary.first_pass_yield);
        assert!(fixed_summary.escape_rate < condition_summary.escape_rate);
        assert!(fixed_summary.throughput > condition_summary.throughput);
    }
}
