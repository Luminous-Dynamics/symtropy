# Symtropy Physics Phase-Zero Convergence Gate

## Purpose

The Physics Excellence Program defines the long-range target. This document defines the shorter prerequisite gate that must be cleared before broad feature expansion or competitive benchmark claims are treated as meaningful.

The governing rule is:

> **Do not optimize or compare a state transition whose authority, identity, lifetime, or accounting boundary is still ambiguous.**

A fast result is not useful evidence if the wrong world was stepped, an old body survived a reload, a reservoir silently appeared, a stale broadphase omitted a collider, or modeled heat came from a quantity that was never energy.

## Gate G0 — reproducible executable evidence

Required:

- one repository-declared Rust toolchain is authoritative locally and in CI;
- cheap formatting/invariant/license/Clippy/test preflight exists before the expensive platform matrix;
- job-level status is used as the evidence source rather than stale workflow aggregate state;
- every promoted physics layer receives an executable validation lane before it becomes the base for the next layer;
- validation PRs used only to trigger `pull_request -> main` CI are clearly marked **not for merge**.

Current evidence:

- PR #1 thermal foundation has passed the full macOS, Ubuntu, and Windows pipeline at head `39147c60ca15f297b90db23edaf7599031bc8384`.
- PR #2 has since gained stronger representability and replay contracts; validation run #47 on head `3331c489f84c0f4b30f195a719a4039a35d514d7` remains the authoritative pending gate for that newer foundation.
- PR #22 proposes a pinned, staged preflight workflow but is not itself yet established as the merged repository CI contract.

**Exit:** the current stack layer has real compiler/lint/test evidence, not static inspection alone.

## Gate G1 — authoritative world identity and lifetime

Required:

- `NetId <-> BodyHandle` is one-to-one and failed identity mutations are transactional;
- deterministic batch insertion cannot partially mutate before rejecting duplicate identity;
- static/kinematic insertion and removal invalidate the cached static broadphase correctly;
- body handles remain monotonic/non-reused within the declared world lifetime;
- body removal repairs handle indexes and NetId ownership;
- constraints, contacts, sensor/collision events, and both warm-start caches are pruned deterministically;
- body removal/scene clear cannot leave stale broadphase or replay-visible state;
- safe lifecycle APIs exist before unrestricted mutable world collections are made private/read-only.

Relevant work:

- #24 contact-cache eviction primitive;
- #25 generic constraint ownership predicate;
- #26 contact/event ownership predicates;
- #27 deterministic static-broadphase invalidation defect/regression;
- #28 / #29 NetId transaction and bijection work;
- #19 authoritative body/session lifecycle contract.

**Exit:** repeated add/remove/clear/rebuild campaigns remain bounded, deterministic, and free of stale identity or cache state.

## Gate G2 — reservoir identity and lifecycle provenance

Energy accounting requires more than numeric closure.

Current #10 reconciliation establishes:

- absent reservoir is **not** equivalent to a represented `0 J` reservoir;
- invalid/non-finite measured reservoirs cannot enter valid reconciliation evidence;
- internal ledger ports without represented state block full reconciliation;
- reservoir appearance/disappearance is detected explicitly and cannot pass `fully_reconciled`.

Still required:

- explicit provenance for thermal reservoir attachment, replacement, and detachment;
- body creation/removal provenance for every represented energy reservoir;
- representation-transition receipts when a physical region changes solver/state representation;
- displaced reservoir values returned or journaled rather than silently discarded;
- lifecycle operations declare whether energy stays internal, crosses the modeled boundary, changes form, or becomes an explicit numerical residual.

**Exit:** a reservoir may appear/disappear only through an explicit causal lifecycle operation whose energy/accounting consequence is auditable.

## Gate G3 — flagship runtime uses the physics engine as authority

A physics engine cannot claim production readiness while its own flagship application bypasses it.

Required:

