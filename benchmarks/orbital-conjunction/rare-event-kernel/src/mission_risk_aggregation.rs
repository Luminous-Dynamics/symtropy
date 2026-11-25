// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite-support aggregation of dependent mission encounter events.
//!
//! Every trajectory atom carries one shared outcome signature across the
//! complete declared event set. Union risk is computed from that joint
//! signature, not from an independence assumption or a sum of marginal risks.
//! This module does not propagate trajectories, establish event completeness,
//! estimate a continuous-domain collision probability, or authorize maneuvers.

use std::collections::BTreeMap;

const MAX_EVENTS: usize = 32;
const MAX_TRAJECTORY_ATOMS: usize = 65_536;

/// A definite or unresolved event outcome for one shared trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterEventDisposition {
    /// The trajectory realization intersects this event's collision set.
    DefinitelyContact,
    /// The trajectory realization is proven outside this event's collision set.
    DefinitelyNoContact,
    /// Available evidence cannot classify this event for the realization.
    Ambiguous,
}

/// One event classification in a trajectory's joint event signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncounterEventOutcome {
    /// Stable event identifier from the frozen mission event inventory.
    pub event_id: String,
    /// Classification for this event on the shared trajectory realization.
    pub disposition: EncounterEventDisposition,
}

/// One atom in a finite discrete target-measure support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteMissionTrajectory {
    /// Stable nonzero trajectory-atom identity.
    pub atom_id: [u8; 32],
    /// Integer target-measure mass assigned to this atom.
    pub mass_ticks: u64,
    /// Exactly one definite outcome for every event in the frozen inventory.
    pub event_outcomes: Vec<EncounterEventOutcome>,
}

/// Frozen identities and completeness declarations for finite mission risk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteMissionRiskManifest {
    /// Opaque source identity of this manifest; the caller supplies its digest.
    pub manifest_digest: [u8; 32],
    /// Opaque source identity of the declared nominal target measure.
    pub target_measure_digest: [u8; 32],
    /// Opaque source identity of the event inventory and its revision.
    pub event_inventory_digest: [u8; 32],
    /// Canonical strictly increasing event IDs.
    pub event_ids: Vec<String>,
    /// Canonical strictly increasing expected trajectory atom IDs.
    pub expected_atom_ids: Vec<[u8; 32]>,
    /// Exact sum of all trajectory-atom mass ticks.
    pub total_mass_ticks: u64,
    /// Declared mission-level risk-budget numerator.
    pub risk_budget_numerator: u64,
    /// Declared mission-level risk-budget denominator.
    pub risk_budget_denominator: u64,
    /// Caller declaration that the event inventory covers the mission horizon.
    pub event_inventory_complete: bool,
    /// Caller declaration that the finite trajectory support is complete.
    pub trajectory_support_complete: bool,
}

/// An exact rational mass over the declared finite target measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactMass {
    /// Reduced numerator.
    pub numerator: u64,
    /// Reduced, nonzero denominator.
    pub denominator: u64,
}

impl ExactMass {
    fn new(numerator: u64, denominator: u64) -> Self {
        let divisor = gcd(numerator, denominator);
        Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        }
    }
}

/// Exact marginal mass for one event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventMarginalMass {
    /// Event identifier.
    pub event_id: String,
    /// Marginal mass on the declared finite target measure.
    pub mass: ExactMass,
}

/// Exact intersection mass for one unordered event pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairwiseIntersectionMass {
    /// Lexicographically earlier event identifier.
    pub left_event_id: String,
    /// Lexicographically later event identifier.
    pub right_event_id: String,
    /// Joint mass that both events occur on the same trajectory atom.
    pub mass: ExactMass,
}

/// Exact mass of one complete joint event signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointEventSignatureMass {
    /// Canonically sorted events that occur on this signature.
    pub contacting_event_ids: Vec<String>,
    /// Total mass of trajectory atoms with exactly this signature.
    pub mass: ExactMass,
}

/// Sharp dependence-free bounds on the union derived from event marginals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrechetUnionBounds {
    /// Lower bound: the maximum marginal event mass.
    pub lower_bound: ExactMass,
    /// Upper bound: min(1, sum of the marginal event masses).
    pub upper_bound: ExactMass,
}

/// Comparison of the exact finite-support union against a declared risk budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRiskBudgetStatus {
    /// Exact finite-support union mass does not exceed the declared budget.
    WithinDeclaredFiniteSupportBudget,
    /// Exact finite-support union mass exceeds the declared budget.
    ExceedsDeclaredFiniteSupportBudget,
}

