//! Fail-closed reference for proof-carrying concurrent action batches.
//!
//! This crate checks only typed composition inputs and owner-supplied
//! pairwise verdict references. It does not calculate swept volumes,
//! authenticate owner receipts, evaluate runtime assurance, authorize
//! actuators, dispatch commands, or establish physical independence.
//!
//! Geometry/occupancy and resource-capacity truth remain owned by the
//! existing Symtropy owners. Missing/unknown inputs block concurrent
//! admission. Serial execution remains the fallback.

use std::collections::{BTreeMap, BTreeSet};

pub type Digest = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ActionRef {
    pub action_id: String,
    pub attempt: u64,
    pub ticket_digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedContext {
    pub binding_digest: Digest,
    pub authority_epoch: u64,
    pub clock_domain_id: String,
    pub boot_epoch_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    Read,
    Write,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateAccess {
    pub key: String,
    pub mode: AccessMode,
    pub generation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeInterval {
    pub time_basis_digest: Digest,
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExclusiveClaim {
    pub resource_id: String,
    pub interval: TimeInterval,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityClaim {
    pub resource_id: String,
    pub unit_profile_digest: Digest,
    pub capacity_profile_digest: Digest,
    pub interval: TimeInterval,
    pub amount: u64,
    pub capacity: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerDisposition {
    Clear,
    Conflict,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairwiseOwnerCheck {
    pub left: ActionRef,
    pub right: ActionRef,
    pub owner_family: String,
    pub input_digest: Digest,
    pub result_digest: Digest,
    pub disposition: OwnerDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FootprintQualification {
    Qualified { footprint_digest: Digest },
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionFootprint {
    pub action: ActionRef,
    pub shared_context: SharedContext,
    pub qualification: FootprintQualification,
    pub state_accesses: Vec<StateAccess>,
    pub exclusive_claims: Vec<ExclusiveClaim>,
    pub capacity_claims: Vec<CapacityClaim>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionProfile {
    pub profile_id: String,
    pub policy_digest: Digest,
    /// Exact owner families required for every action pair. The list must be
    /// unique and non-empty; the verifier does not infer missing owners.
    pub required_owner_families: Vec<String>,
    pub max_actions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityPeak {
    pub resource_id: String,
    pub peak_amount: u64,
    pub capacity: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedBatch {
    /// These actions are eligible as one concurrent batch under this profile.
    /// This is not actuator authority and does not replace each action's
    /// native application-boundary check.
    pub profile_id: String,
    pub policy_digest: Digest,
    pub actions: Vec<ActionRef>,
    pub shared_context: SharedContext,
    pub capacity_peaks: Vec<CapacityPeak>,
    pub pairwise_owner_check_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchBlock {
    InvalidProfile,
    TooFewActions,
    TooManyActions { actual: usize, maximum: usize },
    EmptyActionIdentity,
    DuplicateAction,
    DuplicateTicket,
    StateGenerationMismatch {
        left: String,
        right: String,
        key: String,
    },
    EmptySharedContext,
    MixedSharedContext,
    UnknownFootprint { action_id: String },
    EmptyFootprintDigest { action_id: String },
    EmptyStateKey { action_id: String },
    DuplicateStateAccess { action_id: String, key: String },
    InvalidInterval { action_id: String },
    EmptyResourceId { action_id: String },
    ResourceClaimKindMismatch { resource: String },
    ActionCapacityExceeded { action_id: String, resource: String, amount: u64, capacity: u64 },
    EmptyOwnerFamily,
    DuplicateRequiredOwnerFamily,
    MissingOwnerCheck { left: String, right: String, owner: String },
    DuplicateOwnerCheck { left: String, right: String, owner: String },
    UnexpectedOwnerCheck { left: String, right: String, owner: String },
    InvalidOwnerCheck { left: String, right: String, owner: String },
    OwnerConflict { left: String, right: String, owner: String },
    OwnerUnknown { left: String, right: String, owner: String },
    StateConflict { left: String, right: String, key: String },
    ExclusiveResourceConflict { left: String, right: String, resource: String },
    UncomparableTimeBasis { left: String, right: String, resource: String },
    CapacityProfileMismatch { resource: String },
    CapacityExceeded {
        resource: String,
        peak_amount: u64,
        capacity: u64,
    },
    CapacityArithmeticOverflow { resource: String },
}

fn is_nonzero_digest(digest: &Digest) -> bool {
    digest.iter().any(|byte| *byte != 0)
}

fn overlaps(a: &TimeInterval, b: &TimeInterval) -> Option<bool> {
    if a.start >= a.end || b.start >= b.end {
        return None;
    }
    if a.time_basis_digest != b.time_basis_digest {
        return None;
    }
    Some(a.start < b.end && b.start < a.end)
}

fn pair_key(a: &ActionRef, b: &ActionRef) -> (ActionRef, ActionRef) {
    if a <= b {
        (a.clone(), b.clone())
    } else {
        (b.clone(), a.clone())
    }
}

fn validate_profile(profile: &CompositionProfile) -> Result<(), BatchBlock> {
    if profile.profile_id.trim().is_empty()
        || !is_nonzero_digest(&profile.policy_digest)
        || profile.max_actions < 2
        || profile.required_owner_families.is_empty()
    {
        return Err(BatchBlock::InvalidProfile);
    }

    let mut seen = BTreeSet::new();
    for owner in &profile.required_owner_families {
        if owner.trim().is_empty() {
            return Err(BatchBlock::EmptyOwnerFamily);
        }
        if !seen.insert(owner.clone()) {
            return Err(BatchBlock::DuplicateRequiredOwnerFamily);
        }
    }
    Ok(())
}

fn validate_footprints(
    profile: &CompositionProfile,
    footprints: &[ActionFootprint],
) -> Result<SharedContext, BatchBlock> {
    if footprints.len() < 2 {
        return Err(BatchBlock::TooFewActions);
    }
    if footprints.len() > profile.max_actions {
        return Err(BatchBlock::TooManyActions {
            actual: footprints.len(),
            maximum: profile.max_actions,
        });
    }

    let first = &footprints[0].shared_context;
    if !is_nonzero_digest(&first.binding_digest)
        || first.clock_domain_id.trim().is_empty()
        || first.boot_epoch_id.trim().is_empty()
    {
        return Err(BatchBlock::EmptySharedContext);
    }

    let mut actions = BTreeSet::new();
    let mut action_ids = BTreeSet::new();
    let mut tickets = BTreeSet::new();
    let mut exclusive_resources = BTreeSet::new();
    let mut capacity_resources = BTreeSet::new();

    for footprint in footprints {
        let action = &footprint.action;
        if action.action_id.trim().is_empty() || !is_nonzero_digest(&action.ticket_digest) {
            return Err(BatchBlock::EmptyActionIdentity);
        }
        if !actions.insert(action.clone()) || !action_ids.insert(action.action_id.clone()) {
            return Err(BatchBlock::DuplicateAction);
        }
        if !tickets.insert(action.ticket_digest) {
            return Err(BatchBlock::DuplicateTicket);
        }
        if &footprint.shared_context != first {
            return Err(BatchBlock::MixedSharedContext);
        }

        match &footprint.qualification {
            FootprintQualification::Unknown => {
                return Err(BatchBlock::UnknownFootprint {
                    action_id: action.action_id.clone(),
                });
            }
            FootprintQualification::Qualified { footprint_digest } => {
                if !is_nonzero_digest(footprint_digest) {
                    return Err(BatchBlock::EmptyFootprintDigest {
                        action_id: action.action_id.clone(),
                    });
                }
            }
        }

        let mut access_keys = BTreeSet::new();
        for access in &footprint.state_accesses {
            if access.key.trim().is_empty() || access.generation.trim().is_empty() {
                return Err(BatchBlock::EmptyStateKey {
                    action_id: action.action_id.clone(),
                });
            }
            if !access_keys.insert(access.key.clone()) {
                return Err(BatchBlock::DuplicateStateAccess {
                    action_id: action.action_id.clone(),
                    key: access.key.clone(),
                });
            }
        }

        for claim in &footprint.exclusive_claims {
            if claim.resource_id.trim().is_empty() {
                return Err(BatchBlock::EmptyResourceId {
                    action_id: action.action_id.clone(),
                });
            }
            exclusive_resources.insert(claim.resource_id.clone());
            if claim.interval.start >= claim.interval.end
                || !is_nonzero_digest(&claim.interval.time_basis_digest)
            {
                return Err(BatchBlock::InvalidInterval {
                    action_id: action.action_id.clone(),
                });
            }
        }

        for claim in &footprint.capacity_claims {
            if claim.resource_id.trim().is_empty() {
                return Err(BatchBlock::EmptyResourceId {
                    action_id: action.action_id.clone(),
                });
            }
            if claim.interval.start >= claim.interval.end
                || !is_nonzero_digest(&claim.interval.time_basis_digest)
                || !is_nonzero_digest(&claim.unit_profile_digest)
                || !is_nonzero_digest(&claim.capacity_profile_digest)
                || claim.amount == 0
                || claim.capacity == 0
            {
                return Err(BatchBlock::InvalidInterval {
                    action_id: action.action_id.clone(),
                });
            }
            capacity_resources.insert(claim.resource_id.clone());
            if claim.amount > claim.capacity {
                return Err(BatchBlock::ActionCapacityExceeded {
                    action_id: action.action_id.clone(),
                    resource: claim.resource_id.clone(),
                    amount: claim.amount,
                    capacity: claim.capacity,
                });
            }
        }
    }

    if let Some(resource) = exclusive_resources.intersection(&capacity_resources).next() {
        return Err(BatchBlock::ResourceClaimKindMismatch {
            resource: resource.clone(),
        });
    }

    Ok(first.clone())
}

fn validate_pairwise_owner_checks(
    profile: &CompositionProfile,
    footprints: &[ActionFootprint],
    checks: &[PairwiseOwnerCheck],
) -> Result<usize, BatchBlock> {
    let mut indexed: BTreeMap<(ActionRef, ActionRef, String), &PairwiseOwnerCheck> =
        BTreeMap::new();

    let known_actions: BTreeSet<_> =
        footprints.iter().map(|footprint| footprint.action.clone()).collect();

    for check in checks {
        let (left, right) = pair_key(&check.left, &check.right);
        if !known_actions.contains(&left) || !known_actions.contains(&right) {
            return Err(BatchBlock::UnexpectedOwnerCheck {
                left: left.action_id.clone(),
                right: right.action_id.clone(),
                owner: check.owner_family.clone(),
            });
        }
        if left == right
            || check.owner_family.trim().is_empty()
            || !is_nonzero_digest(&check.input_digest)
            || !is_nonzero_digest(&check.result_digest)
        {
            return Err(BatchBlock::InvalidOwnerCheck {
                left: check.left.action_id.clone(),
                right: check.right.action_id.clone(),
                owner: check.owner_family.clone(),
            });
        }
        let key = (left.clone(), right.clone(), check.owner_family.clone());
        if indexed.insert(key, check).is_some() {
            return Err(BatchBlock::DuplicateOwnerCheck {
                left: left.action_id,
                right: right.action_id,
                owner: check.owner_family.clone(),
            });
        }
    }

    let mut required_count = 0usize;
    for i in 0..footprints.len() {
        for j in (i + 1)..footprints.len() {
            let (left, right) =
                pair_key(&footprints[i].action, &footprints[j].action);
            for owner in &profile.required_owner_families {
                required_count += 1;
                let check = indexed.get(&(left.clone(), right.clone(), owner.clone()));
                let check = match check {
                    Some(check) => check,
                    None => {
                        return Err(BatchBlock::MissingOwnerCheck {
                            left: left.action_id.clone(),
                            right: right.action_id.clone(),
                            owner: owner.clone(),
                        });
                    }
                };
                match check.disposition {
                    OwnerDisposition::Clear => {}
                    OwnerDisposition::Conflict => {
                        return Err(BatchBlock::OwnerConflict {
                            left: left.action_id.clone(),
                            right: right.action_id.clone(),
                            owner: owner.clone(),
                        });
                    }
                    OwnerDisposition::Unknown => {
                        return Err(BatchBlock::OwnerUnknown {
                            left: left.action_id.clone(),
                            right: right.action_id.clone(),
                            owner: owner.clone(),
                        });
                    }
                }
            }
        }
    }
    if indexed.len() != required_count {
        return Err(BatchBlock::UnexpectedOwnerCheck {
            left: String::new(),
            right: String::new(),
            owner: "unexpected-or-unrequired-check".to_owned(),
        });
    }
    Ok(required_count)
}

fn validate_state_access_conflicts(
    footprints: &[ActionFootprint],
) -> Result<(), BatchBlock> {
    for i in 0..footprints.len() {
        for j in (i + 1)..footprints.len() {
            for a in &footprints[i].state_accesses {
                for b in &footprints[j].state_accesses {
                    if a.key == b.key && a.generation != b.generation {
                        return Err(BatchBlock::StateGenerationMismatch {
                            left: footprints[i].action.action_id.clone(),
                            right: footprints[j].action.action_id.clone(),
                            key: a.key.clone(),
                        });
                    }
                    if a.key == b.key && a.generation == b.generation
                        && (a.mode == AccessMode::Write || b.mode == AccessMode::Write)
                    {
                        return Err(BatchBlock::StateConflict {
                            left: footprints[i].action.action_id.clone(),
                            right: footprints[j].action.action_id.clone(),
                            key: a.key.clone(),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_exclusive_claims(
    footprints: &[ActionFootprint],
) -> Result<(), BatchBlock> {
    for i in 0..footprints.len() {
        for j in (i + 1)..footprints.len() {
            for a in &footprints[i].exclusive_claims {
                for b in &footprints[j].exclusive_claims {
                    if a.resource_id != b.resource_id {
                        continue;
                    }
                    match overlaps(&a.interval, &b.interval) {
                        Some(true) => {
                            return Err(BatchBlock::ExclusiveResourceConflict {
                                left: footprints[i].action.action_id.clone(),
                                right: footprints[j].action.action_id.clone(),
                                resource: a.resource_id.clone(),
                            });
                        }
                        Some(false) => {}
                        None => {
                            return Err(BatchBlock::UncomparableTimeBasis {
                                left: footprints[i].action.action_id.clone(),
                                right: footprints[j].action.action_id.clone(),
                                resource: a.resource_id.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct CapacityEvent {
    tick: u64,
    amount: i128,
    is_start: bool,
}

fn validate_capacity_claims(
    footprints: &[ActionFootprint],
) -> Result<Vec<CapacityPeak>, BatchBlock> {
    let mut resources: BTreeMap<String, Vec<&CapacityClaim>> = BTreeMap::new();
    for footprint in footprints {
        for claim in &footprint.capacity_claims {
            resources
                .entry(claim.resource_id.clone())
                .or_default()
                .push(claim);
        }
    }

    let mut peaks = Vec::new();
    for (resource_id, claims) in resources {
        let first = claims[0];
        let mut events = Vec::with_capacity(claims.len() * 2);
        for claim in &claims {
            if claim.capacity_profile_digest != first.capacity_profile_digest
                || claim.unit_profile_digest != first.unit_profile_digest
                || claim.capacity != first.capacity
                || claim.interval.time_basis_digest != first.interval.time_basis_digest
            {
                return Err(BatchBlock::CapacityProfileMismatch {
                    resource: resource_id.clone(),
                });
            }

            let amount = i128::from(claim.amount);
            events.push(CapacityEvent {
                tick: claim.interval.start,
                amount,
                is_start: true,
            });
            events.push(CapacityEvent {
                tick: claim.interval.end,
                amount: -amount,
                is_start: false,
            });
        }

        // Half-open intervals: ends are applied before starts at the same tick.
        events.sort_by_key(|event| (event.tick, event.is_start));
        let mut active = 0i128;
        let mut peak = 0i128;
        for event in events {
            active = active
                .checked_add(event.amount)
                .ok_or_else(|| BatchBlock::CapacityArithmeticOverflow {
                    resource: resource_id.clone(),
                })?;
            peak = peak.max(active);
            if active < 0 {
                return Err(BatchBlock::CapacityArithmeticOverflow {
                    resource: resource_id.clone(),
                });
            }
        }

        if peak > i128::from(first.capacity) {
            return Err(BatchBlock::CapacityExceeded {
                resource: resource_id.clone(),
                peak_amount: u64::try_from(peak).unwrap_or(u64::MAX),
                capacity: first.capacity,
            });
        }
        peaks.push(CapacityPeak {
            resource_id,
            peak_amount: u64::try_from(peak).map_err(|_| {
                BatchBlock::CapacityArithmeticOverflow {
                    resource: first.resource_id.clone(),
                }
            })?,
            capacity: first.capacity,
        });
    }
    Ok(peaks)
}

/// Qualify one exact set of actions for concurrent admission.
///
/// This returns a structural eligibility result, not an actuator capability.
/// Every member still requires its own current M1/M2 ticket check at the
/// native application boundary. Owner-check results are references/claims;
/// their issuer, signatures and semantic truth must be verified by the
/// corresponding upstream owner before this function is relied upon.
pub fn qualify_concurrent_batch(
    profile: &CompositionProfile,
    footprints: &[ActionFootprint],
    owner_checks: &[PairwiseOwnerCheck],
) -> Result<QualifiedBatch, BatchBlock> {
    validate_profile(profile)?;
    let shared_context = validate_footprints(profile, footprints)?;
    let pairwise_owner_check_count =
        validate_pairwise_owner_checks(profile, footprints, owner_checks)?;
    validate_state_access_conflicts(footprints)?;
    validate_exclusive_claims(footprints)?;
    let capacity_peaks = validate_capacity_claims(footprints)?;

    let mut actions: Vec<_> = footprints.iter().map(|f| f.action.clone()).collect();
    actions.sort();

    Ok(QualifiedBatch {
        profile_id: profile.profile_id.clone(),
        policy_digest: profile.policy_digest,
        actions,
        shared_context,
        capacity_peaks,
        pairwise_owner_check_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: u8) -> Digest {
        [value; 32]
    }

    fn action(id: &str, attempt: u64, ticket: u8) -> ActionRef {
        ActionRef {
            action_id: id.to_owned(),
            attempt,
            ticket_digest: d(ticket),
        }
    }

    fn context() -> SharedContext {
        SharedContext {
            binding_digest: d(1),
            authority_epoch: 4,
            clock_domain_id: "platform-clock".to_owned(),
            boot_epoch_id: "boot-17".to_owned(),
        }
    }

    fn profile() -> CompositionProfile {
        CompositionProfile {
            profile_id: "dual-arm-independent-v0.1".to_owned(),
            policy_digest: d(2),
            required_owner_families: vec![
                "space-time-occupancy".to_owned(),
                "domain-runtime-assurance".to_owned(),
            ],
            max_actions: 8,
        }
    }

    fn footprint(id: &str, ticket: u8) -> ActionFootprint {
        ActionFootprint {
            action: action(id, 1, ticket),
            shared_context: context(),
            qualification: FootprintQualification::Qualified {
                footprint_digest: d(ticket + 20),
            },
            state_accesses: vec![StateAccess {
                key: format!("object-{id}"),
                mode: AccessMode::Write,
                generation: "object-gen-4".to_owned(),
            }],
            exclusive_claims: vec![ExclusiveClaim {
                resource_id: format!("actuator-{id}"),
                interval: TimeInterval {
                    time_basis_digest: d(3),
                    start: 10,
                    end: 20,
                },
            }],
            capacity_claims: Vec::new(),
        }
    }

    fn checks(a: &ActionRef, b: &ActionRef) -> Vec<PairwiseOwnerCheck> {
        let (left, right) = pair_key(a, b);
        vec![
            PairwiseOwnerCheck {
                left: left.clone(),
                right: right.clone(),
                owner_family: "space-time-occupancy".to_owned(),
                input_digest: d(4),
                result_digest: d(5),
                disposition: OwnerDisposition::Clear,
            },
            PairwiseOwnerCheck {
                left,
                right,
                owner_family: "domain-runtime-assurance".to_owned(),
                input_digest: d(6),
                result_digest: d(7),
                disposition: OwnerDisposition::Clear,
            },
        ]
    }

    #[test]
    fn disjoint_actions_with_complete_owner_checks_are_eligible() {
        let a = footprint("arm-a", 10);
        let b = footprint("arm-b", 11);
        let result = qualify_concurrent_batch(
            &profile(),
            &[a.clone(), b.clone()],
            &checks(&a.action, &b.action),
        )
        .unwrap();
        assert_eq!(result.actions.len(), 2);
        assert_eq!(result.pairwise_owner_check_count, 2);
    }

    #[test]
    fn unknown_footprint_blocks_concurrent_admission() {
        let mut a = footprint("arm-a", 10);
        a.qualification = FootprintQualification::Unknown;
        let b = footprint("arm-b", 11);
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::UnknownFootprint {
                action_id: "arm-a".to_owned()
            })
        );
    }

    #[test]
    fn write_read_state_conflict_blocks_batch() {
        let mut a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        a.state_accesses = vec![StateAccess {
            key: "shared-object".to_owned(),
            mode: AccessMode::Write,
            generation: "generation-2".to_owned(),
        }];
        b.state_accesses = vec![StateAccess {
            key: "shared-object".to_owned(),
            mode: AccessMode::Read,
            generation: "generation-2".to_owned(),
        }];
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::StateConflict {
                left: "arm-a".to_owned(),
                right: "arm-b".to_owned(),
                key: "shared-object".to_owned(),
            })
        );
    }

    #[test]
    fn exclusive_resource_overlap_blocks_batch() {
        let a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        b.exclusive_claims[0].resource_id = a.exclusive_claims[0].resource_id.clone();
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::ExclusiveResourceConflict {
                left: "arm-a".to_owned(),
                right: "arm-b".to_owned(),
                resource: "actuator-arm-a".to_owned(),
            })
        );
    }

    #[test]
    fn adjacent_half_open_exclusive_intervals_do_not_conflict() {
        let a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        b.exclusive_claims[0].resource_id = a.exclusive_claims[0].resource_id.clone();
        b.exclusive_claims[0].interval.start = 20;
        b.exclusive_claims[0].interval.end = 30;
        assert!(qualify_concurrent_batch(
            &profile(),
            &[a.clone(), b.clone()],
            &checks(&a.action, &b.action),
        ).is_ok());
    }

    #[test]
    fn shared_capacity_is_checked_as_an_aggregate_hyperedge() {
        let mut a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        let claim = CapacityClaim {
            resource_id: "shared-power-bus".to_owned(),
            unit_profile_digest: d(30),
            capacity_profile_digest: d(31),
            interval: TimeInterval {
                time_basis_digest: d(3),
                start: 10,
                end: 20,
            },
            amount: 4,
            capacity: 6,
        };
        a.capacity_claims = vec![claim.clone()];
        b.capacity_claims = vec![CapacityClaim { amount: 3, ..claim }];
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::CapacityExceeded {
                resource: "shared-power-bus".to_owned(),
                peak_amount: 7,
                capacity: 6,
            })
        );
    }

    #[test]
    fn capacity_intervals_touching_at_boundary_do_not_double_count() {
        let mut a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        let claim = CapacityClaim {
            resource_id: "shared-power-bus".to_owned(),
            unit_profile_digest: d(30),
            capacity_profile_digest: d(31),
            interval: TimeInterval {
                time_basis_digest: d(3),
                start: 10,
                end: 20,
            },
            amount: 4,
            capacity: 4,
        };
        a.capacity_claims = vec![claim.clone()];
        b.capacity_claims = vec![CapacityClaim {
            interval: TimeInterval {
                time_basis_digest: d(3),
                start: 20,
                end: 30,
            },
            ..claim
        }];
        assert!(qualify_concurrent_batch(
            &profile(),
            &[a.clone(), b.clone()],
            &checks(&a.action, &b.action),
        ).is_ok());
    }

    #[test]
    fn missing_owner_pair_check_blocks_batch() {
        let a = footprint("arm-a", 10);
        let b = footprint("arm-b", 11);
        let mut all_checks = checks(&a.action, &b.action);
        let only_one = vec![all_checks.remove(0)];
        assert!(matches!(
            qualify_concurrent_batch(&profile(), &[a.clone(), b.clone()], &only_one),
            Err(BatchBlock::MissingOwnerCheck { .. })
        ));
    }

    #[test]
    fn unknown_owner_result_blocks_batch() {
        let a = footprint("arm-a", 10);
        let b = footprint("arm-b", 11);
        let mut owner_checks = checks(&a.action, &b.action);
        owner_checks[0].disposition = OwnerDisposition::Unknown;
        assert!(matches!(
            qualify_concurrent_batch(&profile(), &[a.clone(), b.clone()], &owner_checks),
            Err(BatchBlock::OwnerUnknown { .. })
        ));
    }

    #[test]
    fn owner_conflict_result_blocks_batch() {
        let a = footprint("arm-a", 10);
        let b = footprint("arm-b", 11);
        let mut owner_checks = checks(&a.action, &b.action);
        owner_checks[1].disposition = OwnerDisposition::Conflict;
        assert!(matches!(
            qualify_concurrent_batch(&profile(), &[a.clone(), b.clone()], &owner_checks),
            Err(BatchBlock::OwnerConflict { .. })
        ));
    }

    #[test]
    fn mixed_authority_snapshot_blocks_batch() {
        let a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        b.shared_context.authority_epoch += 1;
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::MixedSharedContext)
        );
    }

    #[test]
    fn duplicate_ticket_digest_blocks_batch() {
        let a = footprint("arm-a", 10);
        let b = footprint("arm-b", 10);
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::DuplicateTicket)
        );
    }

    #[test]
    fn capacity_time_basis_mismatch_blocks_batch() {
        let mut a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        let claim = CapacityClaim {
            resource_id: "shared-power-bus".to_owned(),
            unit_profile_digest: d(30),
            capacity_profile_digest: d(31),
            interval: TimeInterval {
                time_basis_digest: d(3),
                start: 10,
                end: 20,
            },
            amount: 2,
            capacity: 6,
        };
        a.capacity_claims = vec![claim.clone()];
        b.capacity_claims = vec![CapacityClaim {
            interval: TimeInterval {
                time_basis_digest: d(9),
                ..claim.interval.clone()
            },
            ..claim
        }];
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::CapacityProfileMismatch {
                resource: "shared-power-bus".to_owned()
            })
        );
    }
    #[test]
    fn same_action_id_with_different_attempts_blocks_batch() {
        let a = footprint("arm-a", 10);
        let mut b = footprint("arm-a", 11);
        b.action.attempt = 2;
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::DuplicateAction)
        );
    }

    #[test]
    fn mismatched_object_generation_is_not_treated_as_independent() {
        let mut a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        a.state_accesses = vec![StateAccess {
            key: "shared-object".to_owned(),
            mode: AccessMode::Read,
            generation: "generation-2".to_owned(),
        }];
        b.state_accesses = vec![StateAccess {
            key: "shared-object".to_owned(),
            mode: AccessMode::Read,
            generation: "generation-3".to_owned(),
        }];
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::StateGenerationMismatch {
                left: "arm-a".to_owned(),
                right: "arm-b".to_owned(),
                key: "shared-object".to_owned(),
            })
        );
    }

    #[test]
    fn owner_check_for_unknown_action_is_rejected() {
        let a = footprint("arm-a", 10);
        let b = footprint("arm-b", 11);
        let mut owner_checks = checks(&a.action, &b.action);
        owner_checks[0].right = action("not-in-batch", 1, 99);
        assert!(matches!(
            qualify_concurrent_batch(&profile(), &[a, b], &owner_checks),
            Err(BatchBlock::UnexpectedOwnerCheck { .. })
        ));
    }

    #[test]
    fn same_resource_cannot_mix_exclusive_and_capacity_claim_semantics() {
        let mut a = footprint("arm-a", 10);
        let mut b = footprint("arm-b", 11);
        let resource = "shared-resource".to_owned();
        a.exclusive_claims[0].resource_id = resource.clone();
        b.capacity_claims = vec![CapacityClaim {
            resource_id: resource.clone(),
            unit_profile_digest: d(30),
            capacity_profile_digest: d(31),
            interval: TimeInterval {
                time_basis_digest: d(3),
                start: 10,
                end: 20,
            },
            amount: 1,
            capacity: 4,
        }];
        assert_eq!(
            qualify_concurrent_batch(
                &profile(),
                &[a.clone(), b.clone()],
                &checks(&a.action, &b.action),
            ),
            Err(BatchBlock::ResourceClaimKindMismatch { resource })
        );
    }

}
