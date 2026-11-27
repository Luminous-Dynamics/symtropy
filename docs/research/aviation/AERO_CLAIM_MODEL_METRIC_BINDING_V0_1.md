# AERO Claim-to-Model Metric Semantic Binding v0.1

**Status:** research contract / executable typed adapter  
**Branch:** `research/aero-claim-model-metric-binding-v0.1`  
**Parent:** claim specification assurance PR #7781  
**Claim ceiling:** semantic binding only; no VVUQ calculation, physical predicate evaluation, engineering acceptance, airworthiness, certification or execution authority

## 1. Problem

The model-credibility cut already binds exact model identity, solver configuration, claim, physical configuration, operating envelope, applicability profile, required validation modalities, uncertainty components and required metric/stratum reports.

However, `MetricStratumReportV0_1` deliberately stores report digests, an uncertainty-report digest, a population-manifest digest, sample/adverse counts and status. It does **not** express the metric's engineering semantics: measurand, units, aggregation, population quantifier, predicate, threshold or uncertainty interpretation.

Therefore, a report can be valid for the correct model/configuration and still be the wrong metric for the claim. A report about maximum displacement must not satisfy a maximum-principal-stress proposition merely because both belong to the same model.

## 2. New contract

`ClaimMetricSemanticBindingV0_1` is an owner-referenced, typed mapping from:

- exact claim specification raw-payload digest and typed semantic commitment;
- exact decision-context commitment;
- exact model-credibility-cut digest;
- exact owner-evidence-cut digest;
- owner model metric ID and definition;
- explicitly reviewed metric-to-measurand mapping basis;
- exact unit and aggregation semantics;
- exact population, sampling unit, manifest, quantifier and confidence;
- exact predicate operator, rational threshold and unit;
- exact uncertainty treatment, method and confidence;
- a one-to-one map for every claim-required stratum;
- the exact model report, semantic mapping, owner attestation, producer-authority receipt and semantic-validation receipt records;
- receiver-first-seen interval relative to the historical evidence cutoff.

The mapping is not inferred from display names. No implicit unit conversion is permitted. If a conversion is needed, it requires a separate versioned conversion profile, exact transformation rule, source and target units, uncertainty propagation, owner receipt and independent qualification.

## 3. Disposition

| Disposition | Meaning |
|---|---|
| `SEMANTICS_BOUND` | The claim proposition is explicitly bound to the exact owner metric and its per-stratum model reports |
| `INCOMPLETE` | Required metric/report/population/stratum or semantic input is missing or indeterminate |
| `UNVERIFIED` | Owner receipt, payload, freshness or metric evidence verification is unresolved |
| `NOT_KNOWN_AT_SNAPSHOT` | Receiver-first-seen is later than the historical claim-evaluation cutoff |
| `OUT_OF_SCOPE` | The claim specification was not effective over the entire physical acquisition interval |
| `REJECTED` | Metric, subject, configuration, unit, aggregation, population, predicate, uncertainty or receipt identity contradicts the claim |
| `UNSUPPORTED_PROFILE` | The requested binding schema/profile is not implemented |

A `SEMANTICS_BOUND` result is **not** a model pass, a claim pass or a reviewable aircraft decision. It means only that the owner-defined metric is explicitly mapped to the claim proposition and that the corresponding exact report strata and receipt identities are present and internally consistent.

The model report must still be interpreted by its owner under its approved metric profile. In particular, `MetricStratumStatus::WithinDeclaredProfile` means the report lies within the model owner's declared metric profile; it does not, by itself, prove the engineering predicate's numeric threshold has passed.

## 4. Non-compensatory joins

The adapter rejects any mismatch in:

1. claim / subject / configuration / operating envelope;
2. specification payload digest / typed semantic digest / decision context;
3. model-credibility cut digest / exact model metric ID;
4. measurand / unit / aggregation;
5. population / sampling unit / population manifest / quantifier;
6. operator / threshold / uncertainty treatment and method;
7. complete one-to-one claim-stratum ↔ model-stratum mapping;
8. per-stratum report population and status;
9. owner metric-semantic, model report, attestation, authority and semantic-validation receipt identities;
10. decision required-evidence manifest membership;
11. owner lifecycle, freshness and dependency coverage;
12. receiver-first-seen before the historical decision cutoff.