/// Exact dependence-aware finite mission-risk report.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionRiskEnvelope {
    /// Source identity of the finite support manifest.
    pub manifest_digest: [u8; 32],
    /// Source identity of the declared nominal target measure.
    pub target_measure_digest: [u8; 32],
    /// Source identity of the mission event inventory.
    pub event_inventory_digest: [u8; 32],
    /// Total integer mass of the declared finite target measure.
    pub total_mass_ticks: u64,
    /// Number of events in the complete declared inventory.
    pub event_count: usize,
    /// Number of trajectory atoms in the exact support.
    pub trajectory_atom_count: usize,
    /// Event marginals in canonical event-ID order.
    pub event_marginals: Vec<EventMarginalMass>,
    /// Pairwise intersections in canonical lexicographic order.
    pub pairwise_intersections: Vec<PairwiseIntersectionMass>,
    /// Complete joint event signatures in canonical order.
    pub joint_signatures: Vec<JointEventSignatureMass>,
    /// Exact probability mass of at least one event, computed pathwise.
    pub pathwise_union_mass: ExactMass,
    /// Exact probability mass of no event over the declared event inventory.
    pub no_event_mass: ExactMass,
    /// Sum of all marginal numerators before capping at total mass.
    pub marginal_sum_ticks: u128,
    /// Sharp Fréchet bounds using marginals only.
    pub frechet_union_bounds: FrechetUnionBounds,
    /// Independence-model union approximation; diagnostic only, never used
    /// for the exact union or budget decision.
    pub independence_union_approximation: f64,
    /// Declared mission-level probability budget.
    pub risk_budget: ExactMass,
    /// Comparison of exact finite-support union with the declared budget.
    pub budget_status: MissionRiskBudgetStatus,
    /// Always false: this mathematical kernel has no operational authority.
    pub authority_granted: bool,
}

/// Fail-closed validation errors for finite mission-risk aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRiskAggregationError {
    /// The manifest digest is zero.
    InvalidManifestDigest,
    /// The nominal target-measure digest is zero.
    InvalidTargetMeasureDigest,
    /// The event-inventory digest is zero.
    InvalidEventInventoryDigest,
    /// The caller did not declare the full mission event inventory complete.
    IncompleteEventInventory,
    /// The caller did not declare the finite trajectory support complete.
    IncompleteTrajectorySupport,
    /// The event inventory is empty or exceeds the supported bound.
    InvalidEventInventorySize,
    /// An event ID is blank, malformed, duplicated, or not canonical.
    InvalidEventIdentity,
    /// The trajectory inventory is empty or exceeds the supported bound.
    InvalidTrajectoryInventorySize,
    /// An expected atom ID is zero, duplicated, or not in canonical order.
    InvalidExpectedAtomInventory,
    /// An observed atom ID is zero or duplicated.
    InvalidTrajectoryAtomIdentity,
    /// The observed atom inventory differs from the frozen manifest.
    TrajectoryAtomInventoryMismatch,
    /// The risk budget has an invalid numerator or denominator.
    InvalidRiskBudget,
    /// The finite measure has zero or malformed mass.
    InvalidTrajectoryMass,
    /// The total observed mass differs from the manifest.
    TotalMassMismatch,
    /// An event outcome names an event outside the frozen inventory.
    UnknownEventOutcome,
    /// A trajectory repeats an event outcome.
    DuplicateEventOutcome,
    /// A trajectory omits a declared event outcome.
    MissingEventOutcome,
    /// An ambiguous outcome cannot be coerced to contact or no-contact.
    AmbiguousEventOutcome,
    /// Integer accumulation exceeded the supported exact range.
    MassOverflow,
    /// The observed union lies outside its mathematically required Fréchet bounds.
    InconsistentJointMass,
}

