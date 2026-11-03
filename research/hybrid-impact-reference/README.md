# Hybrid impact reference (V0)

This isolated Rust crate implements an analytical reference for the first impact
of a one-dimensional point mass falling toward a fixed plane under constant
gravity with Newton restitution.

It supports the Symtropy research issue
[PHYS-HYBRID-STRUCTURE-001](https://github.com/Luminous-Dynamics/symtropy/issues/3393).

## Model

With the plane at y = 0, positive y directed away from the plane, initial
height h > 0, initial velocity v0, and gravity magnitude g > 0,

    y(t) = h + v0*t - 0.5*g*t^2
    v(t) = v0 - g*t

Let s = sqrt(v0^2 + 2*g*h). The first positive impact time is
t* = (v0 + s)/g. A cancellation-resistant equivalent is used when v0 < 0.
The incoming velocity is v_minus = -s; Newton restitution gives
v_plus = -e*v_minus, with e in [0, 1].

The impact dissipation is D = 0.5*m*(1-e^2)*v_minus^2.

The ledger reports initial mechanical energy, pre/post-impact kinetic energies,
physical impact dissipation, restitution residual, and energy-balance residual.
Physical dissipation is kept distinct from numerical residual.

## Scope and evidence boundary

- Analytical, one-dimensional, first-impact reference only.
- Constant gravity, fixed plane, point mass, Newton restitution.
- No time-stepping integration, friction, coupled bodies, repeated impacts,
  event-capturing method, or physical experiment.
- This crate is not a production Symtropy solver and does not validate the
  general engine.
- Passing these tests would qualify only the analytic reference implementation
  against its stated equations and hand-checkable fixtures.

## Run

    cargo fmt --check
    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings


## Repeated impacts and Zeno classification

The optional repeated-bounce reference classifies the ideal one-dimensional
sequence without numerically stepping increasingly short flight intervals.

Let s0 be the incoming speed at the first impact and e the constant Newton
restitution coefficient. For impact k >= 1:

    incoming_speed(k) = s0 * e^(k - 1)
    outgoing_speed(k) = s0 * e^k

The time of impact k is

    t_k = t_1 + (2*s0/g) * sum(j=1..k-1, e^j)

The infinite-sequence regimes are distinct:

- e = 0: the first impact settles the body at the plane; there is no second
  distinct impact.
- 0 < e < 1: infinitely many impacts have finite accumulation time
  t_Z = t_1 + (2*s0/g) * e/(1-e).
- e = 1: impacts continue with constant period 2*s0/g and no finite-time
  accumulation.

For impact k, cumulative impact dissipation is
0.5*m*s0^2*(1 - e^(2*k)); the remaining post-impact kinetic energy is
0.5*m*s0^2*e^(2*k). Stable expm1-based forms are used near e = 1. If distinct
impact times become unrepresentable in f64, the state query returns the typed
TimeResolutionExhausted result rather than report coincident events as distinct.

This analytic Zeno classification does not choose a physically appropriate
post-Zeno continuation. Real systems may require compliance, deformation,
surface roughness, velocity-dependent restitution or another richer model;
that continuation is a separate modeling decision.


## Bracketed event-localization baseline

The event-localization module implements a separate bisection lane for the
constant-gravity point-mass guard y=0. The caller supplies a bracket [t_lo,
t_hi] where y(t_lo)>0 and y(t_hi)<=0. Since the free-flight parabola with a
positive initial height and positive gravity has exactly one positive root,
the interval contains the first downward impact.

The locator does not use the closed-form impact time internally. It reports:

- final time bracket and midpoint estimate;
- absolute guard residual at the midpoint;
- bracket width and iteration count;
- CONVERGED, EVENT_AT_LOWER_BOUNDARY, ITERATION_LIMIT, or
  TIME_RESOLUTION_EXHAUSTED status.

The numerical lane is compared to the closed-form first-impact solution in
tests and in the event-localization benchmark. It includes an exact step-end
event at either bracket boundary, just-before and just-after step-boundary
controls, an initially
downward trajectory, explicit iteration exhaustion and invalid-bracket
rejection. A later event-capturing or adaptive integrator should use the same
metric separation, not treat a robust bisection reference as an engine-wide
event-handling solution.


## Partial event-order relation

The event-order module compares two event-time intervals in a shared time basis.
For certified enclosures, it proves A before B only when A's upper bound is
strictly less than B's lower bound; the reverse applies for B before A. All
overlaps and touching endpoints remain unresolved.

The evidence grade is explicit:
- Certified enclosures can produce PROVEN_BEFORE or PROVEN_AFTER.
- Numerical estimates can produce only ESTIMATED_BEFORE or ESTIMATED_AFTER.
- Overlapping certified intervals produce UNRESOLVED_OVERLAP.
- Overlapping numerical estimates produce ESTIMATED_OVERLAP.

Even identical certified singleton intervals do not produce a simultaneous
impact claim from this pairwise API. Simultaneity needs an independent joint
event/contact certificate that binds all guards, a common state/time, the
joint impact law and its admissible solution set.

This avoids epsilon-based timestamp merging, callback-order assumptions and
false simultaneity from rounded f64 values. The comparison rejects empty event
IDs, incompatible time bases, invalid intervals and non-finite endpoints.


The comparator requires a non-empty external evidence reference on every event
record. It does not authenticate that receipt itself: callers must verify the
receipt and only set CertifiedEnclosure after that independent verification.
A numerical bracket from the bisection locator remains NumericalEstimate until
a separate sound interval method establishes a certified enclosure.


## Deterministic partial event-order graph

The event-order reference can build a bounded graph over a finite event set
(up to 256 records). Event IDs are sorted canonically before pairwise
classification, so graph output is independent of caller insertion order.

It separates:
- proof-grade order edges from disjoint certified enclosures;
- estimate-only order edges from disjoint numerical intervals;
- unresolved/estimated-overlap pairs, which remain explicit.

Only proof-grade edges enter the proof-order DAG; a cycle is a typed error.
The graph does not invent a total order or treat overlapping/singleton-equal
times as simultaneous. The graph cap bounds the O(n²) pairwise classification
cost. A later joint-event certificate must establish simultaneity and joint
reset semantics independently.


## Loss-aware hybrid-physics projection receipt

The `projection_receipt` module defines a consumer-side receipt for projecting
a Symthaea `ScientificModel.v0.2` with `HYBRID_DYNAMICS.v0.1` into a target
Symtropy model. It carries projection identity and loss accounting only; it
does not parse or redefine the source PhysicsIR and does not prove semantic
equivalence.

The receipt binds source model and hybrid digests, target model digest,
projection implementation/configuration digests, target schema, and an
independently extracted inventory of source semantic IDs.

Every source semantic ID must have exactly one ledger entry:
- PRESERVED: a target semantic ID is present;
- TRANSFORMED: a target ID and equivalence-evidence reference are present;
- APPROXIMATED: a target ID and uncertainty/bounds evidence reference are present;
- EXTERNALIZED: an external artifact reference is present, without a target ID;
- UNSUPPORTED: no target ID or evidence reference is asserted;
- UNPROVEN: a proposed target or external mapping may be recorded but remains
  explicitly unqualified.

The validator rejects missing/extra/duplicate ledger entries, stale source
model/hybrid digests, malformed digests, and treatment/field combinations that
would overstate preservation. `validate_source_binding` compares the receipt
against a separately extracted semantic inventory; the receipt's own list is
not accepted as its own authority.

A verification receipt binding is optional but must include both verifier
identity and receipt digest. This module checks presence and syntax only; it
does not authenticate the verifier or establish that a claimed equivalence
proof is sound.

Canonical bytes sort source IDs and entries and use length-prefixed fields.
They are deterministic and unambiguous; callers compute and record the chosen
cryptographic digest over those bytes separately.

This is the first receipt/coverage slice for PHYS-HYBRID-PROJECTION-001. It
does not yet implement a parser or the actual point-impact model lowering.


## One-dimensional Coulomb stick/slip branch oracle

The `frictional_contact` module is an algebraic reference for a point mass on
a horizontal plane under a tangential force. It implements the static-cone
criterion and positive/negative kinetic branches without time stepping.

At an exact static threshold, both the static response and incipient kinetic
slip response are retained:
- If `mu_static == mu_kinetic`, acceleration is continuous but kinked; the
  module returns both one-sided derivatives, 0 on the stick side and 1/m on
  the slip side.
- If `mu_static > mu_kinetic`, acceleration jumps at slip onset. The module
  reports the signed jump and deliberately supplies no finite single
  derivative. It never averages the stick and slip branch derivatives.

A caller-supplied absolute force tolerance is bound into deterministic model
identity bytes. If a floating-point force lies within that band but is not
exactly at the mathematical boundary, the result is `NearBoundary` and the
oracle abstains instead of promoting the input to exact equality.

The fixture distinguishes:
- strict interior stick;
- positive and negative overload/sliding;
- exact positive/negative threshold;
- continuous-kink versus discontinuous branch switching;
- frictionless zero-coefficient limit;
- unresolved near-boundary inputs and invalid parameters.

This is a branch-local algebraic oracle, not a Coulomb differential inclusion
solver, multi-contact solver, or physical validation result. It contributes
the frictional contact benchmark lane in PHYS-DIFF-CONTACT-GRAD-001 #3425 and
the stick/slip boundary work tracked by PHYS-FRICTION-BRANCH-001 #3436.


## Qualified gradient evidence receipt

The `gradient_receipt` module records a solver-independent, verifier-checkable
claim about a derivative produced by a contact-dynamics run. It does not
compute the derivative, authenticate the verifier, or prove the claimed
mathematics on its own.

A receipt binds exact digests for the physical model, forward trajectory,
objective, solver configuration, solver implementation, and gradient source.
It also carries a declared normalization identity and optional, explicitly
unknown residual/conditioning metrics.

Five structurally distinct result kinds are supported:

- `BRANCH_LOCAL`: exactly one derivative, branch identity, validity-region
  digest, and verifier binding.
- `DIRECTIONAL_SET`: at least two one-sided responses, each with a distinct
  direction ID and a branch/validity identity. Responses may share the same
  branch identity.
- `SET_VALUED`: at least two distinct branch identities, each retaining its
  own response and validity region.
- `SURROGATE`: one estimator response with explicit estimator identity and
  bias-evidence reference. It need not claim a certified branch or validity
  region.
- `UNAVAILABLE`: zero gradient responses and a typed abstention reason.

Event-sensitive exact/directional/set-valued receipts set
`requires_transversality`; they then require a strictly positive independently
established lower bound on the absolute transversality margin. A missing or
zero margin is not accepted as a qualified event derivative.

All available responses must use the same derivative-coordinate roster.
Every coordinate and reported metric must be finite; negative residual/error
metrics are rejected. A missing metric stays unknown and is encoded
differently from an explicit zero. Canonical bytes normalize response order
and coordinate order, but do not average branches or directions.

This schema is a binding and admission contract. The independent verifier
must still fetch the bound source artifacts, reconstruct the branch/direction
domain, and recompute the reported residuals. A verification reference stored
inside the receipt is not self-authenticating evidence.

The initial consumers are the analytical impact/saltation fixture (#3407), the
stick/slip branch oracle (#3438), the hard-contact gradient benchmark (#3425),
and the multi-branch sensitivity lane (#3358/#3365). It is not a gradient
engine or physical validation result.
