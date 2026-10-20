# Animal Decision Qualification by Model-Set Adequacy V1

Status: draft integration contract  
Tracking issue: [ANIMAL-DECISION-SCOPE-001 #2350](https://github.com/Luminous-Dynamics/symtropy/issues/2350)  
Depends on the decision-risk kernel in [#2247](https://github.com/Luminous-Dynamics/symtropy/pull/2247) and the blind challenge contract in [#2335](https://github.com/Luminous-Dynamics/symtropy/pull/2335).

## Why this boundary exists

The decision-risk kernel computes expected loss, worst-case loss, CVaR, per-model regret, admissible actions and sensitivity to model weights for a **declared model set**. Correct arithmetic over that set cannot reveal a missing trophic interaction, delayed response, threshold, movement process, sensor bias or another omitted mechanism.

The model-set-adequacy benchmark tests a separate question with blinded challenger families. This integration makes the boundary machine-readable: a report may preserve its numerical risk summary while refusing to label its decision as qualified outside an explicit, matched challenge scope.

This follows the warning of Malchow and Hartig's 2026 ecological modelling review that poorly designed multi-model ensembles can create false confidence, and the 2026 uncertainty-analysis review's recommendation to identify, propagate, attribute and communicate uncertainties rather than collapse them into one number. Neither paper establishes performance of this proposed Symtropy interface:
- https://doi.org/10.1111/gcb.70987
- https://doi.org/10.1111/ele.70375

## Separate four claims

1. `risk_summary_computed`: arithmetic over submitted model outcomes passed its current input checks.
2. `adequacy_receipt_present`: a typed assessment receipt was supplied and its required fields/identity digests parsed.
3. `within_scope`: receipt disposition is positive and the receipt's model-registry and challenger-family digests match the expected digests in the decision input.
4. **Empirical/biological validity**: not established by any status in this interface.

Never translate a positive status into universal model completeness or verified biological truth. Finite challenge families provide bounded omission sensitivity, not proof that every plausible mechanism was considered.

## Input contract

The decision outcomes JSON may include two optional lowercase SHA-256 digests:

- `model_set_registry_sha256`: expected digest for the exact candidate model registry under assessment.
- `required_challenge_family_manifest_sha256`: expected digest for the blind challenge-family manifest required by this decision scope.

If either expected digest is absent, qualification remains `not_assessed`, even when a positive receipt is present. If either digest disagrees with the receipt, qualification becomes `blocked_unresolved`. Malformed supplied digest fields reject the input.

An optional second CLI argument supplies a JSON assessment receipt:

```sh
cargo run -p symtropy-animal-model-uncertainty --bin score_animal_model_ensemble -- outcomes.json
cargo run -p symtropy-animal-model-uncertainty --bin score_animal_model_ensemble -- outcomes.json adequacy-receipt.json
```

With no receipt, numeric risk scoring still runs and the report explicitly states `not_assessed`. A malformed receipt is rejected; it is not silently downgraded.

## Assessment receipt V1

Schema: [ANIMAL_MODEL_SET_ADEQUACY_ASSESSMENT_RECEIPT_V1.schema.json](ANIMAL_MODEL_SET_ADEQUACY_ASSESSMENT_RECEIPT_V1.schema.json)

The typed receipt binds the assessment disposition to:
- exact candidate model-set registry digest;
- challenger-family manifest digest;
- evaluator source commit and version;
- independent-unit manifest;
- pre-reveal prediction digest;
- assessment ID, bounded scope, challenge-case count and independent-unit count.

The CLI hashes the exact supplied receipt bytes with SHA-256 and preserves that digest in the report. It rejects unknown fields, missing values, malformed hashes/commit IDs, non-positive sample counts and unrecognized dispositions. The receipt is not an untyped label: it is a fixed-shape object tied to the run's declared scope.

### Important provenance limitation

A SHA-256 digest binds bytes; it does **not** authenticate the issuer. This V1 CLI validates receipt structure and the two scope-digest matches, but does not verify a digital signature, GitHub artifact attestation or trusted-run provenance. The producer and verifier must come from a trusted assessment pipeline before relying on `within_scope` for consequential action. Until that attestation layer and a runtime receipt emitter exist, this is a qualification-reporting boundary—not an execution authority or biological certification.

## Disposition mapping

| Receipt disposition | Decision qualification | Interpretation |
|---|---|---|
| no receipt | `not_assessed` | risk scores are conditional on submitted models; no adequacy evidence supplied |
| `qualified_within_challenge_scope` with both digests matching | `within_scope` | qualifies only the declared registry and challenge family |
| `omission_detected` | `blocked_by_omission` | challenge detected a decision-relevant omission |
| `unresolved` | `blocked_unresolved` | evidence does not resolve adequacy |
| `incomplete` | `blocked_incomplete` | required challenge evidence was incomplete |
| any scope digest mismatch | `blocked_unresolved` | receipt is for a different registry or challenge family |
| expected scope digest missing | `not_assessed` | the report cannot establish identity match |

The report includes reason codes and always says that empirical biology was not validated. The qualification envelope does not modify the kernel's numerical losses, CVaR, regrets, weight bounds or action-preference ranges.

## Negative controls

The Rust tests exercise:
- missing receipt;
- positive receipt with matching digests;
- candidate registry and challenge manifest mismatches;
- missing expected identity;
- omission-detected, unresolved and incomplete dispositions;
- malformed digest and unexpected fields;
- digest binding to exact receipt bytes;
- frozen receipt schema contract and all disposition values;
- invalid optional expected digests at decision-input parse time.

## Non-goals

- No global “animal realism” scalar.
- No claim of universal model completeness.
- No implicit authority to actuate an agent or manage wildlife.
- No cryptographic issuer authentication in this V1 interface.
- No ecological runtime, empirical data ingestion or receipt generation in this patch.
