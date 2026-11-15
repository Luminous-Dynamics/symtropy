# AeroWeft — Bitemporal Model-Evidence Change Impact

**Tracking issue:** #5371  
**Consumes:** Track T / #5362, Track P / #5337, Track Q / #5351  
**Status:** synthetic-first aviation-specific impact oracle  
**Claim ceiling:** deterministic impact classification for supplied dependency claims and change-feed coverage only.

## Research question

When a model, aircraft, sensor, control, solver, reference instrument or operating-domain dependency changes, which exact Track T claims remain applicable, become historical-only, require requalification, are contradicted, or become indeterminate because the change feed is incomplete?

The target is not a new "current" boolean. It is claim-specific, bitemporal reconstruction under explicit dependencies.

## Paradigm shift

A model qualification is not one monolithic PASS. It is a set of evidence claims with exact dependency identities and validity assumptions.

The result must preserve both:
- **effective-time view:** what was true for the aircraft/model environment at the requested operational cutoff;
- **knowledge-time view:** which change events were actually recorded and available by the requested evidence cutoff.

A late-recorded event may alter a later reconstruction without rewriting what was known to the evaluator at an earlier cutoff.

## Ownership boundary

Track U owns only the aviation dependency profile, bitemporal test cases, claim-impact output and minimum requalification mapping.

It composes:
- Mycelix #6219 — generic claim-specific evidence invalidation and targeted requalification benchmark;
- Mycelix #1087 — applicability and dimension-specific requalification;
- Mycelix #2631 / #1207 — temporal assertions, evidence epochs and causal ordering;
- Mycelix #181 — qualified monotone EvidenceLease, which may shorten a reusable horizon but cannot decide model applicability;
- Mycelix #2688 — ownership firewall: no duplicate generic lease/currentness;
- Track T #5362 — exact model/dataset envelope;
- Track P #5337 — causal receipt binding;
- Track Q #5351 — authenticated provenance and trust-policy evidence.

Track U does **not** create a generic event store, trusted clock, universal currentness field, generic EvidenceLease, trust root, cryptographic verifier or action/authority engine.

## Dependency dimensions

The initial V0 vocabulary covers exact identities for:
- aircraft configuration;
- dynamics/model artifact;
- control/guidance stack;
- sensor/receiver/calibration stack;
- solver/integration configuration;
- independent reference-measurement profile;
- operating-domain profile;
- navigation/time/frame/datum basis;
- evaluation policy/evaluator build.

Each claim binds only the dimensions it actually depends on, plus the owner/source namespace that reports changes for each dimension. Dependency relations are explicit, versioned profile data—not inferred from similar names.

## Bitemporal change event

Each synthetic event carries event ID, owner/source ID, source sequence, dependency kind, exact prior and successor digest, effective time, recorded time, change class, owner-declared evidence state and an event commitment string.

Only events visible under both requested cutoffs participate in historical event-integrity validation; a future-recorded malformed/duplicate event cannot rewrite an earlier as-known view. The evaluator does not authenticate or hash actual event bytes. OwnerQualified is an input state from the domain source contract; it is not a cryptographic guarantee. Source sequence orders causal digest succession; effective time and recorded/knowledge time independently determine whether each event is eligible at a requested cutoff.

Change classes:
- MaterialChange: dependent model evidence requires owner-defined requalification;
- CorrectiveChange: a prior source/reference statement is corrected and dependent claim is contradicted;
- Retirement: historical evidence remains preserved but is no longer relied on currently;
- Unsupported: no automatic positive transition is permitted.

## Coverage contract

Every dependency requires a matching coverage record with:
- source namespace and dependency kind;
- effective-time interval beginning no later than the original qualification time and ending no earlier than the requested effective cutoff;
- knowledge-time interval beginning no later than the original claim-recorded time and ending no earlier than the requested knowledge cutoff;
- recorded/availability time at which this coverage manifest itself became known;
- sequence interval;
- completeness state;
- owner-declared evidence state;
- source sequence interval containing each supplied event used in the assessment;
- manifest digest.

