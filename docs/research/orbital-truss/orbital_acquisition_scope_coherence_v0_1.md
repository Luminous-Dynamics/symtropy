# Orbital acquisition scope coherence v0.1

## Purpose

A set of individually valid evidence receipts is not necessarily a valid evidence bundle. A motion bound for camera A, a calibration for a previous lens, a support envelope for another asset revision, or a clock map from a different event timeline must not be composed just because each source reports PASS and includes a 64-character digest.

The cross-receipt contract therefore binds all owner evidence to one exact acquisition scope and one already-admitted canonical time basis.

## Owner boundary

This adapter does not duplicate:
- #7339 calibration validity and half-open calibration scope;
- #5631 clock conversion, uncertainty and event ordering;
- #6925/#7136/#7164 geometry, support and sampling;
- #7043 feature identity/conditioning;
- #6846 datum registration, covariance and conformity;
- #7384/#7399/#7422 acquisition-motion/time bounds;
- Mycelix custody, issuer identity and byte authentication.

It verifies that those owners' evidence claims are applicable to the same acquisition. The raw-clock conversion and source validity decision must already be made by the respective owner.

## Canonical scope

The acquisition descriptor identifies:
- acquisition/event ID;
- camera, sensor mode, lens/intrinsics revision;
- robot, mount/tool revision;
- target asset and revision;
- camera and asset coordinate frames plus transform convention;
- characteristic/feature and support-envelope ID;
- motion-model revision;
- calibration-scope ID;
- clock-map revision and canonical time basis;
- exposure profile and projector/pattern sequence.

Owner-specific required fields are explicit in the Rust contract. Each binding provides exact values for those fields. The adapter does not normalize semantically different frame names, infer revision ancestry or accept approximate string matches. A future schema version may introduce owner-defined canonical identifiers, but those transformations must be source-qualified and replayable.

## Time interval semantics

All acquisition and evidence validity intervals must already be expressed in the same canonical time basis from #5631. Raw camera, robot, projector and host timestamps are never compared directly by this adapter.

The acquisition interval is required to be strictly contained in the evidence validity interval [valid_from, valid_until). An interval with a different canonical time-basis identity, a start after acquisition start, or an end at/before acquisition end is OUT_OF_SCOPE. Invalid interval ordering is INVALID_INPUT.

Calibration validity semantics remain owned by #7339. This layer only checks that the upstream admitted interval belongs to the exact acquisition scope.

## Dispositions

- SCOPE_COHERENT: every required owner binding is present, current, complete, scope-identical and interval-applicable.
- OUT_OF_SCOPE: at least one known identity/revision/frame/feature/time-basis/validity mismatch.
- INDETERMINATE: missing scope fields or owner evidence, or any stale/revoked/contradicted/unknown source disposition.
- INVALID_INPUT: malformed or contradictory input, duplicated field/binding, unsupported claim field or invalid digest shape.

Precedence is fail-closed: malformed input dominates; a known mismatch dominates an unresolved field; unresolved evidence cannot be converted into coherence.

Every receipt states source_bytes_authenticated=false and authority_granted=false. The adapter shape-checks digest references and preserves them. It does not authenticate issuer signatures or retrieve and hash external source bytes.

## Frozen corpus

The 28 synthetic cases cover:
1. all identities and intervals match;
2. camera, lens/intrinsics and robot/mount mismatch;
3. target asset revision and coordinate-frame mismatch;
4. transform convention mismatch;
5. feature and support identity mismatch;
6. motion-model revision mismatch;
7. clock-map and canonical-time-basis mismatch;
8. acquisition/event identity mismatch;
9. individually valid calibration scoped to a different instrument configuration;
10. evidence interval is definitely disjoint (before or after) from the acquisition;
11. acquisition uncertainty envelope overlaps or touches the half-open calibration validity boundary (INDETERMINATE);\n12. exact zero-width acquisition-time interval at a single instant is allowed; zero-width evidence-validity interval is INVALID_INPUT;
12. missing feature/sensor-pattern scope;
13. duplicate scope claims and malformed digest shape;
14. revoked source;
15. digest-shaped but unauthenticated evidence;
16. missing required timing evidence owner;
17. extra, non-required source claim that contradicts the canonical acquisition identity.

Rust tests also reverse case order, claim order and binding order; the canonical per-case result must remain unchanged.

## Benchmark metrics and prospective gates

Report zero false coherent admissions on known scope mismatches; false OUT_OF_SCOPE on valid same-scope controls; INDETERMINATE rate for missing/revoked evidence; field- and owner-specific mismatch detection; exact-boundary interval behavior; canonicalization invariance; source receipt-reference preservation; deterministic replay and independent-oracle parity.

The primary prospective target is zero mismatched or incomplete bundles admitted as SCOPE_COHERENT. There is no score that allows a stronger match on one field to compensate for a mismatch on another.

## Claim ceiling

This is a synthetic structural composition check. It does not establish issuer authenticity, byte integrity, calibration validity, clock accuracy, physical capture, feature measurability, metrology conformity, POD/NDE, structural acceptance, safety or flight readiness.
