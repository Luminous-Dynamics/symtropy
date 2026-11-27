# AERO State-Vector Source Binding v0.1

**Status:** prospective integration contract; schema/example are synthetic and not independently adjudicated  
**Authority ceiling:** research-only; no physical qualification, certification or work authorization

## 1. Problem

The Rust state-vector evaluator is currently a pure projection over a typed obligation vector. That is a useful, testable kernel, but the vector must not be treated as self-authenticating input. An arbitrary caller could choose which obligations exist, mark a domain-specific obligation optional, or assert that evidence was known at a historical cutoff without supplying the owner receipt that proves when the receiver first saw it.

The repository already has stronger upstream contracts:
- ClaimSpecificationV0_1 defines subject, configuration, operating envelope, intended use, measurand, predicate, population, required strata, uncertainty treatment, required evidence classes, assumptions, exclusions, rationale, verification method, acceptance-authority role and limitations.
- DecisionContextV0_1 commits to claim/subject/configuration/envelope, physical-acquisition interval, as-known cutoff, source scope, evidence-knowledge cut, trust policy, claim-specification digest, required-evidence manifest, risk classification and model-applicability profile.
- OwnerEvidenceCutV0_1 binds owner records, lifecycle, freshness, dependency coverage, receiver receipt manifest/watermark, BASALT run and observer-coverage receipt.
- ModelCredibilityCutV0_1 binds exact model, solver, validation package/protocol, applicability profile, domain coverage and split integrity.
- CoverageAssessment, CutAssessment and EffectivityRelation are outputs from their own evaluators, already bound to the decision context by component report digests.
- ClaimGateInput already composes these contracts non-compensatorily.

The next step must derive the state vector from those typed facts. It must not become a parallel authority source or duplicate BASALT, AeroCommons, Mycelix or model-owner evaluation.

## 2. Normative adapter rule

**No caller-authored obligation policy.** The adapter receives exact typed specifications and upstream assessments, validates their commitments and produces the state vector. It never accepts an unbound mandatory or hard-gate boolean from an external request.

For each obligation:
1. Resolve its requirement source from the exact claim specification or a versioned, authority-approved requirement profile.
2. Derive mandatory and hard-gate status from that profile, not from the case submitter.
3. Resolve evidence references from the decision context's required-evidence manifest and owner-native cut.
4. Verify owner record digest, subject, configuration, lifecycle, freshness and owner receipt against the source manifest.
5. Derive producer authority from the named trust-policy snapshot, including policy-effective and receiver-first-seen chronology.
6. Derive support/conflict from owner/model reports and independently verified evidence references. A digest alone is not support.
7. Derive subject-time effectivity through the existing interval comparator; never reimplement interval ordering.
8. Derive observer state from the qualified observer-coverage receipt and source/change-class/interval scope.
9. Preserve every source commitment and reason code in the resulting vector.
10. Run the pure disposition projection only after all source-binding checks complete.

A schema-valid input is not enough. The adapter must recompute canonical digests, evaluate typed upstream objects' validation and assessment results, and match identifiers before projection. Unknown or failed source binding cannot be normalized into supported evidence.

## 3. Derive obligations from the claim specification

| Claim-specification field | Derived obligations / constraints |
|---|---|
| claim, subject and configuration IDs | Mandatory claim identity and exact physical/configuration binding |
| operating envelope, intended use and validity | Applicability and context-of-use obligations |
| measurand, predicate, unit and aggregation | Semantic validity and measurement obligations |
| population, population manifest and required strata | One mandatory coverage obligation per required stratum; callers cannot remove a stratum |
| uncertainty treatment, method and confidence | Model/measurement uncertainty obligation with the exact method and bound |
| required evidence classes | Mandatory evidence obligations with exact owner references |
| assumptions, exclusions and limitations | Explicit defeaters / scope boundaries; changes are retained |
| rationale and verification method | Required argument and verification artefacts |
| acceptance-authority role | Role expected for the independent decision; never a grant from AERO |

The adapter must fail closed if the claim profile is unsupported, formally incomplete, or not bound to the decision context's exact claim-specification digest.

## 4. Source-derived time, not caller-declared labels

Temporal enums in the state vector are derived outputs. The source-binding envelope carries an explicit replay mode and raw clocks:
- issuer-asserted time;
- source/event effective-time interval;
- physical/acquisition-time interval from the decision context;
- receiver first-seen time from the owner receipt;
- trust-policy effective interval;
- trust-policy receiver first-seen time;
- selected as-known cutoff.

