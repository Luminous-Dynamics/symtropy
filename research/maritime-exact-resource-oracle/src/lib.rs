//! Exhaustive micro-instance oracle for time-indexed shared maritime resources.
//!
//! The oracle selects exactly one candidate action per vessel, sums its
//! integer reservations by (resource, time slot), rejects capacity violations,
//! and reports the full feasible set plus the Pareto frontier.

use std::collections::{BTreeMap, BTreeSet};

type ResourceKey = (String, u32);
type CapacityIndex = BTreeMap<ResourceKey, u32>;
type CandidateIndex<'a> = BTreeMap<String, Vec<&'a CandidateAction>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceCapacity {
    pub resource_id: String,
    pub slot: u32,
    pub capacity: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reservation {
    pub resource_id: String,
    pub slot: u32,
    pub units: u32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ObjectiveVector {
    pub vessel_wait: u64,
    pub receiver_delay: u64,
    pub energy_cost: u64,
    pub emissions: u64,
    pub missed_connections: u64,
}

impl ObjectiveVector {
    fn checked_add(self, other: Self) -> Result<Self, OracleError> {
        Ok(Self {
            vessel_wait: self
                .vessel_wait
                .checked_add(other.vessel_wait)
                .ok_or(OracleError::ObjectiveOverflow)?,
            receiver_delay: self
                .receiver_delay
                .checked_add(other.receiver_delay)
                .ok_or(OracleError::ObjectiveOverflow)?,
            energy_cost: self
                .energy_cost
                .checked_add(other.energy_cost)
                .ok_or(OracleError::ObjectiveOverflow)?,
            emissions: self
                .emissions
                .checked_add(other.emissions)
                .ok_or(OracleError::ObjectiveOverflow)?,
            missed_connections: self
                .missed_connections
                .checked_add(other.missed_connections)
                .ok_or(OracleError::ObjectiveOverflow)?,
        })
    }

    /// Strict Pareto dominance over all declared objective axes.
    pub fn dominates(self, other: Self) -> bool {
        let no_worse = self.vessel_wait <= other.vessel_wait
            && self.receiver_delay <= other.receiver_delay
            && self.energy_cost <= other.energy_cost
            && self.emissions <= other.emissions
            && self.missed_connections <= other.missed_connections;
        let strictly_better = self.vessel_wait < other.vessel_wait
            || self.receiver_delay < other.receiver_delay
            || self.energy_cost < other.energy_cost
            || self.emissions < other.emissions
            || self.missed_connections < other.missed_connections;
        no_worse && strictly_better
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateAction {
    pub candidate_id: String,
    pub vessel_id: String,
    pub reservations: Vec<Reservation>,
    pub objective: ObjectiveVector,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scenario {
    pub vessel_ids: Vec<String>,
    pub capacities: Vec<ResourceCapacity>,
    pub candidates: Vec<CandidateAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plan {
    /// Candidate IDs are ordered by canonical vessel ID.
    pub candidate_ids: Vec<String>,
    pub objective: ObjectiveVector,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult {
    pub feasible_plans: Vec<Plan>,
    pub pareto_front: Vec<Plan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleError {
    EmptyScenario,
    EmptyIdentifier,
    DuplicateVessel(String),
    DuplicateCapacity {
        resource_id: String,
        slot: u32,
    },
    DuplicateCandidate(String),
    UnknownVessel {
        candidate_id: String,
        vessel_id: String,
    },
    NoCandidatesForVessel(String),
    UnknownResourceSlot {
        resource_id: String,
        slot: u32,
    },
    ReservationOverflow,
    ObjectiveOverflow,
}

impl std::fmt::Display for OracleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for OracleError {}

fn reservation_totals(
    reservations: &[Reservation],
) -> Result<BTreeMap<ResourceKey, u32>, OracleError> {
    let mut totals = BTreeMap::new();
    for reservation in reservations {
        if reservation.resource_id.is_empty() {
            return Err(OracleError::EmptyIdentifier);
        }
        let key = (reservation.resource_id.clone(), reservation.slot);
        let current = totals.entry(key).or_insert(0_u32);
        *current = current
            .checked_add(reservation.units)
            .ok_or(OracleError::ReservationOverflow)?;
    }
    Ok(totals)
}

fn validate_and_index<'a>(
    scenario: &'a Scenario,
) -> Result<(Vec<String>, CapacityIndex, CandidateIndex<'a>), OracleError> {
    if scenario.vessel_ids.is_empty() || scenario.candidates.is_empty() {
        return Err(OracleError::EmptyScenario);
    }

    let mut vessel_set = BTreeSet::new();
    for vessel_id in &scenario.vessel_ids {
        if vessel_id.is_empty() {
            return Err(OracleError::EmptyIdentifier);
        }
        if !vessel_set.insert(vessel_id.clone()) {
            return Err(OracleError::DuplicateVessel(vessel_id.clone()));
        }
    }
    let vessel_ids: Vec<String> = vessel_set.iter().cloned().collect();

    let mut capacities: CapacityIndex = BTreeMap::new();
    for capacity in &scenario.capacities {
        if capacity.resource_id.is_empty() {
            return Err(OracleError::EmptyIdentifier);
        }
        let key = (capacity.resource_id.clone(), capacity.slot);
        if capacities.insert(key.clone(), capacity.capacity).is_some() {
            return Err(OracleError::DuplicateCapacity {
                resource_id: key.0,
                slot: key.1,
            });
        }
    }

    let mut candidate_ids = BTreeSet::new();
    let mut candidates_by_vessel: CandidateIndex<'a> = BTreeMap::new();
    for candidate in &scenario.candidates {
        if candidate.candidate_id.is_empty() || candidate.vessel_id.is_empty() {
            return Err(OracleError::EmptyIdentifier);
        }
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(OracleError::DuplicateCandidate(
                candidate.candidate_id.clone(),
            ));
        }
        if !vessel_set.contains(&candidate.vessel_id) {
            return Err(OracleError::UnknownVessel {
                candidate_id: candidate.candidate_id.clone(),
                vessel_id: candidate.vessel_id.clone(),
            });
        }

        let totals = reservation_totals(&candidate.reservations)?;
        for (resource_key, _) in totals {
            if !capacities.contains_key(&resource_key) {
                return Err(OracleError::UnknownResourceSlot {
                    resource_id: resource_key.0,
                    slot: resource_key.1,
                });
            }
        }
        candidates_by_vessel
            .entry(candidate.vessel_id.clone())
            .or_default()
            .push(candidate);
    }

    for vessel_id in &vessel_ids {
        let options = candidates_by_vessel
            .get_mut(vessel_id)
            .ok_or_else(|| OracleError::NoCandidatesForVessel(vessel_id.clone()))?;
        options.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    }

    Ok((vessel_ids, capacities, candidates_by_vessel))
}

fn can_apply(
    totals: &BTreeMap<ResourceKey, u32>,
    capacities: &CapacityIndex,
    used: &BTreeMap<ResourceKey, u32>,
) -> Result<bool, OracleError> {
    for (key, units) in totals {
        let capacity = capacities
            .get(key)
            .ok_or_else(|| OracleError::UnknownResourceSlot {
                resource_id: key.0.clone(),
                slot: key.1,
            })?;
        let current = used.get(key).copied().unwrap_or(0);
        let proposed = current
            .checked_add(*units)
            .ok_or(OracleError::ReservationOverflow)?;
        if proposed > *capacity {
            return Ok(false);
        }
    }
    Ok(true)
}

fn apply(totals: &BTreeMap<ResourceKey, u32>, used: &mut BTreeMap<ResourceKey, u32>) {
    for (key, units) in totals {
        *used.entry(key.clone()).or_default() += *units;
    }
}

fn undo(totals: &BTreeMap<ResourceKey, u32>, used: &mut BTreeMap<ResourceKey, u32>) {
    for (key, units) in totals {
        let remove = {
            let current = used.get_mut(key).expect("applied reservation must exist");
            *current -= *units;
            *current == 0
        };
        if remove {
            used.remove(key);
        }
    }
}

fn enumerate_recursive<'a>(
    vessel_index: usize,
    vessel_ids: &[String],
    capacities: &CapacityIndex,
    candidates_by_vessel: &CandidateIndex<'a>,
    used: &mut BTreeMap<ResourceKey, u32>,
    chosen: &mut Vec<&'a CandidateAction>,
    output: &mut Vec<Plan>,
) -> Result<(), OracleError> {
    if vessel_index == vessel_ids.len() {
        let mut objective = ObjectiveVector::default();
        let mut candidate_ids = Vec::with_capacity(chosen.len());
        for candidate in chosen.iter() {
            objective = objective.checked_add(candidate.objective)?;
            candidate_ids.push(candidate.candidate_id.clone());
        }
        output.push(Plan {
            candidate_ids,
            objective,
        });
        return Ok(());
    }

    let vessel_id = &vessel_ids[vessel_index];
    let options = candidates_by_vessel
        .get(vessel_id)
        .ok_or_else(|| OracleError::NoCandidatesForVessel(vessel_id.clone()))?;

    for candidate in options {
        let totals = reservation_totals(&candidate.reservations)?;
        if !can_apply(&totals, capacities, used)? {
            continue;
        }
        apply(&totals, used);
        chosen.push(candidate);
        enumerate_recursive(
            vessel_index + 1,
            vessel_ids,
            capacities,
            candidates_by_vessel,
            used,
            chosen,
            output,
        )?;
        chosen.pop();
        undo(&totals, used);
    }
    Ok(())
}

fn pareto_front(feasible_plans: &[Plan]) -> Vec<Plan> {
    let mut front: Vec<Plan> = feasible_plans
        .iter()
        .enumerate()
        .filter(|(index, candidate)| {
            !feasible_plans
                .iter()
                .enumerate()
                .any(|(other_index, other)| {
                    index != &other_index && other.objective.dominates(candidate.objective)
                })
        })
        .map(|(_, plan)| plan.clone())
        .collect();
    front.sort_by(|left, right| left.candidate_ids.cmp(&right.candidate_ids));
    front
}

/// Exhaustively selects exactly one candidate action per vessel.
///
/// Time-varying resources are represented as explicit resource/slot capacities.
/// A capacity of zero models a closed tide window or unavailable access
/// corridor. Every reservation is conjunctive; no objective penalty can make
/// an over-capacity plan feasible.
pub fn enumerate(scenario: &Scenario) -> Result<SearchResult, OracleError> {
    let (vessel_ids, capacities, candidates_by_vessel) = validate_and_index(scenario)?;
    let mut feasible_plans = Vec::new();
    enumerate_recursive(
        0,
        &vessel_ids,
        &capacities,
        &candidates_by_vessel,
        &mut BTreeMap::new(),
        &mut Vec::new(),
        &mut feasible_plans,
    )?;
    feasible_plans.sort_by(|left, right| left.candidate_ids.cmp(&right.candidate_ids));
    let pareto_front = pareto_front(&feasible_plans);
    Ok(SearchResult {
        feasible_plans,
        pareto_front,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(
        candidate_id: &str,
        vessel_id: &str,
        resource_id: &str,
        slot: u32,
        units: u32,
        objective: ObjectiveVector,
    ) -> CandidateAction {
        CandidateAction {
            candidate_id: candidate_id.to_owned(),
            vessel_id: vessel_id.to_owned(),
            reservations: vec![Reservation {
                resource_id: resource_id.to_owned(),
                slot,
                units,
            }],
            objective,
        }
    }

    fn base_scenario() -> Scenario {
        Scenario {
            vessel_ids: vec!["V2".to_owned(), "V1".to_owned()],
            capacities: vec![ResourceCapacity {
                resource_id: "BRIDGE".to_owned(),
                slot: 0,
                capacity: 1,
            }],
            candidates: vec![
                action(
                    "V1_A",
                    "V1",
                    "BRIDGE",
                    0,
                    1,
                    ObjectiveVector {
                        vessel_wait: 1,
                        receiver_delay: 2,
                        energy_cost: 4,
                        emissions: 2,
                        missed_connections: 0,
                    },
                ),
                action(
                    "V1_B",
                    "V1",
                    "BRIDGE",
                    0,
                    1,
                    ObjectiveVector {
                        vessel_wait: 2,
                        receiver_delay: 3,
                        energy_cost: 0,
                        emissions: 3,
                        missed_connections: 0,
                    },
                ),
                action(
                    "V2_A",
                    "V2",
                    "BRIDGE",
                    0,
                    1,
                    ObjectiveVector {
                        vessel_wait: 0,
                        receiver_delay: 1,
                        energy_cost: 5,
                        emissions: 1,
                        missed_connections: 0,
                    },
                ),
                action(
                    "V2_B",
                    "V2",
                    "BRIDGE",
                    0,
                    1,
                    ObjectiveVector {
                        vessel_wait: 3,
                        receiver_delay: 4,
                        energy_cost: 1,
                        emissions: 4,
                        missed_connections: 1,
                    },
                ),
            ],
        }
    }

    #[test]
    fn jointly_overbooked_candidates_are_not_admitted() {
        let result = enumerate(&base_scenario()).unwrap();
        assert!(result.feasible_plans.is_empty());
    }

    #[test]
    fn zero_capacity_models_a_closed_resource_window() {
        let mut scenario = base_scenario();
        scenario.capacities[0].capacity = 0;
        assert!(enumerate(&scenario).unwrap().feasible_plans.is_empty());
    }

    #[test]
    fn candidate_order_does_not_change_canonical_receipt() {
        let mut scenario = base_scenario();
        scenario.capacities[0].capacity = 2;
        let first = enumerate(&scenario).unwrap();

        scenario.candidates.reverse();
        scenario.vessel_ids.reverse();
        let second = enumerate(&scenario).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_candidate_identity_is_rejected() {
        let mut scenario = base_scenario();
        scenario.candidates.push(scenario.candidates[0].clone());
        assert!(matches!(
            enumerate(&scenario),
            Err(OracleError::DuplicateCandidate(_))
        ));
    }

    #[test]
    fn unknown_resource_time_slot_is_rejected() {
        let mut scenario = base_scenario();
        scenario.candidates[0].reservations[0].resource_id = "MISSING".to_owned();
        assert!(matches!(
            enumerate(&scenario),
            Err(OracleError::UnknownResourceSlot { .. })
        ));
    }

    #[test]
    fn exact_frontier_retains_incomparable_objectives() {
        let mut scenario = base_scenario();
        scenario.capacities[0].capacity = 2;
        let result = enumerate(&scenario).unwrap();
        assert_eq!(result.feasible_plans.len(), 4);
        assert_eq!(result.pareto_front.len(), 3);
    }

    #[test]
    fn sea_rail_transfer_tracks_are_shared_time_indexed_resources() {
        let candidate = |candidate_id: &str,
                         vessel_id: &str,
                         rail_track: &str,
                         objective: ObjectiveVector| CandidateAction {
            candidate_id: candidate_id.to_owned(),
            vessel_id: vessel_id.to_owned(),
            reservations: vec![
                Reservation {
                    resource_id: rail_track.to_owned(),
                    slot: 3,
                    units: 1,
                },
                Reservation {
                    resource_id: "RMGC-POOL".to_owned(),
                    slot: 3,
                    units: 1,
                },
                Reservation {
                    resource_id: "RECEIVER-RAIL-GATE".to_owned(),
                    slot: 4,
                    units: 1,
                },
            ],
            objective,
        };

        let scenario = Scenario {
            vessel_ids: vec!["V1".to_owned(), "V2".to_owned()],
            capacities: vec![
                ResourceCapacity {
                    resource_id: "RAIL-TRACK-A".to_owned(),
                    slot: 3,
                    capacity: 1,
                },
                ResourceCapacity {
                    resource_id: "RAIL-TRACK-B".to_owned(),
                    slot: 3,
                    capacity: 1,
                },
                ResourceCapacity {
                    resource_id: "RMGC-POOL".to_owned(),
                    slot: 3,
                    capacity: 2,
                },
                ResourceCapacity {
                    resource_id: "RECEIVER-RAIL-GATE".to_owned(),
                    slot: 4,
                    capacity: 2,
                },
            ],
            candidates: vec![
                candidate(
                    "V1_TRACK_A",
                    "V1",
                    "RAIL-TRACK-A",
                    ObjectiveVector {
                        vessel_wait: 1,
                        receiver_delay: 2,
                        energy_cost: 4,
                        emissions: 2,
                        missed_connections: 0,
                    },
                ),
                candidate(
                    "V1_TRACK_B",
                    "V1",
                    "RAIL-TRACK-B",
                    ObjectiveVector {
                        vessel_wait: 2,
                        receiver_delay: 3,
                        energy_cost: 2,
                        emissions: 3,
                        missed_connections: 0,
                    },
                ),
                candidate(
                    "V2_TRACK_A",
                    "V2",
                    "RAIL-TRACK-A",
                    ObjectiveVector {
                        vessel_wait: 1,
                        receiver_delay: 1,
                        energy_cost: 5,
                        emissions: 1,
                        missed_connections: 0,
                    },
                ),
                candidate(
                    "V2_TRACK_B",
                    "V2",
                    "RAIL-TRACK-B",
                    ObjectiveVector {
                        vessel_wait: 2,
                        receiver_delay: 2,
                        energy_cost: 1,
                        emissions: 4,
                        missed_connections: 0,
                    },
                ),
            ],
        };

        let result = enumerate(&scenario).unwrap();
        assert_eq!(result.feasible_plans.len(), 2);
        assert_eq!(result.pareto_front.len(), 2);
        assert_eq!(
            result
                .feasible_plans
                .iter()
                .map(|plan| plan.candidate_ids.clone())
                .collect::<Vec<_>>(),
            vec![
                vec!["V1_TRACK_A".to_owned(), "V2_TRACK_B".to_owned()],
                vec!["V1_TRACK_B".to_owned(), "V2_TRACK_A".to_owned()],
            ]
        );
    }


    #[test]
    fn tidal_mooring_orientation_and_approach_bridge_are_joint_resources() {
        let candidate = |candidate_id: &str,
                         vessel_id: &str,
                         berth_segment: &str,
                         bridge: &str,
                         tide_slot: u32,
                         wait: u64| CandidateAction {
            candidate_id: candidate_id.to_owned(),
            vessel_id: vessel_id.to_owned(),
            reservations: vec![
                Reservation {
                    resource_id: berth_segment.to_owned(),
                    slot: 1,
                    units: 1,
                },
                Reservation {
                    resource_id: "TIDE-WINDOW".to_owned(),
                    slot: tide_slot,
                    units: 1,
                },
                Reservation {
                    resource_id: bridge.to_owned(),
                    slot: 2,
                    units: 1,
                },
            ],
            objective: ObjectiveVector {
                vessel_wait: wait,
                receiver_delay: 1,
                energy_cost: 2,
                emissions: 1,
                missed_connections: 0,
            },
        };

        let scenario = Scenario {
            vessel_ids: vec!["V1".to_owned(), "V2".to_owned()],
            capacities: vec![
                ResourceCapacity {
                    resource_id: "BERTH-NORTH".to_owned(),
                    slot: 1,
                    capacity: 1,
                },
                ResourceCapacity {
                    resource_id: "BERTH-SOUTH".to_owned(),
                    slot: 1,
                    capacity: 1,
                },
                ResourceCapacity {
                    resource_id: "TIDE-WINDOW".to_owned(),
                    slot: 0,
                    capacity: 0,
                },
                ResourceCapacity {
                    resource_id: "TIDE-WINDOW".to_owned(),
                    slot: 1,
                    capacity: 2,
                },
                ResourceCapacity {
                    resource_id: "BRIDGE-EAST".to_owned(),
                    slot: 2,
                    capacity: 1,
                },
                ResourceCapacity {
                    resource_id: "BRIDGE-WEST".to_owned(),
                    slot: 2,
                    capacity: 1,
                },
            ],
            candidates: vec![
                candidate("V1_NORTH", "V1", "BERTH-NORTH", "BRIDGE-EAST", 1, 0),
                candidate("V1_SOUTH", "V1", "BERTH-SOUTH", "BRIDGE-WEST", 1, 1),
                candidate("V1_TIDE_CLOSED", "V1", "BERTH-NORTH", "BRIDGE-EAST", 0, 0),
                candidate("V2_NORTH", "V2", "BERTH-NORTH", "BRIDGE-EAST", 1, 0),
                candidate("V2_SOUTH", "V2", "BERTH-SOUTH", "BRIDGE-WEST", 1, 1),
            ],
        };

        let result = enumerate(&scenario).unwrap();
        assert_eq!(result.feasible_plans.len(), 2);
        assert_eq!(
            result
                .feasible_plans
                .iter()
                .map(|plan| plan.candidate_ids.clone())
                .collect::<Vec<_>>(),
            vec![
                vec!["V1_NORTH".to_owned(), "V2_SOUTH".to_owned()],
                vec!["V1_SOUTH".to_owned(), "V2_NORTH".to_owned()],
            ]
        );
        assert!(result
            .feasible_plans
            .iter()
            .all(|plan| !plan.candidate_ids.contains(&"V1_TIDE_CLOSED".to_owned())));
    }

    #[test]
    fn duplicate_vessel_selection_cannot_be_constructed_by_enumerator() {
        let mut scenario = base_scenario();
        scenario.candidates[0].vessel_id = "V3".to_owned();
        assert!(matches!(
            enumerate(&scenario),
            Err(OracleError::UnknownVessel { .. })
        ));
    }

    #[test]
    fn reservation_sum_overflow_is_rejected() {
        let mut scenario = base_scenario();
        scenario.candidates[0].reservations = vec![
            Reservation {
                resource_id: "BRIDGE".to_owned(),
                slot: 0,
                units: u32::MAX,
            },
            Reservation {
                resource_id: "BRIDGE".to_owned(),
                slot: 0,
                units: 1,
            },
        ];
        assert_eq!(enumerate(&scenario), Err(OracleError::ReservationOverflow));
    }

    #[test]
    fn objective_sum_overflow_is_rejected() {
        let mut scenario = base_scenario();
        scenario.capacities[0].capacity = 2;
        scenario.candidates[0].objective.vessel_wait = u64::MAX;
        scenario.candidates[2].objective.vessel_wait = 1;
        assert_eq!(enumerate(&scenario), Err(OracleError::ObjectiveOverflow));
    }

    #[test]
    fn duplicate_capacity_entries_are_rejected() {
        let mut scenario = base_scenario();
        scenario.capacities.push(scenario.capacities[0].clone());
        assert!(matches!(
            enumerate(&scenario),
            Err(OracleError::DuplicateCapacity { .. })
        ));
    }
}
