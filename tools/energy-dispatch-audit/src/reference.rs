//! Separate, bounded exhaustive reference enumeration for tiny dispatch cases.
//!
//! This intentionally has its own control flow and does not call Sol-Atlas's
//! search or transition functions. It only runs on small finite requests; if
//! the reference expansion budget is exceeded, the verifier reports UNKNOWN.

use std::cmp::Reverse;

use super::{DispatchObjective, DispatchRequest};

pub(crate) const DEFAULT_REFERENCE_EXPANSION_LIMIT: u64 = 100_000;

#[derive(Clone, Debug)]
pub(crate) struct ReferenceResult {
    pub expansions: u64,
    pub feasible_complete_schedules: u64,
    pub frontier: Vec<DispatchObjective>,
}

#[derive(Clone, Debug)]
pub(crate) enum ReferenceError {
    BudgetExceeded { expansions: u64 },
    CaseTooLarge { intervals: usize },
    ArithmeticOverflow,
}

#[derive(Clone, Copy, Debug, Default)]
struct Totals {
    essential_unserved: u64,
    flexible_unserved: u64,
    cost_units: u64,
    carbon_grams: u64,
    curtailed_pv: u64,
}

impl Totals {
    fn add(
        self,
        essential_unserved: u16,
        flexible_unserved: u16,
        grid_import: u16,
        grid_cost: u32,
        grid_carbon: u32,
        curtailed_pv: u16,
    ) -> Option<Self> {
        Some(Self {
            essential_unserved: self.essential_unserved.checked_add(u64::from(essential_unserved))?,
            flexible_unserved: self.flexible_unserved.checked_add(u64::from(flexible_unserved))?,
            cost_units: self.cost_units.checked_add(u64::from(grid_import).checked_mul(u64::from(grid_cost))?)?,
            carbon_grams: self.carbon_grams.checked_add(u64::from(grid_import).checked_mul(u64::from(grid_carbon))?)?,
            curtailed_pv: self.curtailed_pv.checked_add(u64::from(curtailed_pv))?,
        })
    }
}

struct Enumerator {
    expansion_limit: u64,
    expansions: u64,
    feasible_complete_schedules: u64,
    frontier: Vec<DispatchObjective>,
}

impl Enumerator {
    fn tick(&mut self) -> Result<(), ReferenceError> {
        if self.expansions >= self.expansion_limit {
            return Err(ReferenceError::BudgetExceeded { expansions: self.expansions });
        }
        self.expansions = self.expansions.checked_add(1).ok_or(ReferenceError::ArithmeticOverflow)?;
        Ok(())
    }

    fn record(&mut self, objective: DispatchObjective) -> Result<(), ReferenceError> {
        self.feasible_complete_schedules = self.feasible_complete_schedules
            .checked_add(1)
            .ok_or(ReferenceError::ArithmeticOverflow)?;

        if self.frontier.iter().any(|known| weakly_dominates(known, &objective)) {
            return Ok(());
        }
        self.frontier.retain(|known| !strictly_dominates(&objective, known));
        self.frontier.push(objective);
        Ok(())
    }

