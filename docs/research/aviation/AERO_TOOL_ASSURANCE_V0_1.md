# AERO Tool Assurance v0.1

**Status:** prospective tool operational requirements and failure-mode contract  
**Authority ceiling:** advisory / evidence-organization use only; no engineering acceptance, physical qualification, airworthiness, certification credit or physical-work authorization  
**Regulatory applicability:** not determined; no DO-330 Tool Qualification Level is claimed  
**Verification state:** requirements and failure modes are specified; independent tool-assurance review and exact-head verification remain pending

## 1. Purpose and governing distinction

AERO evaluates and organizes engineering evidence. Its own software can also malfunction. A correct engineering claim can be routed incorrectly by a defective parser, adapter, evaluator, capsule composer, replay verifier or report generator.

Therefore keep three distinct questions:

1. **Engineering evidence:** are the exact claim, physical/configuration identity, owner receipts, model applicability, chronology and observer facts admissible?
2. **Tool correctness:** did the AERO software execute its declared tool operational requirements correctly for the exact build and invocation?
3. **Use acceptance:** is the proposed AERO use accepted by the applicant / engineering authority / competent regulator under the applicable development-assurance and tool-assurance framework?

AERO must not use its own disposition as the sole evidence that it operated correctly. Build provenance establishes how an artifact was produced; it does not prove the tool's algorithm is correct. A signature authenticates an issuer under a trust policy; it does not prove physical truth or intended-use suitability.

FAA AC 20-115D recognizes RTCA DO-330 / EUROCAE ED-215 Tool Qualification Considerations alongside DO-178C / ED-12C. This document does **not** conclude that DO-330 applies to every AERO deployment. Any applicability assessment and required Tool Qualification Level depend on intended use, failure effects and whether the tool replaces/reduces another verification activity. The applicant and competent authority determine the applicable approach.