- fixed-step player intent drives authoritative physics state;
- NPC intent drives the same authority path where the FEP feature is enabled;
- Bevy transforms are synchronized **from** physics after stepping rather than moved independently;
- map collision semantics are canonical and shared between generation, navigation, presentation, and physics;
- session replacement removes old ECS/physics/consciousness/AI episode state before spawning the next run;
- repeated Loading -> Playing -> MainMenu -> Loading cycles keep entity/body counts bounded;
- same-seed replay reconstructs the same initial authoritative state and resets episode-local stochastic/controller state;
- Old Waterworks remains described as a 2D horizontal proxy until a real `PhysicsWorld<3>` owns its collision scene.

Relevant work:

- #18 authoritative 2D runtime;
- #23 gameplay-session lifetime;
- #17 flagship runtime authority issue;
- #20 closed-loop FEP action experiment.

**Exit:** the game continuously dogfoods the same solver/lifecycle contracts being benchmarked.

## Gate G4 — canonical 3D angular dynamics

Required before off-center dissipation/contact work is treated as physical evidence:

- one documented Rotor/Bivector angular-velocity convention across orientation integration, point velocity, torque impulse, contacts, joints, and warm starts;
- finite-difference point-velocity tests agree with Rotor orientation evolution;
- full 3D principal/body inertia replaces scalar-mean inertia for asymmetric bodies;
- world angular momentum and rotational kinetic energy use orientation-dependent inertia;
- torque-free asymmetric-top cases preserve world angular momentum and show timestep-convergent energy behavior;
- contact effective mass uses the actual world inverse-inertia operator;
- N-D modes outside the validated 3D tensor implementation retain an explicit fallback/claim boundary rather than pretending full anisotropic support.

Relevant work:

- #16 standalone asymmetric-top reference and validation protocol.

**Exit:** off-center impulses and rotational energy can be measured without mixing convention error or scalar-inertia approximation into physical dissipation.

## Gate G5 — measured contact work, not impulse heuristics

The existing contact loop historically reports a friction-dissipation quantity proportional to `|j_t| * 0.1`. That is dimensionally momentum, not energy, and must never be routed into temperature.

Required:

- the heuristic is removed from any energy/thermal interpretation;
- centered dynamic-body friction uses measured pre/post kinetic state and exact ledger decomposition as established in #9;
- off-center friction waits for G4;
- static/kinematic boundary work has an explicit owner/reservoir model rather than disappearing from the closed pair;
- restitution, stabilization/bias, damping, clamping, and other solver corrections are separated from physical friction work;
- any unexplained mechanical loss becomes an explicit numerical residual, not physical heat by default;
- state-versus-ledger reconciliation from #10 is run around the authoritative world integration.

**Exit:** every joule converted from mechanical state to heat has a measured source reservoir and a typed causal mechanism.

## Phase-zero exit campaign

Phase zero is complete only when all of the following can run together:

1. create a deterministic world with static geometry, dynamic bodies, thermal reservoirs, identities, and constraints;
2. snapshot its authoritative state and accounting model;
3. execute player/NPC-like intent through fixed-step physics;
4. produce collisions including centered frictional dissipation;
5. reconcile measured reservoir changes against the causal ledger;
6. remove bodies and replace the gameplay session;
7. rebuild the same seed and prove bounded lifetime plus deterministic initial identity;
8. rerun under the documented determinism mode with no stale handles/caches/reservoirs;
9. fail deliberately injected negative controls: unjournaled heat, duplicate NetId, stale static cache, zero-energy reservoir appearance, non-finite state, and energy-injecting friction impulse.

Only after this campaign is green should the program treat broad competitive benchmarking, multicore optimization, GPU acceleration, or wider multiphysics as the primary engineering frontier.

## Execution order after the current stack

The recommended near-term order is:

1. finish executable validation of the current thermodynamics foundation;
2. close #27 static broadphase invalidation and #28 NetId identity integrity;
3. validate #24 -> #26 and implement `PhysicsWorld::remove_body` plus scene clear;
4. validate #23 repeated-session teardown and #18 authoritative 2D dogfooding;
5. validate #16 and migrate the canonical 3D angular convention/full inertia into production;
6. integrate #9 measured dissipation into the world solver and remove the heuristic;
7. run #10 reconciliation around real contact work;
8. then proceed to general convex/angular CCD, manifold torture, production APIs, deterministic parallelism, and matched engine benchmarks.

This order is intentionally less glamorous than adding another solver. Its purpose is to make every later result easier to trust.
