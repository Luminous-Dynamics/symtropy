//! Exact process-transfer accounting for a small synthetic aquatic compartment.
//!
//! This crate intentionally does not predict ecological rates. It receives explicit
//! process transfers and checks that state transitions are valid, atomic, and
//! conservative on their declared phosphorus-equivalent basis. Oxygen has a
//! separate explicit source/sink budget because photosynthesis, respiration,
//! decomposition, and gas exchange are not a closed oxygen-conservation system.

use std::collections::BTreeSet;
use std::fmt;

/// One compartment's non-negative stock inventory.
/// Nutrient and organic compartments use the same declared P-equivalent unit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stocks {
    pub dissolved_p_umol: u64,
    pub living_biomass_p_umol: u64,
    pub water_detritus_p_umol: u64,
    pub sediment_organic_p_umol: u64,
    pub dissolved_oxygen_umol: u64,
}

impl Stocks {
    pub fn nutrient_total_umol(&self) -> u128 {
        u128::from(self.dissolved_p_umol)
            + u128::from(self.living_biomass_p_umol)
            + u128::from(self.water_detritus_p_umol)
            + u128::from(self.sediment_organic_p_umol)
    }
}

/// Canonical stocks plus transfer identities already committed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EcosystemState {
    pub stocks: Stocks,
    applied_step_ids: BTreeSet<u64>,
}

impl EcosystemState {
    pub fn new(stocks: Stocks) -> Self {
        Self {
            stocks,
            applied_step_ids: BTreeSet::new(),
        }
    }

    pub fn has_applied_step(&self, step_id: u64) -> bool {
        self.applied_step_ids.contains(&step_id)
    }

    /// Apply one simultaneous net-flux interval transactionally.
    ///
    /// Transfers within the interval are represented as fluxes; every final stock
    /// is computed from its original inventory plus all incoming terms minus all
    /// outgoing terms. Nothing is committed if any stock would underflow/overflow,
    /// a budget identity fails, or the step ID has already been committed.
    pub fn apply_step(&mut self, step: &ProcessStep) -> Result<StepReceipt, StepError> {
        if self.applied_step_ids.contains(&step.step_id) {
            return Err(StepError::DuplicateStepId {
                step_id: step.step_id,
            });
        }

        let before = self.stocks;
        let nutrient_before = before.nutrient_total_umol();
        let oxygen_before = u128::from(before.dissolved_oxygen_umol);

        let next = Stocks {
            dissolved_p_umol: update_stock(
                before.dissolved_p_umol,
                &[step.external_p_in_umol, step.mineralization_p_umol],
                &[step.external_p_out_umol, step.uptake_p_umol],
                Compartment::DissolvedNutrient,
            )?,
            living_biomass_p_umol: update_stock(
                before.living_biomass_p_umol,
                &[step.uptake_p_umol],
                &[step.mortality_p_umol],
                Compartment::LivingBiomass,
            )?,
            water_detritus_p_umol: update_stock(
                before.water_detritus_p_umol,
                &[step.mortality_p_umol],
                &[step.mineralization_p_umol, step.settling_p_umol],
                Compartment::WaterDetritus,
            )?,
            sediment_organic_p_umol: update_stock(
                before.sediment_organic_p_umol,
                &[step.settling_p_umol],
                &[],
                Compartment::SedimentOrganic,
            )?,
            dissolved_oxygen_umol: update_stock(
                before.dissolved_oxygen_umol,
                &[step.photosynthesis_o2_umol, step.reaeration_o2_in_umol],
                &[
                    step.reaeration_o2_out_umol,
                    step.respiration_o2_umol,
                    step.decomposition_o2_umol,
                ],
                Compartment::DissolvedOxygen,
            )?,
        };

        let nutrient_after = next.nutrient_total_umol();
        let nutrient_lhs = nutrient_after + u128::from(step.external_p_out_umol);
        let nutrient_rhs = nutrient_before + u128::from(step.external_p_in_umol);
        if nutrient_lhs != nutrient_rhs {
            return Err(StepError::NutrientBudgetMismatch);
        }

        let oxygen_after = u128::from(next.dissolved_oxygen_umol);
        let oxygen_sources =
            u128::from(step.photosynthesis_o2_umol) + u128::from(step.reaeration_o2_in_umol);
        let oxygen_sinks = u128::from(step.reaeration_o2_out_umol)
            + u128::from(step.respiration_o2_umol)
            + u128::from(step.decomposition_o2_umol);
        if oxygen_after + oxygen_sinks != oxygen_before + oxygen_sources {
            return Err(StepError::OxygenBudgetMismatch);
        }

        let receipt = StepReceipt {
            step_id: step.step_id,
            nutrient_before_umol: nutrient_before,
            nutrient_after_umol: nutrient_after,
            external_p_in_umol: step.external_p_in_umol,
            external_p_out_umol: step.external_p_out_umol,
            oxygen_before_umol: oxygen_before,
            oxygen_after_umol: oxygen_after,
            oxygen_sources_umol: oxygen_sources,
            oxygen_sinks_umol: oxygen_sinks,
        };

        // Commit only after every state and budget check has succeeded.
        self.stocks = next;
        self.applied_step_ids.insert(step.step_id);
        Ok(receipt)
    }
}

