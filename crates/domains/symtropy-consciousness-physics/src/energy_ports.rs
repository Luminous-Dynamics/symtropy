// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Typed energy-transfer events and system-boundary aggregation.
//!
//! This module models transfers between named energy reservoirs. A transfer
//! wholly inside (or wholly outside) a selected boundary does not cross that
//! boundary. Only a transfer whose source and destination lie on opposite
//! sides contributes to the boundary's work, heat, or matter-carried energy.
//!
//! Event amounts are Joules values, not impulse, force, power, or arbitrary
//! telemetry scalars. The caller must perform any physically valid conversion
//! before constructing an event. An impulse alone is insufficient to infer
//! work or heat.
//!
//! Boundary completeness is an explicit caller assertion. A list of events
//! does not prove that every store or transfer has been instrumented. Unknown
//! or omitted pathways must use BoundaryCoverage::Incomplete.

use std::collections::BTreeSet;

use crate::energy_balance::{
    BoundaryCoverage, BoundaryEnergyFlows, EnergyBalanceInput, Joules,
};

/// Stable identity for a reservoir in one instrumentation namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReservoirId(pub u64);

/// Stable identity for a declared system boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundaryId(pub u64);

/// Physical transfer channel crossing a declared reservoir boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnergyPortChannel {
    /// Mechanical, electrical, or other work crossing the boundary.
    Work,
    /// Heat transferred across the boundary.
    Heat,
    /// Energy carried across the boundary by matter.
    Matter,
}

/// One energy transfer, with direction represented by its endpoints.
///
/// The amount is always a non-negative magnitude in joules. This avoids
/// ambiguous signed-scalar conventions and ensures each physical transfer is
/// recorded once.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyPortEvent {
    event_id: u64,
    channel: EnergyPortChannel,
    source: ReservoirId,
    destination: ReservoirId,
    amount: Joules,
}

impl EnergyPortEvent {
    /// Construct one transfer event.
    ///
    /// A reservoir cannot be both source and destination. Transfer magnitudes
    /// must be finite and non-negative. The amount is explicitly typed as
    /// Joules; an impulse/force/power value cannot be passed without an
    /// explicit conversion.
    pub fn new(
        event_id: u64,
        channel: EnergyPortChannel,
        source: ReservoirId,
        destination: ReservoirId,
        amount: Joules,
    ) -> Result<Self, EnergyPortError> {
        if source == destination {
            return Err(EnergyPortError::SelfTransfer { event_id });
        }
        if !amount.get().is_finite() {
            return Err(EnergyPortError::NonFiniteAmount { event_id });
        }
        if amount.get() < 0.0 {
            return Err(EnergyPortError::NegativeAmount { event_id });
        }

        Ok(Self {
            event_id,
            channel,
            source,
            destination,
            amount,
        })
    }

    pub fn event_id(&self) -> u64 {
        self.event_id
    }

    pub fn channel(&self) -> EnergyPortChannel {
        self.channel
    }

    pub fn source(&self) -> ReservoirId {
        self.source
    }

    pub fn destination(&self) -> ReservoirId {
        self.destination
    }

    pub fn amount(&self) -> Joules {
        self.amount
    }
}

/// A caller-declared cut through the reservoir graph.
///
/// Reservoir membership defines the selected system boundary. A transfer from
/// an inside reservoir to another inside reservoir is internal and cancels at
/// this boundary. The same transfer can correctly appear as an outflow for a
/// smaller boundary that contains only the source reservoir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyPortBoundary {
    id: BoundaryId,
    reservoirs: BTreeSet<ReservoirId>,
    coverage: BoundaryCoverage,
}