    fn visit(
        &mut self,
        request: &DispatchRequest,
        interval_index: usize,
        soc_before: u16,
        totals: Totals,
    ) -> Result<(), ReferenceError> {
        if interval_index == request.intervals.len() {
            if soc_before >= request.battery.terminal_reserve_quanta {
                self.record(DispatchObjective {
                    essential_unserved_quanta: totals.essential_unserved,
                    flexible_unserved_quanta: totals.flexible_unserved,
                    operating_cost_units: totals.cost_units,
                    grid_carbon_grams: totals.carbon_grams,
                    curtailed_pv_quanta: totals.curtailed_pv,
                    terminal_soc_quanta: soc_before,
                })?;
            }
            return Ok(());
        }

        let input = &request.intervals[interval_index];
        let battery = &request.battery;
        let grid_limit = if input.grid_outage { 0 } else { input.grid_import_limit_quanta };
        let withdrawal_limit = battery.discharge_withdrawal_limit_quanta.min(soc_before);

        for grid_import in 0..=grid_limit {
            for withdrawal in 0..=withdrawal_limit {
                for charge in 0..=battery.charge_input_limit_quanta {
                    // Count every visited grid/storage tuple before pruning so
                    // large invalid action spaces still consume the hard budget.
                    self.tick()?;
                    if withdrawal > 0 && charge > 0 {
                        continue;
                    }

                    // Integer efficiencies are part of the v0.1 finite model.
                    let charge_gain = (u64::from(charge)
                        * u64::from(battery.charge_efficiency_numerator)
                        / u64::from(battery.charge_efficiency_denominator.max(1))) as u16;
                    let soc_after = u32::from(soc_before) - u32::from(withdrawal) + u32::from(charge_gain);
                    if soc_after > u32::from(battery.capacity_quanta) {
                        continue;
                    }
                    let discharge_delivered = (u64::from(withdrawal)
                        * u64::from(battery.discharge_efficiency_numerator)
                        / u64::from(battery.discharge_efficiency_denominator.max(1))) as u16;

                    for essential_served in 0..=input.essential_demand_quanta {
                        for flexible_served in 0..=input.flexible_demand_quanta {
                            // Count service allocation pairs too, so the cap also
                            // bounds cases with huge demand ranges.
                            self.tick()?;
                            let demand_and_charge = u64::from(essential_served)
                                + u64::from(flexible_served)
                                + u64::from(charge);
                            let grid_and_storage = u64::from(grid_import) + u64::from(discharge_delivered);
                            if demand_and_charge < grid_and_storage {
                                continue;
                            }
                            let pv_used = demand_and_charge - grid_and_storage;
                            if pv_used > u64::from(input.pv_available_quanta) {
                                continue;
                            }
                            let curtailed_pv = u64::from(input.pv_available_quanta) - pv_used;
                            let next_totals = totals.add(
                                input.essential_demand_quanta - essential_served,
                                input.flexible_demand_quanta - flexible_served,
                                grid_import,
                                input.grid_cost_units_per_quantum,
                                input.grid_carbon_grams_per_quantum,
                                u16::try_from(curtailed_pv).map_err(|_| ReferenceError::ArithmeticOverflow)?,
                            ).ok_or(ReferenceError::ArithmeticOverflow)?;

                            self.visit(request, interval_index + 1, soc_after as u16, next_totals)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn weakly_dominates(lhs: &DispatchObjective, rhs: &DispatchObjective) -> bool {
    lhs.essential_unserved_quanta <= rhs.essential_unserved_quanta
        && lhs.flexible_unserved_quanta <= rhs.flexible_unserved_quanta
        && lhs.operating_cost_units <= rhs.operating_cost_units
        && lhs.grid_carbon_grams <= rhs.grid_carbon_grams
        && lhs.curtailed_pv_quanta <= rhs.curtailed_pv_quanta
        && lhs.terminal_soc_quanta >= rhs.terminal_soc_quanta
}

fn strictly_dominates(lhs: &DispatchObjective, rhs: &DispatchObjective) -> bool {
    weakly_dominates(lhs, rhs) && lhs != rhs
}

pub(crate) fn enumerate(
    request: &DispatchRequest,
    expansion_limit: u64,
) -> Result<ReferenceResult, ReferenceError> {
    // Bound recursion depth independently of action count to avoid stack exhaustion.
    if request.intervals.len() > 64 {
        return Err(ReferenceError::CaseTooLarge { intervals: request.intervals.len() });
    }
    let mut enumerator = Enumerator {
        expansion_limit,
        expansions: 0,
        feasible_complete_schedules: 0,
        frontier: Vec::new(),
    };
    enumerator.visit(request, 0, request.battery.initial_soc_quanta, Totals::default())?;
    enumerator.frontier.sort_by_key(|value| (
        value.essential_unserved_quanta,
        value.flexible_unserved_quanta,
        value.operating_cost_units,
        value.grid_carbon_grams,
        value.curtailed_pv_quanta,
        Reverse(value.terminal_soc_quanta),
    ));
    Ok(ReferenceResult {
        expansions: enumerator.expansions,
        feasible_complete_schedules: enumerator.feasible_complete_schedules,
        frontier: enumerator.frontier,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Battery, DispatchRequest, IntervalInput};

    fn generated_request(seed: u32) -> DispatchRequest {
        let interval_count = 1 + (seed % 2) as usize;
        let capacity = (seed % 3) as u16;
        let mut intervals = Vec::with_capacity(interval_count);
        for index in 0..interval_count {
            let n = seed.wrapping_mul(37).wrapping_add((index as u32) * 19);
            let start = 1_800_000_000_i64 + (index as i64) * 3_600;
            intervals.push(IntervalInput {
                interval_id: format!("generated-{seed}-interval-{index}"),
                start_unix_seconds: start,
                duration_seconds: 3_600,
                grid_outage: n % 5 == 0,
                pv_available_quanta: ((n >> 1) % 2) as u16,
                grid_import_limit_quanta: ((n >> 2) % 2) as u16,
                essential_demand_quanta: ((n >> 3) % 2) as u16,
                flexible_demand_quanta: ((n >> 4) % 2) as u16,
                grid_cost_units_per_quantum: (n % 3) as u32,
                grid_carbon_grams_per_quantum: ((n >> 2) % 4) as u32,
            });
        }

        DispatchRequest {
            schema_version: "energy-dispatch-v0.1".to_owned(),
            case_id: format!("generated-small-case-{seed}"),
            synthetic: true,
            evidence_class: "analyst_assumption".to_owned(),
            evidence_ref: None,
            evidence_as_of: None,
            energy_quantum_kwh: 1.0,
            max_candidate_expansions: 100_000,
            battery: Battery {
                capacity_quanta: capacity,
                initial_soc_quanta: (seed % (u32::from(capacity) + 1)) as u16,
                charge_input_limit_quanta: ((seed >> 1) % 2) as u16,
                discharge_withdrawal_limit_quanta: ((seed >> 2) % 3) as u16,
                charge_efficiency_numerator: 1,
                charge_efficiency_denominator: if seed % 3 == 0 { 2 } else { 1 },
                discharge_efficiency_numerator: 1,
                discharge_efficiency_denominator: if seed % 4 == 0 { 2 } else { 1 },
                terminal_reserve_quanta: (seed % (u32::from(capacity) + 1)) as u16,
            },
            intervals,
        }
    }

    #[test]
    fn generated_small_case_corpus_is_deterministic_and_frontiers_are_nondominated() {
        for seed in 0..128_u32 {
            let request = generated_request(seed);
            let first = enumerate(&request, DEFAULT_REFERENCE_EXPANSION_LIMIT)
                .expect("bounded generated case completes");
            let second = enumerate(&request, DEFAULT_REFERENCE_EXPANSION_LIMIT)
                .expect("same bounded generated case completes again");

            assert_eq!(first.expansions, second.expansions, "seed={seed}");
            assert_eq!(
                first.feasible_complete_schedules,
                second.feasible_complete_schedules,
                "seed={seed}"
            );
            assert_eq!(first.frontier, second.frontier, "seed={seed}");
            assert!(first.expansions <= DEFAULT_REFERENCE_EXPANSION_LIMIT, "seed={seed}");

            for (left_index, left) in first.frontier.iter().enumerate() {
                for (right_index, right) in first.frontier.iter().enumerate() {
                    if left_index != right_index {
                        assert!(
                            !weakly_dominates(left, right),
                            "seed={seed}: retained frontier contains a dominated pair: {left:?} vs {right:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn reference_enumerator_reports_budget_exhaustion_as_unknown_eligible_error() {
        let request = generated_request(37);
        assert!(matches!(
            enumerate(&request, 1),
            Err(ReferenceError::BudgetExceeded { .. })
        ));
    }

    #[test]
    fn reference_enumerator_refuses_unsafe_recursion_depth() {
        let mut request = generated_request(1);
        request.intervals = (0..65)
            .map(|index| IntervalInput {
                interval_id: format!("deep-{index}"),
                start_unix_seconds: 1_800_000_000_i64 + (index as i64) * 3_600,
                duration_seconds: 3_600,
                grid_outage: true,
                pv_available_quanta: 0,
                grid_import_limit_quanta: 0,
                essential_demand_quanta: 0,
                flexible_demand_quanta: 0,
                grid_cost_units_per_quantum: 0,
                grid_carbon_grams_per_quantum: 0,
            })
            .collect();
        assert!(matches!(
            enumerate(&request, DEFAULT_REFERENCE_EXPANSION_LIMIT),
            Err(ReferenceError::CaseTooLarge { intervals: 65 })
        ));
    }
}
