# SEL-07D1 Complete Randomization Reference V1

## Governing theorem

`qualified assignment criterion != executable assignment reference distribution != causal randomization test`

SEL-07C1 establishes that assignment-mechanism and interference evidence have qualified authorities. Those authorities remain opaque evidence bindings. SEL-07D1 adds a narrower theorem: for one exact randomized-interventional frame, those authorities have been materialized and qualified as a complete fixed-count individual randomization mechanism.

D1 produces no p-value, confidence interval, effect estimate, selection coefficient, fitness claim, adaptation claim, or speciation claim.

## V1 mechanism

V1 supports exactly one assignment model:

- persistent-individual assignment units;
- binary `Reference` / `Comparison` realized assignment;
- fixed observed Comparison count;
- every assignment vector over the exact estimand population with that Comparison count is in support;
- uniform complete-randomization support semantics;
- exact support size `C(N, n_comparison)`;
- maximum 64 estimand units.

There is no generic assignment DSL in V1.

## Required current authorities

Construction requires both:

- current `ValidatedSelectionAnalysisFrame<'_>`;
- current `ValidatedCausalSelectionIdentification<'_>`.

The identification must be:

- `InterventionIdentified`;
- `RandomizedInterventional`;
- bound to the exact frame and comparison design;
- targeted to `BinaryViabilityRiskDifference`.

Controlled-simulation identification is deliberately rejected. Simulation intervention reference distributions belong to a separate lane and must not masquerade as physical complete randomization.

## Exact assignment derivation

Callers do not supply the realized assignment vector or group counts.

D1 derives them directly from every row in the current validated analysis frame. Every estimand row must contain an observed binary predictor value. Missing or nonbinary values fail closed.

The persisted `realized_assignments` vector is canonical persistent-individual-ID order because the validated B1 frame itself requires that order.

Both Reference and Comparison groups must be nonempty.

## Complete fixed-count support

For `N` estimand individuals and `k` realized Comparison assignments, V1 defines the assignment support as all binary assignments containing exactly `k` Comparison labels.

The support size is exactly:

`C(N, k)`.

The persisted support size is locally recomputed during validation. A serialized model cannot change its support size independently of its population and Comparison count.

The 64-unit bound keeps `C(64,32)` inside `u64`; intermediate binomial construction uses checked `u128` arithmetic and fails closed on overflow or arithmetic inconsistency.

## C1 criterion bindings

D1 stores exact copies of the C1 records most directly relevant to executable randomization semantics:

- `AssignmentMechanism`;
- `AllocationIntegrity`;
- `TemporalPrecedence`;
- `InterventionFidelity`;
- `InterferencePolicy`;
- `AnalysisPopulationIntegrity`;
- `ProtocolFreezeProvenance`.

Each stored record must retain the exact C1 design/frame subject binding and the expected criterion type.

This redundancy is intentional. The full C1 identification digest remains bound, while the mechanism-critical criterion records remain directly inspectable in the D1 representation.

## Assignment materialization authority

`AssignmentMechanismMaterializationRef` is a separate authority stating that the opaque C1 assignment-mechanism evidence has been materialized as this exact complete fixed-count randomization model.

It binds:

- semantic authority ID;
- revision;
- content digest;
- exact C1 identification digest.

An authority bound to another identification cannot be reused.

This authority identity is an auditable trust binding. Evolution-core does not infer scientific competence or truth from the hash itself.

## Individual exchangeability authority

`IndividualAssignmentExchangeabilityRef` separately binds the claim that the qualified C1 interference policy permits individual-level assignment exchangeability under this complete-randomization reference mechanism.

It also binds the exact C1 identification digest.

This prevents D1 from silently assuming that individual permutation is valid when the relevant interference/assignment unit is a household, cluster, deme, colony, network neighborhood, or another structure.

Future cluster/block assignment models must encode their actual exchangeability structure explicitly.

## Assignment is not delivered exposure

V1's causal estimand is explicitly:

`IntentionToTreatBinaryViabilityRiskDifference`.

The persisted assignment vector represents randomized assignment, not necessarily received exposure or adherence.

