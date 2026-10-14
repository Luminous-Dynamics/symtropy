# Physics Solver Composition and Fidelity Contract v0.1

Status: normative architecture contract / no runtime qualification claim

## Purpose

Define how Symtropy can borrow the strongest mechanisms from mature physics engines while remaining efficient, deterministic where required, and structurally lean.

This contract does not select one solver implementation. It freezes the distinction between physical authority, semantic solver profile, execution backend, specialized domains, and evidence.

## Core theorem

```text
physical authority
-> admitted semantic solver profile
-> qualified execution backend
```

not:

```text
hardware load / renderer state
-> whichever physics model is fastest
-> canonical future changes implicitly
```

## Rigid-body core

`symtropy-physics` should remain focused on rigid-body responsibilities:

- body and shape state;
- broadphase and narrowphase;
- contact manifolds;
- rigid constraints/joints;
- continuous collision detection;
- sleeping/islands;
- deterministic/replay semantics;
- solver configuration/profile identity;
- numerical diagnostics.

It should not become the implementation home for every soft-body, fluid, hydrology, terrain, thermal, fracture, orbital, or ocean equation.

## Specialized domains

Use dedicated composable domains for:

- deformables/cloth;
- local free-surface fluids;
- terrain/matter;
- hydrology;
- thermal/phase state;
- fracture/destruction;
- orbital/astronomical propagation.

Coupling must name the physical quantities and authority crossing each boundary: forces, impulses, mass, momentum, energy, geometry revisions, fluxes, material state, or explicit evidence/receipts.

## Semantic profile versus backend

A semantic profile defines the equations/approximations, solver semantics, timestep/cadence assumptions, applicability domain, and determinism/error contract that may affect canonical physical history.

A backend defines how an already selected semantic profile executes: scalar CPU, SIMD, threaded CPU, GPU, or another equivalent implementation.

Backend selection may depend on workload/hardware only when the candidate backends are qualified under the required equivalence contract for the same semantic profile.

Changing from exact to approximate physics is a semantic transition and requires explicit canonical policy. It is not a backend optimization.

## Canonical selection

Canonical physical fidelity may depend on:

- active physical processes;
- spatial/temporal resolution requirements;
- interaction/nonlinearity;
- consequential threshold events;
- retained information/state;
- solver applicability;
- closure/error horizon;
- explicit versioned policy.

It must not depend directly on:

- camera distance;
- renderer FPS;
- graphics quality;
- CPU/GPU load;
- thread completion order;
- wall-clock scheduling.

## Industry-donor policy

Borrow mechanisms, not assumptions.

Candidate ideas worth measuring include:

- independent-island parallel work and background preparation from modern game solvers;
- SIMD batching of contact/manifold work;
- block contact solving and controlled solver experiments;
- specialized GPU/deformable pipelines;
- hierarchical hydrology equation families;
- conservative coarse/fine flux reconciliation.

An imported technique earns promotion only through Symtropy's own invariants, differential tests, performance measurements, and authority model.

## Differential references

An external engine may act as a reference lane but is never hidden truth.

The existing Rapier bridge should become a real stepped reference runner under PHYS-DIFF-01 / #443. Identical declared scenarios should produce machine-readable native/Rapier observations and divergence reports.

A disagreement is evidence to investigate. It becomes a failure only when an explicit qualification profile defines a bound.

## Solver hardening

The current 10-box stack regression remains a valuable scaling failure. PHYS-SOLVER-02 / #444 should treat it as a coupled-constraint convergence problem rather than hiding it behind larger global iteration counts.

Candidate experimental profiles should be versioned and measured, including block-normal manifold solving, positional correction/relaxation variants, support-aware ordering, and friction coupling changes where justified.

No promoted solver change may regress already-earned contact-generation fixes.

## Performance

Performance work should proceed after numerical semantics are characterized.

Preferred direction:

1. measure broadphase/narrowphase/manifold/solver/CCD/island/allocation costs independently;
2. parallelize independent islands;
3. batch compatible contacts through SIMD/SoA layouts;
4. reuse scratch allocations and improve locality;
5. activate expensive backends only when workload thresholds justify overhead;
6. retain scalar/reference paths for qualification.

PHYS-PERF-03 / #453 owns this direction.

## GPU

Current GPU physics code contains prepared compute pipelines whose execution path is unfinished. GPU work must therefore remain an implementation candidate until real dispatch, synchronization, readback, parity/evidence, and workload thresholds are established.

PHYS-GPU-04 / #454 should complete that work behind qualified CPU semantics rather than creating a second physics model accidentally.

## CCD

CCD presence is not sufficient evidence of robust continuous collision handling.

PHYS-CCD-07 / #463 should define the protected motion/shape/timestep domain and attack small fast objects, thin barriers, rotational sweeps, moving barriers, multiple impacts, and dense contact interactions.

CCD activation should be deterministic from physical state/profile requirements and should remain selective so ordinary bodies do not pay unconditional cost.

## Deformables

The existing XPBD soft-body seed should remain a specialized domain.

PHYS-SOFT-05 / #456 should add production-oriented cloth/volumetric constraints, collision, self-collision where needed, and material/failure integration without moving deformable implementation into the rigid core.

## Local fluids and hydrology

Fluid and hydrology are specialized physical domains with their own scale hierarchies. The rigid solver may exchange forces/impulses with them, but it does not become a global Navier-Stokes or watershed engine.

The Earth Water Continuity contract in the sibling document defines water ownership and cross-fidelity conservation.

## Qualification corpus

PHYS-VALID-06 / #460 should become the machine-readable physics evidence surface.

A physics result should bind:

- exact code/tree;
- toolchain;
- scenario identity;
- semantic solver profile;
- timestep/substep/iteration policy;
- execution backend;
- initial/final state identity where available;
- numerical-health metrics;
- acceptance thresholds;
- evidence class.

Unit tests remain necessary but do not by themselves establish engine-wide maturity.

## Evidence hierarchy

Always preserve:

```text
feature present
!= regression test authored
!= regression test executed
!= exact-head qualification PASS
!= performance competitiveness
!= real-world engineering accuracy
```

Claims must stay inside the evidence actually earned.

## Sequencing

Preferred sequence from the current repository state:

1. PHYS-DIFF-01 / #443 — real Rapier differential runner;
2. PHYS-SOLVER-02 / #444 — diagnose and harden tall-stack scaling;
3. PHYS-VALID-06 / #460 — consolidate machine-readable qualification evidence;
4. PHYS-PERF-03 / #453 — parallel islands, SIMD, scratch/locality;
5. PHYS-CCD-07 / #463 — deepen CCD qualification;
6. PHYS-GPU-04 / #454 — finish GPU execution behind CPU/reference semantics;
7. PHYS-SOFT-05 / #456 — mature deformables as a specialized domain;
8. PHYS-FIDELITY-08 / #466 — generalize semantic-profile/backend selection across multiphysics.

## Non-goals

This contract does not require:

- replacing native physics with Rapier/Jolt/PhysX;
- one solver for every physical domain;
- global GPU execution;
- maximum fidelity everywhere;
- bit-identical results across every backend/profile;
- claims of industry-leading performance without benchmark evidence.

The target is a leaner and stronger engine:

> specialized, measurable physical solvers composed through explicit authority and conservation boundaries, with canonical semantics independent of presentation and hardware accidents.