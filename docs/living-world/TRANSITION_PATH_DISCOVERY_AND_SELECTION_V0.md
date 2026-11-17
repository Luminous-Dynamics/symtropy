# Living World Transition Path Discovery and Selection V0

Status: normative design contract; does not by itself claim executable qualification.

## Purpose

A deterministic graph search is not automatically an ecological policy. In particular, the shortest representation-transition path by edge count may discard more information, require unavailable evidence, use a weaker closure, or fail a state-domain theorem while another path is admissible and preserves stronger canonical truth.

This contract separates deterministic **candidate discovery** from canonical **semantic path selection**.

## Core theorem

**Structural graph search discovers candidate authority transitions. Canonical simulation selects among candidates only after each candidate has been evaluated for information sufficiency, information loss, evidence authority, state-domain applicability, spatiotemporal validity, and transition reachability. Graph distance is not ecological authority.**

## Candidate discovery

A candidate-discovery layer may enumerate or lazily discover simple paths under an explicit bounded search envelope.

Each candidate should expose at least:

- exact transition-policy authority identity;
- source/destination representation identities;
- ordered transition identities;
- cumulative discarded information;
- R0 retained-authority requirements;
- R1 transform requirements;
- R2 closure evidence/debt;
- R3 measurement requirements;
- R4 conditional derivation requirements;
- path length;
- applicability-domain descriptors once joined with applicability policy.

Stable enumeration order exists only for replay/inspection. It must not implicitly select a winner.

## Bounded search

Search must terminate deterministically and make incompleteness explicit.

If configured limits would hide a possible candidate, the search must either:

- prove that the hidden continuation cannot produce an admissible destination path; or
- fail closed and report that the candidate set is incomplete.

It must not return a known-partial candidate list under an API whose caller can mistake it for exhaustive discovery.

Cycles require explicit handling. For representation-fidelity planning, simple paths are the default V0 model unless a later qualified theorem establishes authority-relevant cycle semantics.

## Admissibility filtering

For every discovered candidate, resolve all relevant constraints before semantic selection:

- registered process requirements and exact policy-corpus identity;
- information/evidence sufficiency;
- spatial support, temporal freshness/cadence, aggregation semantics;
- snapshot-consistent composite authority context;
- transition-graph content identity;
- applicability-policy content identity;
- state-domain authority/proofs;
- R0 retained authority availability;
- R1 qualified transform authority;
- R2 closure observable/metric/horizon acceptance;
- R3 measurement authority availability;
- R4 conditional-state semantics;
- source/evidence freshness and revocation state.

Candidate evaluation is read-only. An inadmissible candidate is removed without mutating ecology.

## Canonical selection

Only a complete/admitted candidate set may enter canonical selection.

Selection is governed by an explicit versioned semantic policy. Policy may prefer, for example:

1. paths that preserve exact canonical information;
2. paths that avoid unnecessary R2/R3/R4 information debt;
3. paths that reuse retained exact authority rather than reconstructing or reacquiring it;
4. paths with less authority churn when ecological semantics are otherwise equivalent;
5. stable semantic cost/tie-break criteria defined by policy.

No preference above is universal unless the selected policy version says so. The invariant is that the preference is explicit, deterministic, replayable, and authority-bound.

Invalid canonical selectors include:

- camera distance;
- FPS;
- renderer quality;
- GPU/CPU load;
- host/device class;
- thread scheduling;
- discovery iteration order;
- map insertion order;
- wall clock.

Backend execution choice is separate from semantic fidelity selection.

## Example

Suppose two routes reach the same target representation:

Path A:
- 2 edges;
- loses exact covariance;
- later requires R3 measurement to reacquire enough information.

Path B:
- 3 edges;
- preserves exact information through qualified R1 transforms.

A shortest-path BFS chooses A structurally. Canonical fidelity policy must not infer that A is preferable from edge count alone. If policy prefers exact preservation and B's R1/domain evidence is fully qualified, B may be the canonical choice.

Likewise, an unsatisfied state domain on the shortest path must not hide a longer Universal/admissible alternative.

## Prepared-plan binding

A selected path must bind:

- exact process-set generation;
- exact source-state/context identity;
- exact information-policy corpus identity;
- exact transition-policy graph identity;
- exact applicability-policy identity;
- domain/transform/closure/measurement authority used to admit the path;
- semantic selection-policy identity/version/content;
- selected ordered transition sequence;
- selection inputs and deterministic tie-break result.

Any authority-relevant change after PREPARE makes the selection stale.

## Qualification fixtures

A compliant implementation should demonstrate:

- two-edge lossy vs three-edge exact path -> exact-preserving policy chooses the qualified exact path;
- shortest path needs unavailable R3 while longer path uses retained R0 -> retained path remains selectable;
- shortest path fails domain proof while alternate Universal path passes -> alternate remains eligible;
- equal semantic scores resolve by a stable policy-defined tie-break;
- transition registration order does not alter selected path;
- graph cycles terminate under the admitted search model;
- candidate bound/edge bound cannot silently hide alternatives;
- no admissible candidate -> explicit refuse/defer with zero mutation;
- source/process/policy drift after selection -> stale.

## Non-goals

This contract does not require enumerating every path in arbitrarily large graphs, does not state that highest detail is always preferable, and does not permit performance pressure to silently weaken canonical ecological semantics.