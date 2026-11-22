//! Read-only, fail-closed dependency-component quarantine reference.
//! Supplied owner inventories are structural assertions, not authenticated truth.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub type Digest = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ActionRef {
    pub action_id: String,
    pub attempt: u64,
    pub ticket_digest: Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Scope {
    pub scope_id: String,
    pub expected_action_count: usize,
    pub actions: Vec<ActionRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    pub profile_id: String,
    pub profile_digest: Digest,
    pub required_owners: Vec<String>,
    pub max_actions: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    NotApplied,
    ReconciledNoEffect,
    ReconciledEffect,
    Unknown,
    PartialEffect,
    AppliedUnreconciled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionOutcome {
    pub action: ActionRef,
    pub outcome: Outcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completeness {
    Complete,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Impact {
    Component,
    Global,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyKind {
    State,
    PreconditionsOrEffects,
    SpatialTemporal,
    ExclusiveResource,
    SharedCapacity,
    ActuatorOrTool,
    Controller,
    ObserverOrEvidence,
    CommonCause,
    AuthorityOrMission,
    ClockOrBoot,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Domain {
    pub id: String,
    pub kind: DependencyKind,
    pub impact: Impact,
    pub digest: Digest,
    /// Complete member set for this owner-qualified dependency hyperedge.
    pub members: Vec<ActionRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnerInventory {
    pub owner: String,
    pub scope_id: String,
    pub expected_action_count: usize,
    pub actions: Vec<ActionRef>,
    pub expected_domain_count: usize,
    pub digest: Digest,
    pub completeness: Completeness,
    pub domains: Vec<Domain>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Block {
    InvalidProfile,
    InvalidScope,
    ActionCountMismatch,
    InvalidAction,
    DuplicateAction,
    ForeignOutcome,
    DuplicateOutcome,
    MissingOutcome,
    MissingOwner,
    DuplicateOwner,
    UnexpectedOwner,
    UnknownInventory,
    MissingInventoryDigest,
    InventoryScopeMismatch,
    IncompleteInventory,
    InvalidDomain,
    UnknownImpact,
    DomainMemberOutsideScope,
    GlobalDomainScopeMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Decision {
    /// No unresolved effects; this is not a dispatch or concurrency grant.
    NoUnresolvedEffects { scope_id: String, action_count: usize },
    /// Outside the closure means only "not held by this computation".
    /// It does not release authority, reservations or runtime-assurance gates.
    ComponentQuarantine {
        scope_id: String,
        seeds: Vec<ActionRef>,
        quarantined: Vec<ActionRef>,
        outside_quarantine: Vec<ActionRef>,
        traversed_domains: Vec<String>,
    },
    /// Any uncertainty in dependency reconstruction retains the whole scope.
    GlobalHold {
        scope_id: String,
        all_actions: Vec<ActionRef>,
        seeds: Vec<ActionRef>,
        reason: Block,
    },
}

fn nz(d: &Digest) -> bool { d.iter().any(|b| *b != 0) }

fn action_set(actions: &[ActionRef]) -> Option<BTreeSet<ActionRef>> {
    let mut set = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for a in actions {
        if a.action_id.trim().is_empty() || !nz(&a.ticket_digest) || !set.insert(a.clone()) {
            return None;
        }
        // A scope has one current attempt per action ID.
        if !ids.insert(a.action_id.clone()) { return None; }
    }
    Some(set)
}

fn hold(scope: &Scope, seeds: Vec<ActionRef>, reason: Block) -> Decision {
    Decision::GlobalHold {
        scope_id: scope.scope_id.clone(),
        all_actions: scope.actions.clone(),
        seeds,
        reason,
    }
}

fn valid_profile(p: &Profile) -> bool {
    if p.profile_id.trim().is_empty() || !nz(&p.profile_digest)
        || p.max_actions == 0 || p.required_owners.is_empty() { return false; }
    let mut seen = BTreeSet::new();
    p.required_owners.iter().all(|o| !o.trim().is_empty() && seen.insert(o.clone()))
}

fn inventories(
    p: &Profile,
    s: &Scope,
    expected: &BTreeSet<ActionRef>,
    all: &[OwnerInventory],
) -> Result<Vec<(String, Domain)>, Block> {
    let required: BTreeSet<_> = p.required_owners.iter().cloned().collect();
    if all.len() < required.len() { return Err(Block::MissingOwner); }
    if all.len() > required.len() { return Err(Block::UnexpectedOwner); }
    let mut seen = BTreeSet::new();
    let mut domains = Vec::new();
    for inv in all {
        if !required.contains(&inv.owner) { return Err(Block::UnexpectedOwner); }
        if !seen.insert(inv.owner.clone()) { return Err(Block::DuplicateOwner); }
        if inv.scope_id != s.scope_id || inv.expected_action_count != s.expected_action_count
            || inv.actions.len() != expected.len()
            || action_set(&inv.actions).as_ref() != Some(expected) {
            return Err(Block::InventoryScopeMismatch);
        }
        if inv.expected_domain_count != inv.domains.len() { return Err(Block::IncompleteInventory); }
        if !nz(&inv.digest) { return Err(Block::MissingInventoryDigest); }
        if inv.completeness == Completeness::Unknown { return Err(Block::UnknownInventory); }

        let mut ids = BTreeSet::new();
        for d in &inv.domains {
            if d.id.trim().is_empty() || !ids.insert(d.id.clone())
                || !nz(&d.digest) || d.members.is_empty() {
                return Err(Block::InvalidDomain);
            }
            let Some(members) = action_set(&d.members) else { return Err(Block::InvalidDomain); };
            if !members.is_subset(expected) { return Err(Block::DomainMemberOutsideScope); }
            match d.impact {
                Impact::Component if members.len() < 2 => return Err(Block::InvalidDomain),
                Impact::Global if &members != expected => return Err(Block::GlobalDomainScopeMismatch),
                Impact::Unknown => return Err(Block::UnknownImpact),
                _ => {}
            }
            domains.push((inv.owner.clone(), d.clone()));
        }
    }
    if seen != required { return Err(Block::MissingOwner); }
    Ok(domains)
}

/// Quarantine the union of unresolved seeds' transitive dependency closure.
/// Any incomplete/unknown owner view returns GlobalHold. This function does
/// not cancel motion, reconcile effects, authorize actions or dispatch work.
pub fn qualify(
    p: &Profile,
    s: &Scope,
    outcomes: &[ActionOutcome],
    owner_views: &[OwnerInventory],
) -> Decision {
    if !valid_profile(p) { return hold(s, vec![], Block::InvalidProfile); }
    if s.scope_id.trim().is_empty() || s.actions.is_empty() {
        return hold(s, vec![], Block::InvalidScope);
    }
    if s.expected_action_count != s.actions.len() || s.actions.len() > p.max_actions {
        return hold(s, vec![], Block::ActionCountMismatch);
    }
    let Some(expected) = action_set(&s.actions) else {
        let invalid = s.actions.iter().any(|a| a.action_id.trim().is_empty() || !nz(&a.ticket_digest));
        return hold(s, vec![], if invalid { Block::InvalidAction } else { Block::DuplicateAction });
    };

    let mut result = BTreeMap::new();
    for o in outcomes {
        if !expected.contains(&o.action) { return hold(s, vec![], Block::ForeignOutcome); }
        if result.insert(o.action.clone(), o.outcome).is_some() {
            return hold(s, vec![], Block::DuplicateOutcome);
        }
    }
    if result.len() != expected.len() { return hold(s, vec![], Block::MissingOutcome); }

    let seeds: Vec<_> = s.actions.iter().filter(|a| matches!(
        result.get(*a),
        Some(Outcome::Unknown | Outcome::PartialEffect | Outcome::AppliedUnreconciled)
    )).cloned().collect();
    if seeds.is_empty() {
        return Decision::NoUnresolvedEffects { scope_id: s.scope_id.clone(), action_count: s.actions.len() };
    }

    let domains = match inventories(p, s, &expected, owner_views) {
        Ok(d) => d,
        Err(reason) => return hold(s, seeds, reason),
    };

    // Each hyperedge is traversed at most once; all its exact members join
    // the closure. Deterministic ordered indexes make receipts reproducible.
    let mut incident: BTreeMap<ActionRef, Vec<usize>> = BTreeMap::new();
    for (i, (_, d)) in domains.iter().enumerate() {
        for a in &d.members { incident.entry(a.clone()).or_default().push(i); }
    }
    let mut held: BTreeSet<_> = seeds.iter().cloned().collect();
    let mut queue: VecDeque<_> = seeds.iter().cloned().collect();
    let mut visited = BTreeSet::new();
    let mut traversed = BTreeSet::new();
    while let Some(a) = queue.pop_front() {
        if let Some(edges) = incident.get(&a) {
            for i in edges {
                if !visited.insert(*i) { continue; }
                let (owner, d) = &domains[*i];
                traversed.insert(format!("{owner}:{}", d.id));
                for member in &d.members {
                    if held.insert(member.clone()) { queue.push_back(member.clone()); }
                }
            }
        }
    }
    Decision::ComponentQuarantine {
        scope_id: s.scope_id.clone(),
        seeds,
        outside_quarantine: expected.difference(&held).cloned().collect(),
        quarantined: held.into_iter().collect(),
        traversed_domains: traversed.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const D: Digest = [1; 32];
    const E: Digest = [2; 32];

    fn a(id: &str) -> ActionRef {
        ActionRef { action_id: id.into(), attempt: 1, ticket_digest: D }
    }
    fn p() -> Profile {
        Profile { profile_id: "p1".into(), profile_digest: D,
            required_owners: vec!["state".into(), "resource".into(), "observer".into()],
            max_actions: 16 }
    }
    fn scope(xs: &[ActionRef]) -> Scope {
        Scope { scope_id: "mission-7".into(), expected_action_count: xs.len(), actions: xs.to_vec() }
    }
    fn domain(id: &str, impact: Impact, xs: &[ActionRef]) -> Domain {
        Domain { id: id.into(), kind: DependencyKind::State, impact, digest: E, members: xs.to_vec() }
    }
    fn inv(owner: &str, s: &Scope, ds: Vec<Domain>) -> OwnerInventory {
        OwnerInventory { owner: owner.into(), scope_id: s.scope_id.clone(),
            expected_action_count: s.expected_action_count, actions: s.actions.clone(),
            expected_domain_count: ds.len(), digest: D, completeness: Completeness::Complete, domains: ds }
    }
    fn views(s: &Scope, state: Vec<Domain>, resource: Vec<Domain>, observer: Vec<Domain>) -> Vec<OwnerInventory> {
        vec![inv("state", s, state), inv("resource", s, resource), inv("observer", s, observer)]
    }
    fn outcomes(xs: &[ActionRef], seeds: &[&str]) -> Vec<ActionOutcome> {
        xs.iter().map(|x| ActionOutcome { action: x.clone(),
            outcome: if seeds.contains(&x.action_id.as_str()) { Outcome::Unknown } else { Outcome::NotApplied }
        }).collect()
    }
    fn parts(d: Decision) -> (Vec<ActionRef>, Vec<ActionRef>) {
        match d { Decision::ComponentQuarantine { quarantined, outside_quarantine, .. } => (quarantined, outside_quarantine),
            other => panic!("expected component, got {other:?}") }
    }
    fn global(d: Decision, want: Block) {
        match d { Decision::GlobalHold { reason, .. } => assert_eq!(reason, want),
            other => panic!("expected global hold, got {other:?}") }
    }

    #[test]
    fn transitive_closure_crosses_owner_domains_and_preserves_disjoint_action() {
        let xs = vec![a("a"), a("b"), a("c"), a("d")]; let s = scope(&xs);
        let vs = views(&s, vec![domain("ab", Impact::Component, &xs[0..2])],
            vec![domain("bc", Impact::Component, &xs[1..3])], vec![]);
        let (held, outside) = parts(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs));
        assert_eq!(held, xs[0..3]); assert_eq!(outside, vec![xs[3].clone()]);
    }

    #[test]
    fn hyperedge_connects_every_member_and_multiple_seeds_union_closures() {
        let xs = vec![a("a"), a("b"), a("c"), a("d"), a("e")]; let s = scope(&xs);
        let vs = views(&s, vec![domain("capacity", Impact::Component, &xs[0..3])],
            vec![domain("fixture", Impact::Component, &xs[3..5])], vec![]);
        let (held, outside) = parts(qualify(&p(), &s, &outcomes(&xs, &["a", "d"]), &vs));
        assert_eq!(held, xs); assert!(outside.is_empty());
    }

    #[test]
    fn global_hyperedge_holds_exact_scope() {
        let xs = vec![a("a"), a("b"), a("c")]; let s = scope(&xs);
        let vs = views(&s, vec![domain("authority", Impact::Global, &xs)], vec![], vec![]);
        let (held, outside) = parts(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs));
        assert_eq!(held, xs); assert!(outside.is_empty());
    }

    #[test]
    fn isolated_seed_leaves_proven_disjoint_actions_outside_closure() {
        let xs = vec![a("a"), a("b"), a("c")]; let s = scope(&xs);
        let vs = views(&s, vec![], vec![], vec![]);
        let (held, outside) = parts(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs));
        assert_eq!(held, vec![xs[0].clone()]); assert_eq!(outside, xs[1..]);
    }

    #[test]
    fn missing_owner_forces_global_hold() {
        let xs = vec![a("a"), a("b")]; let s = scope(&xs);
        let mut vs = views(&s, vec![], vec![], vec![]); vs.pop();
        global(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs), Block::MissingOwner);
    }

    #[test]
    fn unknown_owner_inventory_forces_global_hold() {
        let xs = vec![a("a"), a("b")]; let s = scope(&xs);
        let mut vs = views(&s, vec![], vec![], vec![]);
        vs[1].completeness = Completeness::Unknown;
        global(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs), Block::UnknownInventory);
    }

    #[test]
    fn incomplete_domain_count_forces_global_hold() {
        let xs = vec![a("a"), a("b")]; let s = scope(&xs);
        let mut vs = views(&s, vec![domain("ab", Impact::Component, &xs)], vec![], vec![]);
        vs[0].expected_domain_count = 0;
        global(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs), Block::IncompleteInventory);
    }

    #[test]
    fn wrong_attempt_in_domain_forces_global_hold() {
        let xs = vec![a("a"), a("b")]; let s = scope(&xs);
        let mut stale = xs[1].clone(); stale.attempt += 1;
        let vs = views(&s, vec![domain("stale", Impact::Component, &[xs[0].clone(), stale])], vec![], vec![]);
        global(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs), Block::DomainMemberOutsideScope);
    }

    #[test]
    fn unknown_domain_impact_forces_global_hold() {
        let xs = vec![a("a"), a("b")]; let s = scope(&xs);
        let vs = views(&s, vec![domain("unknown", Impact::Unknown, &xs)], vec![], vec![]);
        global(qualify(&p(), &s, &outcomes(&xs, &["a"]), &vs), Block::UnknownImpact);
    }

    #[test]
    fn missing_duplicate_and_foreign_outcomes_force_global_hold() {
        let xs = vec![a("a"), a("b")]; let s = scope(&xs); let vs = views(&s, vec![], vec![], vec![]);
        let missing = outcomes(&xs[0..1], &["a"]);
        global(qualify(&p(), &s, &missing, &vs), Block::MissingOutcome);
        let mut duplicate = outcomes(&xs, &["a"]); duplicate.push(duplicate[0].clone());
        global(qualify(&p(), &s, &duplicate, &vs), Block::DuplicateOutcome);
        let mut foreign = outcomes(&xs, &["a"]); foreign[1].action = a("elsewhere");
        global(qualify(&p(), &s, &foreign, &vs), Block::ForeignOutcome);
    }

    #[test]
    fn malformed_global_domain_and_duplicate_domain_id_force_global_hold() {
        let xs = vec![a("a"), a("b"), a("c")]; let s = scope(&xs);
        let partial_global = views(&s, vec![domain("g", Impact::Global, &xs[0..2])], vec![], vec![]);
        global(qualify(&p(), &s, &outcomes(&xs, &["a"]), &partial_global), Block::GlobalDomainScopeMismatch);
        let duplicate = views(&s, vec![
            domain("same", Impact::Component, &xs[0..2]),
            domain("same", Impact::Component, &xs[1..3])
        ], vec![], vec![]);
        global(qualify(&p(), &s, &outcomes(&xs, &["a"]), &duplicate), Block::InvalidDomain);
    }

    #[test]
    fn no_unresolved_effects_is_not_a_release_capability() {
        let xs = vec![a("a"), a("b")]; let s = scope(&xs);
        let mut os = outcomes(&xs, &[]); os[1].outcome = Outcome::ReconciledEffect;
        assert_eq!(qualify(&p(), &s, &os, &[]), Decision::NoUnresolvedEffects {
            scope_id: "mission-7".into(), action_count: 2 });
    }
}
