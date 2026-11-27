# AERO Claim Specification Assurance v0.1

**Status:** research increment / bounded typed validator  
**Parent:** AERO model-evidence artifact assurance PR #7765  
**Branch:** `research/aero-claim-specification-assurance-v0.1`  
**Claim ceiling:** formal specification completeness and internal consistency only; no engineering acceptance, model validation, physical qualification, airworthiness, certification or execution authority

## 1. Why this exists

AERO's typed decision context includes `claim_specification_sha256`. That binds a digest, but a digest is not a semantic review of the proposition. A syntactically valid, correctly hashed specification can still be ambiguous or incomplete.

This increment makes one bounded class of engineering proposition machine-checkable before evidence is composed:

- exact claim, physical subject, configuration and operating-envelope identities;
- intended use and validity interval;
- measurand definition, aggregation and closed unit code;
- explicit comparison operator and exact rational threshold;
- declared population, sampling unit, quantifier and required strata;
- explicit uncertainty treatment and method reference;
- mandatory evidence classes, assumptions, exclusions and limitations;
- rationale, verification method and acceptance-authority role.

The current profile intentionally supports one scalar predicate. It does not invent generic compound logic or silently coerce unsupported constructs.

## 2. Disposition semantics

| Result | Meaning |
|---|---|
| `FORMALLY_WELL_SPECIFIED` | The bounded v0.1 contract is complete and internally consistent |
| `INCOMPLETE` | Required semantic input is absent |
| `REJECTED` | A contradiction, malformed identity, impossible interval or inconsistent assertion exists |
| `UNSUPPORTED_PROFILE` | The requested schema/profile is not implemented here |

A positive result means *reviewable as a formally specified proposition*, not “correctly specified” in the full engineering sense. An independent competent reviewer must still verify that the selected proposition, threshold, assumptions, exclusions and acceptance basis are appropriate to the engineering requirement and intended use.

## 3. Important commitment distinction

Three objects must remain distinct:

1. **Raw specification payload digest:** SHA-256 of the exact supplied source bytes.
2. **Typed semantic commitment:** SHA-256 of the parsed, validated specification under `rust-serde-json-struct-order-v1`.
3. **Decision-context commitment:** the larger AERO context digest that binds the claim, subject, configuration, time, source scope, trust, specification and evidence cut.

These are not interchangeable. The new module exposes a typed semantic commitment. It does not authenticate the raw payload, a producer, or an owner receipt. The current `claim_specification_sha256` field remains an opaque digest until a subsequent integration explicitly states which of these commitments it carries. Cross-language exchange should eventually adopt a named, independently reproduced canonicalization profile (for example, a fully specified JCS profile) rather than assume Rust struct serialization is universal.

## 4. Formal contract choices

- Exact rational numbers; no floating-point threshold values.
- Closed, versioned unit vocabulary.
- No implicit unit conversion in v0.1. Identical unit codes are required on measurand and predicate.
- Explicit population quantifier and sampling unit.
- At least one required stratum, including an explicit “unstratified” level where appropriate.
- Explicit confidence level when statistical confidence-bound semantics are selected.
- Required evidence classes and named assumptions/exclusions.
- A nonempty limitations list.
- Unknown JSON fields rejected by the schema and typed Rust structs.

The validator checks consistency of those declarations. It does not compute the measured quantity, inspect physical evidence, test dimensional equations outside the closed unit table, verify the statistical method, or decide whether the proposed threshold is adequate.

## 5. Relationship to existing AERO owners

This is not a second evidence graph, change-impact engine, trust registry or model validator.

- #7742 remains the model-credibility integration (not #7741).
- #7765 owns the owner-facing model-report artifact assurance proposal.
- Track U / #5371 owns aviation model-evidence bitemporal change impact and targeted requalification.
- #7352 maps aviation resources and claims to BASALT / Mycelix owner-native freshness and lifecycle machinery.
- #7395 owns AP242/QIF semantic interoperability benchmarking.
- #7455 owns trusted chronology and historical replay research.
- #5721 owns assurance-case adversarial challenge generation.
- AeroWeft/model owners retain VVUQ and model-applicability authority; Mycelix retains physical-unit and artifact custody; Symthaea retains generic observer/freshness/lifecycle machinery.

The follow-on integration should add the typed specification to the AERO claim-gate input and reject a mismatch between the exact source payload, typed semantic commitment and decision context. This branch does not silently modify the existing gate contract.

## 6. Challenge and benchmark programme

The challenge seed contains 12 proposed cases: a positive control plus missing-authority, missing-strata, unit substitution, malformed rational, missing confidence, interval inversion, duplicate identity, semantic digest substitution and unsupported-profile challenges.

The corpus is explicitly `NOT_INDEPENDENTLY_REVIEWED` and `NOT_EXECUTED`. Schema validation only checks its machine-readable shape. The Rust unit tests exercise the validator's implemented rules but are not independent adjudication.

Prospective targets:

| Metric | Research target |
|---|---:|
| Malformed/ambiguous specification accepted by the validator | 0 observed in independent challenge set |
| Missing subject/configuration/operating-envelope binding accepted | 0 |
| Predicate/threshold/unit inconsistency accepted | 0 |
| Missing population, quantifier, stratum or uncertainty semantics accepted | 0 |
| Unsupported profile silently downgraded to v0.1 | 0 |
| Typed semantic digest mismatch detected | 100% |
| Reproducible disposition and reason-code set | 100% |
| False rejection on independently adjudicated valid specifications | Report separately; do not optimize by reject-all |

The challenge set should be expanded by independent engineering teams and stratified by claim family, physical discipline, unit system, population design, model type and operating envelope. Holdout splitting must be by root-cause family and engineering campaign—not random JSON rows. A hundred textual mutations of one missing-field defect are not a hundred independent trials.

## 7. Industry positioning

This increment operationalizes a small part of established requirements-engineering discipline; it does not replace ISO/IEC/IEEE 29148 or an aircraft systems-development process such as SAE ARP4754B. NASA guidance already emphasizes clarity, correctness, feasibility, unambiguity and verifiability of requirements. NASA-STD-7009B requires acceptance criteria defined by the programme/project and approved by the delegated NASA Technical Authority; it also distinguishes model verification, validation and credibility.

AERO's value is to turn the claim contract into an executable, versioned input to evidence assurance and replay. It should complement PLM, QMS, MBSE, configuration, metrology and model-assurance systems rather than replace them.

## 8. Verification boundaries

- Rust unit tests are included in the existing `symtropy-aero-material-qual` package test target.
- CI schema and challenge-corpus checks are added to the existing AERO workflow.
- No local Cargo/rustfmt run is claimed for this draft; the repository could not be cloned into the current execution environment.
- No exact-head GitHub Actions result is claimed until one is attached to the new branch/PR.
- No independent oracle review, real owner receipt, physical test or regulatory acceptance is claimed.
