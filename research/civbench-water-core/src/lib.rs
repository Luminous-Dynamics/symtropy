#![forbid(unsafe_code)]

//! A minimal, headless water stock-flow kernel for CivBench.
//!
//! All volumes are integer litres per discrete simulation tick. The model
//! makes storage, inflow, demand, spill, delivery, and ending stock explicit.
//! It intentionally does not model hydrology, water quality, ecology, or
//! inter-settlement transfers.

use core::fmt;

/// Explicit allocation order for the two demand categories.
///
/// This is a policy input, not a normative claim about which demand should
/// receive priority in a real society.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationPriority {
    /// Satisfy household demand before agriculture demand.
    HouseholdFirst,
    /// Satisfy agriculture demand before household demand.
    AgricultureFirst,
}

/// Requested water for one simulation tick, measured in litres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterDemand {
    /// Requested household water in litres for this tick.
    pub household_l: u64,
    /// Requested agricultural water in litres for this tick.
    pub agriculture_l: u64,
}

impl WaterDemand {
    /// Construct a demand record.
    pub const fn new(household_l: u64, agriculture_l: u64) -> Self {
        Self {
            household_l,
            agriculture_l,
        }
    }
}

/// Errors returned when a water stock-flow step cannot be represented safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterError {
    /// The initial stock exceeds the declared storage capacity.
    InitialStockExceedsCapacity {
        /// The invalid starting stock.
        stored_l: u64,
        /// The declared maximum storage.
        capacity_l: u64,
    },
    /// Adding inflow to the initial stock exceeds the u64 litre range.
    AvailableWaterOverflow,
}

impl fmt::Display for WaterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitialStockExceedsCapacity {
                stored_l,
                capacity_l,
            } => write!(
                f,
                "initial water stock {stored_l} L exceeds capacity {capacity_l} L"
            ),
            Self::AvailableWaterOverflow => {
                write!(f, "initial stock plus inflow exceeds the u64 litre range")
            }
        }
    }
}

impl std::error::Error for WaterError {}

/// Water storage state at the start of a simulation tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterStorage {
    capacity_l: u64,
    stored_l: u64,
}

impl WaterStorage {
    /// Create storage with a declared capacity and initial stock.
    ///
    /// Returns an error rather than silently clamping an invalid initial state.
    pub const fn new(capacity_l: u64, stored_l: u64) -> Result<Self, WaterError> {
        if stored_l > capacity_l {
            return Err(WaterError::InitialStockExceedsCapacity {
                stored_l,
                capacity_l,
            });
        }

        Ok(Self {
            capacity_l,
            stored_l,
        })
    }

    /// Return the storage capacity in litres.
    pub const fn capacity_l(&self) -> u64 {
        self.capacity_l
    }

    /// Return the currently stored water in litres.
    pub const fn stored_l(&self) -> u64 {
        self.stored_l
    }

