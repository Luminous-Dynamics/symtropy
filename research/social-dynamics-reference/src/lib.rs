//! Small, dependency-free research reference model.
//!
//! This crate is a synthetic benchmark scaffold, not a validated model of real
//! people. Keep rules explicit and simple until benchmark validity is shown.

/// Formal consensus baselines used for canonical dynamics checks.
pub mod canonical;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Treatment {
    NoCommitments,
    StaticTrust,
    Reciprocity,
    EvidenceBasedTrust,
}

impl Treatment {
    pub const ALL: [Self; 4] = [
        Self::NoCommitments,
        Self::StaticTrust,
        Self::Reciprocity,
        Self::EvidenceBasedTrust,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoCommitments => "no_commitments",
            Self::StaticTrust => "static_trust",
            Self::Reciprocity => "reciprocity",
            Self::EvidenceBasedTrust => "evidence_based_trust",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scenario {
    pub population: usize,
    pub ticks: usize,
    pub shock_start_tick: usize,
    pub shock_duration_ticks: usize,
    pub baseline_capacity: f64,
    pub shock_capacity: f64,
    pub observation_noise: f64,
    pub observation_cost: f64,
    pub attestation_tampering_rate: f64,
    pub repair_gain_per_contribution: f64,
    pub trust_learning_rate: f64,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            population: 128,
            ticks: 80,
            shock_start_tick: 20,
            shock_duration_ticks: 15,
            baseline_capacity: 1.0,
            shock_capacity: 0.45,
            observation_noise: 0.05,
            observation_cost: 0.15,
            attestation_tampering_rate: 0.02,
            repair_gain_per_contribution: 0.00125,
            trust_learning_rate: 0.18,
        }
    }
}

