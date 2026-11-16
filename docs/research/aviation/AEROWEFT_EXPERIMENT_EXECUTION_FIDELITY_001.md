# AeroWeft — Experiment Execution Fidelity and Evidence Completeness

**Tracking issue:** #5670  
**Consumes:** Track V / #5394, Track T / #5362, Track U / #5371, Track P / #5337, Track Q / #5351  
**Status:** synthetic-first structural run verifier  
**Claim ceiling:** exact planned-vs-actual execution and data-window accounting only.

## Research question

Did the actual experiment run execute the exact approved Track V design, under the applicable authority and envelope, with the actual model/aircraft/control/sensor/reference configuration expected, and does the captured evidence account for every required observation interval?

A selected experiment is not an executed experiment. A result file is not proof that the approved procedure, environment, configuration, measurement system and initial conditions were used.

## Paradigm shift: four immutable stages

Do not represent experiment work as one mutable PASS record. Keep four distinct artifacts:

1. **Planned** — Track V candidate, selection policy, experiment design, procedure, scenario, measurement plan and required data windows.
2. **Authorized** — responsible authority's scope, conditions and validity interval, supplied independently.
3. **Executed** — actual configuration, actual environment, actual time interval, deviations, aborts and capture records.
4. **Adjudicated** — structural execution assessment, followed by Track T metric/model validation and Track U applicability/requalification.

The planned values are never overwritten with actual values. Deviations and missing intervals remain visible.

## Ownership and composition

Track W owns the aviation experiment-run contract, actual-vs-planned conformance oracle, window/segment completeness accounting and synthetic adversarial fixtures.

It composes:
- Track V #5394 — decision-relevant experiment selection;
- Track T #5362 — exact model/configuration/reference/domain credibility;
- Track U #5371 — bitemporal change impact and requalification;
- Track P #5337 — subject-scoped causal evidence graph;
- Track Q #5351 / Mycelix #5899/#5895/#5863/#181 — authenticated custody, authority, revocation and evidence leases;
- Symtropy #460 — sole canonical physics qualification corpus;
- #3749 — analytic/manufactured numerical oracles;
- #4366 — semantic equivalence for cross-implementation comparisons;
- #4446 — differential fuzzing and minimized counterexamples.

Track W does not implement a simulator, test executor, aircraft controller, trust registry, authorization service, generic event store, lease/currentness primitive, or physics qualification corpus.

## Frozen execution contract

The plan binds exact Track V candidate and selection-policy digests, experiment-design and procedure revisions, initial-condition/scenario manifest, measurement plan, physical/model/control/sensor/solver/reference configuration, operating-domain profile, permitted run interval, allowed environmental/traffic strata, authority scope and expected observation windows.

Every expected data window binds:
- source and stream identity;
- half-open time interval;
- minimum independent sample count;
- measurement/calibration profile digest;
- exact calibration receipt digest;
- time scale, coordinate frame and vertical datum.

The actual run separately records the selected candidate, plan digests, actual configuration, realized strata, actual start/end, authority evidence, deviations, abort, capture build and raw-data segments.

Each raw segment binds:
- segment ID and source stream sequence;
- exact time interval and sample count;
- payload digest string;
- raw-data manifest digest;
- capture receipt digest;
- calibration receipt digest;
- measurement profile, time scale, frame and datum.

The verifier checks digest syntax and exact equality only. It does not recompute raw bytes, authenticate receipts, verify signatures, or establish issuer authority.

## Status semantics

- EXECUTION_CONFORMING — exact plan/actual match, authority valid, all windows complete, no deviations or abort.
- EXECUTION_DEVIATED — data windows complete but documented noncritical deviation exists.
- ABORTED — run was terminated; full-run claim cannot be promoted.
- INDETERMINATE — authority/deviation impact cannot be established.
- EVIDENCE_INCOMPLETE — one or more planned windows are partial, absent or below sample floor.
- NOT_ADMISSIBLE_FOR_CLAIM — candidate, policy, design, procedure, configuration, authorization, time, domain or evidence basis does not match.

The assessment retains mismatch dimensions, deviation IDs, abort state, incomplete-window IDs, per-window covered duration/sample count and whether the full planned claim is structurally supported.

## Hard invariants