impl core::fmt::Display for MissionRiskAggregationError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidManifestDigest => "mission risk manifest digest is invalid",
            Self::InvalidTargetMeasureDigest => "target measure digest is invalid",
            Self::InvalidEventInventoryDigest => "event inventory digest is invalid",
            Self::IncompleteEventInventory => "mission event inventory is not declared complete",
            Self::IncompleteTrajectorySupport => "trajectory support is not declared complete",
            Self::InvalidEventInventorySize => "mission event inventory size is invalid",
            Self::InvalidEventIdentity => "mission event identity is invalid",
            Self::InvalidTrajectoryInventorySize => "trajectory atom inventory size is invalid",
            Self::InvalidExpectedAtomInventory => "expected trajectory atom inventory is invalid",
            Self::InvalidTrajectoryAtomIdentity => "trajectory atom identity is invalid",
            Self::TrajectoryAtomInventoryMismatch => "observed trajectory atoms differ from manifest",
            Self::InvalidRiskBudget => "mission risk budget is invalid",
            Self::InvalidTrajectoryMass => "trajectory target-measure mass is invalid",
            Self::TotalMassMismatch => "observed trajectory mass differs from manifest",
            Self::UnknownEventOutcome => "trajectory outcome names an unknown event",
            Self::DuplicateEventOutcome => "trajectory repeats an event outcome",
            Self::MissingEventOutcome => "trajectory omits a declared event outcome",
            Self::AmbiguousEventOutcome => "ambiguous event outcome cannot enter exact union mass",
            Self::MassOverflow => "mission risk mass accumulation overflowed",
            Self::InconsistentJointMass => "joint event mass violates Frechet bounds",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for MissionRiskAggregationError {}