The exact C1 `InterventionFidelity` criterion remains bound and visible, but D1 does not rewrite the randomized assignment vector according to treatment receipt.

`assignment != delivered exposure`.

Per-protocol, as-treated, instrumental-variable, complier-average, or other receipt-dependent causal estimands require separate identification and execution contracts.

## Timing

D1 binds the exact C1 `TemporalPrecedence` criterion rather than inventing a second informal timestamp rule.

The criterion remains an external qualified authority binding. D1 does not claim to independently reconstruct experimental chronology from a digest.

## Population integrity

The D1 reference binds:

- exact C1 identification digest;
- exact SEL-07A design digest;
- exact B1 frame digest;
- original denominator;
- estimand denominator;
- exact canonical realized-assignment unit set;
- exact Reference/Comparison counts.

The assignment population is not a convenient complete-case subset reconstructed at D1 time.

## Persisted representation versus current authority

`CompleteRandomizationReferenceModel` is serializable and has a deterministic canonical digest.

A serialized representation is not current assignment-reference authority.

`ValidatedCompleteRandomizationReference<'_>` is non-Serde and is reconstructed only by replaying:

- the current validated frame;
- the current validated C1 identification;
- a fresh assignment-mechanism materialization authority;
- a fresh individual-exchangeability authority.

The replay reconstructs the entire model and requires exact equality.

Therefore:

`serialized reference != current randomization-reference authority`.

## Local representation invariants

Before canonical identity is available, the persisted model must locally prove:

- supported reference version;
- exact V1 estimand/unit/support policy;
- randomized-interventional source design;
- binary viability causal target;
- nonzero valid denominators;
- at most 64 assignment units;
- canonical unique persistent-individual ordering;
- both groups nonempty;
- stored counts equal counts recomputed from the assignment vector;
- stored support size equals `C(N,k)`;
- mechanism-critical criterion fields carry their exact expected criterion types and design/frame subjects;
- D1 materialization/exchangeability authorities bind the exact stored C1 identification digest.

Current replay is still required after those representation-level invariants pass.

## Relationship to B2 and C2

D1 does not consume the numerical B2 descriptive p-value and does not define its support from observed outcome margins.

D1 also does not require the C2 causal point-effect magnitude to define the assignment mechanism.

The assignment reference is pre-outcome causal design structure.

SEL-07D2 may consume D1 together with the current C2/frame lineage to execute an exact sharp-null randomization test.

For the simplest 2x2 complete-randomization case, D2's resulting probability distribution may be numerically isomorphic to B2's fixed-margin Fisher/hypergeometric distribution. That equality must be demonstrated from the D1 assignment law rather than obtained by copying B2's p-value.

## Adversarial corpus

`complete_randomization_reference_v1` establishes at least:

1. two Reference / two Comparison units produce exact support size six;
2. the realized assignment vector is derived from the frame and binds the exact C1 assignment record;
3. Serde-restored models require fresh current replay;
4. reversing realized group labels changes model identity while preserving the six-vector support size;
5. D1 materialization authority from another C1 identification fails;
6. D1 exchangeability authority from another C1 identification fails;
7. fresh D1 authority drift stales persisted replay;
8. controlled-simulation C1 authority cannot mint a physical complete-randomization model;
9. altered serialized assignment vectors fail local count invariants;
10. wire shape contains no p-value, probability, effect estimate, selection coefficient, fitness, adaptation, or speciation claim.

## Explicit non-claims

SEL-07D1 does not establish:

- a causal p-value;
- causal significance;
- a confidence interval;
- average treatment-effect heterogeneity;
- compliance-adjusted effects;
- generic/lifetime fitness;
- population-genetic selection coefficient;
- beneficial/deleterious mutation status;
- adaptation;
- reproductive isolation;
- speciation.

It also does not independently validate the scientific truth of the external C1 or D1 qualification authorities.

## Successor

SEL-07D2 may execute one exact sharp-null randomization test only from current `ValidatedCompleteRandomizationReference<'_>` plus the exact causal/frame lineage.

No raw D1 digest, B2 p-value, C1 design label, or C2 point-effect value is sufficient by itself.