- Exact candidate, selection policy, design, procedure, scenario and measurement-plan identity.
- Actual aircraft/model/control/sensor/solver/reference configuration matches the approved plan.
- Actual run start/end is inside the permitted interval; required authority scope and validity cover the entire run.
- Realized environment/traffic strata are a subset of the approved strata.
- Every expected window is accounted for as complete, partial/missing, aborted or otherwise explicitly unresolved.
- Missing/censored intervals never become observed-good intervals.
- Every segment has a valid digest-shaped raw payload, capture receipt, raw-data manifest and calibration receipt reference.
- Segments for a planned window use the exact calibration receipt and measurement profile.
- One planned window cannot silently combine multiple raw-data manifests.
- Duplicate segment IDs, stream sequence reuse, source-time inversion and overlapping segments fail closed. Planned windows on one source stream must also be non-overlapping. Within an expected window, later captured segments after a gap remain in the captured-time/sample totals, but do not close the gap or make the window complete.
- A fresh wrapper cannot freshen stale raw observations; Track Q/Mycelix owns authenticity and time evidence.
- Aborts and adverse runs remain in the campaign record; no favorable-only filtering.
- Input/segment permutation does not change the canonical result.
- Partial runs do not support the full planned claim by default. A reduced claim requires a separate explicitly scoped plan and assessment.

## Benchmark matrix

The Rust suite includes positive complete-run controls; contiguous segment assembly; candidate/selection-policy/design/procedure/scenario/measurement-plan mismatch; model, sensor, control and reference changes; unknown/denied/expired/wrong-scope authority; run-time and environment-envelope deviations; documented/unknown/out-of-envelope deviations; abort; absent/partial/low-sample windows; capture/calibration/raw-manifest receipt substitution; duplicate IDs, sequence forks, time inversion and overlapping segments; unexpected streams; capture-build/policy mismatch; coverage monotonicity, deterministic segment permutation, later-segment accounting across gaps and non-overlapping expected-window constraints.

Prospective hard targets:
- 100% exact agreement with an independently authored finite execution-contract oracle.
- Zero wrong-scope, expired, denied or missing authority accepted as conforming.
- Zero configuration/procedure/measurement substitution accepted as the originally planned run.
- Zero missing/censored time counted as captured/valid observation time.
- Zero invalid capture/calibration/raw-manifest links accepted.
- 100% exact expected-window time and sample accounting on synthetic known-truth runs.
- 100% deterministic replay under input permutation.
- Zero aborted/adverse runs silently omitted from campaign denominators.
- Benchmark 10/100/1,000/10,000 runs and 10/100/1,000/10,000 segments per run; report p50/p95/p99 runtime, peak memory, accounting cost and mismatch-detection latency.

## Industry comparison

- **NASA-STD-7009B** establishes lifecycle M&S credibility practices and requires project-specific acceptance criteria approved by delegated technical authority. Track W complements this by binding what was actually executed to the approved intended-use evidence.
- **NASA-HDBK-7009B**, dated 3 February 2026, gives current implementation guidance for the standard; it distinguishes M&S capability assessment from results/use assessment. A conforming run does not by itself validate a model.
- **ASTM F3705-25** describes DAA test methodology for its stated small-UA/lower-risk crewed-traffic domain and notes that flight data may be needed to validate fast-time simulation models.
- **ASTM F3201-26** is a lifecycle software-dependability practice for UAS and third-party UTM/ADSP software, not a technical performance threshold.
- **NASA NESC-RP-23-01853** documents multi-simulation check-cases under common inputs, including configuration, reference-frame and implementation differences. Track W should preserve exact run inputs/actuals so such comparisons remain defensible.
- **FAA DAA CMD**, revised 11 March 2026, requests test methodology, actual aircraft types, environmental conditions, anomalies, tracking/detection accuracy, false alerts, avoidance manoeuvre details and closest point of approach. Track W's run-record design makes those fields traceable but is not a substitute for FAA submission or approval.

References:
- NASA-STD-7009B: https://standards.nasa.gov/standard/nasa/nasa-std-7009
- NASA-HDBK-7009B: https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009
- ASTM F3705-25: https://store.astm.org/f3705-25.html
- ASTM F3201-26: https://store.astm.org/f3201-26.html
- NASA NESC-RP-23-01853: https://ntrs.nasa.gov/citations/20240013559
- FAA DAA CMD (revision 2026-03-11): https://www.faa.gov/uas/public_safety_gov/public_safety_toolkit/All_Purpose_CMD-DAA.pdf

## Implementation and claim ceiling

The dependency-free Rust crate is tools/aeroweift-execution-fidelity/. Its positive result proves only that a supplied synthetic execution record is structurally consistent with a frozen run contract and evidence-completeness profile. It does not prove physical execution occurred, authenticate source bytes/receipts, validate a model, establish safety, authorize a test or demonstrate regulatory compliance.