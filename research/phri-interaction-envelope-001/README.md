# PHRI interaction envelope reference

This crate is a dependency-free Rust reference for compositional admission in physical human–robot interaction (pHRI). It treats interaction safety as a conjunction of separately qualified evidence domains rather than a weighted score.

## Why this layer exists

Separate PHRI references now address variable-compliance passivity optimization, sampled-data energy observation, local contact-exposure qualification, wearer-governed adaptation, and multi-port energy boundaries. Those checks are valuable only when a candidate action is admitted against evidence that describes the same physical and control event.

This reference binds the domains to one shared identity:

- interaction and proposal sequence;
- sample timestamp, clock identity, and boot epoch;
- wearer/agency epoch;
- state-revision digest;
- control-profile digest;
- exact command digest;
- energy port-graph digest.

Admission is conjunctive. An energy surplus cannot compensate for excessive local contact exposure. Good contact metrics cannot compensate for missing wearer authorization. An optimizer receipt cannot override a stale observation, a revoked agency epoch, or absent actuator authority.

## Independent evidence domains

1. **Energy:** a caller-qualified residual upper bound, historical maximum prefix deficit, and explicit completeness assertion for the selected energy boundary.
2. **Contact:** caller-qualified upper bounds for local pressure, pressure-time exposure, deformation-rate proxy, and contact power, bound to calibration and sensor-profile digests.
3. **Agency:** explicit user authorization and a wearer epoch. Pressure/tactile data and inferred intent are not authorization.
4. **Freshness and identity:** common event identity, monotonic sequence/time, bounded age and sample gap, exact state/profile/command/port-graph agreement.
5. **Actuator authority:** a separate permit and authority epoch bound to the exact command.

The gate returns deterministic rejection reasons and does not mutate admission state on rejection. A successful admission creates an opaque, one-use receipt that must be revalidated with current evidence immediately before the caller's application boundary. Revocation latches the software gate and advances the agency epoch; re-arming requires an explicit, digest-bound user action.

## What this does not establish

This crate validates caller-supplied evidence records. It does not calibrate sensors, derive a conservative wrench/twist work bound, establish that an exposure upper bound is physically valid, prove multi-port completeness, schedule a hard-real-time loop, control an actuator, actuate a safety stop, establish human consent, or certify a robot.

The caller remains responsible for independent evidence production, measurement uncertainty, synchronized clocks, cryptographic receipt verification where needed, hardware safety channels, and the independent physical stop. The booleans in this reference are claims from those upstream authorities, not self-authenticating proofs.

Contact limits are deployment-profile inputs. They are not universal pain, injury, tissue-damage, or clinical thresholds. A non-human calibrated phantom is the appropriate next physical metrology step; human-subject work requires separate ethical and safety review.

## Tests and intended qualification

The unit tests exercise agreement, freshness, replay, authority, revocation, energy and contact limits, and pre-application revalidation. They are software conformance tests only.

The benchmark target is zero false admissions across the finite adversarial conformance matrix and at least 30,000 fixed-seed generated fault combinations, with false rejection reported separately. Runtime targets must be measured on declared target hardware and never inferred from host unit tests. The benchmark should publish p50/p95/p99/worst-case latency, gate-specific failure counts, evidence coverage, and the joint admission result.

## Related work

- #4738 / #4745: variable-compliance passivity and basis-free matrix-rate optimizer.
- #4778: sampled-data energy observer.
- #4787: expiring wearer-governed adaptation proposal gate.
- #4779: multi-port energy boundaries and cut sets.
- #4790: local contact exposure envelope.
- #4755: matched PHRI qualification benchmark.
- #4808: compositional interaction-envelope programme.

All results must preserve the distinction between authored tests, CI execution, numerical replay, calibrated physical measurements, and human-subject outcomes.