A missing, partial, unknown or unqualified coverage receipt produces an indeterminate dimension. Absence of a change event is not proof of no change unless the declared coverage is complete through both requested cutoffs.

## Per-claim output

The primary status is one of:
- STILL_APPLICABLE
- HISTORICAL_ONLY
- REQUALIFICATION_REQUIRED
- CONTRADICTED
- INDETERMINATE
- UNSUPPORTED_CHANGE_TYPE

The assessment also returns separate lists for:
- affected dimensions;
- requalification dimensions;
- contradicted dimensions;
- historical-only dimensions;
- unsupported dimensions;
- indeterminate dimensions;
- exact event IDs and evaluated successor digest snapshot.

A known contradiction takes precedence over unrelated missing feed coverage, while the unknown dimensions remain explicit. A known material change plus an unrelated coverage gap produces an indeterminate overall reliance status while preserving the known requalification dimension.

The original claim and its historical evidence are never mutated or deleted. Every assessment carries the exact claim package digest, both requested cutoffs and policy digest. Coverage manifests have their own recorded/availability time, so a future coverage correction cannot affect an earlier historical query.

## Frozen synthetic adversarial corpus

The Rust suite currently exercises 36 tests including:
- positive control with complete coverage;
- sensor-only recalibration leaves the model-artifact digest unchanged;
- model-only change leaves the sensor dimension unchanged;
- late-recorded event is absent from an earlier knowledge cutoff and present at a later one;
- scheduled but not-yet-effective event does not invalidate an earlier effective cutoff;
- missing/partial/out-of-frontier feed coverage, including sequence-frontier gaps and coverage that starts after the claim baseline;
- unqualified event input;
- corrective, retirement and unsupported change classes;
- broken and valid multi-step dependency digest chains;
- pre-claim changes incorporated in the qualification baseline;
- late-recorded correction and historical/current reconstruction;
- duplicate event IDs, source-sequence reuse/forks and duplicate coverage;
- event-array permutation invariance;
- future-recorded duplicate events and coverage manifests do not poison an earlier as-known query but are rejected once they enter the knowledge cutoff;
- future-effective and unrelated dependency coverage receipts do not poison the current claim's coverage assessment;
- assessment output binds the exact claim package, effective cutoff, knowledge cutoff and policy digest;
- claim-relevant coverage receipts are filtered by dependency/source and effective/knowledge overlap before structural validation;
- claim dependency permutation produces a canonical identical result;
- source-sequence digest succession remains deterministic even when a corrective event has an earlier effective time than its predecessor.
- evaluation before claim existence;
- coverage deletion monotonicity;
- known contradiction not hidden by an unrelated coverage gap;
- broken pre-claim baseline digest chains even when the final digest matches.

The fixtures are fictional and synthetic. The owner source state and coverage assertions are not authenticated by this crate.

## Hard targets

Prospective targets, not universal guarantees:
- 100% exact agreement with an independently authored bitemporal/dependency oracle;
- zero stale dependent claims remain STILL_APPLICABLE;
- zero complete, unrelated changes invalidate claims whose dependency sets are disjoint;
- zero missing/partial feed coverage becomes evidence of no change;
- exact historical-versus-later-knowledge replay for late events and corrections;
- 100% deterministic output under event-array permutation;
- zero duplicate/forked event identity or broken digest-chain acceptance;
- 10 / 100 / 1,000 / 10,000 claims crossed with 10 / 100 / 1,000 / 10,000 events, with p50/p95/p99 latency, memory, impacted-claim count and unnecessary-requalification rate after correctness gates pass.

## Claim ceiling

A passing result establishes only that the supplied synthetic claim dependencies, change events and coverage declarations were evaluated deterministically under this frozen profile. It does not prove the source event feed is complete, the event is authentic, the change is physically real, the model is valid, the aircraft is safe, or flight is authorized. Real deployment must integrate Track Q trust custody and Mycelix's bitemporal evidence owners.
