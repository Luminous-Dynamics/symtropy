# Risk-aware aerospace claim gate v0.1

## Purpose

Compose two existing policies into one replayable decision:

1. The claim-admission policy declares evidence required for the specific prediction/use class and consequence.
2. The model-credibility policy declares additional assurance controls from model creation type, logic transparency, artifact accessibility, model influence and decision consequence.

The new gate binds both policies, every assertion about a credibility control, the base claim decision and the combined decision digest to one exact scope hash.

This is a Luminous project policy. It is not an ASME or NIST formula, regulatory classification, certification basis, or flight authorization.

## Consequence mapping

The gate requires an explicit mapping between the two existing consequence vocabularies:

| Claim-admission consequence | Model-credibility consequence |
|---|---|
| `exploratory` | `limited` |
| `engineering` | `significant` |
| `high` | `severe` |
| `safety_critical` | `severe` |

A mismatched pair is rejected rather than silently mapped. The model influence dimension (advisory/material/controlling) remains separate and can elevate the credibility tier.

## Policy requirements versus evidence

A required control is an assertion about an assurance activity or artifact that can be evidenced: code verification, grid/time convergence, model-form sensitivity, held-out validation, independent reproduction, third-party artifact audit, etc.

Two statements are not evidence artifacts and are not required to be marked `Verified`:

- This assessment does not authorize flight, manufacturing or operational release.
- Outputs remain screening/requirements-only, not released designs.

Those are permanent policy restrictions, included in the credibility-assessment digest and exported result. They cannot be satisfied by attaching an arbitrary file.

## Control assertion model

Every policy-generated evidence control has one exact-scope assertion:

- `Verified`: artifact digest, verifier identity, verifier executable digest and verifier receipt digest.
- `Unknown`: an explicit reason; this causes abstention where required.
- `Failed`: an explicit reason; this rejects the evidence set.
- Missing assertion: causes abstention.
- Scope mismatch: rejects the evidence set.

Assertions for control IDs not required by the evaluated policy are rejected. Duplicate IDs are rejected.

The decision records the risk assessment digest, the canonical control-manifest digest, per-control results, the base claim decision digest and the final decision digest. Replaying the original request recomputes both policies and every assertion.

## Decision outcomes

| Outcome | Meaning |
|---|---|
| `admitted_within_declared_use` | Base claim policy and all risk-credibility controls pass for this exact scope; does not authorize release |
| `abstain_missing_or_unknown_controls` | At least one required control is missing or unknown |
| `rejected_failed_or_mismatched_controls` | A required control failed or belongs to another scope |
| `blocked_decisive_artifacts_unavailable` | The decisive source artifacts cannot be independently accessed |
| `requires_human_authority_decision` | Evidence is complete but claim/consequence or Critical tier requires external authority |

A `Critical` credibility tier, `FlightRelease` claim, or safety-critical base claim never becomes automatic authorization even when all supplied controls are marked verified.

For restricted artifact access, the `approved_third_party_artifact_audit` control is mandatory and the export keeps the restriction visible: no unrestricted reproducibility claim is made.

For unavailable artifacts, the gate blocks admission regardless of caller-supplied `Verified` labels.

## Trust boundary and limitations

**Verifier authentication is not implemented.** A `Verified` assertion is currently structurally validated and bound to verifier identifiers/digests. The gate does not verify those identities against a trusted registry, check revocation/expiry, validate certificate chains, re-run artifact verifiers, or authenticate signatures. The base claim-admission module has the same limitation.

The next security layer must separately establish:

1. Versioned verifier trust registry with immutable registry digest.
2. Key/certificate identity and signature verification.
3. Revocation and validity-window checks at the assertion's evidence time.
4. Role separation and verifier authorization for each evidence kind.
5. Signed verifier receipts binding artifact digest, scope, verifier build, policy version and outcome.
6. Independent trust-policy replay and negative controls for revoked, expired, unauthorized and substituted verifiers.

Until that layer exists, the gate's evidence level is structural reference binding, not authenticated verification.

## Tests in v0.1

Synthetic known-answer tests cover:
- Complete low-consequence claim plus risk controls.
- Missing/unknown control abstention.
- Failed and mismatched-scope control rejection.
- Unavailable decisive artifacts.
- Restricted artifacts requiring an approved third-party audit and preserving the no-unrestricted-reproducibility restriction.
- Malformed verifier references.
- Critical-tier human-authority requirement.
- Claim/credibility consequence mismatch.
- Replay rejection after control mutation.
- Policy restrictions not being misrepresented as artifact-verifiable controls.

All fixtures are synthetic. They do not establish a completed Symtropy V&V programme or any aircraft prediction's validity.

## Benchmarks and advancement

The immediate software benchmark is zero false admissions across frozen adversarial controls, including duplicate IDs, wrong scope, missing/unknown/failed assertions, unavailable artifacts, mismatched consequences, altered digests, expired/revoked/unauthorized verifier identities (once a trust registry is implemented), and human-authority gates.

The next research benchmark is *risk-appropriate assurance cost*: compare a fixed flat checklist with the influence/consequence policy on identical claims and evidence sets. Report (a) any false admissions, (b) missing controls detected, (c) unnecessary controls added, (d) review effort/cost, and (e) whether the policy's requirements change monotonically as influence or consequence increases. This is a policy-efficiency benchmark, not evidence that the resulting model is scientifically valid.
