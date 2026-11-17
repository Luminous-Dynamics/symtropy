# Living World Information Promotion Provenance V0

Status: normative design contract; documentation only.

## Purpose

A process becoming information-hungry does not cause missing exact information to exist.

A richer representation may be sufficient *if available* yet unreachable from the current canonical state because a previous coarse representation discarded the required relationship/history.

## Core invariant

> A lossy representation cannot recreate discarded exact canonical information by deterministic assertion. Every newly available fact in a promotion carries explicit provenance and retains its true evidence class.

## Provenance classes

V0 distinguishes conceptually:

### R0 — retained / revealed exact

The exact information already existed canonically under another owner/store and is exposed operationally without semantic reconstruction.

### R1 — lossless exact derivation

The new exact fact follows uniquely from retained canonical state under a versioned deterministic transform.

### R2 — qualified closure reconstruction

Approximate/derived information is reconstructed under recognized closure evidence. It remains closure evidence and MUST NOT be upgraded to Exact merely because the reconstruction is deterministic.

### R3 — new measurement / assimilation

A qualified observation/measurement transaction introduces stronger evidence after commit.

### R4 — conditional active microstate derivation

Level-A D-state fills previously unresolved degrees of freedom under a qualified conditional model. It is future-bearing once used, but it is not retroactive historical K-state.

Only R0/R1/R3 may introduce new exact K-state, with explicit provenance.

## Transition graph

Fidelity changes are versioned authority transitions rather than arbitrary type conversions.

Each transition edge SHOULD state:

- source representation/schema;
- destination representation/schema;
- source scope/revision requirements;
- information preserved exactly;
- information discarded;
- newly exposed/derived/measured information;
- provenance class for each newly available capability;
- closure/evidence requirements;
- deterministic transform/version;
- settlement/ownership effects;
- reversibility and collapse implications.

## Marginals -> strata example

Independent exact age, condition, and occupancy marginals do not contain historical age×condition×occupancy covariance.

Therefore this is forbidden:

`marginals -> deterministic zip/sample -> call resulting strata Exact`

Legal alternatives include:

- reveal retained latent exact strata (R0);
- use a genuinely lossless derivation where one exists (R1);
- use a registered reconstruction closure if the process permits it (R2);
- acquire new qualified measurements (R3);
- keep/defer/refuse the process requiring unavailable exact covariance.

## Level A

Marginal-source Level-A realization may derive unresolved individual state as D-state, but that does not upgrade the source population to exact covariance.

Strata-source realization may preserve its actually occupied joint stratum as K-state because that relation already existed canonically.

## Collapse

Any information-losing collapse records what becomes unrecoverable and whether an exact richer owner remains elsewhere.

A later promotion may claim R0 restoration only when that exact information was retained under authority. Otherwise it must use a weaker evidence class or reject.

Repeated promotion/collapse MUST NOT ratchet D-state or closure reconstructions into K-state merely through repetition.

## Sufficiency vs reachability

These are separate predicates:

- **destination sufficiency**: would this representation/context satisfy the process if it existed?
- **transition reachability**: can current canonical state produce that destination under a legal provenance path?

Canonical fidelity selection considers only candidates satisfying both.

## Qualification fixtures

- marginals + exact joint requirement: strata schema is sufficient but exact transition unreachable without another source;
- pseudo-strata sampled from marginals rejects Exact joint requirement;
- same pseudo-strata may satisfy only a compatible registered closure requirement;
- retained hidden strata promote exactly and preserve original joint state;
- measurement enrichment creates exact K-state only after measurement commit;
- collapse-discarded covariance cannot later be losslessly restored unless retained elsewhere;
- failed promotion leaves source unchanged;
- stale source revision invalidates prepared transition;
- renderer/camera/FPS cannot create provenance/evidence.

## Relationship

This contract governs the edges between authority representations. It complements collapse admissibility, K/D/M provenance, process sufficiency, deterministic fidelity selection, and atomic process activation.

Relates to #170, #191, #210, #252, #258, #263, #270, #271, #272, #280.