    /// Apply one tick of inflow and demand, updating the storage stock.
    ///
    /// Water above capacity spills before allocation. The selected category
    /// receives priority; the other category receives any remaining water.
    /// Unmet demand is reported rather than hidden or carried forward.
    pub fn step(
        &mut self,
        inflow_l: u64,
        demand: WaterDemand,
        priority: AllocationPriority,
    ) -> Result<WaterStepReport, WaterError> {
        if self.stored_l > self.capacity_l {
            return Err(WaterError::InitialStockExceedsCapacity {
                stored_l: self.stored_l,
                capacity_l: self.capacity_l,
            });
        }

        let stock_before_l = self.stored_l;
        let total_available_l = stock_before_l
            .checked_add(inflow_l)
            .ok_or(WaterError::AvailableWaterOverflow)?;

        let spill_l = total_available_l.saturating_sub(self.capacity_l);
        let allocatable_l = total_available_l.min(self.capacity_l);

        let (first_demand_l, second_demand_l) = match priority {
            AllocationPriority::HouseholdFirst => {
                (demand.household_l, demand.agriculture_l)
            }
            AllocationPriority::AgricultureFirst => {
                (demand.agriculture_l, demand.household_l)
            }
        };

        let first_delivered_l = first_demand_l.min(allocatable_l);
        let remaining_after_first_l = allocatable_l - first_delivered_l;
        let second_delivered_l = second_demand_l.min(remaining_after_first_l);
        let stock_after_l = remaining_after_first_l - second_delivered_l;

        let (household_delivered_l, agriculture_delivered_l) = match priority {
            AllocationPriority::HouseholdFirst => {
                (first_delivered_l, second_delivered_l)
            }
            AllocationPriority::AgricultureFirst => {
                (second_delivered_l, first_delivered_l)
            }
        };

        let report = WaterStepReport {
            stock_before_l,
            inflow_l,
            spill_l,
            household_demand_l: demand.household_l,
            agriculture_demand_l: demand.agriculture_l,
            household_delivered_l,
            agriculture_delivered_l,
            household_unmet_l: demand.household_l - household_delivered_l,
            agriculture_unmet_l: demand.agriculture_l - agriculture_delivered_l,
            stock_after_l,
            priority,
        };

        debug_assert!(report.conservation_holds());
        self.stored_l = stock_after_l;
        Ok(report)
    }
}

/// Auditable result of one water allocation step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterStepReport {
    /// Storage at the start of the tick.
    pub stock_before_l: u64,
    /// External water entering during the tick.
    pub inflow_l: u64,
    /// Water that could not fit in storage.
    pub spill_l: u64,
    /// Requested household water.
    pub household_demand_l: u64,
    /// Requested agricultural water.
    pub agriculture_demand_l: u64,
    /// Household water delivered.
    pub household_delivered_l: u64,
    /// Agricultural water delivered.
    pub agriculture_delivered_l: u64,
    /// Household demand not met during this tick.
    pub household_unmet_l: u64,
    /// Agricultural demand not met during this tick.
    pub agriculture_unmet_l: u64,
    /// Storage at the end of the tick.
    pub stock_after_l: u64,
    /// Allocation rule used for this tick.
    pub priority: AllocationPriority,
}

impl WaterStepReport {
    /// Check the exact stock-flow accounting identity using a wider accumulator.
    ///
    /// The identity is:
    /// stock_before + inflow = stock_after + spill + delivered_to_households
    ///                       + delivered_to_agriculture.
    pub fn conservation_holds(&self) -> bool {
        let input = u128::from(self.stock_before_l) + u128::from(self.inflow_l);
        let output = u128::from(self.stock_after_l)
            + u128::from(self.spill_l)
            + u128::from(self.household_delivered_l)
            + u128::from(self.agriculture_delivered_l);
        input == output
    }

