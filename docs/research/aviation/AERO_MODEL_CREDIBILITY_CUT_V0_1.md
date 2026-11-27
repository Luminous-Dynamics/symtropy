# AERO Model Credibility Cut v0.1

**Status:** typed research adapter to the existing AeroWeft model-credibility contract; pending exact-head CI and independent owner review  
**Consumes:** exact AeroWeft model/package/domain identities, validation modalities, data split declarations, independent-reference provenance, uncertainty component reports and per-stratum metrics  
**Purpose:** preserve claim-specific model-applicability and credibility evidence inside the same decision context as physical identity, time and owner evidence  
**Claim ceiling:** structural/evidence admissibility for independent review only; no model validation, physical truth, flight authorization or certification

## 1. Why a separate cut exists

The AERO decision context already commits to a model-applicability profile and a risk classification. A digest alone is insufficient for review: the claim evaluator must preserve what the model owner actually asserted about the exact model, solver, configuration and operating envelope, which validation modalities are required, whether confirmatory groups are independent, which uncertainty sources were characterized and which metrics/strata were reported.

`ModelCredibilityCutV0_1` is the typed adapter for AeroWeft's existing model-credibility dossier. It does not recreate the VVUQ calculation engine or invent a universal hierarchy of validation modalities.

## 2. Exact identity envelope

Every cut binds:

- claim ID and model evidence reference;
- model artifact digest and solver/integration configuration digest;
- exact aircraft/component configuration;
- exact operating-envelope profile;
- applicability-profile digest and risk-classification digest;
- validation package, protocol, calibration, development, confirmatory and independent-reference manifest digests.

The cut must agree with the claim's exact configuration, operating envelope, model-applicability digest and risk-classification digest before its outputs can support reviewability. A model build, solver, aircraft configuration, control/sensor stack or operating-domain change is not silently covered by a previous profile.

## 3. Validation modalities: no universal ladder

AeroWeft distinguishes:

1. synthetic known-truth;
2. analytic reference;
3. software-in-the-loop;
4. hardware-in-the-loop;
5. calibrated component bench;
6. controlled flight test;
7. shadow-mode field replay.

These are not a total ordering. Each required modality is declared by the owner for the specific claim and domain. The adapter verifies that every required modality has a qualified-for-declared-purpose evidence record. It does not let an abundance of synthetic cases compensate for a missing HIL/bench/flight modality when that modality is required, and it does not infer that flight testing automatically validates every component or metric.

Each modality carries its own evidence reference, report digest and scope-manifest digest. `Unqualified` or `NotAvailable` does not count as satisfied.

## 4. Independent data splits and reference measurements

The cut preserves exact calibration, development and confirmatory manifest digests, plus the independent-reference manifest digest.

The split owner must declare `DisjointByIndependentUnit`, with the independent unit chosen to prevent leakage across the relevant claim: e.g. physical aircraft/article, configuration, flight, production lot, route, environmental episode or campaign. A random-row split is not presumed independent.

- `LeakageDetected` is a rejection condition.
- `Unverified` split integrity blocks reviewability.
- An absent or unverified independent reference blocks reviewability.
- The adapter does not decide whether a particular reference method is adequate; that remains in the model/measurement protocol and independent technical review.

## 5. Uncertainty components remain distinct

The cut requires separate evidence for:
- observation noise;
- measurement uncertainty;
- parameter uncertainty;
- structural model discrepancy;
- environmental uncertainty.

For each component it commits to a method-profile digest, report digest and status (`Characterized`, `Uncharacterized`, `Unverified`). A characterized parameter uncertainty cannot compensate for uncharacterized model-form discrepancy. No weighted confidence score, pooled uncertainty value or average coverage score is computed by AERO.

The uncertainty report must retain the method, assumptions, units, dependence structure, coverage level and provenance in its owner-owned bytes. AERO only preserves identities and the declared disposition. Probabilistic intervals must not be silently promoted to hard deterministic time bounds.

## 6. Per-metric/per-stratum reporting, never one aggregate pass

Every required metric is paired with an explicit stratum (for example prediction-interval coverage by material, or out-of-domain recall by environmental regime). Each report binds:

- exact metric ID and stratum;
- population/grouping manifest digest;
- metric result report digest;
- uncertainty report digest;
- sample denominator and adverse count;
- status: within declared profile, outside declared profile, indeterminate or unverified.

A missing report, zero denominator, adverse count larger than the denominator, duplicate pair or out-of-profile required stratum is not averaged away by other strata. The adapter makes no statistical calculation from the counts; reviewers must inspect the owner report bytes and uncertainty method.

## 7. Fail-closed reviewability blockers

| Condition | AERO handling |
|---|---|
| Malformed ID/digest, duplicate required identity/pair or invalid count arithmetic | Reject the cut |
| Validation package incomplete/rejected | Incomplete/rejected, no reviewability |
| Confirmatory leakage | Reject |
| Split integrity unverified | Unverified |
| Independent reference absent/unverified | Unverified |
| Model claim outside validated domain | Out of scope |
| Domain coverage indeterminate/unverified | Incomplete/unverified |
| Required validation modality not qualified for its declared purpose | Unverified |
| Required uncertainty component uncharacterized/unverified | Unverified |
| Required metric/stratum absent | Incomplete |
| Metric/stratum outside declared profile | Out of scope / no performance credit |
| Metric/stratum indeterminate or unverified | Incomplete / unverified |