/// Explicit process transfers over one interval.
///
/// Nutrient-equivalent transfer meanings:
/// - uptake: dissolved nutrient -> living producer biomass;
/// - mortality: living biomass -> water-column detritus;
/// - mineralization: detritus -> dissolved nutrient;
/// - settling: water-column detritus -> sediment organic stock.
///
/// All these nutrient-like quantities use the same explicitly declared P-equivalent
/// inventory unit in this oracle. This is not a biological carbon/biomass conversion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProcessStep {
    pub step_id: u64,
    pub external_p_in_umol: u64,
    pub external_p_out_umol: u64,
    pub uptake_p_umol: u64,
    pub mortality_p_umol: u64,
    pub mineralization_p_umol: u64,
    pub settling_p_umol: u64,
    pub photosynthesis_o2_umol: u64,
    pub reaeration_o2_in_umol: u64,
    pub reaeration_o2_out_umol: u64,
    pub respiration_o2_umol: u64,
    pub decomposition_o2_umol: u64,
}

/// Machine-readable summary of the committed stock and budget change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StepReceipt {
    pub step_id: u64,
    pub nutrient_before_umol: u128,
    pub nutrient_after_umol: u128,
    pub external_p_in_umol: u64,
    pub external_p_out_umol: u64,
    pub oxygen_before_umol: u128,
    pub oxygen_after_umol: u128,
    pub oxygen_sources_umol: u128,
    pub oxygen_sinks_umol: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compartment {
    DissolvedNutrient,
    LivingBiomass,
    WaterDetritus,
    SedimentOrganic,
    DissolvedOxygen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepError {
    DuplicateStepId { step_id: u64 },
    InsufficientStock { compartment: Compartment },
    StockOverflow { compartment: Compartment },
    NutrientBudgetMismatch,
    OxygenBudgetMismatch,
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateStepId { step_id } => {
                write!(f, "step ID {step_id} was already committed")
            }
            Self::InsufficientStock { compartment } => {
                write!(f, "insufficient stock in {compartment:?}")
            }
            Self::StockOverflow { compartment } => {
                write!(f, "stock overflow in {compartment:?}")
            }
            Self::NutrientBudgetMismatch => write!(f, "nutrient budget mismatch"),
            Self::OxygenBudgetMismatch => write!(f, "oxygen budget mismatch"),
        }
    }
}

impl std::error::Error for StepError {}

