# Orbital Service Receipt Admission Benchmark v0.1

**Status:** synthetic exact-byte receipt-admission reference; no producer signature or physical truth is authenticated.  
**Owner:** [ORBITAL-SERVICE-RECEIPT-ADMISSION-001 #6384](https://github.com/Luminous-Dynamics/symtropy/issues/6384).  
**Parents:** capacity reconfiguration #6379 and end-to-end service composition #6221.

## Paradigm shift

A planned or qualified capacity is not an operation. A command is not an observation. A source-side quantity moved is not receiver-accepted mass. Only the exact receiver-accepted quantity from a correctly scoped composition receipt may satisfy a service floor.

This crate consumes:
1. The exact serialized capacity-plan receipt emitted by #6379, binding provider selection and qualification to each event tick.
2. The exact serialized composition CLI receipt emitted by #6221, binding delivery → capture → receiver-accepted transfer.

It does not reimplement the three-stage composition logic, fluid-transfer physics, robust interface evaluator, or provider reconfiguration algorithm.

## Admission rules

- Hash exact receipt bytes before parsing; digest mismatch is Rejected.
- Pin exact #6379 and #6221 validator commits.
- Verify the #6221 CLI/bundle schemas, pass disposition, synthetic claim ceiling, no-authority flags, three upstream source commits and four artifact digests.
- Join receiver/client, interface ID/revision, fluid ID/class/phase and fluid-transfer event ID.
- Bind each receipt to the provider selected by the capacity plan at the event time, plus exact configuration and lifecycle effectivity.
- Keep event time and observation/receipt time separate; a late receipt may be admitted if it arrives before finalization and was valid at event time.
- An identical idempotent replay counts once. Conflicting reuse of an event ID or idempotency key is Disputed.
- A causal predecessor must exist and be earlier in event time; missing causal evidence is Indeterminate.
- Explicit correction/supersession replaces the prior receipt for mass accounting while retaining both records and counting only the corrected version.
- Unknown, Disputed, missing, or malformed evidence never becomes accepted service.
- All statuses preserve `authority_granted=false`.

## Frozen corpus

The 34-case synthetic corpus covers complete receipt chains, partial accepted mass, missing and late receipts, exact-byte tampering, source/schema/identity mismatch, receiver/interface/fluid mismatch, duplicate and conflicting replay, causal order, provider configuration effectivity, supersession, and authority-negative controls.

## Metrics

Report receiver-accepted mass in grams, required floor, unique counted receipt count, duplicate replays, superseded receipts, missing operation ticks, and outcome disposition. Do not use provider capacity or gross source-side transfer as accepted receiver mass.

## Baselines

Compare on the same exact plan/receipt bytes:
1. capacity-only planning proxy (#6379);
2. gross transferred mass;
3. arrival-time-wins receipt ingestion;
4. event-ID-only duplicate suppression;
5. exact-byte, idempotent, event-time/effectivity-aware receiver receipt admission (this crate).

## Claim ceiling

A pass establishes only synthetic receipt-admission and accepted-mass accounting semantics. It does not authenticate producers, verify real signatures, establish source-byte provenance beyond the supplied hash, validate measurement instruments, prove physical seals, qualify hardware, guarantee availability, or grant mission authority.