impl Scenario {
    pub fn validate(&self) -> Result<(), String> {
        if self.population < 5 {
            return Err("population must be at least 5 for the ring-lattice topology".into());
        }
        if self.ticks == 0 || self.shock_start_tick >= self.ticks {
            return Err("ticks must be positive and shock_start_tick must be inside the run".into());
        }
        if self.shock_duration_ticks > self.ticks - self.shock_start_tick {
            return Err("shock window must fit within the run".into());
        }
        for (name, value) in [
            ("baseline_capacity", self.baseline_capacity),
            ("shock_capacity", self.shock_capacity),
            ("observation_noise", self.observation_noise),
            ("observation_cost", self.observation_cost),
            ("attestation_tampering_rate", self.attestation_tampering_rate),
            ("repair_gain_per_contribution", self.repair_gain_per_contribution),
            ("trust_learning_rate", self.trust_learning_rate),
        ] {
            if !value.is_finite() {
                return Err(format!("{name} must be finite"));
            }
        }
        if self.baseline_capacity <= 0.0
            || self.shock_capacity < 0.0
            || self.shock_capacity >= self.baseline_capacity
        {
            return Err("capacity must satisfy 0 <= shock_capacity < baseline_capacity".into());
        }
        for (name, value) in [
            ("observation_noise", self.observation_noise),
            ("observation_cost", self.observation_cost),
            ("attestation_tampering_rate", self.attestation_tampering_rate),
            ("trust_learning_rate", self.trust_learning_rate),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(format!("{name} must be in [0, 1]"));
            }
        }
        if self.repair_gain_per_contribution < 0.0 {
            return Err("repair_gain_per_contribution must be non-negative".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Metrics {
    pub treatment: Treatment,
    /// Sum across all agent-ticks of max(need - service_capacity, 0).
    pub unmet_need_integral: f64,
    /// Maximum sum of unmet need in a single tick.
    pub peak_unmet_need: f64,
    /// Gini coefficient of cumulative unmet need across agents.
    pub unmet_need_inequality_gini: f64,
    pub contribution_count: usize,
    /// Tick offset from shock start; None means no in-window recovery.
    pub recovery_tick: Option<usize>,
    /// Count of neighbor-evidence checks multiplied by per-check cost.
    pub observation_overhead: f64,
    /// Repair progress when the declared shock window closes.
    pub repair_progress_at_shock_end: f64,
}

#[derive(Clone, Debug)]
struct Agent {
    need: f64,
    reserve: f64,
    willingness: f64,
    neighbors: Vec<usize>,
    trust: Vec<f64>,
}

fn gini_nonnegative(values: &[f64]) -> f64 {
    let total = values.iter().sum::<f64>();
    if values.is_empty() || total <= f64::EPSILON {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let weighted_sum = sorted
        .iter()
        .enumerate()
        .map(|(index, value)| (index as f64 + 1.0) * value)
        .sum::<f64>();
    let n = sorted.len() as f64;
    ((2.0 * weighted_sum) / (n * total) - (n + 1.0) / n).clamp(0.0, 1.0)
}

#[derive(Clone, Debug)]
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
    }
}

/// Run one synthetic scenario for one treatment and seed.
///
/// Treatment arms use fixed per-agent/per-edge random draws each tick so that
/// paired seeds share exogenous draws even when a treatment uses fewer signals.
pub fn simulate(
    scenario: &Scenario,
    seed: u64,
    treatment: Treatment,
) -> Result<Metrics, String> {
    scenario.validate()?;
    let n = scenario.population;
    let mut rng = SplitMix64::new(seed);

    let neighborhoods = canonical::ring_lattice(n, 4)?;
    let mut agents = Vec::with_capacity(n);
    for neighbors in neighborhoods {
        let trust = neighbors.iter().map(|_| 0.30 + 0.65 * rng.unit_f64()).collect();
        agents.push(Agent {
            need: 0.80 + 0.40 * rng.unit_f64(),
            reserve: 0.45 + 0.55 * rng.unit_f64(),
            willingness: 0.10 + 0.40 * rng.unit_f64(),
            neighbors,
            trust,
        });
    }

    let shock_end = scenario.shock_start_tick + scenario.shock_duration_ticks;
    let mut previous_actions = vec![false; n];
    let mut repair_progress: f64 = 0.0;
    let mut unmet_need_integral = 0.0;
    let mut unmet_by_agent = vec![0.0; n];
    let mut peak_unmet_need: f64 = 0.0;
    let mut contribution_count = 0_usize;
    let mut recovery_tick = None;
    let mut observation_overhead = 0.0;
    let mut repair_progress_at_shock_end = 0.0;

    for tick in 0..scenario.ticks {
        let in_shock_window =
            tick >= scenario.shock_start_tick && tick < shock_end;
        let capacity = if in_shock_window {
            scenario.shock_capacity
                + (scenario.baseline_capacity - scenario.shock_capacity) * repair_progress
        } else {
            scenario.baseline_capacity
        };

        let mut tick_unmet = 0.0;
        for (i, agent) in agents.iter_mut().enumerate() {
            let unmet = (agent.need - capacity).max(0.0);
            tick_unmet += unmet;
            unmet_by_agent[i] += unmet;
            // A small reserve buffer makes contribution feasibility stateful.
            let inflow = capacity * 0.30;
            let consumption = agent.need * 0.30;
            agent.reserve = (agent.reserve + inflow - consumption).clamp(0.0, 1.5);
        }
        unmet_need_integral += tick_unmet;
        peak_unmet_need = peak_unmet_need.max(tick_unmet);

        // Pre-draw all randomness independently of treatment. This supports
        // paired-seed comparisons without branch-dependent RNG stream drift.
        let action_draws: Vec<f64> = (0..n).map(|_| rng.unit_f64()).collect();
        let mut decision_noise_draws: Vec<Vec<f64>> = Vec::with_capacity(n);
        let mut decision_capture_draws: Vec<Vec<f64>> = Vec::with_capacity(n);
        let mut update_noise_draws: Vec<Vec<f64>> = Vec::with_capacity(n);
        let mut update_capture_draws: Vec<Vec<f64>> = Vec::with_capacity(n);
        for agent in &agents {
            decision_noise_draws.push(
                (0..agent.neighbors.len()).map(|_| rng.unit_f64()).collect(),
            );
            decision_capture_draws.push(
                (0..agent.neighbors.len()).map(|_| rng.unit_f64()).collect(),
            );
            update_noise_draws.push(
                (0..agent.neighbors.len()).map(|_| rng.unit_f64()).collect(),
            );
            update_capture_draws.push(
                (0..agent.neighbors.len()).map(|_| rng.unit_f64()).collect(),
            );
        }

        let mut actions = vec![false; n];
        if in_shock_window && capacity + f64::EPSILON < scenario.baseline_capacity {
            let urgency = (1.0 - capacity / scenario.baseline_capacity).clamp(0.0, 1.0);
            for i in 0..n {
                let agent = &agents[i];
                if agent.reserve < 0.15 {
                    continue;
                }
                let mean_trust =
                    agent.trust.iter().sum::<f64>() / agent.trust.len() as f64;
                let neighbor_contribution_rate = agent
                    .neighbors
                    .iter()
                    .map(|neighbor| if previous_actions[*neighbor] { 1.0 } else { 0.0 })
                    .sum::<f64>()
                    / agent.neighbors.len() as f64;

                let mut evidence_rate = 0.0;
                if treatment == Treatment::EvidenceBasedTrust {
                    for (edge, neighbor) in agent.neighbors.iter().copied().enumerate() {
                        let mut observed = if previous_actions[neighbor] { 1.0 } else { 0.0 };
                        if decision_noise_draws[i][edge] < scenario.observation_noise {
                            observed = 1.0 - observed;
                        }
                        if decision_capture_draws[i][edge] < scenario.attestation_tampering_rate {
                            observed = 1.0 - observed;
                        }
                        evidence_rate += observed / agent.neighbors.len() as f64;
                    }
                    observation_overhead +=
                        scenario.observation_cost * agent.neighbors.len() as f64;
                }

                let probability = match treatment {
                    Treatment::NoCommitments => {
                        0.12 + 0.25 * urgency + 0.15 * agent.willingness
                    }
                    Treatment::StaticTrust => {
                        0.04 + 0.34 * mean_trust + 0.23 * urgency
                            + 0.08 * agent.willingness
                    }
                    Treatment::Reciprocity => {
                        0.03 + 0.20 * mean_trust
                            + 0.42 * neighbor_contribution_rate
                            + 0.22 * urgency
                            + 0.05 * agent.willingness
                    }
                    Treatment::EvidenceBasedTrust => {
                        0.03 + 0.20 * mean_trust
                            + 0.35 * evidence_rate
                            + 0.20 * urgency
                            + 0.05 * agent.willingness
                            - 0.50 * scenario.observation_cost
                    }
                }
                .clamp(0.0, 0.95);

                if action_draws[i] < probability {
                    actions[i] = true;
                }
            }
        }

        let mut contribution_this_tick = 0_usize;
        for i in 0..n {
            if actions[i] {
                agents[i].reserve = (agents[i].reserve - 0.08).max(0.0);
                contribution_this_tick += 1;
            }
        }
        contribution_count += contribution_this_tick;

        if in_shock_window {
            repair_progress = (repair_progress
                + contribution_this_tick as f64 * scenario.repair_gain_per_contribution)
                .clamp(0.0, 1.0);
            let post_action_capacity = scenario.shock_capacity
                + (scenario.baseline_capacity - scenario.shock_capacity) * repair_progress;
            if recovery_tick.is_none()
                && post_action_capacity >= scenario.baseline_capacity * 0.98
            {
                recovery_tick = Some(tick - scenario.shock_start_tick);
            }
        }

        if treatment == Treatment::EvidenceBasedTrust {
            for i in 0..n {
                for edge in 0..agents[i].neighbors.len() {
                    let neighbor = agents[i].neighbors[edge];
                    let mut observed = if actions[neighbor] { 1.0 } else { 0.0 };
                    if update_noise_draws[i][edge] < scenario.observation_noise {
                        observed = 1.0 - observed;
                    }
                    if update_capture_draws[i][edge] < scenario.attestation_tampering_rate {
                        observed = 1.0 - observed;
                    }
                    let old_trust = agents[i].trust[edge];
                    agents[i].trust[edge] =
                        old_trust + scenario.trust_learning_rate * (observed - old_trust);
                }
            }
        }

        previous_actions = actions;
        if tick + 1 == shock_end {
            repair_progress_at_shock_end = repair_progress;
        }
    }

    Ok(Metrics {
        treatment,
        unmet_need_integral,
        peak_unmet_need,
        unmet_need_inequality_gini: gini_nonnegative(&unmet_by_agent),
        contribution_count,
        recovery_tick,
        observation_overhead,
        repair_progress_at_shock_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gini_handles_equal_and_concentrated_values() {
        assert!((gini_nonnegative(&[1.0, 1.0, 1.0]) - 0.0).abs() < 1e-12);
        assert!((gini_nonnegative(&[0.0, 1.0]) - 0.5).abs() < 1e-12);
        assert_eq!(gini_nonnegative(&[0.0, 0.0]), 0.0);
    }

    #[test]
    fn identical_seed_and_treatment_replay_identically() {
        let scenario = Scenario::default();
        let first = simulate(&scenario, 42, Treatment::Reciprocity).unwrap();
        let second = simulate(&scenario, 42, Treatment::Reciprocity).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn invalid_parameters_fail_closed() {
        let mut scenario = Scenario::default();
        scenario.observation_noise = f64::NAN;
        assert!(simulate(&scenario, 1, Treatment::NoCommitments).is_err());

        let mut scenario = Scenario::default();
        scenario.shock_capacity = scenario.baseline_capacity;
        assert!(scenario.validate().is_err());
    }

    #[test]
    fn all_treatments_emit_finite_bounded_metrics() {
        let scenario = Scenario::default();
        for treatment in Treatment::ALL {
            let metrics = simulate(&scenario, 7, treatment).unwrap();
            assert!(metrics.unmet_need_integral.is_finite());
            assert!(metrics.peak_unmet_need.is_finite());
            assert!(
                (0.0..=1.0).contains(&metrics.unmet_need_inequality_gini)
            );
            assert!(metrics.observation_overhead.is_finite());
            assert!(metrics.repair_progress_at_shock_end.is_finite());
            assert!((0.0..=1.0).contains(&metrics.repair_progress_at_shock_end));
            assert_eq!(
                metrics.observation_overhead > 0.0,
                treatment == Treatment::EvidenceBasedTrust
            );
        }
    }

    #[test]
    fn zero_duration_shock_produces_no_repair_contributions() {
        let mut scenario = Scenario::default();
        scenario.shock_duration_ticks = 0;
        let metrics = simulate(&scenario, 5, Treatment::EvidenceBasedTrust).unwrap();
        assert_eq!(metrics.contribution_count, 0);
        assert_eq!(metrics.repair_progress_at_shock_end, 0.0);
        assert_eq!(metrics.recovery_tick, None);
    }

    #[test]
    fn all_treatment_arms_are_replayable_for_a_fixed_seed() {
        let scenario = Scenario::default();
        for treatment in Treatment::ALL {
            let first = simulate(&scenario, 99, treatment).unwrap();
            let second = simulate(&scenario, 99, treatment).unwrap();
            assert_eq!(first, second, "treatment {:?} did not replay", treatment);
        }
    }
}