The adapter returns a stable sorted set of blockers. It never combines RMSE, throughput, latency, energy, coverage or safety outcomes into a scalar score. A failed containment, missed event, unsupported stratum or unverified validation obligation cannot be offset by a better average.

## 8. Canonical commitment

The owner cut uses a versioned Rust/Serde field-order profile:
- schema: `AeroModelCredibilityCutV0_1`;
- canonicalization profile: `rust-serde-json-struct-order-v1`;
- enum arrays sorted by their declared Rust enum order;
- modality records sorted by modality order then evidence reference;
- uncertainty records sorted by component order;
- required metric/stratum pairs and reports sorted lexicographically by metric ID and stratum;
- SHA-256 over compact UTF-8 JSON.

This is not a general RFC 8785/JCS implementation. A Python workflow independently validates the schema and reproduces the example digest. Cross-language production consumers must reproduce the exact profile or adopt a new version with byte-level vectors.

The checked-in example is a **synthetic structural control**, not actual AeroWeft validation evidence. Its expected SHA-256 is:

`970cae8ea26f237e528952ec458049e97d8992a7f7f055c89c415fd3814ccf2e`

## 9. Integration and ownership boundaries

AERO claim gate should consume the cut alongside the owner evidence cut. The model-cut evidence reference and its computed digest must be present in the owner cut; the model cut's applicability-profile and risk digests must match the typed decision context; claim/configuration/envelope identities must match the current claim.

- **AeroWeft / model credibility owner:** VVUQ calculations, validation protocol, independent reference, model applicability domain, evidence modalities and per-stratum metrics.
- **Mycelix/AeroCommons:** physical unit/configuration, artifact bytes/digests, evidence records/lifecycle and source provenance.
- **BASALT:** resource generations, evidence dependency freshness, invalidation and re-verification.
- **Owner trust/receipt service:** signatures, receiver-first-seen, clocks, timestamp tokens, trust roots and policy history.
- **AERO:** typed commitment and non-compensatory composition.
- **Symthaea:** evidence-action selection and EVSI, not physical authority.
- **Independent engineering authority:** physical acceptance, maintenance and certification.

AERO does not calculate or certify model validity. The highest result remains reviewability of the *submitted package under supplied dispositions*, not proof that the model predicts reality.

## 10. Benchmark programme and prospective targets

The adapter should be benchmarked against a minimal aggregate-PASS baseline and a weighted-score baseline that can compensate for missing obligations. The matched confirmatory inputs and independent oracle must be frozen first.

| Metric | Prospective target |
|---|---|
| Required modality omission detection | 100% of missing/unqualified owner-required modalities block reviewability |
| Confirmatory leakage detection | 100% of known leakage mutations rejected; no row-level random split treated as independent by default |
| Required uncertainty completeness | 100% of required components separately represented; no cross-component compensation |
| Per-stratum completeness | 100% required metric/stratum pairs present with population and uncertainty digests |
| Unsupported-domain handling | 100% NOT_COVERED / OUT_OF_SCOPE or INDETERMINATE when evidence does not cover the claim domain |
| Cross-claim/configuration/model substitution | 0 substitutions accepted |
| False-review rate | 0 on a frozen independently adjudicated hard-gate corpus; publish N and finite-sample bound |
| Order invariance | Exact digest and blockers under every permutation of owner-evidence array ordering |
| Runtime | p50/p95/p99 and allocations for 1/10/100/1k/10k strata and 1/10/50/100 modality records |
| Reproducibility | Independent evaluator rebuilds every digest and blocker from retained report bytes |

Zero observed failures is not zero risk. Report sample/exposure denominator, per-stratum intervals and confidence bounds. Mutations generated by one script do not constitute independent assurance cases.

## 11. Industry comparison

AeroWeft is the model-credibility owner and should map its evidence to established aerospace and computational-model credibility disciplines:

- NASA-STD-7009B / NASA-HDBK-7009B: model and simulation credibility over lifecycle, uncertainty, sensitivity, acceptance criteria and delegated technical authority.
- NIST AMS 100-85 (2026): risk/influence/consequence-scaled VVUQ.
- IAQG/SAE AS9100D and the 9104 certification scheme: controlled QMS and independent certification/audit environment.
- ISO 23247-5:2026: manufacturing digital-thread interoperability.
- ASTM/FAA methods: method-specific POD and aircraft damage-tolerance/inspection obligations.

AERO's differentiator is not a new physics solver or competing VVUQ methodology; it is preserving model credibility as one non-compensatory, claim-specific evidence cut across the owner digital thread, temporal history and physical article. No matched SOTA superiority result is claimed.

## 12. Promotion gates

1. Independent AeroWeft owner review of the typed fields, modality semantics, split policy and per-stratum obligations.
2. Exact integration with AeroWeft's owner validation package and bitemporal requalification change-impact output.
3. Owner record in Mycelix/AeroCommons whose digest matches the model-cut commitment.
4. Typed model cut matches DecisionContext and exact claim configuration/domain.
5. Frozen mutation oracle authored independently; exact-head Rust/schema/Clippy checks pass.
6. Rights-cleared external or physical dataset, qualified independent reference and preregistered method-specific experiment before any physical validation claim.