No number of positive reports on other metrics, strata or modalities compensates for one mismatch.

## 5. Ownership and trust boundary

This module binds owner evidence record IDs and their exact record digests from `OwnerEvidenceCutV0_1`. It requires the owner payload-verification disposition, active lifecycle, current freshness, complete dependency coverage and no pending re-verification for the selected records.

It deliberately sets `owner_authentication_performed=false`. It does not validate a producer's public-key signature, authority scope, key revocation, trust-policy history or actual execution. The referenced producer-authority and semantic-validation receipts must be verified by the existing owner-native trust/evidence boundary (Mycelix/AeroCommons and the model owner) before the composite gate can consume them. A caller-supplied enum or digest is not authentication.

Ownership remains:
- AeroWeft/model owner: metric definition, mapping basis, validation semantics, threshold evaluation and per-stratum report.
- Mycelix/AeroCommons: artifact bytes, owner records, packet custody, producer and authority receipts.
- BASALT: freshness, dependency completeness, observer coverage and invalidation.
- AERO: exact claim-context composition and fail-closed reason preservation.
- Independent engineering authority: decides whether the mapping and its physical basis are appropriate.

## 6. Integration boundary

The next gate integration should require, in one composite evaluation:
- `ClaimSpecificationV0_1`;
- raw specification bytes and their digest;
- `DecisionContextV0_1`;
- `ModelCredibilityCutV0_1`;
- `ClaimMetricSemanticBindingV0_1`;
- the admitted owner evidence/attestation and trust-verifier result.

The composite gate must reject or abstain if the binding is absent, unsupported, stale or inconsistent. It must retain all existing owner, observer, temporal, effectivity and model-applicability blockers. This adapter is a separately reviewable stage and does not silently change the existing gate input API.

## 7. Challenge programme

The companion corpus contains 25 proposed cases spanning correct mapping, wrong metric, unit/aggregation/population/quantifier/predicate/uncertainty substitutions, incomplete and adverse strata, stale or missing owner records, historical receiver time, effectivity, raw-versus-semantic digest substitution, reused receipt roles and coupled faults.

The corpus status is explicitly `NOT_INDEPENDENTLY_REVIEWED` and `NOT_EXECUTED`. CI validates the Draft 2020-12 schema and census only. Rust unit tests exercise this implementation's invariants; they are not an independent engineering oracle.

Next independent review:
1. AeroWeft/model owner authors metric definitions and exact report semantics.
2. Independent structural/material engineers adjudicate mapping adequacy and claim-to-metric equivalence.
3. A different implementation or hand-derived oracle checks raw cases without reusing Rust helpers or reason-code tables.
4. Development/exposed/blinded partitions are separated by root-cause family and engineering programme.
5. Report false binding acceptance, false rejection, incomplete/unverified/abstention, exact reason agreement and valid-claim retention separately.

## 8. Prospective benchmark targets

Add these to the existing AERO 17-metric registry (do not create a second registry):

- `model_metric_mapping_mismatch_acceptance_rate`: 0 observed critical mapping defects accepted.
- `claim_to_metric_semantic_oracle_agreement_rate`: 100% exact agreement with independently adjudicated mapping outcomes.
- `claim_metric_required_stratum_coverage_miss_rate`: 0 missing or substituted required strata accepted.
- `claim_metric_receipt_manifest_bypass_rate`: 0 required owner receipt references omitted from the decision evidence manifest.
- `claim_metric_stale_owner_record_acceptance_rate`: 0 stale/revoked/superseded selected owner records bound as current.

Do not calculate an aggregate metric-binding accuracy that allows hard-gate misses to be compensated by easy positive controls. Report valid-mapping retention to prevent reject-all.

## 9. Reproducibility and current verification status

- Typed Rust module and adversarial unit tests are registered in the existing `symtropy-aero-material-qual` package.
- Draft 2020-12 schema and synthetic shape fixture are provided.
- Challenge seed remains explicitly unreviewed/unexecuted.
- Exact-head formatting, compilation, tests and Clippy are pending hosted verification until run results attach to the new branch.
- No independent semantic review, real owner signature, physical model verification, physical qualification or regulatory approval is claimed.
