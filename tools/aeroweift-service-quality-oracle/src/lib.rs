//! Tiny exact oracle for AeroWeft demand and service-quality feedback.
//!
//! This is a fixture-specific finite reference implementation. It computes
//! deterministic FCFS queue waiting from simultaneous arrivals and identical
//! service servers, then applies a frozen wait-threshold choice table. It
//! enumerates fixed points and detects update cycles.
//!
//! It is not a passenger-demand model, MNL estimator, general queueing library,
//! capacity planner, flight scheduler, or operational aviation product.
//! All cohorts, thresholds, times and servers are synthetic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

/// One anonymous synthetic cohort's maximum accepted mean UAM queue wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cohort {
    /// Stable synthetic cohort identifier.
    pub id: &'static str,
    /// Maximum accepted mean queue wait in integer seconds.
    pub maximum_wait_seconds: u32,
}

/// The deterministic queue and cohort-choice assumptions for one fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedbackConfig {
    /// Number of identical parallel pad service servers.
    pub servers: u32,
    /// Deterministic service duration per server in seconds.
    pub service_seconds: u32,
    /// Potential cohorts, one synthetic demand unit per cohort.
    pub cohorts: &'static [Cohort],
}

/// Exact total waiting seconds across all UAM-choosing units and the count.
///
/// The mean is total_wait_seconds / customers. A zero-customer state has
/// zero total wait and zero customers; choice from that state uses zero wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitTotal {
    /// Sum of individual queue waits in seconds.
    pub total_wait_seconds: u64,
    /// Number of UAM-choosing units.
    pub customers: u64,
}

/// The result of repeatedly applying one deterministic demand update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackOutcome {
    /// Demand is unchanged by one application of the feedback operator.
    FixedPoint {
        /// Exact equilibrium UAM demand in cohort units.
        demand: usize,
        /// Exact service wait at the fixed point.
        wait: WaitTotal,
    },
    /// A previously visited demand state recurred before a fixed point.
    Cycle {
        /// Canonical cycle, rotated to begin with its smallest demand state.
        demand_states: Vec<usize>,
    },
}

/// Calculate mean-queue waiting exactly for simultaneous arrivals.
///
/// Customers are assigned in stable cohort order to identical FCFS servers.
/// The zero-based customer at position i waits floor(i / servers) complete
/// service intervals. The returned total is an exact integer; no floating
/// point arithmetic is used for threshold comparisons.
pub fn mean_wait(
    config: &FeedbackConfig,
    demand: usize,
) -> Option<WaitTotal> {
    if config.servers == 0 || config.service_seconds == 0 {
        return None;
    }
    if demand > config.cohorts.len() {
        return None;
    }
    if demand == 0 {
        return Some(WaitTotal {
            total_wait_seconds: 0,
            customers: 0,
        });
    }

    let servers = u64::from(config.servers);
    let service_seconds = u64::from(config.service_seconds);
    let mut total_wait_seconds = 0_u64;

    for index in 0..demand {
        let service_waves = (index as u64) / servers;
        let wait = service_waves.checked_mul(service_seconds)?;
        total_wait_seconds = total_wait_seconds.checked_add(wait)?;
    }

    Some(WaitTotal {
        total_wait_seconds,
        customers: demand as u64,
    })
}

/// Whether one cohort chooses UAM under the exact mean-wait threshold.
///
/// A zero-demand state has zero queue wait. Ties choose UAM, as declared by
/// the golden fixture.
pub fn cohort_chooses_uam(
    cohort: &Cohort,
    wait: WaitTotal,
) -> bool {
    if wait.customers == 0 {
        return true;
    }

    wait.total_wait_seconds
        <= u64::from(cohort.maximum_wait_seconds) * wait.customers
}

/// Apply one simultaneous demand update to the previous demand state.
pub fn next_demand(
    config: &FeedbackConfig,
    current_demand: usize,
) -> Option<usize> {
    let wait = mean_wait(config, current_demand)?;
    let next = config
        .cohorts
        .iter()
        .filter(|cohort| cohort_chooses_uam(cohort, wait))
        .count();
    Some(next)
}

/// Enumerate every exact fixed point in the finite demand-state space.
pub fn fixed_points(config: &FeedbackConfig) -> Option<Vec<FeedbackOutcome>> {
    if config.servers == 0 || config.service_seconds == 0 {
        return None;
    }

    let mut results = Vec::new();
    for demand in 0..=config.cohorts.len() {
        if next_demand(config, demand)? == demand {
            results.push(FeedbackOutcome::FixedPoint {
                demand,
                wait: mean_wait(config, demand)?,
            });
        }
    }
    Some(results)
}

fn canonical_cycle(mut states: Vec<usize>) -> Vec<usize> {
    if let Some((min_index, _)) = states.iter().enumerate().min_by_key(|(_, v)| *v) {
        states.rotate_left(min_index);
    }
    states
}

/// Iterate the finite feedback map from an explicit initial demand.
///
/// Every state is recorded. If the next state equals the current state, a
/// fixed point is returned. If any earlier state recurs, the exact cycle is
/// returned rather than being mistaken for convergence.
pub fn iterate(
    config: &FeedbackConfig,
    initial_demand: usize,
) -> Option<FeedbackOutcome> {
    if initial_demand > config.cohorts.len()
        || config.servers == 0
        || config.service_seconds == 0
    {
        return None;
    }

    let mut seen: BTreeMap<usize, usize> = BTreeMap::new();
    let mut states: Vec<usize> = Vec::new();
    let mut current = initial_demand;

    loop {
        if let Some(first_index) = seen.get(&current).copied() {
            return Some(FeedbackOutcome::Cycle {
                demand_states: canonical_cycle(states[first_index..].to_vec()),
            });
        }

        seen.insert(current, states.len());
        states.push(current);

        let next = next_demand(config, current)?;
        if next == current {
            return Some(FeedbackOutcome::FixedPoint {
                demand: current,
                wait: mean_wait(config, current)?,
            });
        }

        current = next;
    }
}
