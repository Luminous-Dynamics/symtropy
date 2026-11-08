# GroundAirBench v0

**Status:** initial composition contract and synthetic fixture; not an executed benchmark, physical model, operational permit or SOTA result.

## Purpose

GroundAirBench composes existing ground–air workstreams through references to immutable, typed upstream receipts. It does not duplicate EM propagation, pose estimation, session lifecycle, source-to-load metrology, site scheduling, aircraft energy modelling, EVSI or safety authority.

## Central invariant

An energy opportunity is admissible only when the upstream evidence required for its declared bearer, field configuration, receiver geometry, motion regime, environment and time interval is current and mutually consistent. Signal strength is not unique pose observability. A schedule is not proof of joint physical realizability. A simulated service metric is not electrical efficiency.

## Ownership

- #3448 / #3451: empirical sources and realization identities.
- #3472: source-to-load energy accounting and measurement uncertainty.
- #3473 / #3559: multi-receiver channel behaviour and robust dispatch.
- #3512 / #3640: coupling/load self-sensing and primary-side identification.
- #3980 / #3983: shared ground–air source and PCC coexistence.
- #4069: SAFE-DDPG and EA-RRT reproduction.
- #4080 / #4082: aerial session lifecycle.
- #4115 / #4121: receiver pose observability.
- #4141: standards applicability and hazard-to-evidence mapping.
- #3917 / #4027 / #4031: EVSI and finite-horizon passive measurement/stop policy.
- #3458: optical transfer physics and modality-specific safety.
- #3885 / #3902: aircraft energy architecture and docked charging.

GroundAirBench consumes receipts from these owners. It must not infer, repair or supersede their claims.

## v0 contract

- `ground_air_episode.schema.json` defines a closed episode manifest.
- `ground_air_episode.synthetic.json` is a synthetic shared-site example with one ground IPT opportunity and one RF hover opportunity.
- Every receipt is referenced by owner issue, artifact path, SHA-256, configuration digest, evidence class, disposition and validity interval.
- Cross-receipt ID resolution, validity intersection, role completeness, configuration consistency, energy closure and hard-gate semantics are evaluator responsibilities; JSON Schema alone cannot establish them.
- The synthetic RF opportunity deliberately carries an AMBIGUOUS observability receipt. A policy must not use its power proxy to admit it.

## Scoreboards

Report each lane independently: physical energy/metrology; pose observability; session lifecycle; mission service; fairness and asset health; safety/evidence; computation. Hard authorization, observability, local protection and physical constraints are non-compensatory. No single weighted score may hide a failed gate.

## Evidence classes

- `SYNTHETIC_ONLY`: invented known-truth values for evaluator conformance.
- `SIMULATED`: outputs from a declared model; not physical measurement.
- `BENCH_MEASURED`: calibrated, bounded laboratory evidence.
- `FIELD_MEASURED`: measurements from a declared field campaign.
- `INDEPENDENTLY_REVIEWED`: independent review of the identified artifact; does not automatically mean certification.

Evidence class and disposition are distinct. `SYNTHETIC_ONLY`, `AMBIGUOUS`, `STALE`, `OUT_OF_DOMAIN`, `UNKNOWN`, `REVOKED`, `NOT_ADMITTED` and `NOT_EVALUATED` never qualify as a physical operating permit.

## Benchmark protocol

1. Freeze scenario, source-catalogue, field, receiver-geometry, policy and upstream receipt digests.
2. Split by site, receiver/configuration, environment and mission family, not by random timesteps.
3. Use paired episodes/common random numbers where valid and identify the independent replication unit.
4. Reproduce direct published baselines or publish a machine-readable deviation ledger; do not tune with test partitions.
5. Report paired effects, 95% uncertainty intervals, seed/session distributions, tail service and hard failures.
6. Compare scalable policies against exact bounded oracles on small instances; exceeding an exact-search bound is an explicit failure, not a reason for silent approximation.
7. Keep synthetic, simulated, bench-measured and field-measured results in separate evidence lanes.

## Execution ladder

- L0: deterministic known-truth oracles.
- L1: synthetic system episodes.
- L2: software-in-the-loop and hardware-in-the-loop timing/interface validation.
- L3: non-flight physical receiver-sensing validation with calibrated pose ground truth.
- L4: controlled field/flight validation only after independent modality-specific safety, standards and authority review.

## Claim ceiling

This v0 is a composition schema and fixture only. It does not establish physical transfer, safety, standards compliance, flight authorization, continuous observability, robust control, energy efficiency or SOTA superiority. A schema-valid fixture is not a passing benchmark run.