//! Resource-bounded catalytic-network experiments.
//!
//! This is a deliberately small artificial-life research model, not a
//! biological chemistry simulator. Resource units are abstract and must not
//! be described as joules or chemical free energy.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    CatalystA,
    CatalystB,
    Membrane,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModelError {
    InvalidInitialState,
    InvalidResourceValue,
    TickBudgetExceeded,
    ArithmeticOverflow,
    ResourceInvariantViolation { residual: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InitialState {
    pub environment: f64,
    pub stock: f64,
    pub catalyst_a: f64,
    pub catalyst_b: f64,
    pub membrane: f64,
}

impl Default for InitialState {
    fn default() -> Self {
        Self {
            environment: 100.0,
            stock: 1.0,
            catalyst_a: 1.0,
            catalyst_b: 1.0,
            membrane: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepReceipt {
    pub tick: u64,
    pub uptake: f64,
    pub synthesized_a: f64,
    pub synthesized_b: f64,
    pub synthesized_membrane: f64,
    pub degraded_a: f64,
    pub degraded_b: f64,
    pub degraded_membrane: f64,
    pub resource_total: f64,
    pub resource_residual: f64,
}

#[derive(Clone, Debug)]
struct State {
    environment: f64,
    stock: f64,
    catalyst_a: f64,
    catalyst_b: f64,
    membrane: f64,
    waste: f64,
}

impl State {
    fn total(&self) -> f64 {
        self.environment + self.stock + self.catalyst_a + self.catalyst_b + self.membrane + self.waste
    }

    fn finite_nonnegative(&self) -> bool {
        [
            self.environment,
            self.stock,
            self.catalyst_a,
            self.catalyst_b,
            self.membrane,
            self.waste,
        ]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
    }
}

#[derive(Clone, Debug)]
pub struct AutopoieticNetwork {
    state: State,
    initial_total: f64,
    tick: u64,
    max_ticks: u64,
}

impl AutopoieticNetwork {
    pub fn new(initial: InitialState, max_ticks: u64) -> Result<Self, ModelError> {
        let state = State {
            environment: initial.environment,
            stock: initial.stock,
            catalyst_a: initial.catalyst_a,
            catalyst_b: initial.catalyst_b,
            membrane: initial.membrane,
            waste: 0.0,
        };
        if max_ticks == 0 || !state.finite_nonnegative()
            || state.catalyst_a > 1.0 || state.catalyst_b > 1.0 || state.membrane > 1.0
        {
            return Err(ModelError::InvalidInitialState);
        }
        let initial_total = state.total();
        if !initial_total.is_finite() {
            return Err(ModelError::InvalidInitialState);
        }
        Ok(Self { state, initial_total, tick: 0, max_ticks })
    }

    pub fn tick(&self) -> u64 { self.tick }

    pub fn component(&self, component: Component) -> f64 {
        match component {
            Component::CatalystA => self.state.catalyst_a,
            Component::CatalystB => self.state.catalyst_b,
            Component::Membrane => self.state.membrane,
        }
    }

    pub fn stock(&self) -> f64 { self.state.stock }
    pub fn environmental_resource(&self) -> f64 { self.state.environment }
    pub fn waste(&self) -> f64 { self.state.waste }
    pub fn resource_total(&self) -> f64 { self.state.total() }
    pub fn resource_residual(&self) -> f64 { self.state.total() - self.initial_total }

    /// Operational viability predicate for this toy model, not a definition of life.
    pub fn is_viable(&self) -> bool {
        self.state.catalyst_a >= 0.25
            && self.state.catalyst_b >= 0.25
            && self.state.membrane >= 0.20
    }

    /// Apply an externally imposed component knockout.
    ///
    /// Removed component material is moved to the waste store, preserving
    /// the total inventory. The recovery process itself remains endogenous.
    pub fn knockout(&mut self, component: Component) -> Result<f64, ModelError> {
        let amount = match component {
            Component::CatalystA => {
                let removed = self.state.catalyst_a;
                self.state.catalyst_a = 0.0;
                removed
            }
            Component::CatalystB => {
                let removed = self.state.catalyst_b;
                self.state.catalyst_b = 0.0;
                removed
            }
            Component::Membrane => {
                let removed = self.state.membrane;
                self.state.membrane = 0.0;
                removed
            }
        };
        self.state.waste += amount;
        self.check_invariant()?;
        Ok(amount)
    }

    /// One deterministic model step. All candidate changes are computed in a
    /// cloned state and committed only after resource and finiteness checks.
    pub fn step(&mut self) -> Result<StepReceipt, ModelError> {
        let next_tick = self.tick.checked_add(1).ok_or(ModelError::ArithmeticOverflow)?;
        if next_tick > self.max_ticks {
            return Err(ModelError::TickBudgetExceeded);
        }

        let mut next = self.state.clone();

        // Uptake requires both a functioning boundary and catalytic activity.
        // This gate makes a double-catalyst knockout unable to self-repair.
        let catalyst_activity = self.state.catalyst_a + self.state.catalyst_b;
        let uptake = if self.state.membrane >= 0.20 && catalyst_activity >= 0.10 {
            (0.04 * self.state.membrane * (catalyst_activity.min(2.0) / 2.0))
                .min(self.state.environment)
        } else {
            0.0
        };
        next.environment -= uptake;
        next.stock += uptake;

        // Each catalytic process regenerates the other catalyst. Either
        // remaining catalyst can also help regenerate the membrane.
        // Production desires are computed from the pre-step state and then
        // proportionally scaled if the shared stock is insufficient.
        let desired_a = if self.state.catalyst_b >= 0.05 {
            (0.003 * self.state.catalyst_b).min(1.0 - self.state.catalyst_a)
        } else { 0.0 };
        let desired_b = if self.state.catalyst_a >= 0.05 {
            (0.003 * self.state.catalyst_a).min(1.0 - self.state.catalyst_b)
        } else { 0.0 };
        let desired_membrane = if catalyst_activity >= 0.10 {
            (0.006 * catalyst_activity).min(1.0 - self.state.membrane)
        } else { 0.0 };

        let desired_total = desired_a + desired_b + desired_membrane;
        let scale = if desired_total <= 0.0 {
            0.0
        } else {
            (next.stock / desired_total).min(1.0)
        };
        let made_a = desired_a * scale;
        let made_b = desired_b * scale;
        let made_membrane = desired_membrane * scale;
        let synthesis_cost = made_a + made_b + made_membrane;

        next.stock -= synthesis_cost;
        next.catalyst_a += made_a;
        next.catalyst_b += made_b;
        next.membrane += made_membrane;

        // Component turnover transfers degraded material to the explicit
        // waste store; no material silently disappears.
        let degraded_a = next.catalyst_a * 0.002;
        let degraded_b = next.catalyst_b * 0.002;
        let degraded_membrane = next.membrane * 0.002;
        next.catalyst_a -= degraded_a;
        next.catalyst_b -= degraded_b;
        next.membrane -= degraded_membrane;
        next.waste += degraded_a + degraded_b + degraded_membrane;

        if !next.finite_nonnegative() {
            return Err(ModelError::InvalidResourceValue);
        }

        let total = next.total();
        let residual = total - self.initial_total;
        let tolerance = 1e-10 * self.initial_total.max(f64::MIN_POSITIVE);
        if !residual.is_finite() || residual.abs() > tolerance {
            return Err(ModelError::ResourceInvariantViolation { residual });
        }

        self.state = next;
        self.tick = next_tick;
        Ok(StepReceipt {
            tick: self.tick,
            uptake,
            synthesized_a: made_a,
            synthesized_b: made_b,
            synthesized_membrane: made_membrane,
            degraded_a,
            degraded_b,
            degraded_membrane,
            resource_total: total,
            resource_residual: residual,
        })
    }

    pub fn run_steps(&mut self, steps: u64) -> Result<(), ModelError> {
        let final_tick = self.tick.checked_add(steps).ok_or(ModelError::ArithmeticOverflow)?;
        if final_tick > self.max_ticks {
            return Err(ModelError::TickBudgetExceeded);
        }
        for _ in 0..steps {
            self.step()?;
        }
        Ok(())
    }

    fn check_invariant(&self) -> Result<(), ModelError> {
        let residual = self.resource_residual();
        let tolerance = 1e-10 * self.initial_total.max(f64::MIN_POSITIVE);
        if !self.state.finite_nonnegative() || !residual.is_finite() || residual.abs() > tolerance {
            return Err(ModelError::ResourceInvariantViolation { residual });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recover_after_knockout(component: Component, steps: u64) -> AutopoieticNetwork {
        let mut network = AutopoieticNetwork::new(InitialState::default(), 10_000).unwrap();
        network.run_steps(32).unwrap();
        network.knockout(component).unwrap();
        network.run_steps(steps).unwrap();
        network
    }

    #[test]
    fn resource_inventory_is_conserved_through_steps_and_knockout() {
        let mut network = AutopoieticNetwork::new(InitialState::default(), 2_000).unwrap();
        network.run_steps(1_000).unwrap();
        network.knockout(Component::CatalystA).unwrap();
        network.run_steps(500).unwrap();
        assert!(network.resource_residual().abs() < 1e-8);
    }

    #[test]
    fn catalyst_a_is_regenerated_by_catalyst_b() {
        let network = recover_after_knockout(Component::CatalystA, 600);
        assert!(network.component(Component::CatalystA) > 0.50);
        assert!(network.is_viable());
    }

    #[test]
    fn catalyst_b_is_regenerated_by_catalyst_a() {
        let network = recover_after_knockout(Component::CatalystB, 600);
        assert!(network.component(Component::CatalystB) > 0.50);
        assert!(network.is_viable());
    }

    #[test]
    fn membrane_is_regenerated_by_remaining_catalysts_from_internal_stock() {
        let network = recover_after_knockout(Component::Membrane, 100);
        assert!(network.component(Component::Membrane) > 0.20);
        assert!(network.is_viable());
    }

    #[test]
    fn removing_both_catalysts_breaks_the_regenerative_closure() {
        let mut network = AutopoieticNetwork::new(InitialState::default(), 2_000).unwrap();
        network.knockout(Component::CatalystA).unwrap();
        network.knockout(Component::CatalystB).unwrap();
        network.run_steps(1_000).unwrap();
        assert_eq!(network.component(Component::CatalystA), 0.0);
        assert_eq!(network.component(Component::CatalystB), 0.0);
        assert!(!network.is_viable());
    }

    #[test]
    fn no_resource_means_no_maintenance() {
        let initial = InitialState {
            environment: 0.0,
            stock: 0.0,
            catalyst_a: 1.0,
            catalyst_b: 1.0,
            membrane: 1.0,
        };
        let mut network = AutopoieticNetwork::new(initial, 2_000).unwrap();
        network.run_steps(1_000).unwrap();
        assert!(!network.is_viable());
    }

    #[test]
    fn tick_budget_is_enforced_without_partial_advance() {
        let mut network = AutopoieticNetwork::new(InitialState::default(), 2).unwrap();
        assert_eq!(network.run_steps(3), Err(ModelError::TickBudgetExceeded));
        assert_eq!(network.tick(), 0);
    }
}
