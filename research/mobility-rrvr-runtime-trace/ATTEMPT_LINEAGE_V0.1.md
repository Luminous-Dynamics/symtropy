# RRVR multi-attempt physical-effect lineage v0.1

**Implementation:** src/attempt_lineage.rs in this crate  
**Schema:** schemas/attempt_lineage_v0_1.schema.json  
**Known-answer fixture:** fixtures/attempt_lineage_three_attempts_v0_1.json

## Why this increment exists

The parent runtime projection v0.1 deliberately covers one original attempt and one recovery admission. It does not qualify the physical outcome of the recovery attempt or a second recovery cycle. This module addresses that gap as a separate, bounded reference contract: it consumes owner-issued per-attempt evidence records and verifies a linear history of up to 64 attempts for one stable logical effect.

The key invariant is that a retry is an edge between two distinct attempts, not a repetition of an old command. The old attempt must be proven to have had no physical effect, its persistence result must be a resolved Committed or DefinitelyNotCommitted state, and both the controller queue and every declared downstream execution-holding layer must have confirmed quiescence for the exact new boot/controller epoch. The physical NoEffect and downstream-quiescence claims must each carry their own fault-model-qualified observer quorum with disjoint declared common-cause dependencies.

## Attempt-lineage rules

For every record:

- logical_effect_id is stable across the complete chain.
- attempt_id is unique, and ordinal is contiguous from one.
- The first attempt has no parent; each later attempt names exactly the immediately preceding attempt.
- Every domain uses the existing OwnerEvidenceRef: its domain, owner, revision, evidence ID, status/verdict, asserted value, exact logical-effect/attempt/boot/controller scope, and raw-artifact identity/digest are carried together. The asserted value must exactly equal the typed outcome; a quiescence assertion also includes the exact target child boot/controller epoch in that value.
- Evidence IDs cannot be replayed elsewhere in the same lineage.
- Both boot and controller epochs must strictly increase at each recovery edge.

A parent can have a child only when all of these independently scoped facts hold:

1. Physical outcome is no_effect, with a confirmed physical-outcome verdict.
2. Mission resolution is no_effect, with a confirmed mission verdict.
3. Persistence is either Committed or DefinitelyNotCommitted, with a confirmed persistence verdict; InFlight and IndeterminateCommit remain blocked.
4. The controller-queue fence is quiescent, confirmed, and targets the exact child's boot/controller epoch.
5. The downstream graph is independently reported quiescent, confirmed, and targets that same child epoch.
6. Both NoEffect and quiescence have claim-specific observer quorums tied to this exact parent attempt/epoch and the same declared fault-profile revision; the two proof sets must be independent under the common-cause contract. The proof envelopes reuse the existing OwnerEvidenceRef type, and evidence IDs, claim IDs, proof artifacts and observation IDs cannot be replayed across attempts. A physical observer may be reused on a later attempt only with a new observation and raw artifact; its observer identity alone is not an observation.

The validator trusts the upstream observer-assurance owner to have performed its full dependency-graph, subject-self-report, calibration, placement, clock and fault-scenario assessment. It checks the supplied OwnerEvidenceRef/quorum shape, exact attempt scope, common-cause disjointness and replay constraints; it does not authenticate the owner or reconstruct the full physical dependency graph.

Applied, partial, unknown, disputed, and not_observed are never retryable parent outcomes. Committed, in_flight, or indeterminate_commit persistence is never sufficient for a retry edge. A fence for a different epoch is not transferable.

## What this does not do

This module returns an internal consistency summary only. It does not:

- issue, authenticate, persist, or consume a recovery permit;
- perform M1/M2, mission, ticket, binding, or live native-boundary authorization;
- authenticate owner identity, signatures, or raw artifact contents;
- prove the declared downstream inventory is complete;
- replace the observer-assurance owner or its common-cause evaluator (it reuses that owner’s proof types and validation functions);
- claim Rust/TLA+ refinement or physical robot safety.

The existing owners remain authoritative. This module does not introduce a second journal, queue, observer registry, persistence classifier, mission authority, or action API. The native adapter must continue to revalidate current authority and atomically consume its own one-shot permit at its existing linearization point.

## Known-answer and adversarial coverage

The synthetic three-attempt fixture covers:

- attempt 1: confirmed NoEffect, definite no-commit, controller and downstream quiescence for epoch 2;
- attempt 2: independently scoped confirmed NoEffect, definite no-commit, both fences for epoch 3;
- attempt 3: confirmed Applied terminal outcome.

Unit tests mutate the recording to cover indeterminate persistence, non-confirmed NoEffect, Applied/Partial/Unknown/Disputed/NotObserved parents, wrong ancestry, skipped ordinals, epoch regression, wrong fence target, duplicate evidence IDs, cross-attempt scope substitution, unqualified fences, empty/oversized chains, and unknown JSON fields.

These fixtures use synthetic receipts and digests. They are not robot measurements.

## Next formal increment

The next step is to lift the original TLA+ lifecycle model from its fixed attempt-1/attempt-2 variables to a bounded attempt sequence, then differential-test the Rust lineage validator against the model's reachable transitions. The model must include a crash after attempt 2 has been accepted and before/after its physical effect, and a second recovery admission only after attempt 2 independently satisfies the same NoEffect, persistence, and graph-fence predicates.

Until that refinement exists, this module is a separately tested reference contract, not a formal proof of multi-cycle restart safety.
