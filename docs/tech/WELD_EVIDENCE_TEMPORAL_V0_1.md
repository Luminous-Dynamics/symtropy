# Weld evidence temporal snapshot oracle v0.1

Tracking: [WELD-EVIDENCE-TEMPORAL-001 #4705](https://github.com/Luminous-Dynamics/symtropy/issues/4705).

## Why this is a paradigm shift

A historian query for "latest value" cannot answer what a planner legitimately knew when it made a decision. Evidence must have two independent clocks:

- **Event time:** when the measurement was made in a source clock domain.
- **Knowledge time:** when the governed system first received the evidence.

Validity time and decision purpose are additional constraints. Later NDT/repair findings can improve future models, but they must never leak backwards into a prior pass recommendation or benchmark replay.

The evaluator is the common temporal gate between the fast process, medium inter-pass, and slow quality/requalification loops. It is deliberately separate from process control, scan qualification, NDT acceptance, physical-model VVUQ and the next-pass optimizer.

## Implemented V0

The dependency-free Rust 2024 crate `weld-evidence-temporal` accepts synthetic integer clock ticks and symbolic digests only.

For decision cutoff T, a candidate must satisfy all of the following:
1. Its first-received/knowledge time is no later than T.
2. Its source event-time interval, after an authorized clock mapping, is wholly no later than T.
3. The mapping itself was received and valid by T.
4. The evidence validity interval contains T; upper validity bound is exclusive.
5. Workpiece, joint revision, layer and predecessor pass match exactly.
6. Source-byte and transform/registration receipt digests are non-zero and bound to the event.
7. The evidence explicitly authorizes the requested decision purpose.
8. Measured physical and simulation evidence are rejected until adapters are qualified.
9. A lifecycle supersession/revocation affects a historical snapshot only when it was both received and effective by T. A late-known action cannot rewrite an earlier snapshot.

Timestamp precision is represented as a conservative interval. If that interval straddles the decision cutoff, the event is indeterminate and excluded. Unknown or overly coarse precision never becomes a guessed order.

NDT dispositions, repair records, reinspection results and model-training artifacts are forbidden as FastProcess/MediumNextPass features. They can be used in SlowQualityRequalification, subject to the same point-in-time boundary. A model artifact whose training-data cutoff exceeds T is excluded from historical replay.

## Determinism and digest boundary

The evaluator sorts included evidence, exclusions and pending lifecycle IDs by stable identity and returns a canonical, length-prefixed transcript containing:
- transcript/version and policy revision;
- purpose, scope and decision cutoff;
- selected event IDs, mapped time intervals and exact source/transform/clock-mapping digests;
- excluded event IDs with explicit reason codes;
- lifecycle events known by T but not yet effective.

The crate intentionally does not implement SHA-256. The caller must hash the exact returned transcript bytes and retain that digest as the point-in-time snapshot receipt. This mirrors the digest boundary in #4852 and avoids misrepresenting a non-cryptographic hash as provenance.

## Baselines and frozen tests

The Draft 2020-12 adapter schema at `docs/tech/WELD_EVIDENCE_TEMPORAL_V0_1.schema.json` defines the event ledger, scope, source clock mappings, lifecycle actions, validity intervals, digest fields, purpose grants and decision request. The unit suite contains **33 Rust tests** and covers:
- bitemporal cutoff and late-arrival separation;
- future event-time and precision-boundary rejection;
- authorized clock mapping and mapping knowledge-time;
- exact workpiece/joint/layer/pass identity;
- source and transform digest presence;
- purpose authorization and outcome-label leakage;
- NDT/repair slow-loop eligibility;
- validity intervals, calibration activation and expiry;
- supersession/revocation known-time versus effective-time;
- future-trained model rejection;
- input permutation and canonical transcript determinism;
- duplicate identity and clock arithmetic overflow.

Required future benchmark baselines:
1. naive latest-value join;
2. event-time-only filter;
3. receipt-time-only filter;
4. sorted bitemporal reference evaluator;
5. indexed production evaluator.

All baselines must replay the same frozen event ledger and cutoffs. Report false inclusion, false exclusion, lifecycle semantics, determinism, replay equality, latency and peak memory. Benchmark at 10³, 10⁵, 10⁶ and 10⁷ events; do not optimize throughput before exact membership/leakage correctness is established.

## Claim ceiling

A PASS establishes synthetic as-of selection and deterministic transcript behavior only. It does not prove source clock accuracy, physical scanner calibration, welding quality, NDT acceptance, model validity, WPS/PQR/WPQT qualification or production release.


The canonical transcript includes full applied/pending lifecycle records, not just their IDs or resulting inclusion/exclusion set. Each record contributes kind, target, replacement, effective time and first-received time. Two distinct lifecycle histories therefore cannot collapse to the same replay receipt merely because both happen to exclude the same source event. The caller still computes SHA-256 over the returned canonical bytes.