fn update_stock(
    initial: u64,
    incoming: &[u64],
    outgoing: &[u64],
    compartment: Compartment,
) -> Result<u64, StepError> {
    let incoming_total = incoming
        .iter()
        .try_fold(0_u128, |sum, value| sum.checked_add(u128::from(*value)))
        .ok_or(StepError::StockOverflow { compartment })?;
    let outgoing_total = outgoing
        .iter()
        .try_fold(0_u128, |sum, value| sum.checked_add(u128::from(*value)))
        .ok_or(StepError::StockOverflow { compartment })?;

    let available = u128::from(initial)
        .checked_add(incoming_total)
        .ok_or(StepError::StockOverflow { compartment })?;
    if outgoing_total > available {
        return Err(StepError::InsufficientStock { compartment });
    }

    let remaining = available - outgoing_total;
    u64::try_from(remaining).map_err(|_| StepError::StockOverflow { compartment })
}

#[cfg(test)]
mod tests {
    use super::{Compartment, EcosystemState, ProcessStep, StepError, Stocks, update_stock};

    const INPUTS: &str = include_str!("../fixtures/process_steps_v0.csv");
    const EXPECTED: &str = include_str!("../fixtures/expected_stocks_v0.csv");

    fn field(fields: &[&str], index: usize) -> u64 {
        fields[index]
            .parse::<u64>()
            .unwrap_or_else(|error| panic!("invalid integer in field {index}: {error}"))
    }

    #[test]
    fn frozen_process_corpus_matches_separate_expected_stocks() {
        let mut input_lines = INPUTS.lines();
        let input_header = input_lines.next().expect("input header");
        assert_eq!(input_header.split(',').count(), 17);

        let mut expected_lines = EXPECTED.lines();
        let expected_header = expected_lines.next().expect("expected header");
        assert_eq!(expected_header.split(',').count(), 6);

        let mut cases = 0;
        for input_line in input_lines {
            if input_line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = input_line.split(',').collect();
            assert_eq!(f.len(), 17, "malformed input row: {}", f[0]);

            let expected_line = expected_lines.next().expect("matching expected row");
            let e: Vec<&str> = expected_line.split(',').collect();
            assert_eq!(e.len(), 6, "malformed expected row: {}", f[0]);
            assert_eq!(f[0], e[0], "case ordering / identity mismatch");

            let initial = Stocks {
                dissolved_p_umol: field(&f, 1),
                living_biomass_p_umol: field(&f, 2),
                water_detritus_p_umol: field(&f, 3),
                sediment_organic_p_umol: field(&f, 4),
                dissolved_oxygen_umol: field(&f, 5),
            };
            let step = ProcessStep {
                step_id: (cases + 1) as u64,
                external_p_in_umol: field(&f, 6),
                external_p_out_umol: field(&f, 7),
                uptake_p_umol: field(&f, 8),
                mortality_p_umol: field(&f, 9),
                mineralization_p_umol: field(&f, 10),
                settling_p_umol: field(&f, 11),
                photosynthesis_o2_umol: field(&f, 12),
                reaeration_o2_in_umol: field(&f, 13),
                reaeration_o2_out_umol: field(&f, 14),
                respiration_o2_umol: field(&f, 15),
                decomposition_o2_umol: field(&f, 16),
            };
            let expected = Stocks {
                dissolved_p_umol: field(&e, 1),
                living_biomass_p_umol: field(&e, 2),
                water_detritus_p_umol: field(&e, 3),
                sediment_organic_p_umol: field(&e, 4),
                dissolved_oxygen_umol: field(&e, 5),
            };

            let mut state = EcosystemState::new(initial);
            state.apply_step(&step).expect("fixture transaction");
            assert_eq!(state.stocks, expected, "case {} failed", f[0]);
            cases += 1;
        }
        assert_eq!(cases, 4);
        assert!(
            expected_lines.next().is_none(),
            "unexpected extra oracle rows"
        );
    }

    #[test]
    fn overspending_rejects_the_entire_step() {
        let mut state = EcosystemState::new(Stocks {
            dissolved_p_umol: 10,
            dissolved_oxygen_umol: 5,
            ..Stocks::default()
        });
        let before = state.clone();
        let bad = ProcessStep {
            step_id: 1,
            uptake_p_umol: 11,
            photosynthesis_o2_umol: 100,
            ..ProcessStep::default()
        };
        assert_eq!(
            state.apply_step(&bad),
            Err(StepError::InsufficientStock {
                compartment: Compartment::DissolvedNutrient
            })
        );
        assert_eq!(state, before, "a rejected step must not partially mutate");
    }