impl EnergyPortBoundary {
    /// Declare a boundary from a non-empty set of reservoirs.
    ///
    /// Coverage is an explicit caller assertion. Mark it Incomplete whenever a
    /// relevant store or crossing transfer is unknown.
    pub fn new(
        id: BoundaryId,
        reservoirs: impl IntoIterator<Item = ReservoirId>,
        coverage: BoundaryCoverage,
    ) -> Result<Self, EnergyPortError> {
        let reservoirs: BTreeSet<_> = reservoirs.into_iter().collect();
        if reservoirs.is_empty() {
            return Err(EnergyPortError::EmptyBoundary { boundary_id: id });
        }

        Ok(Self {
            id,
            reservoirs,
            coverage,
        })
    }

    pub fn id(&self) -> BoundaryId {
        self.id
    }

    pub fn coverage(&self) -> BoundaryCoverage {
        self.coverage
    }

    pub fn contains(&self, reservoir: ReservoirId) -> bool {
        self.reservoirs.contains(&reservoir)
    }

    pub fn reservoir_count(&self) -> usize {
        self.reservoirs.len()
    }
}

/// Errors from validating and aggregating typed energy-port events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyPortError {
    EmptyBoundary {
        boundary_id: BoundaryId,
    },
    SelfTransfer {
        event_id: u64,
    },
    NegativeAmount {
        event_id: u64,
    },
    NonFiniteAmount {
        event_id: u64,
    },
    DuplicateEventId {
        event_id: u64,
    },
    ArithmeticOverflow {
        channel: EnergyPortChannel,
    },
}

/// Boundary-cut aggregation of a frozen set of energy-port events.
///
/// Flows contain only transfers that cross this boundary. Internal conversions,
/// such as mechanical energy becoming stored thermal energy inside the same
/// selected system, do not become false boundary outflows.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyPortSummary {
    pub boundary_id: BoundaryId,
    pub boundary_coverage: BoundaryCoverage,
    pub event_count: usize,
    pub crossing_event_count: usize,
    pub boundary_flows: BoundaryEnergyFlows,
}

impl EnergyPortSummary {
    /// Bridge the aggregate into the typed first-law evaluator.
    ///
    /// The supplied stored-energy snapshots must cover the same interval as
    /// the event set. The caller's coverage assertion is preserved verbatim.
    pub fn to_balance_input(
        &self,
        initial_stored_energy_j: Joules,
        final_stored_energy_j: Joules,
    ) -> EnergyBalanceInput {
        EnergyBalanceInput {
            initial_stored_energy_j,
            final_stored_energy_j,
            boundary_flows: self.boundary_flows,
            boundary_coverage: self.boundary_coverage,
        }
    }
}