    /// Check that delivered plus unmet demand equals requested demand.
    pub fn demand_accounting_holds(&self) -> bool {
        u128::from(self.household_delivered_l) + u128::from(self.household_unmet_l)
            == u128::from(self.household_demand_l)
            && u128::from(self.agriculture_delivered_l)
                + u128::from(self.agriculture_unmet_l)
                == u128::from(self.agriculture_demand_l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_initial_stock_above_capacity() {
        let result = WaterStorage::new(10, 11);
        assert_eq!(
            result,
            Err(WaterError::InitialStockExceedsCapacity {
                stored_l: 11,
                capacity_l: 10,
            })
        );
    }

    #[test]
    fn accounts_for_spill_and_delivery() {
        let mut storage = WaterStorage::new(100, 80).unwrap();
        let report = storage
            .step(50, WaterDemand::new(10, 0), AllocationPriority::HouseholdFirst)
            .unwrap();

        assert_eq!(report.spill_l, 30);
        assert_eq!(report.household_delivered_l, 10);
        assert_eq!(report.stock_after_l, 90);
        assert!(report.conservation_holds());
        assert!(report.demand_accounting_holds());
        assert_eq!(storage.stored_l(), 90);
    }

    #[test]
    fn household_priority_is_explicit_and_auditable() {
        let mut storage = WaterStorage::new(100, 60).unwrap();
        let report = storage
            .step(0, WaterDemand::new(80, 50), AllocationPriority::HouseholdFirst)
            .unwrap();

        assert_eq!(report.household_delivered_l, 60);
        assert_eq!(report.agriculture_delivered_l, 0);
        assert_eq!(report.household_unmet_l, 20);
        assert_eq!(report.agriculture_unmet_l, 50);
        assert!(report.demand_accounting_holds());
    }

    #[test]
    fn agriculture_priority_is_explicit_and_auditable() {
        let mut storage = WaterStorage::new(100, 60).unwrap();
        let report = storage
            .step(0, WaterDemand::new(80, 50), AllocationPriority::AgricultureFirst)
            .unwrap();

        assert_eq!(report.agriculture_delivered_l, 50);
        assert_eq!(report.household_delivered_l, 10);
        assert_eq!(report.agriculture_unmet_l, 0);
        assert_eq!(report.household_unmet_l, 70);
        assert!(report.demand_accounting_holds());
    }

    #[test]
    fn reports_shortage_without_creating_water() {
        let mut storage = WaterStorage::new(0, 0).unwrap();
        let report = storage
            .step(0, WaterDemand::new(12, 25), AllocationPriority::HouseholdFirst)
            .unwrap();

        assert_eq!(report.household_delivered_l, 0);
        assert_eq!(report.agriculture_delivered_l, 0);
        assert_eq!(report.household_unmet_l, 12);
        assert_eq!(report.agriculture_unmet_l, 25);
        assert_eq!(storage.stored_l(), 0);
        assert!(report.conservation_holds());
    }

    #[test]
    fn rejects_integer_overflow_in_available_water() {
        let mut storage = WaterStorage::new(u64::MAX, 1).unwrap();
        let result = storage.step(
            u64::MAX,
            WaterDemand::new(0, 0),
            AllocationPriority::HouseholdFirst,
        );
        assert_eq!(result, Err(WaterError::AvailableWaterOverflow));
        assert_eq!(storage.stored_l(), 1);
    }

    #[test]
    fn identical_inputs_produce_identical_reports() {
        let mut first = WaterStorage::new(100, 70).unwrap();
        let mut second = WaterStorage::new(100, 70).unwrap();
        let demand = WaterDemand::new(30, 80);

        let a = first
            .step(15, demand, AllocationPriority::HouseholdFirst)
            .unwrap();
        let b = second
            .step(15, demand, AllocationPriority::HouseholdFirst)
            .unwrap();

        assert_eq!(a, b);
        assert_eq!(first, second);
    }

    #[test]
    fn bounded_exhaustive_accounting_property() {
        for capacity in 0..=8 {
            for initial in 0..=capacity {
                for inflow in 0..=8 {
                    for household in 0..=8 {
                        for agriculture in 0..=8 {
                            for priority in [
                                AllocationPriority::HouseholdFirst,
                                AllocationPriority::AgricultureFirst,
                            ] {
                                let mut storage =
                                    WaterStorage::new(capacity, initial).unwrap();
                                let report = storage
                                    .step(
                                        inflow,
                                        WaterDemand::new(household, agriculture),
                                        priority,
                                    )
                                    .unwrap();

                                assert!(report.conservation_holds());
                                assert!(report.demand_accounting_holds());
                                assert!(report.stock_after_l <= capacity);
                                assert_eq!(storage.stored_l(), report.stock_after_l);
                                assert_eq!(
                                    report.household_delivered_l + report.household_unmet_l,
                                    household
                                );
                                assert_eq!(
                                    report.agriculture_delivered_l + report.agriculture_unmet_l,
                                    agriculture
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