    #[test]
    fn oxygen_sink_excess_is_rejected_atomically() {
        let mut state = EcosystemState::new(Stocks {
            dissolved_oxygen_umol: 9,
            ..Stocks::default()
        });
        let before = state.clone();
        let bad = ProcessStep {
            step_id: 2,
            respiration_o2_umol: 10,
            ..ProcessStep::default()
        };
        assert_eq!(
            state.apply_step(&bad),
            Err(StepError::InsufficientStock {
                compartment: Compartment::DissolvedOxygen
            })
        );
        assert_eq!(state, before);
    }

    #[test]
    fn repeated_step_id_cannot_settle_transfers_twice() {
        let mut state = EcosystemState::new(Stocks {
            dissolved_p_umol: 10,
            ..Stocks::default()
        });
        let transfer = ProcessStep {
            step_id: 77,
            uptake_p_umol: 4,
            ..ProcessStep::default()
        };
        state.apply_step(&transfer).expect("first transfer");
        let committed = state.clone();

        let conflicting_replay = ProcessStep {
            step_id: 77,
            uptake_p_umol: 1,
            ..ProcessStep::default()
        };
        assert_eq!(
            state.apply_step(&conflicting_replay),
            Err(StepError::DuplicateStepId { step_id: 77 })
        );
        assert_eq!(state, committed);
    }

    #[test]
    fn stock_overflow_rejects_atomically() {
        let mut state = EcosystemState::new(Stocks {
            dissolved_p_umol: u64::MAX,
            ..Stocks::default()
        });
        let before = state.clone();
        let bad = ProcessStep {
            step_id: 3,
            external_p_in_umol: 1,
            ..ProcessStep::default()
        };
        assert_eq!(
            state.apply_step(&bad),
            Err(StepError::StockOverflow {
                compartment: Compartment::DissolvedNutrient
            })
        );
        assert_eq!(state, before);
    }

    #[test]
    fn internal_transfers_conserve_nutrient_equivalent_stock() {
        let mut state = EcosystemState::new(Stocks {
            dissolved_p_umol: 80,
            living_biomass_p_umol: 10,
            water_detritus_p_umol: 5,
            ..Stocks::default()
        });
        let total_before = state.stocks.nutrient_total_umol();
        let step = ProcessStep {
            step_id: 1,
            uptake_p_umol: 20,
            mortality_p_umol: 8,
            mineralization_p_umol: 3,
            settling_p_umol: 4,
            ..ProcessStep::default()
        };
        let receipt = state.apply_step(&step).expect("valid transfer");
        assert_eq!(total_before, state.stocks.nutrient_total_umol());
        assert_eq!(receipt.nutrient_before_umol, receipt.nutrient_after_umol);
        assert_eq!(
            state.stocks,
            Stocks {
                dissolved_p_umol: 63,
                living_biomass_p_umol: 22,
                water_detritus_p_umol: 6,
                sediment_organic_p_umol: 4,
                dissolved_oxygen_umol: 0,
            }
        );
    }

    #[test]
    fn stock_flux_algebra_is_independent_of_term_order() {
        let forward = update_stock(80, &[20, 3], &[10, 5], Compartment::DissolvedNutrient);
        let permuted = update_stock(80, &[3, 20], &[5, 10], Compartment::DissolvedNutrient);
        assert_eq!(forward, Ok(88));
        assert_eq!(permuted, forward);
    }

    #[test]
    fn zero_inventory_is_not_falsely_treated_as_a_failed_measurement() {
        // Zero is a known inventory here. Missing values are represented by absent
        // inputs at integration boundaries, not by inventing a value in this kernel.
        let mut state = EcosystemState::new(Stocks::default());
        state
            .apply_step(&ProcessStep {
                step_id: 1,
                ..ProcessStep::default()
            })
            .expect("known zero state may advance");
        assert_eq!(state.stocks, Stocks::default());
    }
}
