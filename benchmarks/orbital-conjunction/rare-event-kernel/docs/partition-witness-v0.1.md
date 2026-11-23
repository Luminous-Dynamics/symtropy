# Orbital partition witness: implementation contract v0.1

Status: research contract; not an implemented verifier, numerical certificate, or operational qualification.

Parent: [Symtropy #7115 — independently verify admitted/residual event-domain partitions](https://github.com/Luminous-Dynamics/symtropy/issues/7115)

Related foundations:
- [#6828 — decision-grade evidence certificate](https://github.com/Luminous-Dynamics/symtropy/issues/6828)
- [#6766 — independent residual event-mass certificate](https://github.com/Luminous-Dynamics/symtropy/issues/6766)
- [#6883 — admitted/residual composition contract](https://github.com/Luminous-Dynamics/symtropy/issues/6883)
- [PR #6884 — event-mass composition](https://github.com/Luminous-Dynamics/symtropy/pull/6884)
- [PR #6886 — affine residual verifier adapter](https://github.com/Luminous-Dynamics/symtropy/pull/6886)

## 1. Why this contract exists

The current rare-event kernel has a useful separation:

1. an estimator evaluates event mass in admitted chart regions;
2. an affine residual traversal bounds possible event mass outside those chart regions;
3. a composition primitive checks shared event/model/target/partition identities and combines the two mass contributions.

The composition primitive correctly requires a caller declaration that the admitted and residual regions are disjoint and exhaustive. It also correctly expects the admitted input to be an **unnormalised target event-mass interval**, not a probability conditional on the admitted region.

The remaining gap is not the addition formula. It is the evidence behind the partition declaration.

The current `AffineUnionResidualMassReport` exposes counts (visited, chart-covered, event-free and unresolved cells), a budget flag and an aggregate residual upper bound. It does not expose a replayable terminal-cell ledger proving which exact regions were assigned to which side. The caller-supplied chart boxes also do not, by themselves, prove proposal support or correct importance weights inside those boxes.

This contract adds the missing *structural witness* while preserving the numerical and physical claim boundaries of the existing kernel.

## 2. Mathematical object being partitioned

For v0.1, use the residual kernel's existing latent representation:

`Z ~ N(0, I_d)`

and finite root domain:

`D = product_i [-R, R]`.

The complement `D^c` is a separate tail stratum. Its target probability upper bound must be retained in the residual ledger; a finite partition of `D` is not a partition of the full probability support unless the tail is also accounted for.

For an event `E`, the required mass accounting is:

`P(E) = P(E ∩ A) + P(E ∩ U) + P(E ∩ F) + P(E ∩ D^c)`

where:

- `A` is the set of latent points assigned to the frozen admitted estimator;
- `U` is the unresolved in-domain set whose possible event mass is bounded by the independent residual verifier;
- `F` is the set proven event-free for every declared event component;
- `D^c` is the outside-root tail.

`P(E ∩ F) = 0` only when the event-free proof is valid for the complete event definition and model. The residual upper bound covers `U ∪ D^c`. The admitted estimator covers `A`. The partition witness proves that the in-domain leaves are assigned once; it does not prove that either numerical mass result is valid.

The final envelope is therefore conditional on four distinct things: the partition witness, admitted estimator validity, residual upper-bound validity, and target/model/event identity. None substitutes for another.

### Conditional probability is not admitted mass

The composable quantity is:

`m_A = P(E ∩ A)`.

An estimator that instead returns:

`p_A = P(E ∩ A | A)`

must also provide an independently qualified interval for `P(A)`; only then may the adapter form a compatible interval for `m_A`. Never add `p_A` directly to a residual event-mass upper bound. This distinction must be represented in types, not only in comments.

## 3. Canonical partition transcript

### 3.1 Reuse the existing subdivision semantics

The existing `CoverageCell` is built from finite lower/upper f64 bounds and carries a stable split path. `split_longest_axis` selects the longest axis, breaks equal-width ties toward the lowest axis index, and splits at the representable midpoint computed by the existing implementation. The left and right child cells share the split face.

The first witness format should record and replay this exact policy. It should **not** silently introduce a second dyadic splitter: the independent verifier must reconstruct the same cells that the residual-mass producer actually classified.

A path is an ordered sequence of split decisions:

- axis index;
- side (`Left` or `Right`);
- split-rule version.

The verifier starts from the root f64 endpoint bit patterns and replays every split using the pinned split algorithm. It checks that the recorded axis is the deterministic longest axis, that the midpoint is finite and strictly interior, and that the child bounds are exactly those produced by the split rule.

For canonical encoding, store f64 endpoints and split midpoints as their 64-bit IEEE-754 bit patterns (fixed-width hexadecimal or unsigned integers), not decimal JSON floats. Bind the root bounds, dimension, split-rule version and all path entries into the partition digest.

### 3.2 Leaf roles

Every terminal path must appear exactly once and have exactly one top-level role:

- `Admitted`: assigned to one canonical estimator owner. The owner includes a stable chart or chart-set ID and digest. The complete leaf must be contained in the declared chart validity region; partial intersection is not admitted and requires further subdivision or residual treatment.
- `EventFree`: every event component has a matching independently replayable event-free witness for this leaf.
- `ResidualBounded`: the leaf is not admitted and is not proven event-free; it has one residual-ledger entry with a conservative event-mass upper bound.
- `Unresolved`: the verifier cannot establish one of the preceding dispositions. This is not a zero-mass state. It is residual only if an admissible conservative mass bound is present; otherwise qualification is withheld.

For a disconnected event union, record the per-component reason for the top-level disposition. A cell may be chart-covered for one component and proven event-free for another. A chart for one event component must never suppress another component's possible event. This preserves the component-specific rule in the existing affine-union residual traversal.

Overlapping chart boxes do not justify counting the same leaf twice. Either select one deterministic owner and require that owner to evaluate the union event for that leaf, or define a canonical chart-set owner with explicit shared-realization union semantics. Do not sum component-wise probabilities as if event modes were disjoint unless that disjointness is separately established.

### 3.3 Structural completeness checks

The independent verifier reconstructs a prefix tree from the terminal paths and checks:

1. The root identity and root bounds match the event, model and target-measure records.
2. Every path is valid under the pinned split rule and every split is strictly representable.
3. No terminal path is a prefix of another terminal path.
4. Every internal path has both left and right descendants; no branch is omitted.
5. Every reachable terminal path has exactly one leaf role and a unique stable leaf ID.
6. No duplicate, unreachable, malformed or contradictory paths exist.
7. Each admitted leaf has complete chart containment evidence and one owner.
8. Every residual-bounded leaf maps to exactly one residual ledger item with matching path, bounds and target measure.
9. Every event-free leaf carries component-complete event-free witnesses.
10. The outside-root tail is separately identified and included exactly once.
11. All source digests and identities agree across the estimator, residual report, event definition, target measure and model revision.

For the standard-normal v0.1 case, the shared faces of two closed sibling boxes have zero probability under the continuous target measure. The witness must state that boundary-measure assumption. If a future target measure contains atoms or otherwise places positive mass on partition boundaries, the boundary must receive an explicit owner or a separate boundary-mass term; it cannot be discarded as measure zero.

## 4. Witness schema (logical, not yet implemented)

A canonical serialized witness should contain:

| Field | Purpose |
| --- | --- |
| schema/version | Reject unknown semantics instead of guessing |
| event identity + event-definition digest | Bind HBR, frame, time scale, horizon and event type |
| model identity + model digest | Bind dynamics and environmental inputs |
| target-measure identity + digest | Bind the latent distribution and coordinate map |
| root-domain digest + f64 endpoint bits | Bind `[-R,R]^d` exactly |
| split-policy ID/version | Bind longest-axis, tie-break and midpoint semantics |
| terminal leaves | Unique path, leaf ID, role and owner |
| per-component evidence | Chart containment or event-free witness identity |
| residual ledger | Per-leaf upper bound, report digest and budget status |
| tail evidence | Outside-root bound and source identity |
| producer identity | Provenance only; it is not the verifier |
| witness digest | Content-addressed canonical serialization |

Canonicalization requirements:
- sort map keys and terminal leaves by canonical path order;
- reject duplicate IDs before hashing;
- encode floats by raw bits, not locale-sensitive decimals;
- use an explicit domain-separation prefix and schema version in the hash preimage;
- include algorithm/build identity for the verifier and each numerical bound producer;
- do not hash mutable URLs or filenames as substitutes for content digests.

The witness is a compact proof transcript, not a claim that every upstream numeric bound is machine-certified.

## 5. Separation of producer and verifier

The estimator/proposal may propose charts and candidate ownership. It must not certify that its own charts cover all relevant target event mass.

The residual producer may emit a traversal ledger. The independent verifier must reconstruct the tree and check the ledger without calling the producer's classification routine. Reusing the same function for construction and verification is not independent verification.

Minimum separation for v0.1:
- separate implementation module and entry point;
- no shared tree-construction or role-assignment helper;
- verifier accepts only serialized witness bytes plus immutable referenced evidence;
- positive and mutation-negative corpora are checked in a clean process;
- the final certificate records exact verifier commit, build and input digests.

A second-language or formally verified verifier is a later strengthening, not a prerequisite for the first structural slice. The first slice must nevertheless be independently implemented and reviewable.

## 6. Required negative and metamorphic tests

The first implementation must include:

- valid one-dimensional root with one split and two correctly assigned leaves;
- omitted left child and omitted right child;
- duplicate left/right path and duplicated leaf ID;
- child path with the wrong split axis;
- invalid or non-interior midpoint;
- terminal path that is a prefix of another terminal path;
- unreachable leaf and unreachable internal node;
- one leaf assigned to both admitted and residual roles;
- one leaf with no role;
- admitted leaf only partially contained in a chart;
- changed chart digest with unchanged witness;
- overlapping charts that would double-count under naive component summation;
- residual report missing one residual leaf;
- mismatched event, HBR, time, frame, model or target-measure digest;
- absent or mismatched outside-root tail evidence;
- budget-exhausted traversal presented as complete;
- conditional probability passed as unnormalised admitted event mass;
- post-freeze proposal/target/witness mutation;
- shared-face boundary case for a continuous Gaussian and a separate test that refuses to discard boundary mass when the declared measure can place mass there.

Metamorphic checks:
- splitting one residual leaf into its two exact children and re-running the verifier preserves the represented union;
- changing a leaf role without changing its evidence causes rejection;
- reordering input records does not change the canonical witness digest;
- changing any root, event, model, target, chart, residual or tail input changes the relevant digest and invalidates composition.

Acceptance target on the finite known-truth corpus: **zero false partition qualifications and zero double-counted leaves**. These tests do not establish correctness over all possible inputs; they are necessary falsification controls.

## 7. Integration with the existing affine verifier

The current `AffineUnionResidualMassConfig` accepts component affine maps and component-specific chart boxes. Its report provides aggregate counts and a total residual upper bound. The current report does not include the per-leaf path/role ledger needed by this contract.

Implementation should therefore be incremental:

1. Add stable chart IDs/digests to chart metadata (preserve the existing public API through an explicit adapter if necessary).
2. Emit an immutable terminal-leaf transcript from the same traversal that computes the residual report. Record per-component disposition, not only the aggregate `chart_covered_cells` count.
3. Record each unresolved leaf's bound and the outside-cube tail as separate ledger entries.
4. Independently replay the transcript from the root and verify prefix coverage, path validity, leaf roles, chart containment and residual-ledger completeness.
5. Only after that structural verifier exists, adapt the admitted estimator to emit unnormalised event mass for the exact admitted leaf set.
6. Replace the caller-attested partition claim in event-mass composition with the independently verified witness result. Keep any migration path explicitly marked conditional on caller attestation.

Do not copy the existing traversal result's `chart_covered_cells` count into a proof field. Counts are useful diagnostics but cannot demonstrate which exact region was covered.

## 8. Numerical and physical claim boundaries

This structural witness does **not** yet qualify the numeric residual mass:

- the current standard-normal cell-mass upper bound is a floating-point density-times-volume calculation;
- the current cube-tail bound is a numerical evaluation of a mathematical union/Chernoff bound;
- the current README explicitly says these are not formally verified outward probability enclosures;
- the current affine arithmetic is a useful component implementation, but not a formally machine-verified proof.

Therefore v0.1 may certify **structural partition consistency conditional on referenced numeric reports**, not independently certify the probability upper bounds. A later numerical-qualification track must outwardly enclose CDF, density, volume, multiplication, summation and final cap, and must bind those verified values to the exact latent measure and cell geometry.

Likewise, a chart-contained leaf is not automatically covered by a valid importance proposal. Proposal support, complete mixture density, likelihood ratio, defensive component, sample dependence, stopping rule and confidence sequence remain separate estimator evidence. The witness must never convert chart containment into estimator qualification.

Finally, the latent target model itself may be inadequate. A correct partition under a standard-normal latent model does not prove covariance realism, cross-object correlation, force-model fidelity, environmental-model adequacy or completeness of the physical model family.

## 9. Benchmarks and promotion gates

Measure the verifier and the full evidence chain, not just traversal speed:

- verifier wall-time and peak memory versus dimension, terminal-leaf count and depth;
- witness bytes per leaf and canonicalization/hash cost;
- counts of admitted, event-free, residual-bounded, unresolved and rejected leaves;
- number of charts per component and overlapping-chart cases;
- residual upper-bound sharpness and tail fraction;
- admitted interval width, residual allowance and final composed interval width separately;
- false partition qualifications and double-counts on finite known-truth fixtures;
- end-to-end cost per independently qualified event-mass envelope, including chart discovery, proposal fitting, sampling, residual traversal, witness generation, independent verification, numerical refinement and replay.

Promotion gates:
- **Gate A — structural:** omitted/duplicated leaf mutations are always rejected; valid finite trees replay identically.
- **Gate B — assignment:** every leaf has exactly one owner and all component-specific evidence matches.
- **Gate C — numerical:** cell and tail bounds have independently qualified outward enclosures.
- **Gate D — estimator:** admitted target event mass, proposal support and statistical coverage are qualified.
- **Gate E — end-to-end:** composed interval is replayable and all model/event/target assumptions are exposed.

No 10x/100x compute-saving claim is made by this witness. Such a claim belongs to matched end-to-end comparison after all gates pass.

## 10. Current status

Implemented upstream:
- affine residual traversal with component-specific chart assignments and aggregate unresolved-mass accounting;
- event-mass composition with identity and partition-digest checks;
- an affine residual-report adapter;
- deterministic pathwise event-window union reduction.

Still missing:
- per-terminal-cell immutable ledger;
- independently implemented partition transcript verifier;
- admitted-estimator mass adapter with an explicit conditional/unnormalised type split;
- independently qualified numerical tail/cell bounds;
- full NASA PeakOverlapPos and adaptive temporal refinement parity;
- physical-model and operational qualification.

This document specifies the next structural increment. It does not change the qualification status of any existing PR or benchmark.
