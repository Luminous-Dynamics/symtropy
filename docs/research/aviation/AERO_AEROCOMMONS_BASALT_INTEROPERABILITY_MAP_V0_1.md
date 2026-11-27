# AERO AeroCommons / BASALT owner-native interoperability map v0.1

**Status:** research adapter contract, pinned to upstream draft interfaces observed 27 November 2026  
**Scope:** compose owner-native evidence and freshness results into AERO's typed decision context and three-axis replay  
**Claim ceiling:** input mapping and evidence-state preservation only; no physical qualification, trust-root delegation, runner authentication or certification

## 1. Why this interface exists

AERO should not create a second evidence ledger, artifact store, dependency-invalidation engine, observer qualification system or PKI. The current owner-side work already establishes two complementary interfaces:

- Mycelix AeroCommons draft PR [#6347](https://github.com/Luminous-Dynamics/mycelix/pull/6347), head `6b160d92cce142b180d67f81837a1f32034444e1`, builds a typed component build packet around `AeroEvidenceV1`, configuration/physical-unit references, exact content artifacts and calibration validity windows.
- Symthaea BASALT draft PR [#9203](https://github.com/Luminous-Dynamics/symthaea/pull/9203), head `6d260c9f7075c1be04d8725ed711d4011270f1d4`, adds a Rust evidence-state timeline auditor with resource digest/generation pins, evidence dependency edges, direct/transitive staleness and a topological re-verification plan.

Both are draft interfaces and may change. This document is a version-pinned mapping proposal, not a claim that either PR has merged or is a production API.

## 2. Mycelix AeroCommons mapping

The upstream `aero-evidence-types` crate defines:

- `AeroEvidenceV1`: evidence identity, claim text/kind, configuration and subject references, basis, method, inputs, toolchain, result, uncertainty, validity domain, producer, review, lifecycle and provenance.
- `EvidenceLifecycle`: `Active`, `Superseded`, `Disputed`, `Revoked`, `Withdrawn`.
- `LifecycleEvent`: state, optional reason and optional predecessor.
- `ArtifactRef`: stable identifier and kind.
- `Uncertainty`: representation, optional value and unit.
- `CalibrationValidityV1`: calibration evidence reference plus `valid_from_unix_ms`, `valid_until_unix_ms` and `used_at_unix_ms`.

The draft component packet groups the physical-unit/configuration identity with design definition, requirements, process plan, material batch, machine/tooling, build evidence, inspection plan/instrument/calibration/result/evidence and test plan/instrument/calibration/result/evidence.

### 2.1 Field mapping into AERO

| Mycelix/AeroCommons field | AERO destination | Required handling |
|---|---|---|
| `evidence_id` | Owner-native mandatory evidence reference | Preserve exact ID and owning namespace; never mint a replacement ID |
| `subject.id`, `configuration.id` | `DecisionContextV0_1.subject_ref`, `configuration_ref` | Exact identity match required across all component reports |
| `claim`, `claim_kind` | Claim packet / claim-specification binding | Keep claim type and literal claim distinct from the derived review disposition |
| `basis`, `method`, `inputs`, `toolchain` | Claim dependency/evidence manifest | Preserve every reference; missing or unknown dependency is not silently treated as unaffected |
| `result`, `uncertainty`, `validity_domain` | Evidence payload and model/metrology context | Keep uncertainty representation separate from deterministic hard time bounds |
| `producer`, `review`, `provenance` | Owner, reviewer and provenance references | These are declarations until issuer/role/authentication evidence is verified by the owning trust service |
| `lifecycle.state`, `predecessor` | Event/lifecycle evidence | Do not treat current lifecycle state as historical as-known state without event effective-time, receiver-first-seen and trust-policy history |
| content artifact `sha256`, `byte_length`, `media_type`, `format_id` | Exact source-artifact commitment | Preserve bytes/digest/length/format; AERO does not rewrite or normalize source artifacts |
| `CalibrationValidityV1` | Certificate effective-time interval and acquisition-time comparison | Keep the half-open validity interval and the actual used-at point; do not infer clock accuracy or a hard acquisition bound from this structure |

### 2.2 Lifecycle mapping is deliberately conservative

- `Active` means the owner has declared the record active. It is not, by itself, proof of issuer authority, signature validity, receiver inclusion, measurement truth or suitability for the current claim.
- `Superseded` with predecessor provides a lineage edge. AERO's current correction semantics remain unqualified; map a supersession to an explicit review/abstention state until the owner supplies a qualified correction rule and receiver/effective chronology.
- `Disputed` must not enter an admitted positive evidence set. Preserve the dispute state and require owner adjudication; do not resolve by latest timestamp.
- `Revoked` is only applicable to a historical measurement if the revocation event's authority, effective interval and receiver-first-seen evidence are established against the selected query cut. A later current lifecycle state must not rewrite an earlier AS-KNOWN result.
- `Withdrawn` has no automatic calibration-revocation semantics in this adapter. Preserve it as an explicit non-positive state until the owner defines whether withdrawal is prospective, retroactive, administrative or evidence-only.

AERO must not coerce all non-Active lifecycle states into one generic Boolean. It should retain the distinct source lifecycle state, owner receipt ID, event identity, reason and predecessor.

### 2.3 Calibration and metrology boundary

Mycelix's current packet verifier checks that `valid_from < valid_until` and that the declared `used_at` timestamp lies in the half-open interval `[valid_from, valid_until)`. Its source comments explicitly state that declared timestamps are replay metadata, not proof of clock accuracy.

AERO's hard-bound extension should therefore:

1. preserve the Mycelix scalar `used_at` as the nominal point;
2. map to a degenerate interval `[used_at, used_at]` only when no independent hard acquisition bounds are supplied;
3. consume an explicit acquisition-time hard-bound pair only with its own clock/method provenance and pinned qualification;
4. never turn a standard uncertainty, confidence interval or `Uncertainty` object into deterministic bounds;
5. abstain if the acquisition interval overlaps either certificate validity boundary.

The structural Mycelix result `STRUCTURALLY_COMPLETE` is a packet-join result only. Artifact byte verification is a separate explicit call to `verify_packet_artifact_payloads`; even successful byte verification establishes integrity against the declared digest, not source authenticity, licensing or truth of engineering content.

## 3. BASALT evidence-state mapping

BASALT PR #9203 defines an event-sourced timeline containing:

- initial resource IDs and digests;
- ordered `resource_changed` records with event ID, sequence, change kind, event digest, resource ID and before/after digest;
- `evidence_recorded` records with evidence ID, check ID/kind/outcome/digest, exact resource pins, evidence dependency IDs, dependency coverage and unresolved inputs.

Each resource pin includes the current resource digest and generation. The auditor increments the resource generation on change, invalidates directly dependent evidence and propagates staleness through explicit evidence dependency edges. Reverting a resource digest does not resurrect evidence from an earlier generation.

The report carries:
- input digest and final-state digest;
- evidence freshness (`fresh`, `stale_direct`, `stale_transitive`);
- dependency coverage (`complete` / `incomplete`) and explicit unresolved inputs;
- `requires_reverification`, direct stale resources, stale ancestors, invalidation event IDs and blocked-by evidence;
- ordered re-verification plan and counts.

### 3.1 AERO consumption rules

| BASALT report field | AERO behavior |
|---|---|
| `input_digest` | Bind exact timeline bytes to the claim context |
| `final_state_digest` | Bind exact audited final resource state to the claim context |
| `report_version`, run ID and BASALT evaluator build | Preserve as verifier identity; do not assume compatibility across versions |
| `freshness=fresh` | Candidate current evidence only; still require owner authenticity, receipt admission, claim/context binding and applicable model/metrology gates |
| `stale_direct` / `stale_transitive` | Mandatory requalification/reverification; never reuse as current evidence |
| `dependency_coverage=incomplete` | Hard incomplete state, even if freshness is fresh and outcome is pass |
| `unresolved_inputs` | Carry into AERO's explicit evidence-gap packet; do not fill with an inferred default |
| `blocked_by_evidence_ids`, stale ancestors, invalidation IDs | Preserve exact dependency frontier and owner change lineage |
| `eligible_under_declared_coverage` | Owner-declared graph eligibility only; not authenticated and not engineering acceptance |
| `confirmatory_eligible=false` | Preserve as a hard non-confirmatory state. AERO must not override it |

The BASALT auditor distinguishes resource/evidence freshness from issuer authentication and explicitly keeps confirmatory eligibility false pending authenticated runner coverage receipts. AERO must preserve this ceiling, not raise it because one individual record is fresh.

### 3.2 What BASALT does not replace

The resource timeline has a monotonic sequence and generation model; it is not itself:
- a wall-clock or physical acquisition timeline;
- a receiver-first-seen receipt service for third-party source events;
- a trusted timestamp authority;
- issuer/signature/key validation;
- a calibration or measurement-validity evaluator;
- an aircraft component acceptance or certification decision.

AERO must not infer receiver time from BASALT sequence numbers, nor use resource freshness as evidence of physical measurement validity.

## 4. Composed three-owner flow

1. **Mycelix/AeroCommons** retains physical-unit/configuration identity, source artifact bytes, owner evidence records, calibration evidence and lifecycle events.
2. **BASALT** audits resource generations and evidence dependencies, returns exact freshness and re-verification state, and maintains its non-confirmatory status until its runner-coverage requirements are met.
3. **Owner trust/receipt service** authenticates producer identity, signatures, receiver-first-seen receipts, source namespace, stream epoch, trust-policy history and any timestamp-token evidence.
4. **AERO temporal replay** receives immutable owner references and authenticated/qualified dispositions, then evaluates AS-KNOWN, current-trust and current-evidence/current-trust views.
5. **AERO claim gate** derives the typed decision context, checks exact claim/subject/configuration/time/evidence-cut/trust/model bindings and returns reviewability or a fail-closed disposition.
6. **Symthaea** may consume the claim state and evidence-gap packet for information-gathering selection. It must not treat the AERO status as a scalar reward or execution authorization.
7. **Independent engineering authority** decides physical significance, allowable use, inspection/repair and certification. Software validation does not replace this role.

## 5. Conformance tests and promotion gates

Before an AERO owner adapter is promoted:

- consume real versioned AeroCommons JSON using the upstream parser, not a copied duplicate schema;
- retrieve exact owner artifact bytes and confirm their byte length/SHA-256 using the owner API;
- preserve owner lifecycle and predecessor semantics, including Superseded/Disputed/Revoked/Withdrawn negative cases;
- bind physical-unit, subject and configuration IDs exactly to the AERO typed decision context;
- compare the entire acquisition interval against each calibration validity interval;
- bind BASALT input/final-state digests and exact evidence IDs to the AERO packet;
- assert that incomplete dependency coverage, stale direct/transitive evidence, unresolved inputs and `confirmatory_eligible=false` cannot result in a positive AERO reviewable status;
- separately verify issuer signatures, receipt bytes, receiver-first-seen time and trust-policy snapshots at the owner boundary;
- independently author a cross-project substitution corpus (wrong unit/configuration, stale resource generation, digest reversion, hidden dependency, non-Active lifecycle, late event, changed toolchain, trust revocation, source namespace alias);
- retain exact component reports, verifier builds, input bytes/digests, policy snapshots and all negative outcomes.

No performance or reliability claim should be made from two synthetic draft PR fixtures. The fair next benchmark is one shared frozen build/inspection/test packet and one shared BASALT timeline replayed through owner-native verification, AERO adapter, bitemporal baseline and manual-review baseline, with correctness gates applied before time/cost comparison.

## 6. Current owner-side interface references

- [Mycelix AeroEvidence primitives](https://github.com/Luminous-Dynamics/mycelix/tree/6b160d92cce142b180d67f81837a1f32034444e1/mycelix-commons/crates/aero-evidence-types)
- [Mycelix component-lineage verifier](https://github.com/Luminous-Dynamics/mycelix/tree/6b160d92cce142b180d67f81837a1f32034444e1/tools/aerocommons-component-lineage)
- [Mycelix draft PR #6347](https://github.com/Luminous-Dynamics/mycelix/pull/6347)
- [BASALT evidence-state timeline auditor](https://github.com/Luminous-Dynamics/symthaea/blob/6d260c9f7075c1be04d8725ed711d4011270f1d4/xtask/src/basalt_state.rs)
- [BASALT timeline schema](https://github.com/Luminous-Dynamics/symthaea/blob/6d260c9f7075c1be04d8725ed711d4011270f1d4/benchmarks/basalt/schema/state-timeline.schema.json)
- [Symthaea BASALT draft PR #9203](https://github.com/Luminous-Dynamics/symthaea/pull/9203)
- [AERO temporal replay PR #7714](https://github.com/Luminous-Dynamics/symtropy/pull/7714)

These are research/draft interfaces observed at their pinned commits. Re-audit the exact merged API and schema digests before wiring a production adapter.