/// Aggregate a complete finite mission trajectory support without an
/// independence assumption.
///
/// The manifest's event and trajectory inventories must already be canonical.
/// The function accepts atom and per-atom outcome rows in any input order,
/// canonicalizes them, and computes all marginals, pairwise intersections,
/// joint signatures and the pathwise union using checked integer arithmetic.
///
/// The three digest fields are opaque source pins. This kernel validates that
/// they are nonzero but does not recompute or authenticate their preimages.
/// Completeness booleans are caller declarations, not proofs that the real
/// orbit or event universe has been exhaustively discovered.
pub fn aggregate_finite_mission_risk(
    manifest: &FiniteMissionRiskManifest,
    trajectories: &[FiniteMissionTrajectory],
) -> Result<MissionRiskEnvelope, MissionRiskAggregationError> {
    if is_zero_digest(&manifest.manifest_digest) {
        return Err(MissionRiskAggregationError::InvalidManifestDigest);
    }
    if is_zero_digest(&manifest.target_measure_digest) {
        return Err(MissionRiskAggregationError::InvalidTargetMeasureDigest);
    }
    if is_zero_digest(&manifest.event_inventory_digest) {
        return Err(MissionRiskAggregationError::InvalidEventInventoryDigest);
    }
    if !manifest.event_inventory_complete {
        return Err(MissionRiskAggregationError::IncompleteEventInventory);
    }
    if !manifest.trajectory_support_complete {
        return Err(MissionRiskAggregationError::IncompleteTrajectorySupport);
    }
    if manifest.event_ids.is_empty() || manifest.event_ids.len() > MAX_EVENTS {
        return Err(MissionRiskAggregationError::InvalidEventInventorySize);
    }
    if manifest.event_ids.iter().any(|id| !valid_event_id(id))
        || manifest
            .event_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(MissionRiskAggregationError::InvalidEventIdentity);
    }
    if trajectories.is_empty() || trajectories.len() > MAX_TRAJECTORY_ATOMS {
        return Err(MissionRiskAggregationError::InvalidTrajectoryInventorySize);
    }
    if manifest.expected_atom_ids.len() != trajectories.len()
        || manifest.expected_atom_ids.is_empty()
        || manifest.expected_atom_ids.iter().any(is_zero_digest)
        || manifest
            .expected_atom_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(MissionRiskAggregationError::InvalidExpectedAtomInventory);
    }
    if manifest.total_mass_ticks == 0
        || manifest.risk_budget_denominator == 0
        || manifest.risk_budget_numerator > manifest.risk_budget_denominator
    {
        return Err(MissionRiskAggregationError::InvalidRiskBudget);
    }

    let mut by_atom_id = BTreeMap::new();
    for trajectory in trajectories {
        if is_zero_digest(&trajectory.atom_id) {
            return Err(MissionRiskAggregationError::InvalidTrajectoryAtomIdentity);
        }
        if trajectory.mass_ticks == 0 {
            return Err(MissionRiskAggregationError::InvalidTrajectoryMass);
        }
        if by_atom_id
            .insert(trajectory.atom_id, trajectory)
            .is_some()
        {
            return Err(MissionRiskAggregationError::InvalidTrajectoryAtomIdentity);
        }
    }
    if by_atom_id.len() != manifest.expected_atom_ids.len()
        || manifest
            .expected_atom_ids
            .iter()
            .any(|atom_id| !by_atom_id.contains_key(atom_id))
    {
        return Err(MissionRiskAggregationError::TrajectoryAtomInventoryMismatch);
    }

    let event_count = manifest.event_ids.len();
    let mut event_mass = vec![0_u64; event_count];
    let mut pairwise_mass = vec![0_u64; event_count * event_count];
    let mut signature_mass = BTreeMap::<Vec<String>, u64>::new();
    let mut observed_mass = 0_u64;
    let mut union_mass = 0_u64;
    let mut no_event_mass = 0_u64;
    let mut independence_log_survival = 0.0_f64;

    for atom in by_atom_id.values() {
        observed_mass = observed_mass
            .checked_add(atom.mass_ticks)
            .ok_or(MissionRiskAggregationError::MassOverflow)?;
        let mut outcome_by_event = vec![None; event_count];
        for outcome in &atom.event_outcomes {
            let index = manifest
                .event_ids
                .binary_search(&outcome.event_id)
                .map_err(|_| MissionRiskAggregationError::UnknownEventOutcome)?;
            if outcome_by_event[index].is_some() {
                return Err(MissionRiskAggregationError::DuplicateEventOutcome);
            }
            outcome_by_event[index] = Some(outcome.disposition);
        }

        let mut contacting_indices = Vec::new();
        for (index, disposition) in outcome_by_event.iter().enumerate() {
            match disposition {
                Some(EncounterEventDisposition::DefinitelyContact) => {
                    contacting_indices.push(index);
                    event_mass[index] = event_mass[index]
                        .checked_add(atom.mass_ticks)
                        .ok_or(MissionRiskAggregationError::MassOverflow)?;
                }
                Some(EncounterEventDisposition::DefinitelyNoContact) => {}
                Some(EncounterEventDisposition::Ambiguous) => {
                    return Err(MissionRiskAggregationError::AmbiguousEventOutcome);
                }
                None => return Err(MissionRiskAggregationError::MissingEventOutcome),
            }
        }

        let signature = contacting_indices
            .iter()
            .map(|index| manifest.event_ids[*index].clone())
            .collect::<Vec<_>>();
        let signature_entry = signature_mass.entry(signature).or_insert(0);
        *signature_entry = signature_entry
            .checked_add(atom.mass_ticks)
            .ok_or(MissionRiskAggregationError::MassOverflow)?;

        if contacting_indices.is_empty() {
            no_event_mass = no_event_mass
                .checked_add(atom.mass_ticks)
                .ok_or(MissionRiskAggregationError::MassOverflow)?;
        } else {
            union_mass = union_mass
                .checked_add(atom.mass_ticks)
                .ok_or(MissionRiskAggregationError::MassOverflow)?;
        }

        for left in 0..contacting_indices.len() {
            for right in (left + 1)..contacting_indices.len() {
                let i = contacting_indices[left];
                let j = contacting_indices[right];
                let index = i * event_count + j;
                pairwise_mass[index] = pairwise_mass[index]
                    .checked_add(atom.mass_ticks)
                    .ok_or(MissionRiskAggregationError::MassOverflow)?;
            }
        }
    }

    if observed_mass != manifest.total_mass_ticks
        || union_mass.checked_add(no_event_mass) != Some(observed_mass)
    {
        return Err(MissionRiskAggregationError::TotalMassMismatch);
    }

    let marginal_sum_ticks = event_mass
        .iter()
        .map(|mass| u128::from(*mass))
        .sum::<u128>();
    let frechet_lower = *event_mass.iter().max().unwrap_or(&0);
    let frechet_upper = marginal_sum_ticks.min(u128::from(manifest.total_mass_ticks)) as u64;
    if union_mass < frechet_lower || union_mass > frechet_upper {
        return Err(MissionRiskAggregationError::InconsistentJointMass);
    }

    let mut event_marginals = Vec::with_capacity(event_count);
    let mut independence_log_survival = 0.0_f64;
    for (index, event_id) in manifest.event_ids.iter().enumerate() {
        event_marginals.push(EventMarginalMass {
            event_id: event_id.clone(),
            mass: ExactMass::new(event_mass[index], manifest.total_mass_ticks),
        });
        let probability = event_mass[index] as f64 / manifest.total_mass_ticks as f64;
        independence_log_survival += libm::log1p(-probability);
    }

    let mut pairwise_intersections = Vec::new();
    for left in 0..event_count {
        for right in (left + 1)..event_count {
            pairwise_intersections.push(PairwiseIntersectionMass {
                left_event_id: manifest.event_ids[left].clone(),
                right_event_id: manifest.event_ids[right].clone(),
                mass: ExactMass::new(
                    pairwise_mass[left * event_count + right],
                    manifest.total_mass_ticks,
                ),
            });
        }
    }

    let joint_signatures = signature_mass
        .into_iter()
        .map(|(contacting_event_ids, mass)| JointEventSignatureMass {
            contacting_event_ids,
            mass: ExactMass::new(mass, manifest.total_mass_ticks),
        })
        .collect::<Vec<_>>();

    let independence_union_approximation = -libm::expm1(independence_log_survival);
    let risk_budget = ExactMass::new(
        manifest.risk_budget_numerator,
        manifest.risk_budget_denominator,
    );
    let within_budget = u128::from(union_mass)
        * u128::from(manifest.risk_budget_denominator)
        <= u128::from(manifest.risk_budget_numerator)
            * u128::from(manifest.total_mass_ticks);

    Ok(MissionRiskEnvelope {
        manifest_digest: manifest.manifest_digest,
        target_measure_digest: manifest.target_measure_digest,
        event_inventory_digest: manifest.event_inventory_digest,
        total_mass_ticks: manifest.total_mass_ticks,
        event_count,
        trajectory_atom_count: trajectories.len(),
        event_marginals,
        pairwise_intersections,
        joint_signatures,
        pathwise_union_mass: ExactMass::new(union_mass, manifest.total_mass_ticks),
        no_event_mass: ExactMass::new(no_event_mass, manifest.total_mass_ticks),
        marginal_sum_ticks,
        frechet_union_bounds: FrechetUnionBounds {
            lower_bound: ExactMass::new(frechet_lower, manifest.total_mass_ticks),
            upper_bound: ExactMass::new(frechet_upper, manifest.total_mass_ticks),
        },
        independence_union_approximation,
        risk_budget,
        budget_status: if within_budget {
            MissionRiskBudgetStatus::WithinDeclaredFiniteSupportBudget
        } else {
            MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget
        },
        authority_granted: false,
    })
}