Rules:
- Receiver first-seen later than the as-known cutoff means NOT_KNOWN_AT_CUTOFF, regardless of issuer-asserted/backdated event time.
- If receiver first-seen is absent for evidence claimed as admitted, the source is unverified/incomplete; it is never presumed known.
- If evidence was received by the cutoff but the event is definitely not effective over the physical acquisition interval, derive NOT_EFFECTIVE_AT_SUBJECT_TIME.
- If event/acquisition bounds overlap, derive TEMPORAL_CONFLICT / INCOMPLETE.
- If either interval is malformed, retain a verified invalidity finding and reject the assertion.
- The replay mode is one of HISTORICAL_AS_KNOWN, CURRENT_POLICY_COUNTERFACTUAL or CURRENT_AS_KNOWN. A trust policy or revocation is evaluated with both effective time and receiver-first-seen time. A later-observed revocation cannot silently rewrite historical as-known state; current-policy counterfactual is a separate evaluation.
- Preserve issuer-asserted time even where it does not determine receiver knowledge.

The adapter consumes the existing EffectivityRelation result; it does not replace the source chronology or interval comparator.

## 5. Source contract

The companion JSON Schema binds claim-specification identity/digest, decision context and acquisition interval, replay mode, trust-policy snapshot/events, a collection of owner-scoped evidence cuts/receipt watermarks, model credibility cut, observer coverage receipt/scope, per-obligation requirement source and owner evidence records, and root-cause partition. Each evidence receipt must resolve to exactly one matching owner namespace/cut; missing or duplicate owner cuts fail validation.

The JSON example is synthetic. Its digests are placeholders and do not represent valid signed owner receipts. It exists only to exercise the envelope's shape; it is not engineering evidence.

## 6. Required adapter checks and negative controls

Before emitting a state vector, test:
1. Claim-specification digest differs from DecisionContext commitment.
2. Claim specification is incomplete or unsupported.
3. Required population stratum is omitted by caller.
4. Required evidence class is omitted or bound to unrelated evidence.
5. Owner record belongs to another physical subject or configuration.
6. Payload digest is valid but producer role is unauthorized under the selected policy.
7. Issuer asserts a pre-cutoff date but receiver first saw the item after the cutoff.
8. Revocation is effective before physical event but first known after historical cutoff.
9. The same revocation replayed under current policy produces a distinct counterfactual.
10. Event-effective interval is definitely after, definitely before, ambiguous with, or invalid relative to acquisition interval.
11. Owner observer is incomplete for one change class, even if no event was observed.
12. Model report belongs to a different solver or applicability profile.
13. A model stratum is missing from the required validation manifest.
14. A post-test bench/configuration change arrives after historical cutoff.
15. A correction/supersession chain preserves the old record and receiver chronology.
16. Reordering source manifests and moving capsule storage does not change canonical result.

Assert primary disposition, complete blocker/reason-code set, replay mode, source digests and affected-claim set.

## 7. Relationship to the existing ClaimGate

The existing evaluate_claim_gate already composes owner cut, model credibility, observer coverage, causal cut, effectivity and required-evidence admissions. The new state vector must not become a second evaluator for those same facts.

Recommended sequence:
1. Add a source-binding adapter that maps exact typed upstream inputs into the richer vector and validates claim-specification / required-obligation profile.
2. Make the existing claim gate emit or internally consume that normalized vector while preserving current public status/reason-code compatibility.
3. Add a crosswalk between ClaimGateStatus and the richer projection, including NOT_KNOWN_AT_SNAPSHOT, NOT_EFFECTIVE_AT_SUBJECT_TIME, evidence conflict and disclosure/replay axes.
4. Remove duplicate projection branches only after parity tests show that the adapter reproduces existing ClaimGate blockers across all current positive and negative fixtures.

Until then, evaluate_assurance_state_case_v0_1 remains a research reference projection, not the sole production gate.

## 8. Adapter benchmarks

- 100% required-obligation derivation agreement with frozen claim specification.
- 0 omitted mandatory strata / evidence classes in critical negative controls.
- 0 accepted subject/configuration substitutions.
- 0 late-receiver-evidence historical-knowledge errors.
- 0 trust-policy effective/first-seen chronology errors.
- 0 unsupported model-domain / solver substitutions.
- 100% replay of context, owner-cut, model-cut and component-report digests.
- 100% unknown-observer propagation for uncovered change classes.
- 100% blocker/reason-code preservation versus independent adjudication.
- 0 generated hypotheses elevated to confirmed findings without a separate evidence receipt.

Report numerator/denominator, one-sided exact confidence bounds and worst critical stratum. Generated mutations remain clustered with their root cause. These are targets, not achieved results.

## 9. Non-claims

The adapter does not authenticate a source merely by hashing its record. Owner signatures, receipt verification, trust roots and source bytes remain in their source-owner workflows. It does not establish physical truth/model credibility, authorize tests, grant certification credit or make a regulator's decision. AERO preserves exact claim-to-evidence bindings and makes the resulting decision independently reconstructable.
