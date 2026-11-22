# Mobility dependency-component quarantine protocol v0.1

Date: 22 November 2026
Tracking: MOBILITY-DEPENDENCY-QUARANTINE-001
Parent: Symtropy #6698; authority boundary #6669
Evidence status: reference contract and finite model only. No simulator, HIL, physical or operational result is claimed.

## 1. Objective

A physical outcome that is Unknown, Partial or AppliedUnreconciled must prevent new actions from relying on state that may have changed. Mission-wide hold is the correct fallback, but can be unnecessarily restrictive when every relevant owner can provide a complete dependency map.

The objective is proof-bounded quarantine: hold the exact transitive closure of unresolved action attempts only when all required owners provide a complete, exact-scope inventory. Any gap returns to global hold.

## 2. Paradigm shift: owner-qualified dependency hypergraph

A pairwise conflict graph is insufficient for shared capacity and multi-party invariants. Three actuators can each draw less than a power limit while their combined load exceeds it. An action can also affect a chain of state predicates, or share a calibration/observer failure domain without direct geometric overlap.

Represent:
- vertex: exact action ID, attempt and ticket digest;
- hyperedge: a complete owner-declared dependency domain containing all member actions;
- owner inventory: one owner's complete enumeration for the exact action scope, including an explicitly empty inventory;
- outcome seed: an action classified Unknown, PartialEffect or AppliedUnreconciled.

The quarantine is the union of all seeds' transitive hypergraph closures.

## 3. Scope and owner completeness

The request binds a non-empty scope ID, exact action attempts, expected action count and versioned profile. The profile lists a unique, non-empty set of required owner families.

Every required owner must provide exactly one inventory containing:
- matching scope ID and exact action references, including attempt and ticket digest;
- matching expected action count;
- explicit Complete status;
- matching expected domain count;
- nonzero inventory commitment;
- well-formed uniquely identified domains, or an explicitly empty domain list.

Complete is an owner assertion; this reference does not authenticate it. Production owners must define how completeness is established, including source coverage, hidden/shared dependencies and currentness. Public structural tests are not evidence that a real owner discovered every relevant dependency.

Each domain binds owner-scoped ID, kind, impact, digest and exact member references:
- Component: at least two actions; the domain connects all listed members.
- Global: membership must equal the entire exact scope; a seed touching it holds all actions.
- Unknown: impact cannot be bounded and therefore forces global hold.

An absent domain is meaningful only inside an explicitly Complete inventory. Missing inventory is not an empty graph.

## 4. Outcome classifications

Each action in the exact scope must have exactly one classification:
- NotApplied;
- ReconciledNoEffect;
- ReconciledEffect;
- Unknown;
- PartialEffect;
- AppliedUnreconciled.

Unknown, PartialEffect and AppliedUnreconciled are quarantine seeds. Missing, duplicate or foreign outcomes force GlobalHold. Reconciled does not authorize a retry or new action; current predicates, reservations, authority and native-boundary checks still apply.

## 5. Closure algorithm

1. Validate profile, scope, action identities and exact action count.
2. Require one outcome classification for every action.
3. Identify unresolved seeds. With no seeds, return NoUnresolvedEffects, which is not a scheduling or release grant.
4. Require exactly one inventory from every required owner.
5. Validate inventory scope, exact action set, counts, commitments and domain membership.
6. Build action-to-domain incidence indexes for every owner-qualified hyperedge.
7. Queue every unresolved seed. On visiting a new domain, add every member to the quarantine and queue newly added actions.
8. Return sorted closure, actions outside that closure and traversed domain IDs.

Each domain is traversed at most once. Ordered-map/set overhead gives approximately O((A + I) log A), where A is actions and I is total action/domain incidence.

## 6. Mandatory global fallback

GlobalHold is mandatory for malformed profile/scope/action identities; action-count mismatch; missing, duplicate or foreign outcomes; missing, duplicated, unexpected, unknown or incomplete owner inventories; action-attempt/scope mismatch; missing commitments; duplicate/empty/out-of-scope domains; unknown impact; and a Global domain that does not bind the entire scope.

The reference never drops an action or silently substitutes an empty dependency graph.

## 7. Execution and recovery boundary

Component quarantine means do not newly admit or rely on actions in that closure until reconciliation and requalification. It does not stop a motor, cancel an accepted command, reverse a physical effect or prove outside actions executable.