/// Aggregate a set of transfer events across a declared reservoir boundary.
///
/// Events are processed in ascending event-ID order, so equivalent event sets
/// use the same floating-point summation order regardless of producer
/// scheduling. Duplicate event IDs fail closed rather than permitting one
/// event to overwrite or double-count another.
pub fn aggregate_boundary_ports(
    boundary: &EnergyPortBoundary,
    events: &[EnergyPortEvent],
) -> Result<EnergyPortSummary, EnergyPortError> {
    let mut ordered: Vec<&EnergyPortEvent> = events.iter().collect();
    ordered.sort_by_key(|event| event.event_id);

    for pair in ordered.windows(2) {
        if pair[0].event_id == pair[1].event_id {
            return Err(EnergyPortError::DuplicateEventId {
                event_id: pair[0].event_id,
            });
        }
    }

    let mut flows = BoundaryEnergyFlows::default();
    let mut crossing_event_count = 0;

    for event in ordered {
        // Constructors enforce these invariants. Keep aggregation defensive
        // so future deserialization/import paths cannot bypass them.
        if event.source == event.destination {
            return Err(EnergyPortError::SelfTransfer {
                event_id: event.event_id,
            });
        }
        if !event.amount.get().is_finite() {
            return Err(EnergyPortError::NonFiniteAmount {
                event_id: event.event_id,
            });
        }
        if event.amount.get() < 0.0 {
            return Err(EnergyPortError::NegativeAmount {
                event_id: event.event_id,
            });
        }

        let source_inside = boundary.contains(event.source);
        let destination_inside = boundary.contains(event.destination);

        // Both endpoints are on the same side of the cut: this is not a
        // transfer across the selected system boundary.
        if source_inside == destination_inside {
            continue;
        }

        crossing_event_count += 1;
        let amount = event.amount.get();
        let incoming = !source_inside && destination_inside;

        let target = match (event.channel, incoming) {
            (EnergyPortChannel::Work, true) => &mut flows.work_in_j,
            (EnergyPortChannel::Work, false) => &mut flows.work_out_j,
            (EnergyPortChannel::Heat, true) => &mut flows.heat_in_j,
            (EnergyPortChannel::Heat, false) => &mut flows.heat_out_j,
            (EnergyPortChannel::Matter, true) => &mut flows.matter_energy_in_j,
            (EnergyPortChannel::Matter, false) => &mut flows.matter_energy_out_j,
        };

        let total = target.get() + amount;
        if !total.is_finite() {
            return Err(EnergyPortError::ArithmeticOverflow {
                channel: event.channel,
            });
        }
        *target = Joules::new(total);
    }

    Ok(EnergyPortSummary {
        boundary_id: boundary.id,
        boundary_coverage: boundary.coverage,
        event_count: events.len(),
        crossing_event_count,
        boundary_flows: flows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::energy_balance::{EnergyBalanceStatus, EnergyBalanceTolerances};

    fn event(
        id: u64,
        channel: EnergyPortChannel,
        source: u64,
        destination: u64,
        amount: f64,
    ) -> EnergyPortEvent {
        EnergyPortEvent::new(
            id,
            channel,
            ReservoirId(source),
            ReservoirId(destination),
            Joules::new(amount),
        )
        .unwrap()
    }

    fn boundary(
        id: u64,
        reservoirs: &[u64],
        coverage: BoundaryCoverage,
    ) -> EnergyPortBoundary {
        EnergyPortBoundary::new(
            BoundaryId(id),
            reservoirs.iter().copied().map(ReservoirId),
            coverage,
        )
        .unwrap()
    }

    fn tolerances() -> EnergyBalanceTolerances {
        EnergyBalanceTolerances::new(Joules::new(1e-9), 1e-9)
    }

    #[test]
    fn internal_heating_is_not_outer_boundary_heat_outflow() {
        // Mechanical energy becomes thermal energy, but both reservoirs are
        // inside this world boundary: this is an internal conversion.
        let world = boundary(1, &[10, 11], BoundaryCoverage::Complete);
        let transfer = event(1, EnergyPortChannel::Heat, 10, 11, 4.0);
        let summary = aggregate_boundary_ports(&world, &[transfer.clone()]).unwrap();

        assert_eq!(summary.boundary_flows.heat_out_j, Joules::new(0.0));
        assert_eq!(summary.crossing_event_count, 0);
        assert_eq!(summary.boundary_coverage, BoundaryCoverage::Complete);

        // For the smaller boundary containing only the source reservoir, the
        // same transfer is heat leaving that subsystem.
        let mechanical_subsystem = boundary(2, &[10], BoundaryCoverage::Complete);
        let summary = aggregate_boundary_ports(&mechanical_subsystem, &[transfer]).unwrap();
        assert_eq!(summary.boundary_flows.heat_out_j, Joules::new(4.0));
        assert_eq!(summary.crossing_event_count, 1);
    }

    #[test]
    fn work_heat_and_matter_are_separate_and_directional() {
        let world = boundary(7, &[2, 3], BoundaryCoverage::Complete);
        let events = [
            event(1, EnergyPortChannel::Work, 1, 2, 10.0),
            event(2, EnergyPortChannel::Heat, 3, 4, 2.0),
            event(3, EnergyPortChannel::Matter, 5, 2, 7.0),
            event(4, EnergyPortChannel::Work, 3, 6, 1.0),
        ];
        let report = aggregate_boundary_ports(&world, &events).unwrap();

        assert_eq!(report.boundary_flows.work_in_j, Joules::new(10.0));
        assert_eq!(report.boundary_flows.work_out_j, Joules::new(1.0));
        assert_eq!(report.boundary_flows.heat_out_j, Joules::new(2.0));
        assert_eq!(report.boundary_flows.matter_energy_in_j, Joules::new(7.0));
        assert_eq!(report.crossing_event_count, 4);
    }

    #[test]
    fn event_input_order_does_not_change_aggregated_floats() {
        let world = boundary(1, &[2], BoundaryCoverage::Complete);
        let a = event(8, EnergyPortChannel::Work, 1, 2, 0.1);
        let b = event(2, EnergyPortChannel::Work, 1, 2, 0.2);
        let c = event(5, EnergyPortChannel::Work, 1, 2, 0.3);

        let forward =
            aggregate_boundary_ports(&world, &[a.clone(), b.clone(), c.clone()]).unwrap();
        let reverse = aggregate_boundary_ports(&world, &[c, b, a]).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(forward.boundary_flows.work_in_j, Joules::new(0.6));
    }

    #[test]
    fn duplicate_event_ids_are_rejected_not_double_counted() {
        let world = boundary(1, &[2], BoundaryCoverage::Complete);
        let first = event(4, EnergyPortChannel::Work, 1, 2, 1.0);
        let duplicate = event(4, EnergyPortChannel::Work, 1, 2, 1.0);

        assert_eq!(
            aggregate_boundary_ports(&world, &[first, duplicate]),
            Err(EnergyPortError::DuplicateEventId { event_id: 4 })
        );
    }

    #[test]
    fn incomplete_coverage_is_preserved_by_the_first_law_bridge() {
        let world = boundary(1, &[2], BoundaryCoverage::Incomplete);
        let summary = aggregate_boundary_ports(
            &world,
            &[event(1, EnergyPortChannel::Work, 1, 2, 5.0)],
        )
        .unwrap();
        let balance = summary
            .to_balance_input(Joules::new(0.0), Joules::new(5.0))
            .evaluate(tolerances())
            .unwrap();

        assert_eq!(summary.boundary_flows.work_in_j, Joules::new(5.0));
        assert_eq!(balance.status, EnergyBalanceStatus::Inconclusive);
    }

    #[test]
    fn invalid_magnitudes_self_transfers_and_empty_boundaries_fail_closed() {
        assert_eq!(
            EnergyPortEvent::new(
                1,
                EnergyPortChannel::Work,
                ReservoirId(1),
                ReservoirId(2),
                Joules::new(-1.0),
            ),
            Err(EnergyPortError::NegativeAmount { event_id: 1 })
        );
        assert_eq!(
            EnergyPortEvent::new(
                2,
                EnergyPortChannel::Heat,
                ReservoirId(1),
                ReservoirId(2),
                Joules::new(f64::NAN),
            ),
            Err(EnergyPortError::NonFiniteAmount { event_id: 2 })
        );
        assert_eq!(
            EnergyPortEvent::new(
                3,
                EnergyPortChannel::Matter,
                ReservoirId(1),
                ReservoirId(1),
                Joules::new(1.0),
            ),
            Err(EnergyPortError::SelfTransfer { event_id: 3 })
        );
        assert_eq!(
            EnergyPortBoundary::new(
                BoundaryId(1),
                std::iter::empty(),
                BoundaryCoverage::Complete
            ),
            Err(EnergyPortError::EmptyBoundary {
                boundary_id: BoundaryId(1)
            })
        );
    }

    #[test]
    fn aggregation_overflow_is_rejected() {
        let world = boundary(1, &[2], BoundaryCoverage::Complete);
        let events = [
            event(1, EnergyPortChannel::Heat, 1, 2, f64::MAX),
            event(2, EnergyPortChannel::Heat, 1, 2, f64::MAX),
        ];
        assert_eq!(
            aggregate_boundary_ports(&world, &events),
            Err(EnergyPortError::ArithmeticOverflow {
                channel: EnergyPortChannel::Heat
            })
        );
    }
}