fn valid_event_id(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(
        event_ids: &[&str],
        atom_ids: &[[u8; 32]],
        total_mass_ticks: u64,
        budget_numerator: u64,
        budget_denominator: u64,
    ) -> FiniteMissionRiskManifest {
        FiniteMissionRiskManifest {
            manifest_digest: [10; 32],
            target_measure_digest: [11; 32],
            event_inventory_digest: [12; 32],
            event_ids: event_ids.iter().map(|id| (*id).to_owned()).collect(),
            expected_atom_ids: atom_ids.to_vec(),
            total_mass_ticks,
            risk_budget_numerator: budget_numerator,
            risk_budget_denominator: budget_denominator,
            event_inventory_complete: true,
            trajectory_support_complete: true,
        }
    }

    fn atom(
        id: u8,
        mass_ticks: u64,
        events: &[(&str, EncounterEventDisposition)],
    ) -> FiniteMissionTrajectory {
        FiniteMissionTrajectory {
            atom_id: [id; 32],
            mass_ticks,
            event_outcomes: events
                .iter()
                .map(|(event_id, disposition)| EncounterEventOutcome {
                    event_id: (*event_id).to_owned(),
                    disposition: *disposition,
                })
                .collect(),
        }
    }

    fn hit() -> EncounterEventDisposition {
        EncounterEventDisposition::DefinitelyContact
    }

    fn miss() -> EncounterEventDisposition {
        EncounterEventDisposition::DefinitelyNoContact
    }

    fn evaluate(
        manifest: &FiniteMissionRiskManifest,
        atoms: &[FiniteMissionTrajectory],
    ) -> MissionRiskEnvelope {
        aggregate_finite_mission_risk(manifest, atoms).expect("finite mission support evaluates")
    }

    #[test]
    fn independent_events_match_the_independence_comparator() {
        let ids = [[1; 32], [2; 32], [3; 32], [4; 32]];
        let manifest = manifest(&["a", "b"], &ids, 100, 3, 4);
        let atoms = [
            atom(1, 25, &[("a", miss()), ("b", miss())]),
            atom(2, 25, &[("a", hit()), ("b", miss())]),
            atom(3, 25, &[("a", miss()), ("b", hit())]),
            atom(4, 25, &[("a", hit()), ("b", hit())]),
        ];
        let report = evaluate(&manifest, &atoms);
        assert_eq!(report.event_marginals[0].mass, ExactMass::new(1, 2));
        assert_eq!(report.event_marginals[1].mass, ExactMass::new(1, 2));
        assert_eq!(report.pairwise_intersections[0].mass, ExactMass::new(1, 4));
        assert_eq!(report.pathwise_union_mass, ExactMass::new(3, 4));
        assert_eq!(report.frechet_union_bounds.lower_bound, ExactMass::new(1, 2));
        assert_eq!(report.frechet_union_bounds.upper_bound, ExactMass::new(1, 1));
        assert!((report.independence_union_approximation - 0.75).abs() < 1e-12);
        assert_eq!(
            report.budget_status,
            MissionRiskBudgetStatus::WithinDeclaredFiniteSupportBudget
        );
        assert_eq!(report.joint_signatures.len(), 4);
        assert!(!report.authority_granted);
    }

    #[test]
    fn perfect_positive_dependence_differs_from_independence() {
        let ids = [[1; 32], [2; 32]];
        let manifest = manifest(&["a", "b"], &ids, 100, 3, 5);
        let atoms = [
            atom(1, 50, &[("a", miss()), ("b", miss())]),
            atom(2, 50, &[("a", hit()), ("b", hit())]),
        ];
        let report = evaluate(&manifest, &atoms);
        assert_eq!(report.pathwise_union_mass, ExactMass::new(1, 2));
        assert_eq!(report.pairwise_intersections[0].mass, ExactMass::new(1, 2));
        assert!((report.independence_union_approximation - 0.75).abs() < 1e-12);
        assert_eq!(
            report.budget_status,
            MissionRiskBudgetStatus::WithinDeclaredFiniteSupportBudget
        );
    }

    #[test]
    fn disjoint_events_show_independence_can_underestimate_union() {
        let ids = [[1; 32], [2; 32], [3; 32]];
        let manifest = manifest(&["a", "b"], &ids, 100, 3, 4);
        let atoms = [
            atom(1, 40, &[("a", hit()), ("b", miss())]),
            atom(2, 40, &[("a", miss()), ("b", hit())]),
            atom(3, 20, &[("a", miss()), ("b", miss())]),
        ];
        let report = evaluate(&manifest, &atoms);
        assert_eq!(report.pathwise_union_mass, ExactMass::new(4, 5));
        assert_eq!(report.pairwise_intersections[0].mass, ExactMass::new(0, 1));
        assert_eq!(report.frechet_union_bounds.lower_bound, ExactMass::new(2, 5));
        assert_eq!(report.frechet_union_bounds.upper_bound, ExactMass::new(4, 5));
        assert!((report.independence_union_approximation - 0.64).abs() < 1e-12);
        assert_eq!(
            report.budget_status,
            MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget
        );
    }

    #[test]
    fn individually_subthreshold_repeated_encounters_can_exceed_mission_budget() {
        let ids = [[1; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32]];
        let manifest = manifest(&["e1", "e2", "e3", "e4", "e5"], &ids, 10_000, 4, 100);
        let atoms = [
            atom(1, 100, &[("e1", hit()), ("e2", miss()), ("e3", miss()), ("e4", miss()), ("e5", miss())]),
            atom(2, 100, &[("e1", miss()), ("e2", hit()), ("e3", miss()), ("e4", miss()), ("e5", miss())]),
            atom(3, 100, &[("e1", miss()), ("e2", miss()), ("e3", hit()), ("e4", miss()), ("e5", miss())]),
            atom(4, 100, &[("e1", miss()), ("e2", miss()), ("e3", miss()), ("e4", hit()), ("e5", miss())]),
            atom(5, 100, &[("e1", miss()), ("e2", miss()), ("e3", miss()), ("e4", miss()), ("e5", hit())]),
            atom(6, 9_500, &[("e1", miss()), ("e2", miss()), ("e3", miss()), ("e4", miss()), ("e5", miss())]),
        ];
        let report = evaluate(&manifest, &atoms);
        assert!(report.event_marginals.iter().all(|row| row.mass == ExactMass::new(1, 100)));
        assert_eq!(report.pathwise_union_mass, ExactMass::new(1, 20));
        assert_eq!(
            report.budget_status,
            MissionRiskBudgetStatus::ExceedsDeclaredFiniteSupportBudget
        );
    }

    #[test]
    fn ambiguous_or_missing_event_rows_fail_closed() {
        let ids = [[1; 32]];
        let manifest = manifest(&["a", "b"], &ids, 100, 1, 10);
        let ambiguous = [atom(
            1,
            100,
            &[
                ("a", hit()),
                ("b", EncounterEventDisposition::Ambiguous),
            ],
        )];
        assert_eq!(
            aggregate_finite_mission_risk(&manifest, &ambiguous),
            Err(MissionRiskAggregationError::AmbiguousEventOutcome)
        );

        let missing = [atom(1, 100, &[("a", hit())])];
        assert_eq!(
            aggregate_finite_mission_risk(&manifest, &missing),
            Err(MissionRiskAggregationError::MissingEventOutcome)
        );
    }

    #[test]
    fn duplicate_unknown_and_aliased_event_rows_are_rejected() {
        let ids = [[1; 32]];
        let manifest = manifest(&["a", "b"], &ids, 100, 1, 10);
        let duplicate = [atom(1, 100, &[("a", hit()), ("a", miss()), ("b", miss())])];
        assert_eq!(
            aggregate_finite_mission_risk(&manifest, &duplicate),
            Err(MissionRiskAggregationError::DuplicateEventOutcome)
        );

        let unknown = [atom(1, 100, &[("a", hit()), ("c", miss())])];
        assert_eq!(
            aggregate_finite_mission_risk(&manifest, &unknown),
            Err(MissionRiskAggregationError::UnknownEventOutcome)
        );
    }

    #[test]
    fn omitted_atoms_incomplete_manifests_and_mass_mismatch_are_rejected() {
        let ids = [[1; 32], [2; 32]];
        let manifest = manifest(&["a"], &ids, 100, 1, 10);
        let one_atom = [atom(1, 100, &[("a", miss())])];
        assert_eq!(
            aggregate_finite_mission_risk(&manifest, &one_atom),
            Err(MissionRiskAggregationError::InvalidTrajectoryInventorySize)
        );

        let mut incomplete = manifest.clone();
        incomplete.event_inventory_complete = false;
        let atoms = [
            atom(1, 50, &[("a", miss())]),
            atom(2, 50, &[("a", hit())]),
        ];
        assert_eq!(
            aggregate_finite_mission_risk(&incomplete, &atoms),
            Err(MissionRiskAggregationError::IncompleteEventInventory)
        );

        let mut wrong_mass = manifest;
        wrong_mass.total_mass_ticks = 99;
        assert_eq!(
            aggregate_finite_mission_risk(&wrong_mass, &atoms),
            Err(MissionRiskAggregationError::TotalMassMismatch)
        );
    }

    #[test]
    fn input_atom_and_outcome_order_does_not_change_the_report() {
        let ids = [[1; 32], [2; 32], [3; 32]];
        let manifest = manifest(&["a", "b"], &ids, 100, 3, 4);
        let atoms = [
            atom(1, 40, &[("a", hit()), ("b", miss())]),
            atom(2, 30, &[("a", miss()), ("b", hit())]),
            atom(3, 30, &[("a", miss()), ("b", miss())]),
        ];
        let expected = evaluate(&manifest, &atoms);
        let reversed = [
            atom(3, 30, &[("b", miss()), ("a", miss())]),
            atom(2, 30, &[("b", hit()), ("a", miss())]),
            atom(1, 40, &[("b", miss()), ("a", hit())]),
        ];
        assert_eq!(evaluate(&manifest, &reversed), expected);
    }

    #[test]
    fn invalid_budget_and_noncanonical_inventories_are_rejected() {
        let ids = [[1; 32]];
        let atoms = [atom(1, 100, &[("a", miss())])];
        let mut invalid_budget = manifest(&["a"], &ids, 100, 2, 1);
        assert_eq!(
            aggregate_finite_mission_risk(&invalid_budget, &atoms),
            Err(MissionRiskAggregationError::InvalidRiskBudget)
        );

        invalid_budget = manifest(&["b", "a"], &ids, 100, 1, 10);
        assert_eq!(
            aggregate_finite_mission_risk(&invalid_budget, &atoms),
            Err(MissionRiskAggregationError::InvalidEventIdentity)
        );
    }
}