Reference: [FAA AC 20-115D](https://www.faa.gov/airports/resources/advisory_circulars/index.cfm/go/document.information/documentNumber/20-115D).

## 2. Intended-use profiles

### Profile A — advisory investigation

AERO may organize evidence references, propose missing-information questions, identify candidate defeaters and rank permitted acquisition options. It may not modify authoritative records, admit evidence, change the claim disposition, replace independent verification, authorize physical testing or make compliance determinations.

Required containment:
- every hypothesis remains visibly a hypothesis until a separate owner evidence receipt supports it;
- every recommendation cites its exact claim, unresolved obligation and source commitments;
- human/owner review remains mandatory;
- the output is non-binding and cannot be consumed as approval.

### Profile B — evidence consistency and replay support

AERO may validate typed structures, exact digests, source bindings, time cuts, owner receipts and reproducibility envelopes. It may issue a consistency/replay report for an independent decision-maker.

Required containment:
- source-owner identity, authority, physical truth and model credibility remain owner-controlled facts;
- unknown or unavailable source information stays unknown/incomplete;
- a replay success is not automatically a correct engineering decision;
- downstream consumers retain independent checks appropriate to the use.

### Profile C — compliance-supporting or verification-reduction use

A future deployment might be proposed to replace or reduce an otherwise required verification activity, or to receive formal credit in an engineering/certification process. This profile is **not enabled** by v0.1.

Before any such use:
- define the intended use and tool malfunction effects;
- complete a tool-assurance strategy under the applicable framework;
- have the applicant and competent authority determine the applicable qualification/acceptance requirements;
- independently verify the tool's requirements and implementation;
- demonstrate failure detection/containment and an approved operational environment;
- document the exact approved build, configuration, limitations and operating instructions.

No profile transition occurs automatically because unit tests, CI, a benchmark, a build attestation or a draft PR passes.

## 3. AERO component inventory and TORs

The component inventory below is a first baseline. The implementation owner must maintain it as code and deployment boundaries evolve.

| Tool component | Primary operational purpose | Mandatory ceiling |
|---|---|---|
| Claim-specification parser/validator | Parse typed claim and assess bounded formal completeness | Does not prove physical truth or predicate correctness |
| Specification/context binding adapter | Bind raw payload, semantic commitment and exact decision context | Does not authenticate the owner signature or grant authority |
| Owner/trust/chronology integration | Consume owner evidence, trust-policy and receiver-time dispositions | Does not invent owner trust or rewrite historical knowledge |
| Model-credibility consumer | Bind model/solver/context-of-use and owner VVUQ results | Does not calculate or replace discipline-specific VVUQ |
| Observer/change-impact consumer | Consume scope-qualified observer and effectivity outputs | Unknown coverage never means no impact |
| Disposition projector | Deterministically project normalized obligations to a routing result | At most reviewability for independent decision |
| Capsule composer | Package exact source commitments, outputs and disclosure classes | Cannot upgrade digest-only/withheld evidence to replayable |
| Independent capsule verifier | Reconstruct declared outputs and report disagreements | Must not call the producer's decision function as its only oracle |
| Evidence-action EVSI planner | Rank owner-permitted, calibrated evidence-acquisition options | Recommendation only; no claim mutation or work authority |
| Report/export layer | Present exact disposition, reasons, provenance and limitations | Cannot omit blockers or strengthen claims during presentation |

### Tool Operational Requirements

TORs are normative for this v0.1 tool contract. Each TOR must be linked to an implementation, a test/analysis artifact, a result at an exact build, and any remaining assumption. A unit test at an earlier source revision cannot satisfy a later revision.

| TOR ID | Requirement | Verification / evidence | Failure response |
|---|---|---|---|
| AERO-TOR-001 | Every evaluation shall bind one exact claim ID, physical subject, configuration and operating envelope | Typed contract tests; context substitution corpus; independent oracle | Reject mismatched input |
| AERO-TOR-002 | Raw payload digest, typed semantic commitment and decision-context digest shall remain distinct objects | Digest substitution tests; byte-level fixtures | Reject mismatch; never coerce one digest into another |
| AERO-TOR-003 | Required obligations/strata shall be derived from the exact specification or versioned approved profile, not caller booleans | Requirement-to-obligation trace; omitted-stratum mutations | Reject or mark unsupported/incomplete |
| AERO-TOR-004 | Mandatory and hard-gate status shall not be caller-downgradable | Intrinsic-gate mutation tests | Unverified/rejected; never reviewable |
| AERO-TOR-005 | Owner evidence shall bind owner namespace, record/payload digest, subject/configuration, lifecycle, freshness and owner receipt | Owner-cut parity tests; cross-owner substitution controls | Incomplete/unverified/rejected |
| AERO-TOR-006 | Producer authority shall be consumed from the selected owner trust-policy evaluation | Trust-policy substitution and role/revocation tests | Unverified; no implicit authority |
| AERO-TOR-007 | Historical replay shall use receiver-first-seen chronology and the selected trust-policy snapshot | Late-receipt/backdated-assertion tests; counterfactual replay tests | Exclude late knowledge from historical view |
| AERO-TOR-008 | Physical acquisition intervals and event-effective intervals shall use the existing effectivity evaluator | Boundary, non-effective and malformed interval tests | Incomplete/rejected; no local reimplementation |
| AERO-TOR-009 | A preservation/no-impact result shall require qualified observer coverage for the relevant source, interval and change classes | Missing-observer / dropped-event fault injection | Unknown/incomplete; never infer no impact |
| AERO-TOR-010 | Four-valued support/rebuttal conflict shall be preserved through disposition and reason reporting | BOTH-state and contradiction fixtures; independent JSON oracle | Preserve conflict; no confidence averaging |
| AERO-TOR-011 | Disposition projection shall be deterministic and non-compensatory | Exhaustive finite-state/property tests; independent oracle | Fail closed; preserve all blockers |
| AERO-TOR-012 | A positive disposition shall never grant physical-work, engineering-acceptance or certification authority | Output schema, API invariants, negative-control tests | Reject any output violating the ceiling |
| AERO-TOR-013 | Capsule composer shall preserve exact inputs, disclosure modes, verifier identity and detached package-integrity receipts | Capsule tamper/substitution and offline replay tests | Report disagreement/unavailable; preserve original bytes |
| AERO-TOR-014 | Independent replay shall not depend on the producer's in-memory state or hidden network state | Clean-room execution and dependency/network policy checks | Replay incomplete/failed; no “fully reproducible” claim |
| AERO-TOR-015 | EVSI action ranking shall require a source-bound unresolved-obligation manifest and complete action-catalogue snapshot receipt | Omission/substitution receipt tests | Invalid input; no ranking |
| AERO-TOR-016 | EVSI permission and calibration receipts shall bind exact claim/context/configuration/action/model/likelihood table and time | Receipt scope, late-known and model substitution tests | Exclude action or reject input |
| AERO-TOR-017 | EVSI shall not mutate the assurance state or authorize a physical action | API/output invariants; deny-all and positive controls | Reject any authority escalation |
| AERO-TOR-018 | Every report shall present the complete sorted blocker/reason-code set and actual disclosure mode | Report/evaluator parity tests; reason-omission mutations | Report fails validation |
| AERO-TOR-019 | Every run shall identify source revision, build, compiler, dependency lock, schema/profile versions, runtime and invocation | Build-provenance and configuration-index audit | Run is not reproducible; withhold claimed replay |
| AERO-TOR-020 | Every published result shall distinguish implemented, structural-only, CI-pending, independent-review-pending and physical/domain-pending evidence | Release checklist and signed verification manifest | No status promotion without exact evidence |

## 4. Tool failure-mode and effects analysis (initial FMEA)

Severity is qualitative until a particular deployment's hazard/consequence analysis is completed. “Critical” denotes a failure that could create a false reviewable result, missed invalidation, unauthorized recommendation or misleading compliance/reproducibility claim. Occurrence and detectability scores are not assigned without empirical data.

| Failure ID | Malfunction | Potential effect | Required detection / containment | Status |
|---|---|---|---|---|
| AERO-FMEA-001 | Claim parser binds wrong subject/configuration | Evidence from another article or revision appears applicable | Exact context digest and substitution controls; reject | Software control defined |
| AERO-FMEA-002 | Required obligation/stratum omitted | Incomplete evidence looks sufficient | Derive from typed spec/profile; omitted-obligation mutation | Adapter implementation pending |
| AERO-FMEA-003 | Hard gate downgraded or absent | False reviewable disposition | Intrinsic mandatory gate checks; exhaustive negative controls | Evaluator implementation present; exact-head tests pending |
| AERO-FMEA-004 | Digest field reused across payload/semantic/context objects | Substitution accepted | Distinct typed fields and domain-separated commitments | Binding adapter first increment present |
| AERO-FMEA-005 | Owner receipt or producer role from wrong namespace | Unauthorized producer accepted | Owner namespace + record + authority-policy matching | Owner workflow remains authoritative |
| AERO-FMEA-006 | Issuer backdates evidence received after cutoff | Historical decision uses future knowledge | Receiver-first-seen cutoff and historical replay tests | Source chronology integration required |
| AERO-FMEA-007 | Revocation applied retroactively or ignored currently | Historical/current view conflated | Separate historical and counterfactual replay modes | Semantic contract present |
| AERO-FMEA-008 | Observer misses change class but emits empty impact set | False preservation; missed invalidation | Observer scope/watermark receipt; unknown propagation | Observer owner evidence required |
| AERO-FMEA-009 | Model/solver/context-of-use substitution | Unsupported model evidence admitted | Exact model/solver/profile/risk binding; per-stratum blockers | Model owner remains authoritative |
| AERO-FMEA-010 | Conflict collapsed into confidence average | Decisive rebuttal disappears | Four-valued evidence state and non-compensatory projection | Evaluator implementation present; oracle review pending |
| AERO-FMEA-011 | Capsule omits bytes but labels itself fully reproducible | Consumers over-trust a digest-only pointer | Disclosure-class invariants and clean-room replay | Capsule implementation/benchmark pending |
| AERO-FMEA-012 | Replay verifier uses wrong build/dependency lock | Incorrect result appears independently reproduced | Verifier source/build/provenance and independent replay | Tool provenance experiment pending |
| AERO-FMEA-013 | Report generator drops blocker/reason code | Human reviewer sees incomplete output | Compare rendered report to machine result; mutation test | Report-layer tests pending |
| AERO-FMEA-014 | EVSI action cost or output contract altered after owner permission | Mis-ranked acquisition option | Recomputed action-entry digest bound by permission receipt | Prototype added; exact-head CI pending |
| AERO-FMEA-015 | Caller omits an unresolved blocker or catalogue candidate, then recomputes a local hash | Planner works from incomplete input | Upstream state and complete-catalogue snapshot receipts | Prototype added; upstream trust integration pending |
| AERO-FMEA-016 | EVSI model is uncalibrated, out of domain or known only after cutoff | Misleading action ranking | Calibration receipt / owner / model / chronology gates | Prototype added; independent calibration pending |
| AERO-FMEA-017 | LLM hypothesis is treated as confirmed evidence | Unsupported defeater or false rejection | Hypothesis/finding type separation and independent evidence receipt | Policy contract defined; AI benchmark pending |
| AERO-FMEA-018 | Verifier/toolchain compromise or dependency substitution | Malicious or defective tool output | Pinned builds, provenance, SBOM/lock, offline replay and independent verifier | Tool assurance work open |
| AERO-FMEA-019 | Non-deterministic collection ordering changes digest/result | Replay disagreement or signature mismatch | Canonical field/order contract and permutation tests | v0.1 profile implementation-specific |
| AERO-FMEA-020 | Tool used outside its declared intended-use profile | Advisory output treated as approval or verification credit | Deployment gating, operator instructions, authority approval | Compliance-supporting profile disabled |

This FMEA is an initial hazard-oriented inventory, not a completed safety assessment. A deployment-specific team must add local hazards, severity/likelihood rationale, operational environment, independent detection and residual-risk acceptance.

## 5. Verification strategy: diverse evidence, not self-attestation

The AERO tool-assurance package should use independent evidence sources:

1. **Requirements-based tests:** each TOR maps to positive, negative, boundary and robustness tests.
2. **Implementation-diverse oracle:** a separately authored JSON-level or other independently implemented evaluator; shared fixtures are permitted, shared production helper logic is not.
3. **Mutation testing:** deliberately inject wrong predicates, omitted gates, wrong chronology, receipt substitutions, hidden dependencies, omitted reasons and stale build identities.
4. **Clean-room replay:** reconstruct results in a separate environment with exact disclosed inputs, pinned verifier identity and no undeclared network dependency.
5. **Independent domain adjudication:** reviewers judge engineering argument/evidence applicability, not merely JSON or DAG well-formedness.
6. **Human-factors evaluation:** measure reviewer errors, time, reliance, overrides and automation-bias indicators.
7. **Configuration and provenance review:** source/build/toolchain/lock/profile/runtime/invocation/output manifest are fixed and externally inspectable.

A second build of the same code is not an implementation-diverse verifier. A signed report is not independent domain adjudication. A schema pass is not proof of argument soundness.

## 6. Tool configuration index

Every run intended as verification evidence should emit or reference an immutable configuration record containing:

- repository, branch/tag and exact source commit;
- verifier source-tree and artifact digests;
- compiler/toolchain and build flags;
- dependency lock and resolved dependency manifest;
- schema, semantic-profile, requirement-profile and canonicalization versions;
- runtime/OS/architecture and relevant environment;
- invocation, command parameters and feature flags;
- input manifest, disclosure classes and output manifest;
- network policy and any external service/owner receipts;
- builder identity, build provenance and execution receipt.

A missing field does not imply the field was irrelevant. It means the run's reproducibility/coverage claim is limited or not estimable.

## 7. Release and verification-state rules

Use these labels explicitly:

- **IMPLEMENTED:** code/configuration exists at the cited source revision.
- **STRUCTURAL-ONLY:** schemas, manifests, static fixtures or shape validators pass; no runtime semantic test is implied.
- **CI-PENDING:** the exact source head has no completed passing run.
- **EXACT-HEAD-SOFTWARE-PASS:** exact-head formatting, compile, tests and Clippy passed; only software behavior covered by those tests is established.
- **INDEPENDENT-REVIEW-PENDING:** no independent reviewer has adjudicated the relevant semantics/domain evidence.
- **PHYSICAL/DOMAIN-PENDING:** no physical test, model-owner qualification or domain assessment has been performed.
- **AUTHORITY-ACCEPTED:** only where a named competent authority has accepted a precisely defined intended use and evidence package.

Do not promote a queued, skipped, cancelled or earlier-head workflow to a pass. Do not inherit a pass from a base branch or sibling PR. Do not infer physical qualification from any software status.

## 8. Benchmark extensions

Keep IAB v0.3 frozen. Tool malfunction, owner-hosted evidence verification and robust EVSI require a separate v0.4 protocol (#7899).

Prospective tool-assurance metrics:
- critical false-review / false-pass rate;
- seeded critical malfunction detection and escape rate;
- omitted-blocker / omitted-reason detection;
- exact source and configuration substitution detection;
- verifier-build/provenance substitution detection;
- deterministic replay and cross-implementation output parity;
- unauthorized-action recommendation rate;
- report/evaluator parity;
- user reliance, overrides and automation bias;
- exact numerator/denominator, independent root-cause family count and worst critical stratum.

A zero-failure synthetic corpus is not a field failure-rate estimate. A tool-assurance benchmark does not by itself establish regulatory acceptance.

## 9. Related work

- #7897 — AERO tool-assurance programme.
- #7881 — source-to-state adapter and claim-gate parity.
- #7850 / #7859 — disposition kernel and source-binding contract.
- #7896 — EVSI evidence-action prototype.
- #7812 — portable capsule and independent replay.
- #7898 — owner-hosted evidence verification.
- #7899 — IAB v0.4 benchmark extension.
- FAA AC 20-115D / DO-330 — possible tool-assurance comparator; applicability must be assessed, not assumed.
- SLSA / in-toto — build provenance; not functional correctness or tool qualification.

## 10. Non-claims

This document does not declare a DO-330 TQL, assert FAA/EASA acceptance, qualify the AERO tool, prove physical truth, establish material allowables, validate a simulation model, authorize tests or grant airworthiness/certification credit. It defines a first tool-operational requirements and failure-mode baseline for research and independent review.
