# Navier–Stokes 2026 Validation Contract V0.1

Status: **measurement/provenance contract only**  
Tracking: #509, #510, #511, #512, #513, #514, #543

## Purpose

Symtropy may use the newly released 2026 forced three-dimensional incompressible Navier–Stokes finite-time breakdown result as an adversarial continuum-fluid **validation lineage**. It is not a turnkey CFD algorithm and it is not evidence that the current SPH scaffold reproduces the construction.

This document freezes the V0.1 claim boundary before any executable construction is attempted.

## Captured external provenance

Captured on 2026-09-10:

- release/title: *Finite Time Blowup for Navier–Stokes* / OpenAI's accompanying Millennium-problem publication, released 2026-09-08;
- public formalization repository: `openai/NavierStokesAndEuler`;
- exact captured formalization commit: `8937a8f4cbc7abaab5e9e97d1cc7f5d2319d9538`;
- formalization metadata at that revision reports `sorry_count: 0` for the named main results and describes its review status as **self-assessed**;
- represented theorem domains in the public formal artifact: whole-space `R^3` and periodic three-torus;
- both captured Navier–Stokes comparator adapters use zero initial velocity and smooth forcing for arbitrary positive viscosity;
- external status: primary/formal artifacts captured; independent review/community/Clay promotion not assumed by Symtropy;
- executable Symtropy fixture: **not available in V0.1**.

The manifest in `symtropy_fluid::validation::openai_2026_forced_navier_stokes_manifest()` is the machine-readable form of this capture.

## Provenance hardening rules

V0.1 deliberately fails closed on malformed provenance rather than treating metadata as friendly prose.

- dates are calendar-validated `YYYY-MM-DD` values, and `captured_on` may not predate `released_on`;
- benchmark identifiers, titles, source names, formalization repository names, fixture digests, domain lists, claim lists, and diagnostic-profile identifiers have explicit semantic size/count bounds;
- duplicate domains and duplicate qualitative claims reject;
- a domain-specific claim may reference only a domain actually declared by the manifest;
- a captured formalization repository requires an exact canonical 40-character lowercase hexadecimal commit and an explicit formalization review-status value;
- formalization review status cannot exist without the corresponding formalization artifact identity;
- an executable fixture digest cannot exist while `executable_profile = false`, and an executable profile cannot exist without a non-empty digest.

These are **post-deserialization semantic bounds**, not a hostile-network ingress guarantee. Any untrusted transport still needs an outer encoded-byte limit or a bounded decoder before constructing these values.

## Current implementation boundary

`crates/domains/symtropy-fluid` is currently an early SPH scaffold. Its density evaluation is an all-pairs reference implementation, and pressure/viscosity gradient forces remain placeholders.

Therefore none of these statements is currently permitted:

- “Symtropy solves incompressible Navier–Stokes.”
- “Symtropy reproduces the 2026 blowup construction.”
- “SPH resolves the mathematical singularity.”
- “The result proves ordinary Earth rivers/oceans develop this mechanism.”
- “A numerical NaN is evidence of a Navier–Stokes singularity.”

## What V0.1 does permit

The new validation vocabulary may be used to:

1. bind an external theorem/formalization artifact to an exact captured revision;
2. distinguish theorem provenance from an executable numerical fixture;
3. emit finite, unit-explicit/common continuum diagnostics where a backend genuinely supports them;
4. mark unsupported diagnostics unavailable without substituting zero;
5. distinguish under-resolution, CFL failure, divergence failure, solver non-convergence, non-finite state, and reference-domain failure;
6. build ordinary smooth/manufactured continuum benchmarks before attempting a singular-flow campaign.

## Numerical evidence hierarchy

The intended order is:

```text
zero/uniform-flow controls
        ↓
manufactured divergence-free forced solution
        ↓
known smooth viscous/vortex benchmark
        ↓
spatial + timestep convergence
        ↓
independent continuum implementations
        ↓
directly extracted 2026 construction
        ↓
resolution/control/falsification campaign
```

A solver that fails the smooth rungs is not ready for the singularity lineage.

## The executable-extraction gate

V0.1 sets `executable_profile = false` deliberately.

#510/#518 may change that only after a primary-paper audit has bound the exact construction needed by a numerical solver, including at minimum:

- source version/digest;
- theorem/domain variant;
- viscosity;
- time and coordinate normalization;
- initial velocity;
- smooth forcing;
- pressure treatment where required;
- whole-space truncation or torus periodization;
- boundary conditions;
- any discretization/interpolation/cutoff approximation;
- pre-terminal sampling schedule;
- source-derived qualitative/asymptotic expectations;
- immutable fixture identity.

No field may be reconstructed from a visualization or guessed from a secondary summary.

The current public Lean proof is theorem evidence, not an executable CFD generator. Its selected physical candidate passes through noncomputable/existence machinery, so numerical extraction remains a separate research task even though the theorem statements are formally represented.

## Continuum validity is not singularity detection

The V0.1 runtime-independent vocabulary contains:

```text
Resolved
UnderResolved
CflViolation
DivergenceFailure
SolverNonConvergence
NonFiniteState
ReferenceDomainViolation
BenchmarkProfileUnavailable
```

There is intentionally no `MathematicalSingularityDetected` state.

The critical distinction is:

```text
mathematical reference behavior
!= numerical instability
!= under-resolution
!= continuum-model applicability
```

A concentrating reference flow can be numerically mishandled in at least two opposite ways: excess numerical diffusion may keep it falsely smooth, while an unstable timestep/projection can blow the discrete state up for ordinary numerical reasons. #513 owns the falsification diagnostics needed to tell these cases apart.

## Refinement semantics

A continuum profile reporting `UnderResolved` does **not** imply `switch_to_particles`.

Depending on a separately qualified process/fidelity policy, a successor representation may be:

- finer continuum resolution;
- smaller timestep;
- adaptive mesh refinement;
- a different continuum discretization/order;
- LES/subgrid closure;
- VOF/level-set/free-surface treatment;
- FLIP/APIC/SPH for suitable local fragmented/free-surface processes;
- an explicit unresolved result;
- eventually a kinetic/molecular model where continuum assumptions themselves cease to apply physically.

The semantic choice is driven by required physical information and the profile validity envelope, never by camera distance or frame rate.

## Relationship to Earth hydrology

The new result does not change the established hierarchy:

```text
land/catchment closure
    ↓
1D channels and storage
    ↓
2D floodplain hydraulics
    ↓
local 3D continuum/free-surface refinement when required
    ↓
local particle/grid-particle detail when appropriate
```

Normal rivers, groundwater, floodplains and oceans should continue to use the cheapest qualified representation that preserves the requested physical observables and conserved transfers. Full 3D incompressible CFD is a local/specialized solver, not the global water model.

## Authority boundary

A local high-fidelity fluid solver is a temporary numerical representation of physical state, not an independent owner of world water.

Hydration/collapse and cross-boundary transfer must compose with the one-water authority and flux-reconciliation work (#362/#383/#448/#452). Mass, momentum, energy and declared tracers are settled exactly once across synchronization boundaries.

## Research evidence rule

Every future singular-flow campaign must retain raw diagnostics and exact run identity. A label alone is insufficient. Resolution, timestep, source fixture, solver profile, backend, domain, forcing, boundary policy or diagnostic algorithm changes create a new evidence lineage.

The benchmark earns its value by being difficult to fool: both false smoothness and ordinary numerical blowup are explicit competing hypotheses.
