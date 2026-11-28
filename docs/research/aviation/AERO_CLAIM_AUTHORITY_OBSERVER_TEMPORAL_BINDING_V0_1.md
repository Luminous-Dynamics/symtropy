# AERO Claim Authority, Observer and Temporal Binding v0.1

**Status:** synthetic source-binding adapter; exact-head compile, tests and independent oracle pending  
**Authority ceiling:** claim-level authority, observer-coverage and temporal-knowledge binding only; not a complete claim decision or physical/engineering authority  
**Relationship:** additive after claim-specification and evidence-class source binding; it does not replace the ClaimGate.

## 1. Purpose

Evidence-class binding establishes that the claim's required evidence classes are linked to owner-native records and that every source receipt was known by the selected cutoff. It does not, by itself, bind the claim's designated acceptance-authority role or prove that observer coverage applies to the exact acquisition/source scope.

This increment binds five derived obligations from the closed claim profile:
- claim identity;
- semantic validity;
- claim-level producer/acceptance authority;
- dependency observation;
- temporal knowledge.

Claim identity and semantic validity inherit only the exact upstream typed claim-specification/raw-payload/context binding result. The other three obligations require their own source-owner receipts.

## 2. Claim-level authority receipt

The authority receipt must match:
- the claim's exact `acceptance_authority_role_ref`;
- claim, physical subject and configuration;
- decision-context digest;
- trust-policy snapshot;
- source-scope manifest;
- receiver-first-seen time and half-open effectivity interval.

This is distinct from the per-record producer authorization used by the evidence-class adapter. Per-record authorization establishes that a particular producer may issue a record under a policy. The claim-level receipt identifies the authority role expected to make the claim disposition.

A role string is not a grant: the receipt's `VERIFIED` status remains an upstream trust-verifier disposition. This module does not authenticate its signature, certificate, delegation chain or revocation state.

## 3. Observer-coverage receipt

The observer receipt binds a typed `CoverageAssessment` to:
- claim, subject, configuration and decision context;
- exact physical acquisition interval;
- source-scope manifest;
- observer profile and owner-issued observer receipt already committed in the owner evidence cut;
- exact observer assessment report digest;
- coverage status and replay consequence;
- source-roster, required-capture-obligation and critical-channel manifest digests;
- observer watermark, receiver-first-seen time and effectivity.

The report digest must equal the exact `AssessmentBinding.report_sha256`; the profile, owner receipt and watermark must equal the owner evidence cut. The observer watermark cannot be later than the owner receipt watermark or selected historical cutoff.

An incomplete, unverified, invalid or out-of-scope coverage result remains so. It cannot be interpreted as evidence of absence. In particular, a missing observer receipt is incomplete, not a license to infer that an event or dependency did not occur.

## 4. Temporal-knowledge receipt

The temporal snapshot receipt binds:
- owner namespace, claim and configuration;
- exact decision context and trust-policy snapshot;
- exact owner evidence-cut digest;
- the recomputed complete per-record source-receipt manifest;
- the owner cut's receiver receipt manifest and watermark;
- the selected historical `as_known_at` cutoff;
- receiver-first-seen chronology and half-open effectivity.

The receipt-manifest digest is recomputed from the original owner source receipts. The temporal receipt must agree with both this digest and the owner cut's declared receipt manifest. A late receipt cannot retroactively alter a historical decision cut.

## 5. Dispositions

Each obligation is returned independently as `BOUND`, `INCOMPLETE`, `UNVERIFIED`, `OUT_OF_SCOPE` or `REJECTED`. The adapter's aggregate status is deliberately `INCOMPLETE` when other derived obligations remain unassessed.

It always returns:
- `complete_claim_assurance_state = false`;
- `assurance_state_mutated = false`;
- `physical_truth_established = false`;
- `engineering_acceptance_granted = false`;
- `physical_work_authorized = false`.

## 6. What remains outside this adapter

The adapter does not bind model applicability/credibility, population strata, uncertainty or assumption/exclusion defeaters. Those remain with the separate model-obligation adapter and future defeater-resolution source adapters. It does not alter or replace the existing `evaluate_claim_gate`.

The complete assurance migration remains blocked until all derived obligation classes have source-bound evaluators and a differential parity suite demonstrates exact dispositions and reason-code preservation against the existing ClaimGate over the full frozen corpus, including adversarial omissions, conflicts, wrong owners, wrong configurations, late receipts and out-of-domain models.

## 7. Tests

Rust tests cover exact claim identity and semantic binding inherited from the raw/typed claim binding; authority receipt role, owner, scope and historical chronology; observer report digest and owner observer receipt/profile binding; observer scope, acquisition interval and watermark; temporal receipt source-manifest, owner-cut and receiver-watermark binding; missing observer receipt remaining incomplete; out-of-scope coverage remaining out of scope; and late/failed/unverified receipt dispositions.

The CI-only Python oracle independently checks the receipt relationships and re-runs the evidence-class oracle on the embedded input. These synthetic tests do not authenticate signatures or establish physical truth.