Every action outside the closure still needs:
- current M1/M2 action and attempt validation;
- full ticket/currentness recheck at #6669's native mutation boundary;
- current occupancy, motion-envelope and resource reservations from existing owners;
- local runtime assurance and physical interlocks;
- per-action independent effect evidence and reconciliation;
- fresh precondition checks against the post-outcome world state.

Global authority, clock, controller or platform invalidators must be declared as Global domains by their owning systems when they can affect the whole scope. No automatic retry is introduced.

## 8. Benchmark protocol

Compare on identical exact scopes, controller, policy, owner data, fault injections and evaluator:

A. Mission-wide serial hold after any unresolved outcome.
B. Pairwise-only isolation, diagnostic comparator only; never production.
C. Owner-qualified hypergraph quarantine with exact inventories and global fallback.

Use synthetic ground-truth graphs with evaluator-held labels, then matched simulator histories, then HIL/physical testing only after separate safety approval. Include disjoint components, long transitive chains, capacity/observer hyperedges, global dependencies, missing edges, omitted owners, stale attempts, unknown impact, partial effects, duplicates, restarts, clock changes and mixed failure domains.

Report non-compensatory metrics:
- Under-quarantine: truly dependent actions outside the closure. Hard target: zero in frozen adversarial controls and 100% agreement with exact evaluator closure.
- Containment precision: quarantined actions that truly depend on a seed. Initial hypothesis: at least 95% on the preregistered supported profile.
- Isolation coverage: truly disjoint components outside the closure when inventories are complete. Initial hypothesis: at least 95%; unknown or incomplete inventory must still cause global hold.
- Global-fallback correctness: 100% for missing, unknown, incomplete, malformed, stale-scope and global-impact dependencies.
- Inventory quality: owner coverage, incomplete rate, domain counts, incidence counts and independent source coverage.
- Runtime: latency and memory at 10/100/1,000/10,000 actions, including long chains and one giant hyperedge.
- Operational value: unnecessary held robot-seconds, verified throughput, makespan, rework, energy, intervention and reconciliation time.

A downstream execution target remains at least 20% median verified-makespan reduction on a pre-registered independent subset, with 25% as a stretch target on matched dual-robot TAMP tasks and zero hard-boundary violations. The 2026 dual-robot SAT/geometric-validation paper reports up to 19.8% simulation improvement and 20–25% on its own heterogeneous dual-Franka hardware tasks. That is a comparator, not a matched result; no superiority claim is valid without the same task, hardware, controller, policy, reservations, evidence and evaluator.

## 9. Current benchmark comparison

RoboCasa365 provides 365 household tasks across 2,500 kitchen environments and is useful for task diversity/generalization. SafeVLA-Bench demonstrates why success-only evaluation is inadequate: its September 2026 leaderboard reports many successful RoboCasa-365 trajectories that still violate task-aware safety specifications.

Those benchmarks measure policy/task behavior. This work measures dependency integrity and failure containment. They are complementary, not interchangeable scores. The goal is an assurance layer applicable to such rollouts and eventual execution traces, not a claimed higher manipulation or SafeVLA score.

References:
- Dual-robot SAT/geometric validation TAMP (2026): https://www.sciencedirect.com/science/article/pii/S0921889026004446
- RoboCasa365, ICLR 2026: https://proceedings.iclr.cc/paper_files/paper/2026/hash/a05003fdb1e9562ab0c0a9719ea4de10-Abstract-Conference.html
- SafeVLA-Bench leaderboard (updated 26 September 2026): https://safevla.org/leaderboard
- SafeVLA-Bench paper: https://arxiv.org/abs/2606.00773

## 10. Claim ceiling

This work can establish deterministic closure over supplied structurally complete owner inventories and fail-closed behavior for named malformed cases. It cannot authenticate owner evidence, prove real dependency discovery complete, cancel a physical action, reconcile effects, authorize new work, establish restart durability, real-time bounds, simulator fidelity, HIL, physical safety or regulatory approval.

Next gates: exact-head Rust/model checks; owner-specific completeness contracts and authentic inventories; history/reconciliation integration with #6669 and Mycelix #803/#830; evaluator-held graph mutation tests; matched simulation; HIL; supervised physical qualification. M1's global serial fallback remains unchanged until those gates are independently qualified.
